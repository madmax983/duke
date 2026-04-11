#[cfg(test)]
mod tests {
    use crate::parser::parse;
    #[test]
    fn try_crash5() {
        // trying to panic in cp_utf8 or loop
        let code = vec![
            0xca, 0xfe, 0xba, 0xbe, // magic
            0x00, 0x00, 0x00, 0x41, // version
            0x00, 0x04, // cp count
            1, // Utf8
            0x00, 0x04, // length
            b'C', b'o', b'd', b'e',
            5, // Long
            0, 0, 0, 0, 0, 0, 0, 0, // takes 2 slots
            // We have enough slots? cp count is 4. slots: 1=Utf8, 2=Long, 3=Phantom

            // access flags
            0x00, 0x21,
            // this
            0x00, 0x01,
            // super
            0x00, 0x01,
            // interfaces count
            0x00, 0x00,
            // fields count
            0x00, 0x00,
            // methods count
            0x00, 0x00,
            // attributes count
            0x00, 0x01,
            // name_index = 3 (Phantom slot!)
            0x00, 0x03,
            // attribute_length
            0x00, 0x00, 0x00, 0x00,
        ];
        let _ = parse(&code);
    }
}
