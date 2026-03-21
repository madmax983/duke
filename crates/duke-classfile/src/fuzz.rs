#[cfg(test)]
mod tests {
    use crate::parse;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn does_not_crash_parse_random(data in any::<Vec<u8>>()) {
            let _ = parse(&data);
        }

        #[test]
        fn does_not_crash_parse_with_magic_and_version(data in any::<Vec<u8>>()) {
            let mut prefix = vec![0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x00, 0x00, 0x41]; // magic + java 21
            // Try to set constant pool count to 0xFFFF and see if it OOMs or hangs
            prefix.push(0xFF);
            prefix.push(0xFF);
            prefix.extend(data);
            let _ = parse(&prefix);
        }
    }
}
