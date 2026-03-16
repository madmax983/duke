use crate::error::{VmError, VmResult};

/// A single JVM operand stack or local variable slot.
///
/// Note: in the JVM spec, `long` and `double` occupy two computational slots.
/// For Phase 4 we track them as single `Slot` entries for simplicity; Phase 5+
/// will introduce proper two-slot tracking via a `Padding` variant.
///
/// `Slot` derives `Copy` because it is small (fits in registers) and is passed by value
/// across all frame and stack operations to avoid the overhead of cloning on hot paths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Slot {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    /// Object reference (`None` represents the JVM `null` value).
    Reference(Option<u64>),
    /// Return address pushed by `jsr` (legacy subroutine support).
    ReturnAddress(usize),
}

impl Slot {
    /// Extract as `i32`, or return a `TypeMismatch` error.
    ///
    /// # Errors
    /// Returns [`VmError::TypeMismatch`] if the slot is not `Int`.
    pub const fn as_int(&self) -> VmResult<i32> {
        if let Self::Int(v) = self {
            Ok(*v)
        } else {
            Err(VmError::TypeMismatch {
                expected: "int",
                got: self.type_name(),
            })
        }
    }

    /// Extract as `i64`, or return a `TypeMismatch` error.
    ///
    /// # Errors
    /// Returns [`VmError::TypeMismatch`] if the slot is not `Long`.
    pub const fn as_long(&self) -> VmResult<i64> {
        if let Self::Long(v) = self {
            Ok(*v)
        } else {
            Err(VmError::TypeMismatch {
                expected: "long",
                got: self.type_name(),
            })
        }
    }

    /// Extract as `f32`, or return a `TypeMismatch` error.
    ///
    /// # Errors
    /// Returns [`VmError::TypeMismatch`] if the slot is not `Float`.
    pub const fn as_float(&self) -> VmResult<f32> {
        if let Self::Float(v) = self {
            Ok(*v)
        } else {
            Err(VmError::TypeMismatch {
                expected: "float",
                got: self.type_name(),
            })
        }
    }

    /// Extract as `f64`, or return a `TypeMismatch` error.
    ///
    /// # Errors
    /// Returns [`VmError::TypeMismatch`] if the slot is not `Double`.
    pub const fn as_double(&self) -> VmResult<f64> {
        if let Self::Double(v) = self {
            Ok(*v)
        } else {
            Err(VmError::TypeMismatch {
                expected: "double",
                got: self.type_name(),
            })
        }
    }

    /// If this slot is a non-null reference, return the heap index. Otherwise `None`.
    #[must_use]
    pub const fn as_reference(&self) -> Option<u64> {
        if let Self::Reference(Some(r)) = self {
            Some(*r)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Int(_) => "int",
            Self::Long(_) => "long",
            Self::Float(_) => "float",
            Self::Double(_) => "double",
            Self::Reference(_) => "reference",
            Self::ReturnAddress(_) => "returnAddress",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_reference_returns_value_for_nonnull() {
        assert_eq!(Slot::Reference(Some(42)).as_reference(), Some(42));
        assert_eq!(Slot::Reference(Some(2)).as_reference(), Some(2));
        assert_eq!(
            Slot::Reference(Some(u64::MAX)).as_reference(),
            Some(u64::MAX)
        );
    }

    #[test]
    fn as_reference_returns_none_for_null_and_other_types() {
        assert_eq!(Slot::Reference(None).as_reference(), None);
        assert_eq!(Slot::Int(0).as_reference(), None);
        assert_eq!(Slot::Long(0).as_reference(), None);
        assert_eq!(Slot::Float(0.0).as_reference(), None);
        assert_eq!(Slot::Double(0.0).as_reference(), None);
    }

    #[test]
    fn type_name_all_variants() {
        assert_eq!(Slot::Int(0).type_name(), "int");
        assert_eq!(Slot::Long(0).type_name(), "long");
        assert_eq!(Slot::Float(0.0).type_name(), "float");
        assert_eq!(Slot::Double(0.0).type_name(), "double");
        assert_eq!(Slot::Reference(None).type_name(), "reference");
        assert_eq!(Slot::Reference(Some(1)).type_name(), "reference");
        assert_eq!(Slot::ReturnAddress(0).type_name(), "returnAddress");
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn as_methods_return_expected_type_or_mismatch_error() {
        let variants = vec![
            Slot::Int(42),
            Slot::Long(42),
            Slot::Float(42.0),
            Slot::Double(42.0),
            Slot::Reference(Some(42)),
            Slot::ReturnAddress(42),
        ];

        for variant in variants {
            let type_name = variant.type_name();

            // as_int
            if matches!(variant, Slot::Int(_)) {
                assert_eq!(variant.as_int().unwrap(), 42);
            } else {
                assert_eq!(
                    variant.as_int().unwrap_err(),
                    VmError::TypeMismatch {
                        expected: "int",
                        got: type_name,
                    }
                );
            }

            // as_long
            if matches!(variant, Slot::Long(_)) {
                assert_eq!(variant.as_long().unwrap(), 42);
            } else {
                assert_eq!(
                    variant.as_long().unwrap_err(),
                    VmError::TypeMismatch {
                        expected: "long",
                        got: type_name,
                    }
                );
            }

            // as_float
            if matches!(variant, Slot::Float(_)) {
                assert_eq!(variant.as_float().unwrap(), 42.0);
            } else {
                assert_eq!(
                    variant.as_float().unwrap_err(),
                    VmError::TypeMismatch {
                        expected: "float",
                        got: type_name,
                    }
                );
            }

            // as_double
            if matches!(variant, Slot::Double(_)) {
                assert_eq!(variant.as_double().unwrap(), 42.0);
            } else {
                assert_eq!(
                    variant.as_double().unwrap_err(),
                    VmError::TypeMismatch {
                        expected: "double",
                        got: type_name,
                    }
                );
            }
        }
    }
}
