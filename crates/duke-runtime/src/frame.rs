use crate::{
    error::{VmError, VmResult},
    slot::Slot,
};

/// A single JVM method activation frame.
///
/// Holds the local variable array and operand stack for one method invocation.
/// The interpreter tracks the current instruction index externally and updates
/// `Frame` via the push/pop/load/store methods.
pub struct Frame {
    locals: Vec<Slot>,
    stack: Vec<Slot>,
    max_stack: usize,
}

impl Frame {
    /// Create a new frame.
    ///
    /// - `max_stack`: maximum operand stack depth from the `Code` attribute.
    /// - `max_locals`: local variable array size from the `Code` attribute.
    /// - `args`: initial values for `locals[0..args.len()]`.  Extra slots are
    ///   zero-initialised as `Slot::Int(0)`.
    ///
    /// # Errors
    ///
    /// Returns [`VmError::LocalOutOfBounds`] if `args.len() > max_locals`.
    pub fn new(max_stack: usize, max_locals: usize, args: Vec<Slot>) -> VmResult<Self> {
        if args.len() > max_locals {
            return Err(VmError::LocalOutOfBounds {
                index: args.len(),
                max_locals,
            });
        }
        let mut locals = vec![Slot::Int(0); max_locals];
        for (i, arg) in args.into_iter().enumerate() {
            locals[i] = arg;
        }
        Ok(Self {
            locals,
            stack: Vec::new(),
            max_stack,
        })
    }

    /// Push a slot onto the operand stack.
    ///
    /// # Errors
    ///
    /// Returns [`VmError::StackOverflow`] if the stack is already at `max_stack`.
    pub fn push(&mut self, slot: Slot) -> VmResult<()> {
        if self.stack.len() >= self.max_stack {
            return Err(VmError::StackOverflow);
        }
        self.stack.push(slot);
        Ok(())
    }

    /// Pop the top slot from the operand stack.
    ///
    /// # Errors
    ///
    /// Returns [`VmError::StackUnderflow`] if the stack is empty.
    pub fn pop(&mut self) -> VmResult<Slot> {
        self.stack.pop().ok_or(VmError::StackUnderflow)
    }

    /// Pop and unwrap as `i32`.
    ///
    /// # Errors
    /// Returns [`VmError::StackUnderflow`] or [`VmError::TypeMismatch`].
    pub fn pop_int(&mut self) -> VmResult<i32> {
        self.pop()?.as_int()
    }

    /// Pop and unwrap as `i64`.
    ///
    /// # Errors
    /// Returns [`VmError::StackUnderflow`] or [`VmError::TypeMismatch`].
    pub fn pop_long(&mut self) -> VmResult<i64> {
        self.pop()?.as_long()
    }

    /// Pop and unwrap as `f32`.
    ///
    /// # Errors
    /// Returns [`VmError::StackUnderflow`] or [`VmError::TypeMismatch`].
    pub fn pop_float(&mut self) -> VmResult<f32> {
        self.pop()?.as_float()
    }

    /// Pop and unwrap as `f64`.
    ///
    /// # Errors
    /// Returns [`VmError::StackUnderflow`] or [`VmError::TypeMismatch`].
    pub fn pop_double(&mut self) -> VmResult<f64> {
        self.pop()?.as_double()
    }

    /// Peek at the top of the stack without consuming it.
    #[must_use]
    pub fn peek(&self) -> Option<&Slot> {
        self.stack.last()
    }

    /// Load a local variable by index.
    ///
    /// # Errors
    ///
    /// Returns [`VmError::LocalOutOfBounds`] if `index >= max_locals`.
    pub fn load_local(&self, index: usize) -> VmResult<Slot> {
        self.locals
            .get(index)
            .cloned()
            .ok_or(VmError::LocalOutOfBounds {
                index,
                max_locals: self.locals.len(),
            })
    }

    /// Store a slot into a local variable slot.
    ///
    /// # Errors
    ///
    /// Returns [`VmError::LocalOutOfBounds`] if `index >= max_locals`.
    pub fn store_local(&mut self, index: usize, slot: Slot) -> VmResult<()> {
        if let Some(s) = self.locals.get_mut(index) {
            *s = slot;
            Ok(())
        } else {
            Err(VmError::LocalOutOfBounds {
                index,
                max_locals: self.locals.len(),
            })
        }
    }

    /// Current operand stack depth.
    #[must_use]
    pub fn stack_depth(&self) -> usize {
        self.stack.len()
    }
}
