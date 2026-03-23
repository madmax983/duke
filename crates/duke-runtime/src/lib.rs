//! `duke-runtime` — Execution state primitives for the Duke JVM.
//!
//! This crate provides the foundational data structures required for JVM execution:
//! - [`crate::frame::Frame`]: The execution state for a single method invocation (operand stack, local variables).
//! - [`crate::slot::Slot`]: The unit of data storage in the JVM (representing variables like `int`, `float`, or references).
//! - [`crate::error::VmError`]: The canonical error type for runtime failures (e.g., `StackOverflow`, `NullPointerException`).
//!
//! These types are entirely decoupled from the actual bytecode instruction set, ensuring that
//! memory and execution state semantics are strictly isolated from the decoding and interpretation logic.

pub mod error;
pub mod frame;
pub mod slot;

pub use error::{Error, Result, VmError, VmResult};
pub use frame::Frame;
pub use slot::Slot;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_int_round_trip() {
        let s = Slot::Int(42);
        assert_eq!(s.as_int().unwrap(), 42);
    }

    #[test]
    fn frame_push_pop() {
        let mut frame = Frame::new(4, 2, vec![]).expect("new frame");
        frame.push(Slot::Int(7)).expect("push");
        assert_eq!(frame.pop_int().expect("pop"), 7);
    }

    #[test]
    fn frame_locals_from_args() {
        let frame =
            Frame::new(4, 3, vec![Slot::Int(1), Slot::Int(2), Slot::Int(3)]).expect("new frame");
        assert_eq!(frame.load_local(0).unwrap(), Slot::Int(1));
        assert_eq!(frame.load_local(2).unwrap(), Slot::Int(3));
    }

    #[test]
    fn frame_stack_overflow() {
        let mut frame = Frame::new(1, 1, vec![]).expect("new frame");
        frame.push(Slot::Int(1)).expect("push 1");
        let err = frame.push(Slot::Int(2)).unwrap_err();
        assert!(matches!(err, VmError::StackOverflow));
    }

    #[test]
    fn frame_stack_underflow() {
        let mut frame = Frame::new(4, 1, vec![]).expect("new frame");
        let err = frame.pop().unwrap_err();
        assert!(matches!(err, VmError::StackUnderflow));
    }

    #[test]
    fn pop_ref_non_null() {
        let mut f = Frame::new(2, 1, vec![]).unwrap();
        f.push(Slot::Reference(Some(42))).unwrap();
        assert_eq!(f.pop_ref().unwrap(), 42u64);
    }

    #[test]
    fn pop_ref_null_gives_npe() {
        let mut f = Frame::new(2, 1, vec![]).unwrap();
        f.push(Slot::Reference(None)).unwrap();
        assert!(matches!(
            f.pop_ref().unwrap_err(),
            VmError::NullPointerException
        ));
    }

    #[test]
    fn array_index_oob_error_message() {
        let e = VmError::ArrayIndexOutOfBounds {
            index: 5,
            length: 3,
        };
        assert_eq!(e.to_string(), "array index 5 out of bounds for length 3");
    }

    #[test]
    fn negative_array_size_error_message() {
        let e = VmError::NegativeArraySize { size: -1 };
        assert_eq!(e.to_string(), "negative array size: -1");
    }

    #[test]
    fn java_exception_error_message() {
        let e = VmError::JavaException {
            class_name: "java/lang/RuntimeException".to_string(),
        };
        assert_eq!(e.to_string(), "java exception: java/lang/RuntimeException");
    }

    #[test]
    fn class_cast_exception_error_message() {
        let e = VmError::ClassCastException {
            from: "java/lang/RuntimeException".to_string(),
            to: "java/lang/String".to_string(),
        };
        assert_eq!(
            e.to_string(),
            "class cast exception: java/lang/RuntimeException cannot be cast to java/lang/String"
        );
    }

    #[test]
    fn class_not_found_error_message() {
        let e = VmError::ClassNotFound {
            name: "com/example/Missing".to_string(),
        };
        assert_eq!(e.to_string(), "class not found: com/example/Missing");
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn vm_error_display_messages() {
        let cases = vec![
            (VmError::StackOverflow, "operand stack overflow"),
            (VmError::StackUnderflow, "operand stack underflow"),
            (
                VmError::LocalOutOfBounds {
                    index: 5,
                    max_locals: 3,
                },
                "local variable index 5 out of bounds (max_locals=3)",
            ),
            (VmError::DivisionByZero, "integer division by zero"),
            (
                VmError::InvalidBranchTarget { pc: 42 },
                "invalid branch target: pc=42",
            ),
            (
                VmError::FellOffEnd,
                "fell off end of bytecode without a return instruction",
            ),
            (
                VmError::TypeMismatch {
                    expected: "int",
                    got: "long",
                },
                "type mismatch: expected int, got long",
            ),
            (
                VmError::Unimplemented { mnemonic: "nop" },
                "unimplemented instruction: nop",
            ),
            (
                VmError::InvalidCpIndex { index: 99 },
                "invalid constant pool index 99",
            ),
            (
                VmError::MethodNotFound {
                    name: "foo".to_string(),
                    descriptor: "()V".to_string(),
                },
                "method not found: foo()V",
            ),
            (
                VmError::InvalidMethodref { index: 42 },
                "constant pool index 42 is not a valid Methodref",
            ),
            (VmError::NullPointerException, "null pointer dereference"),
            (
                VmError::InvalidRef {
                    address: 0xDEAD_BEEF,
                },
                "invalid heap reference: address=3735928559",
            ),
            (
                VmError::InvalidFieldref { index: 12 },
                "constant pool index 12 is not a valid Fieldref",
            ),
            (
                VmError::ArrayIndexOutOfBounds {
                    index: 5,
                    length: 3,
                },
                "array index 5 out of bounds for length 3",
            ),
            (
                VmError::NegativeArraySize { size: -1 },
                "negative array size: -1",
            ),
            (
                VmError::JavaException {
                    class_name: "java/lang/RuntimeException".to_string(),
                },
                "java exception: java/lang/RuntimeException",
            ),
            (
                VmError::ClassCastException {
                    from: "java/lang/RuntimeException".to_string(),
                    to: "java/lang/String".to_string(),
                },
                "class cast exception: java/lang/RuntimeException cannot be cast to java/lang/String",
            ),
            (
                VmError::ClassNotFound {
                    name: "com/example/Missing".to_string(),
                },
                "class not found: com/example/Missing",
            ),
            (VmError::SystemExit { code: 42 }, "System.exit(42)"),
            (
                VmError::InstantiationError {
                    class_name: "java/lang/Number".to_string(),
                },
                "InstantiationError: cannot instantiate abstract class java/lang/Number",
            ),
            (
                VmError::AbstractMethodError {
                    class_name: "java/lang/Number".to_string(),
                    method_name: "intValue".to_string(),
                },
                "AbstractMethodError: java/lang/Number.intValue",
            ),
        ];

        for (err, expected_msg) in cases {
            assert_eq!(err.to_string(), expected_msg);
        }
    }
}
#[cfg(test)]
mod fuzz;
