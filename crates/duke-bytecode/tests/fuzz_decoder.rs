use duke_bytecode::{decode, verify};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 10000, .. ProptestConfig::default() })]
    #[test]
    fn test_decoder_and_verifier_fuzz(
        bytes in any::<Vec<u8>>(),
        max_locals in 0..10u16,
        max_stack in 0..10u16,
    ) {
        if let Ok(instrs) = decode(&bytes) {
            let _ = verify(&instrs, max_locals, max_stack);
        }
    }
}
