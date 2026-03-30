#[cfg(test)]
mod tests {
    use crate::zip::ZipReader;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn fuzz_zip_reader_from_bytes(data in any::<Vec<u8>>()) {
            if let Ok(reader) = ZipReader::from_bytes(data) {
                let _ = reader.entry_names().collect::<Vec<_>>();
                if let Some(first_name) = reader.entry_names().next() {
                    let _ = reader.read_entry(first_name);
                }
            }
        }
    }
}
