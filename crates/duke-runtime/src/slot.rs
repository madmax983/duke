use crate::error::{VmError, VmResult};

/// A single JVM operand stack or local variable slot.
///
/// Note: in the JVM spec, `long` and `double` occupy two computational slots.
/// For Phase 4 we track them as single `Slot` entries for simplicity; Phase 5+
/// will introduce proper two-slot tracking via a `Padding` variant.
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
