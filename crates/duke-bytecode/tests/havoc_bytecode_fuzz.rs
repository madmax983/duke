use duke_bytecode::decoder;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_decoder(data in any::<Vec<u8>>()) {
        let _ = decoder::decode(&data);
    }
}
