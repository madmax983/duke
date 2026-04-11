use proptest::prelude::*;
use duke_bytecode::decode;

proptest! {
    #[test]
    fn try_crash_decode(bytes in proptest::collection::vec(any::<u8>(), 0..1024)) {
        let _ = decode(&bytes);
    }
}
