fn push_pending_java_exception_message(class_name: &str, message: String) {
    let mut messages = pending_java_exception_messages()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    messages.entry(class_name.to_string()).or_default().push_back(message);
}
fn push_pending_java_exception_cause(class_name: &str, cause: Slot) {
    let mut causes = pending_java_exception_causes()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    causes.entry(class_name.to_string()).or_default().push_back(cause);
}
fn push_base64_triplet(out: &mut Vec<u8>, values: [u8; 4]) {
    out.push((values[0] << 2) | (values[1] >> 4));
    out.push(((values[1] & 0x0f) << 4) | (values[2] >> 2));
    out.push(((values[2] & 0x03) << 6) | values[3]);
}
fn push_u16_escape(out: &mut String, unit: u16) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push('\\');
    out.push('u');
    for shift in [12, 8, 4, 0] {
        let idx = usize::from((unit >> shift) & 0x000f);
        out.push(char::from(HEX[idx]));
    }
}
fn push_unicode_escape(out: &mut String, ch: char) {
    let mut encoded = [0_u16; 2];
    for unit in ch.encode_utf16(&mut encoded).iter().copied() {
        push_u16_escape(out, unit);
    }
}
fn push_escaped_key_char(out: &mut String, ch: char) {
    match ch {
        '\\' => out.push_str("\\\\"),
        '=' => out.push_str("\\="),
        ':' => out.push_str("\\:"),
        ' ' => out.push_str("\\ "),
        '#' => out.push_str("\\#"),
        '!' => out.push_str("\\!"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        '\u{000c}' => out.push_str("\\f"),
        _ if !ch.is_ascii() || u32::from(ch) < 0x20 || u32::from(ch) > 0x7e => {
            push_unicode_escape(out, ch);
        }
        _ => out.push(ch),
    }
}
