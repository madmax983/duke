fn parse_bounded_i32_from_string_arg(
    args: &[Slot],
    heap: &duke_gc::Heap,
    min: i32,
    max: i32,
) -> Result<i32> {
    let s = extract_string_arg_value(args, 0, heap)?;
    parse_bounded_i32_with_radix(&s, 10, min, max)
}
fn parse_bounded_i32_from_string_and_radix_args(
    args: &[Slot],
    heap: &duke_gc::Heap,
    min: i32,
    max: i32,
) -> Result<i32> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let radix = extract_parse_radix_arg(args, 1)?;
    parse_bounded_i32_with_radix(&s, radix, min, max)
}
fn parse_bounded_i32_with_radix(s: &str, radix: u32, min: i32, max: i32) -> Result<i32> {
    let val = i32::from_str_radix(s.trim(), radix)
        .map_err(|_| Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        })?;
    if val < min || val > max {
        return Err(Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    Ok(val)
}
fn parse_i64_from_string_arg(args: &[Slot], heap: &duke_gc::Heap) -> Result<i64> {
    let s = extract_string_arg_value(args, 0, heap)?;
    parse_i64_with_radix(&s, 10)
}
fn parse_i64_from_string_and_radix_args(
    args: &[Slot],
    heap: &duke_gc::Heap,
) -> Result<i64> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let radix = extract_parse_radix_arg(args, 1)?;
    parse_i64_with_radix(&s, radix)
}
fn parse_i64_with_radix(s: &str, radix: u32) -> Result<i64> {
    i64::from_str_radix(s.trim(), radix)
        .map_err(|_| Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        })
}
fn parse_i32_decode_from_string_arg(args: &[Slot], heap: &duke_gc::Heap) -> Result<i32> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let val = parse_i128_decode(&s)?;
    if val < i128::from(i32::MIN) || val > i128::from(i32::MAX) {
        return Err(Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    i32::try_from(val)
        .map_err(|_| Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        })
}
fn parse_i64_decode_from_string_arg(args: &[Slot], heap: &duke_gc::Heap) -> Result<i64> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let val = parse_i128_decode(&s)?;
    if val < i128::from(i64::MIN) || val > i128::from(i64::MAX) {
        return Err(Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    i64::try_from(val)
        .map_err(|_| Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        })
}
fn parse_i128_decode(s: &str) -> Result<i128> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    let (negative, sign_len) = match trimmed.as_bytes().first() {
        Some(b'+') => (false, 1),
        Some(b'-') => (true, 1),
        _ => (false, 0),
    };
    let rest = &trimmed[sign_len..];
    let (radix, prefix_len) = if rest.starts_with("0x") || rest.starts_with("0X") {
        (16, 2)
    } else if rest.starts_with('#') {
        (16, 1)
    } else if rest.starts_with('0') && rest.len() > 1 {
        (8, 1)
    } else {
        (10, 0)
    };
    let digits = &rest[prefix_len..];
    if digits.is_empty() || digits.starts_with('+') || digits.starts_with('-') {
        return Err(Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    let magnitude = i128::from_str_radix(digits, radix)
        .map_err(|_| Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        })?;
    Ok(if negative { -magnitude } else { magnitude })
}
/// Count argument slots in a JVM method descriptor like `(ILjava/lang/String;[I)V`.
fn parse_arg_count(descriptor: &str) -> usize {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut count = 0;
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => count += 1,
            '[' => {
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
                count += 1;
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                count += 1;
            }
            _ => {}
        }
    }
    count
}
/// Parse argument type descriptors from a JVM method descriptor like `(IZLjava/lang/String;)V`.
/// Returns a Vec of single-char type codes: 'I', 'Z', 'L' (for object refs), '[' (for arrays), etc.
fn parse_arg_types(descriptor: &str) -> Vec<char> {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut types = Vec::with_capacity(params.len());
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => types.push(c),
            '[' => {
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
                types.push('[');
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                types.push('L');
            }
            _ => {}
        }
    }
    types
}
fn parse_arg_descriptors(descriptor: &str) -> Vec<String> {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut descriptors = Vec::with_capacity(params.len());
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => {
                descriptors.push(c.to_string());
            }
            '[' => {
                let mut desc = String::from("[");
                while chars.peek() == Some(&'[') {
                    desc.push(chars.next().unwrap_or('['));
                }
                if chars.peek() == Some(&'L') {
                    desc.push(chars.next().unwrap_or('L'));
                    for c2 in chars.by_ref() {
                        desc.push(c2);
                        if c2 == ';' {
                            break;
                        }
                    }
                } else if let Some(elem) = chars.next() {
                    desc.push(elem);
                }
                descriptors.push(desc);
            }
            'L' => {
                let mut desc = String::from("L");
                for c2 in chars.by_ref() {
                    desc.push(c2);
                    if c2 == ';' {
                        break;
                    }
                }
                descriptors.push(desc);
            }
            _ => {}
        }
    }
    descriptors
}
fn parse_service_provider_names(files: Vec<Vec<u8>>) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for bytes in files {
        let text = String::from_utf8(bytes)
            .map_err(|_| Error::JavaException {
                class_name: "java/util/ServiceConfigurationError".to_string(),
            })?;
        for line in text.lines() {
            let live = line.split_once('#').map_or(line, |(before, _)| before).trim();
            if !live.is_empty() {
                names.push(live.to_string());
            }
        }
    }
    Ok(names)
}
fn parse_uuid_hex_u64(bytes: &[u8]) -> Result<u64> {
    let mut value = 0_u64;
    for byte in bytes {
        let nibble = uuid_hex_nibble(*byte).ok_or_else(uuid_invalid_format)?;
        value = (value << 4) | nibble;
    }
    Ok(value)
}
fn parse_uuid_string(text: &str) -> Result<(i64, i64)> {
    let bytes = text.as_bytes();
    if bytes.len() != 36 || bytes[8] != b'-' || bytes[13] != b'-' || bytes[18] != b'-'
        || bytes[23] != b'-'
    {
        return Err(uuid_invalid_format());
    }
    let msb = (parse_uuid_hex_u64(&bytes[0..8])? << 32)
        | (parse_uuid_hex_u64(&bytes[9..13])? << 16)
        | parse_uuid_hex_u64(&bytes[14..18])?;
    let lsb = (parse_uuid_hex_u64(&bytes[19..23])? << 48)
        | parse_uuid_hex_u64(&bytes[24..36])?;
    Ok((msb.cast_signed(), lsb.cast_signed()))
}
fn parse_property_logical_line(line: &str) -> Result<Option<(String, String)>> {
    let chars: Vec<char> = line.chars().collect();
    let mut idx = 0usize;
    while idx < chars.len() && is_properties_whitespace(chars[idx]) {
        idx += 1;
    }
    if idx >= chars.len() || matches!(chars[idx], '#' | '!') {
        return Ok(None);
    }
    let key_start = idx;
    let mut escaped = false;
    while idx < chars.len() {
        let ch = chars[idx];
        if escaped {
            escaped = false;
            idx += 1;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            idx += 1;
            continue;
        }
        if matches!(ch, '=' | ':') || is_properties_whitespace(ch) {
            break;
        }
        idx += 1;
    }
    let key_end = idx;
    let value_start = if idx < chars.len() {
        if is_properties_whitespace(chars[idx]) {
            while idx < chars.len() && is_properties_whitespace(chars[idx]) {
                idx += 1;
            }
            if idx < chars.len() && matches!(chars[idx], '=' | ':') {
                idx += 1;
            }
        } else {
            idx += 1;
        }
        while idx < chars.len() && is_properties_whitespace(chars[idx]) {
            idx += 1;
        }
        idx
    } else {
        chars.len()
    };
    let raw_key: String = chars[key_start..key_end].iter().collect();
    let raw_value: String = chars[value_start..].iter().collect();
    Ok(Some((unescape_property_text(&raw_key)?, unescape_property_text(&raw_value)?)))
}
fn parse_properties_bytes(bytes: &[u8]) -> Result<Vec<(String, String)>> {
    let text: String = bytes.iter().map(|byte| char::from(*byte)).collect();
    let mut entries = Vec::new();
    for line in logical_properties_lines(&text) {
        if let Some((key, value)) = parse_property_logical_line(&line)? {
            entries.push((key, value));
        }
    }
    Ok(entries)
}
fn parse_fraction_to_nanos(fraction: &str) -> Option<i32> {
    if fraction.is_empty() || fraction.len() > 9
        || !fraction.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    let digits: i32 = fraction.parse().ok()?;
    let scale = 10_i32.pow(u32::try_from(9 - fraction.len()).ok()?);
    Some(digits.saturating_mul(scale))
}
fn parse_iso_local_date_components(text: &str) -> Option<(i32, u32, u32)> {
    let mut parts = text.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || day == 0
        || day > days_in_month(year, month)
    {
        return None;
    }
    Some((year, month, day))
}
fn parse_iso_time_components(text: &str) -> Option<(i32, i32, i32, i32)> {
    let (clock_part, nanos) = match text.split_once('.') {
        Some((clock, fraction)) => (clock, parse_fraction_to_nanos(fraction)?),
        None => (text, 0),
    };
    let mut parts = clock_part.split(':');
    let hour: i32 = parts.next()?.parse().ok()?;
    let minute: i32 = parts.next()?.parse().ok()?;
    let second: i32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(0..=23).contains(&hour) || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }
    Some((hour, minute, second, nanos))
}
fn parse_iso_localdatetime_components(
    text: &str,
) -> Option<(i32, u32, u32, i32, i32, i32, i32)> {
    let (date_part, time_part) = text.split_once('T')?;
    let (year, month, day) = parse_iso_local_date_components(date_part)?;
    let (hour, minute, second, nanos) = parse_iso_time_components(time_part)?;
    Some((year, month, day, hour, minute, second, nanos))
}
fn parse_iso_instant_components(text: &str) -> Option<(i64, i32)> {
    let body = text.strip_suffix('Z')?;
    let (year, month, day, hour, minute, second, nanos) = parse_iso_localdatetime_components(
        body,
    )?;
    let epoch_day = i64::from(ymd_to_epoch_days(year, month, day));
    let epoch_seconds = epoch_day * SECONDS_PER_DAY_I64
        + i64::from(hour * 3600 + minute * 60 + second);
    Some((epoch_seconds, nanos))
}
