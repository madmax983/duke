#![allow(missing_docs)]
use duke_bytecode::Instruction;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_switch_targets(
        default_target in any::<i32>(),
        low in any::<i32>(),
        high in any::<i32>(),
        offsets in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let switch = Instruction::Tableswitch {
            default: default_target,
            low,
            high,
            offsets,
        };

        let _ = switch.switch_targets();
        let _ = switch.control_flow_targets(0, Some(10));
    }
}
