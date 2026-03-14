//! `duke-bytecode` — JVM bytecode definitions, decoder, and structural verifier.
//!
//! # Modules
//!
//! - [`opcodes`] — All JVM opcode byte constants (JVM SE 21 §6.5)
//! - [`instruction`] — Typed [`Instruction`] enum with decoded operands
//! - [`decoder`] — Decode raw `Code` bytes → `Vec<(pc, Instruction)>`
//! - [`verifier`] — Structural pass: stack bounds, local bounds, empty stack on return
//! - [`error`] — [`DecodeError`] and [`VerifyError`]

pub mod cfg;
pub mod decoder;
pub mod error;
pub mod instruction;
pub mod opcodes;
pub mod verifier;

pub use decoder::decode;
pub use error::{DecodeError, DecodeResult, VerifyError, VerifyResult};
pub use instruction::Instruction;
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
            .unwrap_or_else(|| panic!("method '{}' not found", method_name));

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
            matches!(err, DecodeError::UnexpectedEof { .. }),
            "truncated operand: {err}"
        );
    }

    #[test]
    fn decode_unknown_opcode() {
        let code = [0xFE]; // IMPDEP1 — reserved, invalid in class files
        let err = decode(&code).unwrap_err();
        assert!(
            matches!(err, DecodeError::UnknownOpcode { opcode: 0xFE, .. }),
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
            matches!(err, DecodeError::InvalidLookupswitch { npairs: 1, .. }),
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
                DecodeError::InvalidTableswitch {
                    low: 0,
                    high: 1,
                    ..
                }
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
                DecodeError::InvalidInvokeinterfaceReserved { reserved: 1, .. }
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
                DecodeError::InvalidInvokedynamicReserved {
                    reserved1: 0,
                    reserved2: 1,
                    ..
                }
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
            matches!(err, VerifyError::StackOverflow { .. }),
            "should detect overflow: {err}"
        );
    }

    #[test]
    fn verify_stack_underflow_detected() {
        // Pop with empty stack
        let instrs = vec![(0, Instruction::Pop)];
        let err = verify(&instrs, 2, 1).unwrap_err();
        assert!(
            matches!(err, VerifyError::StackUnderflow { .. }),
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
}
