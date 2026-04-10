use duke_loader::zip::ZipReader;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_zip_eocd_oom_crash(data in any::<Vec<u8>>()) {
        let mut eocd = vec![0x50, 0x4B, 0x05, 0x06]; // EOCD signature
        eocd.extend(vec![0; 6]); // padding
        eocd.extend(vec![0xFF, 0xFF]); // entry count = 65535
        eocd.extend(vec![0, 0, 0, 0]); // cd_size = 0
        eocd.extend(vec![0, 0, 0, 0]); // cd_offset = 0
        eocd.extend(vec![0, 0]); // comment len
        eocd.extend(data);

        let _ = ZipReader::from_bytes(eocd);
    }
}
