fn decode_utf16_units(units: Vec<u16>, has_trailing_byte: bool) -> String {
    let mut decoded: String = char::decode_utf16(units)
        .map(|item| item.unwrap_or(REPLACEMENT_CHAR))
        .collect();
    if has_trailing_byte {
        decoded.push(REPLACEMENT_CHAR);
    }
    decoded
}
fn decode_utf16_bytes(bytes: &[u8], endian: Utf16Endian) -> String {
    let mut chunks = bytes.chunks_exact(2);
    let mut units = Vec::with_capacity(bytes.len() / 2);
    for chunk in &mut chunks {
        let pair = [chunk[0], chunk[1]];
        let unit = match endian {
            Utf16Endian::Big => u16::from_be_bytes(pair),
            Utf16Endian::Little => u16::from_le_bytes(pair),
        };
        units.push(unit);
    }
    decode_utf16_units(units, !chunks.remainder().is_empty())
}
fn decode_string_with_charset(bytes: &[u8], charset: StandardCharset) -> String {
    match charset {
        StandardCharset::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        StandardCharset::UsAscii => {
            bytes
                .iter()
                .map(|byte| {
                    if *byte <= 0x7f { char::from(*byte) } else { REPLACEMENT_CHAR }
                })
                .collect()
        }
        StandardCharset::Iso88591 => bytes.iter().map(|byte| char::from(*byte)).collect(),
        StandardCharset::Utf16 => {
            match (
                bytes.strip_prefix(&[0xfe, 0xff]),
                bytes.strip_prefix(&[0xff, 0xfe]),
            ) {
                (Some(rest), _) => decode_utf16_bytes(rest, Utf16Endian::Big),
                (None, Some(rest)) => decode_utf16_bytes(rest, Utf16Endian::Little),
                (None, None) => decode_utf16_bytes(bytes, Utf16Endian::Big),
            }
        }
        StandardCharset::Utf16Be => decode_utf16_bytes(bytes, Utf16Endian::Big),
        StandardCharset::Utf16Le => decode_utf16_bytes(bytes, Utf16Endian::Little),
    }
}
const fn decode_pct_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
fn decode_base64(input: &[u8], variant: Base64Variant) -> Result<Vec<u8>> {
    let bytes = filtered_base64_input(input, variant);
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let mut first_padding = None;
    for (idx, byte) in bytes.iter().copied().enumerate() {
        if byte == b'=' {
            first_padding.get_or_insert(idx);
        } else if first_padding.is_some() || base64_decode_value(byte, variant).is_none()
        {
            return Err(invalid_base64_error());
        }
    }
    let data_len = first_padding.unwrap_or(bytes.len());
    if let Some(first_padding_idx) = first_padding {
        let pad_count = bytes.len() - first_padding_idx;
        let data_remainder = data_len % 4;
        if pad_count > 2 || !bytes.len().is_multiple_of(4)
            || (pad_count == 1 && data_remainder != 3)
            || (pad_count == 2 && data_remainder != 2)
        {
            return Err(invalid_base64_error());
        }
    } else if data_len % 4 == 1 {
        return Err(invalid_base64_error());
    }
    let mut out = Vec::with_capacity((data_len / 4) * 3 + 2);
    let mut idx = 0;
    while idx + 4 <= data_len {
        let values = [
            require_base64_value(bytes[idx], variant)?,
            require_base64_value(bytes[idx + 1], variant)?,
            require_base64_value(bytes[idx + 2], variant)?,
            require_base64_value(bytes[idx + 3], variant)?,
        ];
        push_base64_triplet(&mut out, values);
        idx += 4;
    }
    match data_len - idx {
        0 => {}
        2 => {
            let first = require_base64_value(bytes[idx], variant)?;
            let second = require_base64_value(bytes[idx + 1], variant)?;
            out.push((first << 2) | (second >> 4));
        }
        3 => {
            let first = require_base64_value(bytes[idx], variant)?;
            let second = require_base64_value(bytes[idx + 1], variant)?;
            let third = require_base64_value(bytes[idx + 2], variant)?;
            out.push((first << 2) | (second >> 4));
            out.push(((second & 0x0f) << 4) | (third >> 2));
        }
        _ => return Err(invalid_base64_error()),
    }
    Ok(out)
}
