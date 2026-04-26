//! JVM operand stack and local variable slot definitions.
//!
//! This module provides the [`Slot`] enum, which represents a single piece of data
//! on the operand stack or in a local variable array.

use crate::error::{Error, Result};

/// A single JVM operand stack or local variable slot.
///
/// Note: in the JVM spec, `long` and `double` occupy two computational slots.
/// For Phase 4 we track them as single `Slot` entries for simplicity; Phase 5+
/// will introduce proper two-slot tracking via a `Padding` variant.
///
/// `Slot` derives `Copy` because it is small (fits in registers) and is passed by value
/// across all frame and stack operations to avoid the overhead of cloning on hot paths.
///
/// # Examples
///
/// ```
/// use duke_runtime::Slot;
///
/// let int_slot = Slot::Int(42);
/// let long_slot = Slot::Long(100);
/// let float_slot = Slot::Float(3.14);
/// let double_slot = Slot::Double(2.718);
/// let ref_slot = Slot::Reference(Some(1));
/// let null_slot = Slot::Reference(None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Slot {
    /// A 32-bit signed integer.
    Int(i32),
    /// A 64-bit signed integer.
    Long(i64),
    /// A 32-bit IEEE 754 floating-point number.
    Float(f32),
    /// A 64-bit IEEE 754 floating-point number.
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
    /// Returns [`Error::TypeMismatch`] if the slot is not `Int`.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_runtime::Slot;
    ///
    /// let s = Slot::Int(42);
    /// assert_eq!(s.as_int().unwrap(), 42);
    ///
    /// let s = Slot::Long(42);
    /// assert!(s.as_int().is_err());
    /// ```
    pub const fn as_int(&self) -> Result<i32> {
        if let Self::Int(v) = self {
            Ok(*v)
        } else {
            Err(Error::TypeMismatch {
                expected: "int",
                got: self.type_name(),
            })
        }
    }

    /// Extract as `i64`, or return a `TypeMismatch` error.
    ///
    /// # Errors
    /// Returns [`Error::TypeMismatch`] if the slot is not `Long`.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_runtime::Slot;
    ///
    /// let s = Slot::Long(42);
    /// assert_eq!(s.as_long().unwrap(), 42);
    ///
    /// let s = Slot::Int(42);
    /// assert!(s.as_long().is_err());
    /// ```
    pub const fn as_long(&self) -> Result<i64> {
        if let Self::Long(v) = self {
            Ok(*v)
        } else {
            Err(Error::TypeMismatch {
                expected: "long",
                got: self.type_name(),
            })
        }
    }

    /// Extract as `f32`, or return a `TypeMismatch` error.
    ///
    /// # Errors
    /// Returns [`Error::TypeMismatch`] if the slot is not `Float`.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_runtime::Slot;
    ///
    /// let s = Slot::Float(3.14);
    /// assert_eq!(s.as_float().unwrap(), 3.14);
    ///
    /// let s = Slot::Int(42);
    /// assert!(s.as_float().is_err());
    /// ```
    pub const fn as_float(&self) -> Result<f32> {
        if let Self::Float(v) = self {
            Ok(*v)
        } else {
            Err(Error::TypeMismatch {
                expected: "float",
                got: self.type_name(),
            })
        }
    }

    /// Extract as `f64`, or return a `TypeMismatch` error.
    ///
    /// # Errors
    /// Returns [`Error::TypeMismatch`] if the slot is not `Double`.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_runtime::Slot;
    ///
    /// let s = Slot::Double(3.14);
    /// assert_eq!(s.as_double().unwrap(), 3.14);
    ///
    /// let s = Slot::Int(42);
    /// assert!(s.as_double().is_err());
    /// ```
    pub const fn as_double(&self) -> Result<f64> {
        if let Self::Double(v) = self {
            Ok(*v)
        } else {
            Err(Error::TypeMismatch {
                expected: "double",
                got: self.type_name(),
            })
        }
    }

    /// If this slot is a non-null reference, return the heap index. Otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_runtime::Slot;
    ///
    /// let s = Slot::Reference(Some(1));
    /// assert_eq!(s.as_reference(), Some(1));
    ///
    /// let s = Slot::Reference(None);
    /// assert_eq!(s.as_reference(), None);
    ///
    /// let s = Slot::Int(42);
    /// assert_eq!(s.as_reference(), None);
    /// ```
    #[must_use]
    pub const fn as_reference(&self) -> Option<u64> {
        if let Self::Reference(Some(r)) = self {
            Some(*r)
        } else {
            None
        }
    }

    /// Get the JVM type name for this slot's contents.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_runtime::Slot;
    ///
    /// let s = Slot::Int(42);
    /// assert_eq!(s.type_name(), "int");
    /// ```
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
                    Error::TypeMismatch {
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
                    Error::TypeMismatch {
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
                    Error::TypeMismatch {
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
                    Error::TypeMismatch {
                        expected: "double",
                        got: type_name,
                    }
                );
            }
        }
    }
}
