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

    /// Construct a frame from pre-allocated buffers obtained from a frame pool.
    ///
    /// The caller is responsible for:
    /// - Sizing `locals` to `max_locals` elements and filling with default values.
    /// - Ensuring `stack` is empty before passing it here.
    ///
    /// This is the zero-allocation fast path for method calls after pool warmup.
    #[must_use]
    pub fn from_pool_bufs(locals: Vec<Slot>, stack: Vec<Slot>, max_stack: usize) -> Self {
        debug_assert!(stack.is_empty(), "pool stack must be empty on reuse");
        Self {
            locals,
            stack,
            max_stack,
        }
    }

    /// Decompose this frame into its backing Vecs for return to a frame pool.
    ///
    /// Clears the operand stack (retaining capacity). Locals are *not* cleared —
    /// the caller must resize and reinitialise the locals buffer before passing it
    /// to [`Frame::from_pool_bufs`] for reuse.
    #[must_use]
    pub fn into_pool_bufs(mut self) -> (Vec<Slot>, Vec<Slot>) {
        self.stack.clear();
        (self.locals, self.stack)
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

    /// Pop and unwrap as a non-null heap reference.
    ///
    /// # Errors
    /// Returns [`VmError::StackUnderflow`], [`VmError::TypeMismatch`], or
    /// [`VmError::NullPointerException`] if the reference is null.
    pub fn pop_ref(&mut self) -> VmResult<u64> {
        match self.pop()? {
            Slot::Reference(Some(r)) => Ok(r),
            Slot::Reference(None) => Err(VmError::NullPointerException),
            other => Err(VmError::TypeMismatch {
                expected: "reference",
                got: other.type_name(),
            }),
        }
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
            .copied()
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

    /// Clear the operand stack (used by exception handler dispatch).
    pub fn clear_stack(&mut self) {
        self.stack.clear();
    }

    /// Current operand stack depth.
    #[must_use]
    pub const fn stack_depth(&self) -> usize {
        self.stack.len()
    }

    /// Peek at the slot at a given absolute position in the operand stack (0-indexed from bottom).
    /// # Errors
    ///
    /// Returns an error if index is out of bounds.
    pub fn peek_at(&self, index: usize) -> VmResult<Slot> {
        self.stack
            .get(index)
            .copied()
            .ok_or(VmError::StackUnderflow)
    }

    /// Current depth of the operand stack.
    #[must_use]
    pub const fn stack_len(&self) -> usize {
        self.stack.len()
    }

    /// Yields all slots in locals and operand stack — used by GC root gathering.
    pub fn slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.locals.iter().chain(self.stack.iter()).copied()
    }

    /// Mutable iterator over all slots (locals + stack) — used to apply GC
    /// forwarding pointers after a minor collection.
    pub fn slots_mut(&mut self) -> impl Iterator<Item = &mut Slot> {
        self.locals.iter_mut().chain(self.stack.iter_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_pool_bufs_initialises_correctly() {
        let locals = vec![Slot::Int(0); 3];
        let stack = Vec::new();
        let mut f = Frame::from_pool_bufs(locals, stack, 4);
        assert_eq!(f.load_local(0).unwrap(), Slot::Int(0));
        assert_eq!(f.load_local(1).unwrap(), Slot::Int(0));
        f.store_local(2, Slot::Int(99)).unwrap();
        assert_eq!(f.load_local(2).unwrap(), Slot::Int(99));
        f.push(Slot::Int(7)).unwrap();
        assert_eq!(f.pop_int().unwrap(), 7);
    }

    #[test]
    fn into_pool_bufs_clears_stack_preserves_capacity() {
        let mut f = Frame::new(4, 2, vec![Slot::Int(1), Slot::Int(2)]).unwrap();
        f.push(Slot::Int(10)).unwrap();
        f.push(Slot::Int(20)).unwrap();
        let (locals, stack) = f.into_pool_bufs();
        assert_eq!(locals[0], Slot::Int(1));
        assert_eq!(locals[1], Slot::Int(2));
        assert!(stack.is_empty());
        assert!(stack.capacity() >= 2);
    }

    #[test]
    fn slots_yields_locals_and_stack() {
        use crate::Slot;
        let locals = vec![Slot::Int(1), Slot::Int(2)];
        let stack = Vec::new();
        let mut f = Frame::from_pool_bufs(locals, stack, 4);
        f.push(Slot::Int(99)).unwrap();
        let all: Vec<Slot> = f.slots().collect();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&Slot::Int(1)));
        assert!(all.contains(&Slot::Int(2)));
        assert!(all.contains(&Slot::Int(99)));
    }

    #[test]
    fn pool_round_trip_correctness() {
        let mut f = Frame::new(8, 3, vec![Slot::Int(42)]).unwrap();
        f.push(Slot::Int(1)).unwrap();
        let (mut locals_buf, stack_buf) = f.into_pool_bufs();
        locals_buf.resize(2, Slot::Int(0));
        locals_buf[0] = Slot::Int(99);
        let mut f2 = Frame::from_pool_bufs(locals_buf, stack_buf, 4);
        assert_eq!(f2.load_local(0).unwrap(), Slot::Int(99));
        assert_eq!(f2.load_local(1).unwrap(), Slot::Int(0));
        assert_eq!(f2.pop().unwrap_err(), VmError::StackUnderflow);
    }

    #[test]
    fn peek_on_nonempty_stack_returns_top() {
        let mut f = Frame::new(4, 1, vec![]).unwrap();
        f.push(Slot::Int(42)).unwrap();
        assert_eq!(f.peek(), Some(&Slot::Int(42)));
    }

    #[test]
    fn peek_on_empty_stack_returns_none() {
        let f = Frame::new(4, 1, vec![]).unwrap();
        assert_eq!(f.peek(), None);
    }

    #[test]
    fn clear_stack_removes_all_elements() {
        let mut f = Frame::new(4, 1, vec![]).unwrap();
        f.push(Slot::Int(1)).unwrap();
        f.push(Slot::Int(2)).unwrap();
        f.push(Slot::Int(3)).unwrap();
        f.clear_stack();
        assert_eq!(f.pop().unwrap_err(), VmError::StackUnderflow);
    }

    #[test]
    fn stack_depth_reflects_push_count() {
        let mut f = Frame::new(4, 1, vec![]).unwrap();
        assert_eq!(f.stack_depth(), 0);
        f.push(Slot::Int(1)).unwrap();
        assert_eq!(f.stack_depth(), 1);
        f.push(Slot::Int(2)).unwrap();
        assert_eq!(f.stack_depth(), 2);
        f.push(Slot::Int(3)).unwrap();
        assert_eq!(f.stack_depth(), 3);
    }

    #[test]
    fn stack_len_reflects_push_count() {
        let mut f = Frame::new(4, 1, vec![]).unwrap();
        assert_eq!(f.stack_len(), 0);
        f.push(Slot::Int(10)).unwrap();
        assert_eq!(f.stack_len(), 1);
        f.push(Slot::Int(20)).unwrap();
        assert_eq!(f.stack_len(), 2);
        f.push(Slot::Int(30)).unwrap();
        assert_eq!(f.stack_len(), 3);
    }

    #[test]
    fn slots_mut_yields_all_locals_and_stack() {
        let mut f = Frame::new(4, 2, vec![Slot::Int(10), Slot::Int(20)]).unwrap();
        f.push(Slot::Int(30)).unwrap();
        f.push(Slot::Int(40)).unwrap();
        assert_eq!(f.slots_mut().count(), 4); // 2 locals + 2 stack
    }

    #[test]
    fn slots_mut_mutations_are_visible() {
        let mut f = Frame::new(4, 2, vec![Slot::Int(1), Slot::Int(2)]).unwrap();
        f.push(Slot::Int(3)).unwrap();
        for slot in f.slots_mut() {
            if let Slot::Int(v) = slot {
                *v *= 10;
            }
        }
        assert_eq!(f.load_local(0).unwrap(), Slot::Int(10));
        assert_eq!(f.load_local(1).unwrap(), Slot::Int(20));
        assert_eq!(f.peek().unwrap(), &Slot::Int(30));
    }

    #[test]
    fn new_frame_too_many_args_returns_error() {
        let result = Frame::new(4, 1, vec![Slot::Int(1), Slot::Int(2)]);
        match result {
            Err(VmError::LocalOutOfBounds { index, max_locals }) => {
                assert_eq!(index, 2);
                assert_eq!(max_locals, 1);
            }
            _ => panic!("Expected VmError::LocalOutOfBounds"),
        }
    }

    #[test]
    fn load_local_out_of_bounds_returns_error() {
        let f = Frame::new(4, 1, vec![Slot::Int(42)]).unwrap();
        let err = f.load_local(1).unwrap_err();
        assert_eq!(
            err,
            VmError::LocalOutOfBounds {
                index: 1,
                max_locals: 1
            }
        );
    }

    #[test]
    fn store_local_out_of_bounds_returns_error() {
        let mut f = Frame::new(4, 1, vec![Slot::Int(42)]).unwrap();
        let err = f.store_local(1, Slot::Int(99)).unwrap_err();
        assert_eq!(
            err,
            VmError::LocalOutOfBounds {
                index: 1,
                max_locals: 1
            }
        );
    }

    #[test]
    fn peek_at_out_of_bounds_returns_error() {
        let mut f = Frame::new(4, 1, vec![]).unwrap();
        f.push(Slot::Int(1)).unwrap();
        assert_eq!(f.peek_at(0).unwrap(), Slot::Int(1));
        let err = f.peek_at(1).unwrap_err();
        assert_eq!(err, VmError::StackUnderflow);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn pop_typed_methods_succeed_on_correct_type() {
        let mut f = Frame::new(4, 1, vec![]).unwrap();

        f.push(Slot::Int(42)).unwrap();
        assert_eq!(f.pop_int().unwrap(), 42);

        f.push(Slot::Long(42)).unwrap();
        assert_eq!(f.pop_long().unwrap(), 42);

        f.push(Slot::Float(42.0)).unwrap();
        assert_eq!(f.pop_float().unwrap(), 42.0);

        f.push(Slot::Double(42.0)).unwrap();
        assert_eq!(f.pop_double().unwrap(), 42.0);
    }

    #[test]
    fn pop_typed_methods_return_type_mismatch() {
        let mut f = Frame::new(4, 1, vec![]).unwrap();

        f.push(Slot::Long(42)).unwrap();
        assert_eq!(
            f.pop_int().unwrap_err(),
            VmError::TypeMismatch {
                expected: "int",
                got: "long"
            }
        );

        f.push(Slot::Int(42)).unwrap();
        assert_eq!(
            f.pop_long().unwrap_err(),
            VmError::TypeMismatch {
                expected: "long",
                got: "int"
            }
        );

        f.push(Slot::Double(42.0)).unwrap();
        assert_eq!(
            f.pop_float().unwrap_err(),
            VmError::TypeMismatch {
                expected: "float",
                got: "double"
            }
        );

        f.push(Slot::Float(42.0)).unwrap();
        assert_eq!(
            f.pop_double().unwrap_err(),
            VmError::TypeMismatch {
                expected: "double",
                got: "float"
            }
        );

        f.push(Slot::Int(42)).unwrap();
        assert_eq!(
            f.pop_ref().unwrap_err(),
            VmError::TypeMismatch {
                expected: "reference",
                got: "int"
            }
        );
    }

    #[test]
    fn pop_typed_methods_underflow() {
        let mut f = Frame::new(4, 1, vec![]).unwrap();

        assert_eq!(f.pop_int().unwrap_err(), VmError::StackUnderflow);
        assert_eq!(f.pop_long().unwrap_err(), VmError::StackUnderflow);
        assert_eq!(f.pop_float().unwrap_err(), VmError::StackUnderflow);
        assert_eq!(f.pop_double().unwrap_err(), VmError::StackUnderflow);
        assert_eq!(f.pop_ref().unwrap_err(), VmError::StackUnderflow);
    }
}
