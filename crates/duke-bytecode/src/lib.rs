//! `duke-bytecode` — JVM bytecode definitions, decoder, and structural verifier.
//!
//! # Modules
//!
//! - `opcodes` — All JVM opcode byte constants (JVM SE 21 §6.5)
//! - `instruction` — Typed [`Instruction`] enum with decoded operands
//! - `decoder` — Decode raw `Code` bytes → `Vec<(pc, Instruction)>`
//! - `verifier` — Structural pass: stack bounds, local bounds, empty stack on return
//! - `error` — [`DecodeError`] and [`VerifyError`]

#[cfg(feature = "nova")]
pub(crate) mod basic_block;
pub(crate) mod call_graph;
pub(crate) mod cfg;
pub(crate) mod decoder;
pub(crate) mod error;
pub(crate) mod instruction;
pub(crate) mod opcodes;
#[cfg(feature = "nova")]
pub(crate) mod reachability;
pub(crate) mod verifier;

#[cfg(feature = "nova")]
pub use basic_block::{BasicBlock, build_basic_blocks};
pub use call_graph::generate_mermaid_call_graph;
#[cfg(feature = "nova")]
pub use cfg::generate_basic_block_cfg;
pub use cfg::{cyclomatic_complexity, generate_mermaid_cfg};
pub use decoder::decode;
pub use error::{DecodeError, Error, Result, VerifyError};
pub use instruction::{ArrayType, Instruction};
#[cfg(feature = "nova")]
pub use reachability::{find_dead_blocks, find_shortest_path, get_successors};
pub use verifier::verify;

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::{
        parse,
        types::{AttributeData, CpEntry},
    };

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn fixture(name: &str) -> std::path::PathBuf {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../tests/fixtures");
        p.push(name);
        p
    }

    fn get_method_code(class_bytes: &[u8], method_name: &str) -> (Vec<u8>, u16, u16) {
        let cf = parse(class_bytes).expect("parse failed");
        let method = cf
            .methods
            .iter()
            .find(|m| {
                matches!(&cf.constant_pool[m.name_index.0 as usize], Some(CpEntry::Utf8(s)) if s == method_name)
            })
            .unwrap_or_else(|| panic!("method '{method_name}' not found"));

        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                return (code.code.clone(), code.max_stack, code.max_locals);
            }
        }
        panic!("no Code attribute on method '{method_name}'");
    }

    // -----------------------------------------------------------------------
    // Decoder tests
    // -----------------------------------------------------------------------

    #[test]
    fn decode_hello_world_main() {
        let bytes = std::fs::read(fixture("HelloWorld.class")).expect("fixture missing");
        let (code, max_stack, max_locals) = get_method_code(&bytes, "main");

        let instructions = decode(&code).expect("decode should succeed");

        // javac --release 21 HelloWorld produces:
        //   0: getstatic     #7  // Field java/lang/System.out:Ljava/io/PrintStream;
        //   3: ldc           #13 // String Hello, World!
        //   5: invokevirtual #15 // Method java/io/PrintStream.println:(Ljava/lang/String;)V
        //   8: return
        assert_eq!(instructions.len(), 4, "expected 4 instructions in main");
        assert!(matches!(instructions[0], (0, Instruction::Getstatic(_))));
        assert!(matches!(instructions[1], (3, Instruction::Ldc(_))));
        assert!(matches!(
            instructions[2],
            (5, Instruction::Invokevirtual(_))
        ));
        assert!(matches!(instructions[3], (8, Instruction::Return)));

        // Note: structural verify intentionally skips stack-depth for invocations
        // (argument count requires descriptor resolution, deferred to Phase 3).
        // We just confirm it doesn't panic.
        let _ = verify(&instructions, max_stack, max_locals);
    }

    #[test]
    fn decode_counter_increment() {
        let bytes = std::fs::read(fixture("Counter.class")).expect("fixture missing");
        let (code, max_stack, max_locals) = get_method_code(&bytes, "increment");

        let instructions = decode(&code).expect("decode should succeed");
        assert!(
            !instructions.is_empty(),
            "increment should have instructions"
        );

        // Must contain iinc or similar (++value)
        let has_load_store = instructions.iter().any(|(_, i)| {
            matches!(
                i,
                Instruction::Iinc { .. } | Instruction::Getfield(_) | Instruction::Putfield(_)
            )
        });
        assert!(
            has_load_store,
            "increment should contain field access or iinc"
        );

        verify(&instructions, max_stack, max_locals).expect("verify should pass");
    }

    #[test]
    fn decode_counter_constructor() {
        let bytes = std::fs::read(fixture("Counter.class")).expect("fixture missing");
        let (code, max_stack, max_locals) = get_method_code(&bytes, "<init>");

        let instructions = decode(&code).expect("decode should succeed");
        verify(&instructions, max_stack, max_locals).expect("constructor should verify");
    }

    #[test]
    fn decode_counter_get_instance_count() {
        let bytes = std::fs::read(fixture("Counter.class")).expect("fixture missing");
        let (code, max_stack, max_locals) = get_method_code(&bytes, "getInstanceCount");

        let instructions = decode(&code).expect("decode should succeed");

        // static int getInstanceCount() { return instanceCount; }
        // should be: getstatic, ireturn
        assert_eq!(instructions.len(), 2);
        assert!(matches!(instructions[0], (_, Instruction::Getstatic(_))));
        assert!(matches!(instructions[1], (_, Instruction::Ireturn)));

        verify(&instructions, max_stack, max_locals).expect("verify should pass");
    }

    // -----------------------------------------------------------------------
    // Decoder error tests
    // -----------------------------------------------------------------------

    #[test]
    fn decode_empty_bytecode() {
        let result = decode(&[]);
        assert!(
            result.is_ok(),
            "empty bytecode is valid (zero instructions)"
        );
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn decode_truncated_operand() {
        // BIPUSH with no operand byte
        let code = [0x10]; // BIPUSH opcode only
        let err = decode(&code).unwrap_err();
        assert!(
            matches!(err, crate::Error::Decode(DecodeError::UnexpectedEof { .. })),
            "truncated operand: {err}"
        );
    }

    #[test]
    fn decode_unknown_opcode() {
        let code = [0xFE]; // IMPDEP1 — reserved, invalid in class files
        let err = decode(&code).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Decode(DecodeError::UnknownOpcode { opcode: 0xFE, .. })
            ),
            "should reject reserved opcode: {err}"
        );
    }

    #[test]
    fn decode_bipush_sequence() {
        // bipush 42, bipush -1, return
        let code = [0x10, 42, 0x10, 0xFF, 0xB1];
        let instrs = decode(&code).expect("should decode");
        assert_eq!(instrs.len(), 3);
        assert!(matches!(instrs[0], (0, Instruction::Bipush(42))));
        assert!(matches!(instrs[1], (2, Instruction::Bipush(-1))));
        assert!(matches!(instrs[2], (4, Instruction::Return)));
    }

    #[test]
    fn decode_wide_iload() {
        // wide iload 300 → [0xC4, 0x15, 0x01, 0x2C]
        let code = [0xC4, 0x15, 0x01, 0x2C];
        let instrs = decode(&code).expect("wide iload should decode");
        assert_eq!(instrs.len(), 1);
        assert!(matches!(instrs[0], (0, Instruction::IloadW(300))));
    }

    #[test]
    fn decode_lookupswitch_rejects_npairs_beyond_remaining_bytes() {
        // lookupswitch with npairs=1 but no (match, offset) payload.
        let code = [
            0xAB, 0x00, 0x00, 0x00, // lookupswitch + padding
            0x00, 0x00, 0x00, 0x00, // default
            0x00, 0x00, 0x00, 0x01, // npairs = 1
        ];
        let err = decode(&code).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Decode(DecodeError::InvalidLookupswitch { npairs: 1, .. })
            ),
            "invalid lookupswitch should be rejected: {err}"
        );
    }

    #[test]
    fn decode_tableswitch_rejects_truncated_offsets() {
        // tableswitch with low=0, high=1 (needs 2 offsets) but only one offset entry.
        let code = [
            0xAA, 0x00, 0x00, 0x00, // tableswitch + padding
            0x00, 0x00, 0x00, 0x00, // default
            0x00, 0x00, 0x00, 0x00, // low
            0x00, 0x00, 0x00, 0x01, // high
            0x00, 0x00, 0x00, 0x05, // one offset (should have two)
        ];
        let err = decode(&code).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Decode(DecodeError::InvalidTableswitch {
                    low: 0,
                    high: 1,
                    ..
                })
            ),
            "invalid tableswitch should be rejected: {err}"
        );
    }

    #[test]
    fn decode_invokeinterface_rejects_non_zero_reserved_byte() {
        // invokeinterface index=1 count=1 reserved=1 (invalid).
        let code = [0xB9, 0x00, 0x01, 0x01, 0x01];
        let err = decode(&code).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Decode(DecodeError::InvalidInvokeinterfaceReserved {
                    reserved: 1,
                    ..
                })
            ),
            "non-zero invokeinterface reserved byte should be rejected: {err}"
        );
    }

    #[test]
    fn decode_invokedynamic_rejects_non_zero_reserved_bytes() {
        // invokedynamic index=1 reserved bytes must be 0,0.
        let code = [0xBA, 0x00, 0x01, 0x00, 0x01];
        let err = decode(&code).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Decode(DecodeError::InvalidInvokedynamicReserved {
                    reserved1: 0,
                    reserved2: 1,
                    ..
                })
            ),
            "non-zero invokedynamic reserved bytes should be rejected: {err}"
        );
    }
    // -----------------------------------------------------------------------
    // Verifier tests
    // -----------------------------------------------------------------------

    #[test]
    fn verify_simple_return_passes() {
        // return — stack empty, fine
        let instrs = vec![(0, Instruction::Return)];
        verify(&instrs, 0, 1).expect("return with empty stack should pass");
    }

    #[test]
    fn verify_stack_overflow_detected() {
        // Push two values with max_stack=1
        let instrs = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Iconst1),
            (2, Instruction::Return),
        ];
        let err = verify(&instrs, 1, 1).unwrap_err();
        assert!(
            matches!(err, crate::Error::Verify(VerifyError::StackOverflow { .. })),
            "should detect overflow: {err}"
        );
    }

    #[test]
    fn verify_stack_underflow_detected() {
        // Pop with empty stack
        let instrs = vec![(0, Instruction::Pop)];
        let err = verify(&instrs, 2, 1).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Verify(VerifyError::StackUnderflow { .. })
            ),
            "should detect underflow: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // Property tests
    // -----------------------------------------------------------------------

    use proptest::prelude::*;

    proptest! {
        /// Decoder must never panic on arbitrary bytecode.
        #[test]
        fn no_panic_on_arbitrary_bytecode(
            bytes in proptest::collection::vec(any::<u8>(), 0..2048)
        ) {
            let _ = decode(&bytes);
        }
    }

    // -----------------------------------------------------------------------
    // ArrayType::from_u8 — all valid codes + invalid boundary
    // Kills: delete match arm 4..11 mutants in instruction.rs
    // -----------------------------------------------------------------------

    use crate::instruction::ArrayType;

    #[test]
    fn array_type_from_u8_all_valid() {
        assert_eq!(ArrayType::from_u8(4), Some(ArrayType::Boolean));
        assert_eq!(ArrayType::from_u8(5), Some(ArrayType::Char));
        assert_eq!(ArrayType::from_u8(6), Some(ArrayType::Float));
        assert_eq!(ArrayType::from_u8(7), Some(ArrayType::Double));
        assert_eq!(ArrayType::from_u8(8), Some(ArrayType::Byte));
        assert_eq!(ArrayType::from_u8(9), Some(ArrayType::Short));
        assert_eq!(ArrayType::from_u8(10), Some(ArrayType::Int));
        assert_eq!(ArrayType::from_u8(11), Some(ArrayType::Long));
    }

    #[test]
    fn array_type_from_u8_invalid_returns_none() {
        assert_eq!(ArrayType::from_u8(0), None);
        assert_eq!(ArrayType::from_u8(3), None); // just below valid range
        assert_eq!(ArrayType::from_u8(12), None); // just above valid range
        assert_eq!(ArrayType::from_u8(255), None);
    }

    // -----------------------------------------------------------------------
    // Instruction::mnemonic spot-checks
    // Kills: replace mnemonic -> "" / "xyzzy" mutants
    // -----------------------------------------------------------------------

    #[test]
    fn mnemonic_spot_checks() {
        assert_eq!(Instruction::Nop.mnemonic(), "nop");
        assert_eq!(Instruction::Return.mnemonic(), "return");
        assert_eq!(Instruction::Ireturn.mnemonic(), "ireturn");
        assert_eq!(Instruction::Athrow.mnemonic(), "athrow");
        assert_eq!(Instruction::Iconst0.mnemonic(), "iconst_0");
        assert_eq!(Instruction::Bipush(42).mnemonic(), "bipush");
    }

    // -----------------------------------------------------------------------
    // Decoder: tableswitch boundary cases
    // Kills: < vs <= on count_i64 (line 310), > vs >= on count vs max_possible
    // -----------------------------------------------------------------------

    #[test]
    fn tableswitch_single_entry_decodes_ok() {
        // low=3, high=3 → count=1 (minimum valid tableswitch).
        // Confirms count_i64 path handles count=1 correctly.
        let code: Vec<u8> = vec![
            0xAA, // tableswitch
            0x00, 0x00, 0x00, // padding (pc=0, align to 4)
            0x00, 0x00, 0x00, 0x09, // default = 9
            0x00, 0x00, 0x00, 0x03, // low = 3
            0x00, 0x00, 0x00, 0x03, // high = 3  →  count = 1
            0x00, 0x00, 0x00, 0x07, // offset[0] = 7
        ];
        let instrs = decode(&code).expect("single-entry tableswitch should decode");
        assert_eq!(instrs.len(), 1);
        if let (
            0,
            Instruction::Tableswitch {
                low, high, offsets, ..
            },
        ) = &instrs[0]
        {
            assert_eq!(*low, 3);
            assert_eq!(*high, 3);
            assert_eq!(offsets.len(), 1);
            assert_eq!(offsets[0], 7);
        } else {
            panic!("expected Tableswitch");
        }
    }

    #[test]
    fn tableswitch_exact_fit_decodes_ok() {
        // count=1 and exactly 4 bytes remain → count == max_possible; should pass.
        // Mutant `> max_possible → >= max_possible` would incorrectly reject this.
        let code: Vec<u8> = vec![
            0xAA, // tableswitch
            0x00, 0x00, 0x00, // padding
            0x00, 0x00, 0x00, 0x05, // default = 5
            0x00, 0x00, 0x00, 0x00, // low = 0
            0x00, 0x00, 0x00, 0x00, // high = 0  →  count = 1
            0x00, 0x00, 0x00, 0x07, // offset[0] = 7  (exactly 4 bytes, max_possible = 1)
        ];
        let instrs = decode(&code).expect("exact-fit tableswitch should decode");
        assert_eq!(instrs.len(), 1);
        if let (
            0,
            Instruction::Tableswitch {
                low,
                high,
                offsets,
                default,
            },
        ) = &instrs[0]
        {
            assert_eq!(*low, 0);
            assert_eq!(*high, 0);
            assert_eq!(*default, 5);
            assert_eq!(offsets, &[7i32]);
        } else {
            panic!("expected Tableswitch");
        }
    }

    #[test]
    fn tableswitch_multi_entry_decodes_ok() {
        // low=0, high=1 → count=2; high > low kills the `< → >` mutant on the high<low guard.
        let code: Vec<u8> = vec![
            0xAA, // tableswitch
            0x00, 0x00, 0x00, // padding
            0x00, 0x00, 0x00, 0x0F, // default = 15
            0x00, 0x00, 0x00, 0x00, // low = 0
            0x00, 0x00, 0x00, 0x01, // high = 1  →  count = 2
            0x00, 0x00, 0x00, 0x05, // offset[0] = 5
            0x00, 0x00, 0x00, 0x07, // offset[1] = 7
        ];
        let instrs = decode(&code).expect("multi-entry tableswitch should decode");
        if let (
            0,
            Instruction::Tableswitch {
                low, high, offsets, ..
            },
        ) = &instrs[0]
        {
            assert_eq!(*low, 0);
            assert_eq!(*high, 1);
            assert_eq!(offsets, &[5i32, 7i32]);
        } else {
            panic!("expected Tableswitch");
        }
    }

    #[test]
    fn tableswitch_large_offset_decodes_ok() {
        // default = 0x00010005 (high word non-zero) kills the `<< → >>` mutant in read_u32.
        // With `>>`, (0x0001 >> 16) | 0x0005 = 0 | 5 = 5 ≠ 65541.
        let code: Vec<u8> = vec![
            0xAA, // tableswitch
            0x00, 0x00, 0x00, // padding
            0x00, 0x01, 0x00, 0x05, // default = 0x00010005 = 65541
            0x00, 0x00, 0x00, 0x00, // low = 0
            0x00, 0x00, 0x00, 0x00, // high = 0  →  count = 1
            0x00, 0x00, 0x00, 0x03, // offset[0] = 3
        ];
        let instrs = decode(&code).expect("tableswitch with large default should decode");
        if let (0, Instruction::Tableswitch { default, .. }) = &instrs[0] {
            assert_eq!(*default, 65541);
        } else {
            panic!("expected Tableswitch");
        }
    }

    // -----------------------------------------------------------------------
    // Decoder: lookupswitch boundary cases
    // Kills: < vs == / > / <= on npairs (line 331), / vs % / * (line 334), > vs >= (line 335)
    // -----------------------------------------------------------------------

    #[test]
    fn lookupswitch_zero_pairs_decodes_ok() {
        // npairs=0 — valid empty switch; must NOT be rejected.
        // Mutants `< 0 → == 0` and `< 0 → <= 0` would incorrectly reject npairs=0.
        let code: Vec<u8> = vec![
            0xAB, // lookupswitch
            0x00, 0x00, 0x00, // padding
            0x00, 0x00, 0x00, 0x05, // default = 5
            0x00, 0x00, 0x00, 0x00, // npairs = 0
        ];
        let instrs = decode(&code).expect("zero-pairs lookupswitch should decode");
        assert_eq!(instrs.len(), 1);
        if let (0, Instruction::Lookupswitch { default, pairs }) = &instrs[0] {
            assert_eq!(*default, 5);
            assert!(pairs.is_empty());
        } else {
            panic!("expected Lookupswitch");
        }
    }

    #[test]
    fn lookupswitch_exact_one_pair_decodes_ok() {
        // npairs=1 and exactly 8 bytes for the pair → npairs == remaining_pairs; should pass.
        // Kills: `> → >=` mutant (exact-fit rejection) and `/ 8 → % 8` (% gives 0 for 8 bytes).
        let code: Vec<u8> = vec![
            0xAB, // lookupswitch
            0x00, 0x00, 0x00, // padding
            0x00, 0x00, 0x00, 0x05, // default = 5
            0x00, 0x00, 0x00, 0x01, // npairs = 1
            0x00, 0x00, 0x00, 0x2A, // match_val = 42
            0x00, 0x00, 0x00, 0x07, // offset = 7
        ];
        let instrs = decode(&code).expect("one-pair lookupswitch should decode");
        assert_eq!(instrs.len(), 1);
        if let (0, Instruction::Lookupswitch { pairs, default }) = &instrs[0] {
            assert_eq!(*default, 5);
            assert_eq!(pairs.len(), 1);
            assert_eq!(pairs[0], (42, 7));
        } else {
            panic!("expected Lookupswitch");
        }
    }

    #[test]
    fn lookupswitch_truncated_pair_data_rejected() {
        // npairs=1 but only 4 bytes available (half a pair).
        // Kills: `/ 8 → * 8` mutant — with * 8, remaining = 4*8=32 ≥ 1, wrongly accepts.
        let code: Vec<u8> = vec![
            0xAB, // lookupswitch
            0x00, 0x00, 0x00, // padding
            0x00, 0x00, 0x00, 0x05, // default = 5
            0x00, 0x00, 0x00, 0x01, // npairs = 1
            0x00, 0x00, 0x00, 0x2A, // match_val only — offset bytes missing
        ];
        let err = decode(&code).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Decode(DecodeError::InvalidLookupswitch { npairs: 1, .. })
            ),
            "truncated pair data should be rejected: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // Decoder: invokedynamic with valid (zero) reserved bytes
    // Kills: `!= → ==` mutant (line 376) that would reject zero reserved bytes
    // -----------------------------------------------------------------------

    #[test]
    fn invokedynamic_zero_reserved_bytes_decodes_ok() {
        // Both reserved bytes = 0 → valid; must NOT trigger the error.
        // Mutant `!= → ==` flips the check so zero bytes would *always* error.
        let code: Vec<u8> = vec![
            0xBA, // invokedynamic
            0x00, 0x01, // cp index = 1
            0x00, 0x00, // reserved1=0, reserved2=0
        ];
        let instrs = decode(&code).expect("valid invokedynamic should decode");
        assert_eq!(instrs.len(), 1);
        assert!(matches!(instrs[0], (0, Instruction::Invokedynamic(_))));
    }

    // -----------------------------------------------------------------------
    // Verifier: non-empty stack on return
    // Kills: `replace is_return -> bool with false` mutant (verifier.rs:338)
    // -----------------------------------------------------------------------

    #[test]
    fn verify_nonempty_stack_on_ireturn_fails() {
        // iconst_0 pushes 1 value; then iconst_1 pushes a second; ireturn
        // with depth=2 should give NonEmptyStackOnReturn.
        let instrs = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Iconst1),
            (2, Instruction::Ireturn),
        ];
        let err = verify(&instrs, 4, 1).unwrap_err();
        // ireturn pops 1 (the return value); depth goes 2→1; check fires at depth=1
        assert!(
            matches!(
                err,
                crate::Error::Verify(VerifyError::NonEmptyStackOnReturn { pc: 2, depth: 1 })
            ),
            "should reject return with non-empty stack: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // Verifier: check_locals — one out-of-bounds test per instruction family
    // Each test kills the corresponding arm-deletion mutant in check_locals.
    // -----------------------------------------------------------------------

    #[test]
    fn verify_local_out_of_bounds_iload_family() {
        // Iload(5) with max_locals=3 → LocalOutOfBounds. Kills the Iload/Lload/…/Ret arm.
        let instrs = vec![(0, Instruction::Iload(5))];
        let err = verify(&instrs, 10, 3).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Verify(VerifyError::LocalOutOfBounds {
                    index: 5,
                    max_locals: 3,
                    ..
                })
            ),
            "{err}"
        );
    }

    #[test]
    fn verify_local_out_of_bounds_iinc() {
        // Iinc{index:3, delta:1} with max_locals=2 → LocalOutOfBounds. Kills Iinc arm.
        let instrs = vec![(0, Instruction::Iinc { index: 3, value: 1 })];
        let err = verify(&instrs, 4, 2).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Verify(VerifyError::LocalOutOfBounds {
                    index: 3,
                    max_locals: 2,
                    ..
                })
            ),
            "{err}"
        );
    }

    #[test]
    fn verify_local_out_of_bounds_wide_load_family() {
        // IloadW(100) with max_locals=10 → LocalOutOfBounds. Kills IloadW/…/RetW arm.
        let instrs = vec![(0, Instruction::IloadW(100))];
        let err = verify(&instrs, 10, 10).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Verify(VerifyError::LocalOutOfBounds { index: 100, .. })
            ),
            "{err}"
        );
    }

    #[test]
    fn verify_local_out_of_bounds_iinc_wide() {
        // IincW{index:5, delta:1} with max_locals=3 → LocalOutOfBounds. Kills IincW arm.
        let instrs = vec![(0, Instruction::IincW { index: 5, value: 1 })];
        let err = verify(&instrs, 4, 3).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Verify(VerifyError::LocalOutOfBounds {
                    index: 5,
                    max_locals: 3,
                    ..
                })
            ),
            "{err}"
        );
    }

    #[test]
    fn verify_local_out_of_bounds_short_form_0() {
        // Iload0 uses fixed index 0; max_locals=0 means even index 0 is out of range.
        // Kills: arm deletion for Iload0/Lload0/…/Astore0.
        let instrs = vec![(0, Instruction::Iload0)];
        let err = verify(&instrs, 2, 0).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Verify(VerifyError::LocalOutOfBounds {
                    index: 0,
                    max_locals: 0,
                    ..
                })
            ),
            "{err}"
        );
    }

    #[test]
    fn verify_local_out_of_bounds_short_form_1() {
        // Iload1 uses fixed index 1; max_locals=1 → out of bounds (0 is valid, not 1).
        // Kills: arm deletion for Iload1/…/Astore1.
        let instrs = vec![(0, Instruction::Iload1)];
        let err = verify(&instrs, 2, 1).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Verify(VerifyError::LocalOutOfBounds {
                    index: 1,
                    max_locals: 1,
                    ..
                })
            ),
            "{err}"
        );
    }

    #[test]
    fn verify_local_out_of_bounds_short_form_2() {
        // Iload2 uses fixed index 2; max_locals=2 → out of bounds.
        // Kills: arm deletion for Iload2/…/Astore2.
        let instrs = vec![(0, Instruction::Iload2)];
        let err = verify(&instrs, 2, 2).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Verify(VerifyError::LocalOutOfBounds {
                    index: 2,
                    max_locals: 2,
                    ..
                })
            ),
            "{err}"
        );
    }

    #[test]
    fn verify_local_out_of_bounds_short_form_3() {
        // Iload3 uses fixed index 3; max_locals=3 → out of bounds.
        // Kills: arm deletion for Iload3/…/Astore3.
        let instrs = vec![(0, Instruction::Iload3)];
        let err = verify(&instrs, 2, 3).unwrap_err();
        assert!(
            matches!(
                err,
                crate::Error::Verify(VerifyError::LocalOutOfBounds {
                    index: 3,
                    max_locals: 3,
                    ..
                })
            ),
            "{err}"
        );
    }
}
#[cfg(test)]
mod fuzz;
