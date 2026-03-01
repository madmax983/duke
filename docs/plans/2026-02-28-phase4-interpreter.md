# Phase 4: Bytecode Interpreter Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement `duke-runtime` (Slot/Frame/VmError types) and `duke-interpreter` (switch-dispatch execution engine) crates that can execute JVM bytecode methods containing integer/long/float/double arithmetic, locals, and control flow — no heap allocation or method calls required.

**Architecture:** `duke-runtime` is a leaf crate defining `Slot`, `Frame`, and `VmError`. `duke-interpreter` depends on `duke-runtime` + `duke-bytecode` + `duke-classfile` and exposes a single `execute()` function that takes decoded instructions, a constant pool slice, argument slots, and Code attribute bounds, returning `VmResult<Option<Slot>>`. Branch targets are resolved via a pre-built `HashMap<usize, usize>` (pc → instruction index). A `jump!` macro in the execute loop makes branch handling concise.

**Tech Stack:** Rust 2024 edition, thiserror (workspace dep), duke-bytecode Instruction enum, duke-classfile CpEntry for constant pool resolution, javac --release 21 to compile fixtures.

**Instruction subset for Phase 4:**
Constants, integer/long/float/double loads+stores, arithmetic, iinc, conversions, comparisons, conditional branches, goto, void/int/long/float/double returns, Ldc/Ldc2W. NOT implemented: object/array allocation, field access, method calls, athrow, switch.

---

### Task 1: Arithmetic.java fixture

**Files:**
- Create: `tests/fixtures/Arithmetic.java`
- Compile to: `tests/fixtures/Arithmetic.class`

**Step 1: Create `tests/fixtures/Arithmetic.java`**

```java
public class Arithmetic {
    public static int add(int a, int b)       { return a + b; }
    public static int subtract(int a, int b)  { return a - b; }
    public static int multiply(int a, int b)  { return a * b; }
    public static int divide(int a, int b)    { return a / b; }
    public static int remainder(int a, int b) { return a % b; }
    public static int negate(int n)           { return -n; }
    public static int shiftLeft(int n, int s) { return n << s; }
    public static int bitwiseAnd(int a, int b){ return a & b; }
    public static int bitwiseOr(int a, int b) { return a | b; }
    public static int bitwiseXor(int a, int b){ return a ^ b; }

    public static int max(int a, int b) {
        return a > b ? a : b;
    }

    public static int abs(int n) {
        return n < 0 ? -n : n;
    }

    public static int clamp(int v, int lo, int hi) {
        if (v < lo) return lo;
        if (v > hi) return hi;
        return v;
    }

    public static int factorial(int n) {
        int result = 1;
        for (int i = 2; i <= n; i++) {
            result *= i;
        }
        return result;
    }

    public static int fibonacci(int n) {
        if (n <= 1) return n;
        int a = 0, b = 1;
        for (int i = 2; i <= n; i++) {
            int t = a + b;
            a = b;
            b = t;
        }
        return b;
    }

    public static int sumTo(int n) {
        int sum = 0;
        for (int i = 1; i <= n; i++) {
            sum += i;
        }
        return sum;
    }

    public static long addLong(long a, long b)   { return a + b; }
    public static double addDouble(double a, double b) { return a + b; }
}
```

**Step 2: Compile it**

Run: `javac --release 21 tests/fixtures/Arithmetic.java`
Expected: creates `tests/fixtures/Arithmetic.class` with no errors.

**Step 3: Verify it compiled**

Run: `wc -c tests/fixtures/Arithmetic.class`
Expected: some non-zero size (typically 1200-2000 bytes for this class).

**Step 4: Commit**

```bash
git add tests/fixtures/Arithmetic.java tests/fixtures/Arithmetic.class
git commit -m "test(fixtures): add Arithmetic.java fixture for Phase 4 interpreter tests"
```

---

### Task 2: `duke-runtime` crate — Slot, Frame, VmError

**Files:**
- Create: `crates/duke-runtime/Cargo.toml`
- Create: `crates/duke-runtime/src/lib.rs`
- Create: `crates/duke-runtime/src/slot.rs`
- Create: `crates/duke-runtime/src/frame.rs`
- Create: `crates/duke-runtime/src/error.rs`
- Modify: `Cargo.toml` (workspace root — add member)

**Step 1: Add to workspace**

In root `Cargo.toml`, add `"crates/duke-runtime"` to `[workspace]` members.

**Step 2: Create `crates/duke-runtime/Cargo.toml`**

```toml
[package]
name = "duke-runtime"
version.workspace = true
edition.workspace = true
description = "Duke JVM - Runtime data types (Slot, Frame, VmError)"

[dependencies]
thiserror.workspace = true
```

**Step 3: Write the failing test first** (in `crates/duke-runtime/src/lib.rs`)

```rust
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
        let frame = Frame::new(4, 3, vec![Slot::Int(1), Slot::Int(2), Slot::Int(3)])
            .expect("new frame");
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
}
```

**Step 4: Run to verify it fails**

Run: `cargo test -p duke-runtime`
Expected: FAIL — module items not defined yet.

**Step 5: Create `crates/duke-runtime/src/error.rs`**

```rust
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum VmError {
    #[error("operand stack overflow")]
    StackOverflow,

    #[error("operand stack underflow")]
    StackUnderflow,

    #[error("local variable index {index} out of bounds (max_locals={max_locals})")]
    LocalOutOfBounds { index: usize, max_locals: usize },

    #[error("integer division by zero")]
    DivisionByZero,

    #[error("invalid branch target: pc={pc}")]
    InvalidBranchTarget { pc: usize },

    #[error("fell off end of bytecode without a return instruction")]
    FellOffEnd,

    #[error("type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: &'static str, got: &'static str },

    #[error("unimplemented instruction: {mnemonic}")]
    Unimplemented { mnemonic: &'static str },

    #[error("invalid constant pool index {index}")]
    InvalidCpIndex { index: usize },
}

pub type VmResult<T> = Result<T, VmError>;
```

**Step 6: Create `crates/duke-runtime/src/slot.rs`**

```rust
use crate::error::{VmError, VmResult};

/// A single JVM operand stack or local variable slot.
///
/// Note: in the JVM spec, `long` and `double` occupy two computational slots.
/// For Phase 4 we track them as single `Slot` entries for simplicity; Phase 5+
/// will introduce proper two-slot tracking via a `Padding` variant.
#[derive(Debug, Clone, PartialEq)]
pub enum Slot {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    /// Object reference (null when `None`). Used for `aconst_null`.
    Reference(Option<u64>),
    /// Return address pushed by `jsr` (legacy subroutine support).
    ReturnAddress(usize),
}

impl Slot {
    pub fn as_int(&self) -> VmResult<i32> {
        if let Self::Int(v) = self { Ok(*v) }
        else { Err(VmError::TypeMismatch { expected: "int", got: self.type_name() }) }
    }

    pub fn as_long(&self) -> VmResult<i64> {
        if let Self::Long(v) = self { Ok(*v) }
        else { Err(VmError::TypeMismatch { expected: "long", got: self.type_name() }) }
    }

    pub fn as_float(&self) -> VmResult<f32> {
        if let Self::Float(v) = self { Ok(*v) }
        else { Err(VmError::TypeMismatch { expected: "float", got: self.type_name() }) }
    }

    pub fn as_double(&self) -> VmResult<f64> {
        if let Self::Double(v) = self { Ok(*v) }
        else { Err(VmError::TypeMismatch { expected: "double", got: self.type_name() }) }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::Int(_)           => "int",
            Self::Long(_)          => "long",
            Self::Float(_)         => "float",
            Self::Double(_)        => "double",
            Self::Reference(_)     => "reference",
            Self::ReturnAddress(_) => "returnAddress",
        }
    }
}
```

**Step 7: Create `crates/duke-runtime/src/frame.rs`**

```rust
use crate::{
    error::{VmError, VmResult},
    slot::Slot,
};

/// A single JVM method activation frame.
///
/// Holds the local variable array and operand stack for one method invocation.
/// The `pc` field tracks the current instruction index (not byte offset) within
/// the decoded instruction stream — the interpreter owns this value and updates it.
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
    /// - `args`: initial values for locals[0..args.len()].  Extra slots are
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
        Ok(Self { locals, stack: Vec::new(), max_stack })
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
    pub fn pop_int(&mut self) -> VmResult<i32> {
        self.pop()?.as_int()
    }

    /// Pop and unwrap as `i64`.
    pub fn pop_long(&mut self) -> VmResult<i64> {
        self.pop()?.as_long()
    }

    /// Pop and unwrap as `f32`.
    pub fn pop_float(&mut self) -> VmResult<f32> {
        self.pop()?.as_float()
    }

    /// Pop and unwrap as `f64`.
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
        self.locals.get(index).cloned().ok_or_else(|| VmError::LocalOutOfBounds {
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
        let len = self.locals.len();
        self.locals.get_mut(index).map(|s| *s = slot).ok_or(VmError::LocalOutOfBounds {
            index,
            max_locals: len,
        })
    }

    /// Current operand stack depth.
    #[must_use]
    pub fn stack_depth(&self) -> usize {
        self.stack.len()
    }
}
```

**Step 8: Create `crates/duke-runtime/src/lib.rs`**

```rust
pub mod error;
pub mod frame;
pub mod slot;

pub use error::{VmError, VmResult};
pub use frame::Frame;
pub use slot::Slot;

// Tests inline (already written above)
#[cfg(test)]
mod tests { ... }  // paste in the tests from Step 3
```

**Step 9: Run tests**

Run: `cargo test -p duke-runtime`
Expected: 5 tests pass.

**Step 10: Commit**

```bash
git add crates/duke-runtime/ Cargo.toml Cargo.lock
git commit -m "feat(runtime): add duke-runtime crate with Slot, Frame, and VmError types"
```

---

### Task 3: `duke-interpreter` crate scaffolding

**Files:**
- Create: `crates/duke-interpreter/Cargo.toml`
- Create: `crates/duke-interpreter/src/lib.rs`
- Modify: `Cargo.toml` (workspace root — add member)

**Step 1: Add to workspace**

In root `Cargo.toml`, add `"crates/duke-interpreter"` to `[workspace]` members.

**Step 2: Create `crates/duke-interpreter/Cargo.toml`**

```toml
[package]
name = "duke-interpreter"
version.workspace = true
edition.workspace = true
description = "Duke JVM - Switch-dispatch bytecode interpreter"

[dependencies]
duke-runtime = { path = "../duke-runtime" }
duke-bytecode = { path = "../duke-bytecode" }
duke-classfile = { path = "../duke-classfile" }

[dev-dependencies]
duke-loader = { path = "../duke-loader" }
```

**Step 3: Write the first failing test** (in `crates/duke-interpreter/src/lib.rs`)

This tests a hand-crafted instruction stream: `iconst_1, ireturn`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use duke_bytecode::Instruction;
    use duke_runtime::Slot;

    #[test]
    fn execute_iconst_ireturn() {
        let instructions = vec![
            (0, Instruction::Iconst1),
            (1, Instruction::Ireturn),
        ];
        let result = execute(&instructions, &[], vec![], 2, 1).expect("should execute");
        assert_eq!(result, Some(Slot::Int(1)));
    }
}
```

**Step 4: Run to verify it fails**

Run: `cargo test -p duke-interpreter`
Expected: FAIL — `execute` not defined.

**Step 5: Create the `execute` function stub in `lib.rs`**

```rust
use std::collections::HashMap;

use duke_bytecode::Instruction;
use duke_classfile::types::CpEntry;
use duke_runtime::{Frame, Slot, VmError, VmResult};

/// Execute a decoded instruction stream.
///
/// # Parameters
/// - `instructions`: output of `duke_bytecode::decode`
/// - `cp`: constant pool slice from the parsed `ClassFile`
/// - `args`: initial local variable values (method arguments)
/// - `max_stack`: `Code.max_stack`
/// - `max_locals`: `Code.max_locals`
///
/// # Returns
/// `Ok(Some(Slot))` for value-returning methods, `Ok(None)` for void.
///
/// # Errors
/// Returns [`VmError`] on any execution fault.
pub fn execute(
    instructions: &[(usize, Instruction)],
    cp: &[Option<CpEntry>],
    args: Vec<Slot>,
    max_stack: u16,
    max_locals: u16,
) -> VmResult<Option<Slot>> {
    // Build PC → instruction-index map for O(1) branch resolution
    let pc_to_idx: HashMap<usize, usize> =
        instructions.iter().enumerate().map(|(i, &(pc, _))| (pc, i)).collect();

    let mut frame = Frame::new(max_stack as usize, max_locals as usize, args)?;
    let mut idx: usize = 0;

    loop {
        let Some(&(pc, ref instr)) = instructions.get(idx) else {
            return Err(VmError::FellOffEnd);
        };

        // Macro: jump to a PC-relative branch target
        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64 + $offset as i64) as usize;
                idx = *pc_to_idx
                    .get(&target)
                    .ok_or(VmError::InvalidBranchTarget { pc: target })?;
                continue;
            }};
        }

        match instr {
            Instruction::Nop => {}

            Instruction::Iconst1 => frame.push(Slot::Int(1))?,
            // ... more to come in Tasks 4–6

            Instruction::Ireturn => {
                return Ok(Some(Slot::Int(frame.pop_int()?)));
            }
            Instruction::Return => return Ok(None),

            other => {
                return Err(VmError::Unimplemented { mnemonic: other.mnemonic() });
            }
        }

        idx += 1;
        let _ = (pc, &jump); // suppress unused warnings until jump is used
    }
}
```

**Step 6: Run test**

Run: `cargo test -p duke-interpreter`
Expected: PASS (1 test).

**Step 7: Commit**

```bash
git add crates/duke-interpreter/ Cargo.toml
git commit -m "feat(interpreter): scaffold duke-interpreter crate with execute() stub"
```

---

### Task 4: Integer constants, loads, stores

**Files:** Modify `crates/duke-interpreter/src/lib.rs`

**Step 1: Add tests**

```rust
#[test]
fn execute_bipush_istore_iload() {
    // bipush 42, istore_0, iload_0, ireturn
    let instructions = vec![
        (0, Instruction::Bipush(42)),
        (2, Instruction::Istore0),
        (3, Instruction::Iload0),
        (4, Instruction::Ireturn),
    ];
    let result = execute(&instructions, &[], vec![], 2, 2).unwrap();
    assert_eq!(result, Some(Slot::Int(42)));
}

#[test]
fn execute_loads_args() {
    // iload_0, iload_1, ireturn (returns first arg)
    let instructions = vec![
        (0, Instruction::Iload0),
        (1, Instruction::Iload1),
        (2, Instruction::Pop),
        (3, Instruction::Ireturn),
    ];
    let result = execute(&instructions, &[], vec![Slot::Int(99), Slot::Int(0)], 2, 2).unwrap();
    assert_eq!(result, Some(Slot::Int(99)));
}

#[test]
fn execute_sipush() {
    let instructions = vec![
        (0, Instruction::Sipush(1000)),
        (3, Instruction::Ireturn),
    ];
    let result = execute(&instructions, &[], vec![], 2, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(1000)));
}
```

**Step 2: Run to verify they fail**

Run: `cargo test -p duke-interpreter`
Expected: FAIL — Bipush and others return `Unimplemented`.

**Step 3: Implement all integer constants, loads, stores in the `match` block**

Replace the stub `match` block with the full integer subset. **Add ALL of these in one edit** — they are mechanical:

```rust
// ---- Constants ----
Instruction::Nop => {}
Instruction::AconstNull => frame.push(Slot::Reference(None))?,
Instruction::IconstM1 => frame.push(Slot::Int(-1))?,
Instruction::Iconst0  => frame.push(Slot::Int(0))?,
Instruction::Iconst1  => frame.push(Slot::Int(1))?,
Instruction::Iconst2  => frame.push(Slot::Int(2))?,
Instruction::Iconst3  => frame.push(Slot::Int(3))?,
Instruction::Iconst4  => frame.push(Slot::Int(4))?,
Instruction::Iconst5  => frame.push(Slot::Int(5))?,
Instruction::Lconst0  => frame.push(Slot::Long(0))?,
Instruction::Lconst1  => frame.push(Slot::Long(1))?,
Instruction::Fconst0  => frame.push(Slot::Float(0.0))?,
Instruction::Fconst1  => frame.push(Slot::Float(1.0))?,
Instruction::Fconst2  => frame.push(Slot::Float(2.0))?,
Instruction::Dconst0  => frame.push(Slot::Double(0.0))?,
Instruction::Dconst1  => frame.push(Slot::Double(1.0))?,
Instruction::Bipush(v) => frame.push(Slot::Int(i32::from(*v)))?,
Instruction::Sipush(v) => frame.push(Slot::Int(i32::from(*v)))?,

// ---- Ldc: load constant pool entry onto stack ----
Instruction::Ldc(raw_idx) => {
    ldc_push(&mut frame, cp, usize::from(*raw_idx))?;
}
Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
    ldc_push(&mut frame, cp, usize::from(cp_idx.0))?;
}

// ---- Loads ----
Instruction::Iload(i) | Instruction::Lload(i) | Instruction::Fload(i)
| Instruction::Dload(i) | Instruction::Aload(i) => {
    let slot = frame.load_local(usize::from(*i))?;
    frame.push(slot)?;
}
Instruction::Iload0 | Instruction::Lload0 | Instruction::Fload0
| Instruction::Dload0 | Instruction::Aload0 => {
    let s = frame.load_local(0)?; frame.push(s)?;
}
Instruction::Iload1 | Instruction::Lload1 | Instruction::Fload1
| Instruction::Dload1 | Instruction::Aload1 => {
    let s = frame.load_local(1)?; frame.push(s)?;
}
Instruction::Iload2 | Instruction::Lload2 | Instruction::Fload2
| Instruction::Dload2 | Instruction::Aload2 => {
    let s = frame.load_local(2)?; frame.push(s)?;
}
Instruction::Iload3 | Instruction::Lload3 | Instruction::Fload3
| Instruction::Dload3 | Instruction::Aload3 => {
    let s = frame.load_local(3)?; frame.push(s)?;
}
Instruction::IloadW(i) | Instruction::LloadW(i) | Instruction::FloadW(i)
| Instruction::DloadW(i) | Instruction::AloadW(i) => {
    let slot = frame.load_local(usize::from(*i))?;
    frame.push(slot)?;
}

// ---- Stores ----
Instruction::Istore(i) | Instruction::Lstore(i) | Instruction::Fstore(i)
| Instruction::Dstore(i) | Instruction::Astore(i) => {
    let v = frame.pop()?; frame.store_local(usize::from(*i), v)?;
}
Instruction::Istore0 | Instruction::Lstore0 | Instruction::Fstore0
| Instruction::Dstore0 | Instruction::Astore0 => {
    let v = frame.pop()?; frame.store_local(0, v)?;
}
Instruction::Istore1 | Instruction::Lstore1 | Instruction::Fstore1
| Instruction::Dstore1 | Instruction::Astore1 => {
    let v = frame.pop()?; frame.store_local(1, v)?;
}
Instruction::Istore2 | Instruction::Lstore2 | Instruction::Fstore2
| Instruction::Dstore2 | Instruction::Astore2 => {
    let v = frame.pop()?; frame.store_local(2, v)?;
}
Instruction::Istore3 | Instruction::Lstore3 | Instruction::Fstore3
| Instruction::Dstore3 | Instruction::Astore3 => {
    let v = frame.pop()?; frame.store_local(3, v)?;
}
Instruction::IstoreW(i) | Instruction::LstoreW(i) | Instruction::FstoreW(i)
| Instruction::DstoreW(i) | Instruction::AstoreW(i) => {
    let v = frame.pop()?; frame.store_local(usize::from(*i), v)?;
}

// ---- Stack ops ----
Instruction::Pop  => { frame.pop()?; }
Instruction::Pop2 => { frame.pop()?; frame.pop()?; }
Instruction::Dup  => {
    let v = frame.pop()?; frame.push(v.clone())?; frame.push(v)?;
}
Instruction::Swap => {
    let a = frame.pop()?; let b = frame.pop()?;
    frame.push(a)?; frame.push(b)?;
}
```

And add the `ldc_push` helper function outside `execute`:

```rust
fn ldc_push(frame: &mut Frame, cp: &[Option<CpEntry>], idx: usize) -> VmResult<()> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Integer(v)) => frame.push(Slot::Int(*v)),
        Some(CpEntry::Float(v))   => frame.push(Slot::Float(*v)),
        Some(CpEntry::Long(v))    => frame.push(Slot::Long(*v)),
        Some(CpEntry::Double(v))  => frame.push(Slot::Double(*v)),
        _ => Err(VmError::InvalidCpIndex { index: idx }),
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p duke-interpreter`
Expected: 4 tests pass.

**Step 5: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): integer constants, loads/stores, stack ops"
```

---

### Task 5: Integer and long arithmetic

**Files:** Modify `crates/duke-interpreter/src/lib.rs`

**Step 1: Add the tests**

```rust
#[test]
fn hand_coded_add() {
    // iload_0, iload_1, iadd, ireturn
    let instrs = vec![
        (0, Instruction::Iload0),
        (1, Instruction::Iload1),
        (2, Instruction::Iadd),
        (3, Instruction::Ireturn),
    ];
    let result = execute(&instrs, &[], vec![Slot::Int(3), Slot::Int(4)], 2, 2).unwrap();
    assert_eq!(result, Some(Slot::Int(7)));
}

#[test]
fn hand_coded_div_by_zero() {
    let instrs = vec![
        (0, Instruction::Iload0),
        (1, Instruction::Iload1),
        (2, Instruction::Idiv),
        (3, Instruction::Ireturn),
    ];
    let err = execute(&instrs, &[], vec![Slot::Int(10), Slot::Int(0)], 2, 2).unwrap_err();
    assert!(matches!(err, VmError::DivisionByZero));
}

#[test]
fn hand_coded_long_add() {
    let instrs = vec![
        (0, Instruction::Lload0),
        (1, Instruction::Lload1),
        (2, Instruction::Ladd),
        (3, Instruction::Lreturn),
    ];
    let result = execute(
        &instrs, &[],
        vec![Slot::Long(1_000_000_000), Slot::Long(2_000_000_000)],
        2, 2
    ).unwrap();
    assert_eq!(result, Some(Slot::Long(3_000_000_000)));
}
```

**Step 2: Run to verify they fail**

**Step 3: Add all integer and long arithmetic to the `match` block**

```rust
// ---- Integer arithmetic ----
Instruction::Iadd => { let b=frame.pop_int()?; let a=frame.pop_int()?; frame.push(Slot::Int(a.wrapping_add(b)))?; }
Instruction::Isub => { let b=frame.pop_int()?; let a=frame.pop_int()?; frame.push(Slot::Int(a.wrapping_sub(b)))?; }
Instruction::Imul => { let b=frame.pop_int()?; let a=frame.pop_int()?; frame.push(Slot::Int(a.wrapping_mul(b)))?; }
Instruction::Idiv => {
    let b = frame.pop_int()?;
    let a = frame.pop_int()?;
    if b == 0 { return Err(VmError::DivisionByZero); }
    frame.push(Slot::Int(a.wrapping_div(b)))?;
}
Instruction::Irem => {
    let b = frame.pop_int()?;
    let a = frame.pop_int()?;
    if b == 0 { return Err(VmError::DivisionByZero); }
    frame.push(Slot::Int(a.wrapping_rem(b)))?;
}
Instruction::Ineg  => { let a = frame.pop_int()?;  frame.push(Slot::Int(a.wrapping_neg()))?; }
Instruction::Ishl  => { let s=frame.pop_int()?; let a=frame.pop_int()?; frame.push(Slot::Int(a.wrapping_shl(s as u32 & 0x1F)))?; }
Instruction::Ishr  => { let s=frame.pop_int()?; let a=frame.pop_int()?; frame.push(Slot::Int(a.wrapping_shr(s as u32 & 0x1F)))?; }
Instruction::Iushr => { let s=frame.pop_int()?; let a=frame.pop_int()?; frame.push(Slot::Int(((a as u32).wrapping_shr(s as u32 & 0x1F)) as i32))?; }
Instruction::Iand  => { let b=frame.pop_int()?; let a=frame.pop_int()?; frame.push(Slot::Int(a & b))?; }
Instruction::Ior   => { let b=frame.pop_int()?; let a=frame.pop_int()?; frame.push(Slot::Int(a | b))?; }
Instruction::Ixor  => { let b=frame.pop_int()?; let a=frame.pop_int()?; frame.push(Slot::Int(a ^ b))?; }

// ---- Long arithmetic ----
Instruction::Ladd => { let b=frame.pop_long()?; let a=frame.pop_long()?; frame.push(Slot::Long(a.wrapping_add(b)))?; }
Instruction::Lsub => { let b=frame.pop_long()?; let a=frame.pop_long()?; frame.push(Slot::Long(a.wrapping_sub(b)))?; }
Instruction::Lmul => { let b=frame.pop_long()?; let a=frame.pop_long()?; frame.push(Slot::Long(a.wrapping_mul(b)))?; }
Instruction::Ldiv => {
    let b = frame.pop_long()?;
    let a = frame.pop_long()?;
    if b == 0 { return Err(VmError::DivisionByZero); }
    frame.push(Slot::Long(a.wrapping_div(b)))?;
}
Instruction::Lrem => {
    let b = frame.pop_long()?;
    let a = frame.pop_long()?;
    if b == 0 { return Err(VmError::DivisionByZero); }
    frame.push(Slot::Long(a.wrapping_rem(b)))?;
}
Instruction::Lneg  => { let a=frame.pop_long()?;  frame.push(Slot::Long(a.wrapping_neg()))?; }
Instruction::Lshl  => { let s=frame.pop_int()?;  let a=frame.pop_long()?; frame.push(Slot::Long(a.wrapping_shl(s as u32 & 0x3F)))?; }
Instruction::Lshr  => { let s=frame.pop_int()?;  let a=frame.pop_long()?; frame.push(Slot::Long(a.wrapping_shr(s as u32 & 0x3F)))?; }
Instruction::Lushr => { let s=frame.pop_int()?;  let a=frame.pop_long()?; frame.push(Slot::Long(((a as u64).wrapping_shr(s as u32 & 0x3F)) as i64))?; }
Instruction::Land  => { let b=frame.pop_long()?; let a=frame.pop_long()?; frame.push(Slot::Long(a & b))?; }
Instruction::Lor   => { let b=frame.pop_long()?; let a=frame.pop_long()?; frame.push(Slot::Long(a | b))?; }
Instruction::Lxor  => { let b=frame.pop_long()?; let a=frame.pop_long()?; frame.push(Slot::Long(a ^ b))?; }
Instruction::Lcmp  => {
    let b=frame.pop_long()?; let a=frame.pop_long()?;
    frame.push(Slot::Int(a.cmp(&b) as i32))?;
}

// ---- Float arithmetic ----
Instruction::Fadd => { let b=frame.pop_float()?;  let a=frame.pop_float()?;  frame.push(Slot::Float(a + b))?; }
Instruction::Fsub => { let b=frame.pop_float()?;  let a=frame.pop_float()?;  frame.push(Slot::Float(a - b))?; }
Instruction::Fmul => { let b=frame.pop_float()?;  let a=frame.pop_float()?;  frame.push(Slot::Float(a * b))?; }
Instruction::Fdiv => { let b=frame.pop_float()?;  let a=frame.pop_float()?;  frame.push(Slot::Float(a / b))?; }
Instruction::Frem => { let b=frame.pop_float()?;  let a=frame.pop_float()?;  frame.push(Slot::Float(a % b))?; }
Instruction::Fneg => { let a=frame.pop_float()?;  frame.push(Slot::Float(-a))?; }
Instruction::Fcmpl | Instruction::Fcmpg => {
    let b=frame.pop_float()?; let a=frame.pop_float()?;
    let r = if a > b { 1 } else if a < b { -1 } else if a == b { 0 }
            else if matches!(instr, Instruction::Fcmpg) { 1 } else { -1 };
    frame.push(Slot::Int(r))?;
}

// ---- Double arithmetic ----
Instruction::Dadd  => { let b=frame.pop_double()?; let a=frame.pop_double()?; frame.push(Slot::Double(a + b))?; }
Instruction::Dsub  => { let b=frame.pop_double()?; let a=frame.pop_double()?; frame.push(Slot::Double(a - b))?; }
Instruction::Dmul  => { let b=frame.pop_double()?; let a=frame.pop_double()?; frame.push(Slot::Double(a * b))?; }
Instruction::Ddiv  => { let b=frame.pop_double()?; let a=frame.pop_double()?; frame.push(Slot::Double(a / b))?; }
Instruction::Drem  => { let b=frame.pop_double()?; let a=frame.pop_double()?; frame.push(Slot::Double(a % b))?; }
Instruction::Dneg  => { let a=frame.pop_double()?; frame.push(Slot::Double(-a))?; }
Instruction::Dcmpl | Instruction::Dcmpg => {
    let b=frame.pop_double()?; let a=frame.pop_double()?;
    let r = if a > b { 1 } else if a < b { -1 } else if a == b { 0 }
            else if matches!(instr, Instruction::Dcmpg) { 1 } else { -1 };
    frame.push(Slot::Int(r))?;
}

// ---- iinc (no stack change) ----
Instruction::Iinc { index, value } => {
    let v = frame.load_local(usize::from(*index))?.as_int()?;
    frame.store_local(usize::from(*index), Slot::Int(v.wrapping_add(i32::from(*value))))?;
}
Instruction::IincW { index, value } => {
    let v = frame.load_local(usize::from(*index))?.as_int()?;
    frame.store_local(usize::from(*index), Slot::Int(v.wrapping_add(i32::from(*value))))?;
}

// ---- Type conversions ----
Instruction::I2l  => { let v=frame.pop_int()?;    frame.push(Slot::Long(i64::from(v)))?; }
Instruction::I2f  => { let v=frame.pop_int()?;    frame.push(Slot::Float(v as f32))?; }
Instruction::I2d  => { let v=frame.pop_int()?;    frame.push(Slot::Double(f64::from(v)))?; }
Instruction::L2i  => { let v=frame.pop_long()?;   frame.push(Slot::Int(v as i32))?; }
Instruction::L2f  => { let v=frame.pop_long()?;   frame.push(Slot::Float(v as f32))?; }
Instruction::L2d  => { let v=frame.pop_long()?;   frame.push(Slot::Double(v as f64))?; }
Instruction::F2i  => { let v=frame.pop_float()?;  frame.push(Slot::Int(v as i32))?; }
Instruction::F2l  => { let v=frame.pop_float()?;  frame.push(Slot::Long(v as i64))?; }
Instruction::F2d  => { let v=frame.pop_float()?;  frame.push(Slot::Double(f64::from(v)))?; }
Instruction::D2i  => { let v=frame.pop_double()?; frame.push(Slot::Int(v as i32))?; }
Instruction::D2l  => { let v=frame.pop_double()?; frame.push(Slot::Long(v as i64))?; }
Instruction::D2f  => { let v=frame.pop_double()?; frame.push(Slot::Float(v as f32))?; }
Instruction::I2b  => { let v=frame.pop_int()?;    frame.push(Slot::Int(v as i8  as i32))?; }
Instruction::I2c  => { let v=frame.pop_int()?;    frame.push(Slot::Int(v as u16 as i32))?; }
Instruction::I2s  => { let v=frame.pop_int()?;    frame.push(Slot::Int(v as i16 as i32))?; }

// ---- Returns ----
Instruction::Return  => return Ok(None),
Instruction::Ireturn => return Ok(Some(Slot::Int(frame.pop_int()?))),
Instruction::Lreturn => return Ok(Some(Slot::Long(frame.pop_long()?))),
Instruction::Freturn => return Ok(Some(Slot::Float(frame.pop_float()?))),
Instruction::Dreturn => return Ok(Some(Slot::Double(frame.pop_double()?))),
```

**Step 4: Run tests**

Run: `cargo test -p duke-interpreter`
Expected: all pass.

**Step 5: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): integer/long/float/double arithmetic, iinc, conversions, returns"
```

---

### Task 6: Branches and goto

**Files:** Modify `crates/duke-interpreter/src/lib.rs`

**Step 1: Add tests**

```rust
#[test]
fn hand_coded_ifeq_taken() {
    // push 0, ifeq +3 (jump to ireturn at pc=6), push 1, ireturn@3, iconst_2@6, ireturn@7
    // iconst_0 (pc=0), ifeq +5 (pc=1, target=6), iconst_1 (pc=4), ireturn (pc=5),
    // iconst_2 (pc=6), ireturn (pc=7)
    let instrs = vec![
        (0, Instruction::Iconst0),
        (1, Instruction::Ifeq(5)),   // if(0==0) jump to pc=6
        (4, Instruction::Iconst1),
        (5, Instruction::Ireturn),
        (6, Instruction::Iconst2),
        (7, Instruction::Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(2)));
}

#[test]
fn hand_coded_ifeq_not_taken() {
    let instrs = vec![
        (0, Instruction::Iconst1),    // push 1
        (1, Instruction::Ifeq(5)),    // 1 != 0 so NOT taken
        (4, Instruction::Iconst1),    // push 1 (this is reached)
        (5, Instruction::Ireturn),
        (6, Instruction::Iconst2),
        (7, Instruction::Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn hand_coded_goto() {
    // push 5, goto +4 (skip past pop), pop, iconst_0, ireturn; then iconst_5, ireturn
    let instrs = vec![
        (0, Instruction::Iconst5),
        (1, Instruction::Goto(4)),   // jump to pc=5
        (4, Instruction::Pop),       // skipped
        (5, Instruction::Ireturn),   // returns 5
    ];
    let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(5)));
}
```

**Step 2: Run to verify they fail**

**Step 3: Add branches to the `match` block**

```rust
// ---- Unconditional branch ----
Instruction::Goto(offset)  => { jump!(*offset as i64); }
Instruction::GotoW(offset) => { jump!(*offset as i64); }

// ---- Conditional branches: pop 1 int ----
Instruction::Ifeq(offset)     => { if frame.pop_int()? == 0 { jump!(*offset as i64); } }
Instruction::Ifne(offset)     => { if frame.pop_int()? != 0 { jump!(*offset as i64); } }
Instruction::Iflt(offset)     => { if frame.pop_int()? <  0 { jump!(*offset as i64); } }
Instruction::Ifge(offset)     => { if frame.pop_int()? >= 0 { jump!(*offset as i64); } }
Instruction::Ifgt(offset)     => { if frame.pop_int()? >  0 { jump!(*offset as i64); } }
Instruction::Ifle(offset)     => { if frame.pop_int()? <= 0 { jump!(*offset as i64); } }
Instruction::Ifnull(offset)   => {
    if matches!(frame.pop()?, Slot::Reference(None)) { jump!(*offset as i64); }
}
Instruction::Ifnonnull(offset) => {
    if !matches!(frame.pop()?, Slot::Reference(None)) { jump!(*offset as i64); }
}

// ---- Conditional branches: pop 2 ints ----
Instruction::IfIcmpeq(offset) => { let b=frame.pop_int()?; let a=frame.pop_int()?; if a == b { jump!(*offset as i64); } }
Instruction::IfIcmpne(offset) => { let b=frame.pop_int()?; let a=frame.pop_int()?; if a != b { jump!(*offset as i64); } }
Instruction::IfIcmplt(offset) => { let b=frame.pop_int()?; let a=frame.pop_int()?; if a <  b { jump!(*offset as i64); } }
Instruction::IfIcmpge(offset) => { let b=frame.pop_int()?; let a=frame.pop_int()?; if a >= b { jump!(*offset as i64); } }
Instruction::IfIcmpgt(offset) => { let b=frame.pop_int()?; let a=frame.pop_int()?; if a >  b { jump!(*offset as i64); } }
Instruction::IfIcmple(offset) => { let b=frame.pop_int()?; let a=frame.pop_int()?; if a <= b { jump!(*offset as i64); } }
Instruction::IfAcmpeq(offset) => { frame.pop()?; frame.pop()?; /* simplified */ let _ = offset; }
Instruction::IfAcmpne(offset) => { frame.pop()?; frame.pop()?; /* simplified */ let _ = offset; }
```

**Step 4: Run tests**

Run: `cargo test -p duke-interpreter`
Expected: all pass.

**Step 5: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): conditional/unconditional branches"
```

---

### Task 7: Integration tests against Arithmetic.class

Now wire the full pipeline: `duke-loader` → `duke-classfile` → `duke-bytecode` → `duke-interpreter`.

**Files:** Modify `crates/duke-interpreter/src/lib.rs` (test section)

**Step 1: Add integration test helpers** (after the unit tests)

```rust
// -----------------------------------------------------------------------
// Integration tests: load Arithmetic.class and execute real bytecode
// -----------------------------------------------------------------------

fn fixture(name: &str) -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../tests/fixtures");
    p.push(name);
    p
}

/// Find a static method by name and execute it with the given integer args.
/// Returns the integer result.
fn run_static_int(class_name: &str, method_name: &str, args: Vec<i32>) -> i32 {
    use duke_bytecode::decode;
    use duke_classfile::{
        parse,
        types::{AttributeData, CpEntry},
    };

    let bytes = std::fs::read(fixture(class_name)).expect("fixture not found");
    let cf = parse(&bytes).expect("parse failed");

    let method = cf
        .methods
        .iter()
        .find(|m| {
            matches!(
                &cf.constant_pool[m.name_index.0 as usize],
                Some(CpEntry::Utf8(s)) if s == method_name
            )
        })
        .unwrap_or_else(|| panic!("method '{method_name}' not found"));

    let code = method
        .attributes
        .iter()
        .find_map(|a| if let AttributeData::Code(c) = &a.data { Some(c) } else { None })
        .expect("no Code attribute");

    let instructions = decode(&code.code).expect("decode failed");
    let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();

    match execute(&instructions, &cf.constant_pool, slots,
                   code.max_stack, code.max_locals).expect("execute failed")
    {
        Some(Slot::Int(v)) => v,
        other => panic!("unexpected result: {other:?}"),
    }
}
```

**Step 2: Add integration tests**

```rust
// Basic arithmetic
#[test] fn int_add()      { assert_eq!(run_static_int("Arithmetic.class", "add",      vec![3, 4]),   7); }
#[test] fn int_subtract() { assert_eq!(run_static_int("Arithmetic.class", "subtract", vec![10, 3]),  7); }
#[test] fn int_multiply() { assert_eq!(run_static_int("Arithmetic.class", "multiply", vec![3, 4]),  12); }
#[test] fn int_divide()   { assert_eq!(run_static_int("Arithmetic.class", "divide",   vec![10, 2]),  5); }
#[test] fn int_remainder(){ assert_eq!(run_static_int("Arithmetic.class", "remainder",vec![10, 3]),  1); }
#[test] fn int_negate()   { assert_eq!(run_static_int("Arithmetic.class", "negate",   vec![-5]),     5); }
#[test] fn int_shift_left(){ assert_eq!(run_static_int("Arithmetic.class", "shiftLeft",vec![1, 4]), 16); }
#[test] fn int_bitwise_and(){assert_eq!(run_static_int("Arithmetic.class","bitwiseAnd",vec![0b1111,0b1010]), 0b1010); }
#[test] fn int_bitwise_or(){ assert_eq!(run_static_int("Arithmetic.class","bitwiseOr", vec![0b1111,0b1010]), 0b1111); }

// Conditionals
#[test] fn int_max_a_wins(){ assert_eq!(run_static_int("Arithmetic.class", "max", vec![7, 3]),  7); }
#[test] fn int_max_b_wins(){ assert_eq!(run_static_int("Arithmetic.class", "max", vec![3, 7]),  7); }
#[test] fn int_abs_neg()   { assert_eq!(run_static_int("Arithmetic.class", "abs", vec![-5]),    5); }
#[test] fn int_abs_pos()   { assert_eq!(run_static_int("Arithmetic.class", "abs", vec![ 5]),    5); }
#[test] fn int_clamp_mid() { assert_eq!(run_static_int("Arithmetic.class", "clamp", vec![5, 1, 10]), 5); }
#[test] fn int_clamp_lo()  { assert_eq!(run_static_int("Arithmetic.class", "clamp", vec![0, 1, 10]), 1); }
#[test] fn int_clamp_hi()  { assert_eq!(run_static_int("Arithmetic.class", "clamp", vec![15,1, 10]),10); }

// Control flow / loops
#[test] fn int_factorial_0(){ assert_eq!(run_static_int("Arithmetic.class", "factorial", vec![0]), 1); }
#[test] fn int_factorial_5(){ assert_eq!(run_static_int("Arithmetic.class", "factorial", vec![5]), 120); }
#[test] fn int_factorial_10(){assert_eq!(run_static_int("Arithmetic.class","factorial",  vec![10]),3_628_800); }
#[test] fn int_fibonacci_0(){ assert_eq!(run_static_int("Arithmetic.class", "fibonacci", vec![0]), 0); }
#[test] fn int_fibonacci_1(){ assert_eq!(run_static_int("Arithmetic.class", "fibonacci", vec![1]), 1); }
#[test] fn int_fibonacci_10(){assert_eq!(run_static_int("Arithmetic.class","fibonacci",  vec![10]),55); }
#[test] fn int_sum_to_100() { assert_eq!(run_static_int("Arithmetic.class", "sumTo",     vec![100]),5050); }
```

**Step 3: Run tests**

Run: `cargo test -p duke-interpreter`
Expected: all pass (including the 23 new integration tests).

If any fail, investigate which instruction is returning `Unimplemented`. Use `RUST_BACKTRACE=1` and check the error message for the mnemonic.

**Step 4: Run full workspace**

Run: `cargo test && cargo clippy`
Expected: all pass, no warnings.

**Step 5: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "test(interpreter): integration tests against Arithmetic.class — 23 real JVM methods execute correctly"
```

---

### Task 8: Update binary + workspace cleanup

**Files:**
- Modify: `duke/Cargo.toml`
- Modify: `duke/src/main.rs`

**Step 1: Add deps to duke binary**

In `duke/Cargo.toml`:
```toml
duke-runtime = { path = "../crates/duke-runtime" }
duke-interpreter = { path = "../crates/duke-interpreter" }
```

**Step 2: Add `exec` subcommand to `duke/src/main.rs`**

Add to the usage message:
```
duke exec <classfile.class> <method> [arg...]
```

Add dispatch before the file-read block:
```rust
if args.len() >= 3 && args[1] == "exec" {
    exec_method(&args[2..]);
    return;
}
```

Add the `exec_method` function:
```rust
fn exec_method(args: &[String]) {
    use duke_bytecode::decode;
    use duke_interpreter::execute;
    use duke_runtime::Slot;
    use duke_classfile::types::{AttributeData, CpEntry};

    if args.len() < 2 {
        eprintln!("Usage: duke exec <classfile.class> <method> [int-arg...]");
        process::exit(1);
    }
    let path = &args[0];
    let method_name = &args[1];
    let int_args: Vec<Slot> = args[2..].iter().map(|s| {
        Slot::Int(s.parse::<i32>().unwrap_or_else(|_| {
            eprintln!("duke: argument '{}' is not an integer", s);
            process::exit(1);
        }))
    }).collect();

    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    let method = cf.methods.iter().find(|m| {
        matches!(&cf.constant_pool[m.name_index.0 as usize], Some(CpEntry::Utf8(s)) if s == method_name)
    }).unwrap_or_else(|| {
        eprintln!("duke: method '{method_name}' not found");
        process::exit(1);
    });

    let code = method.attributes.iter().find_map(|a| {
        if let AttributeData::Code(c) = &a.data { Some(c) } else { None }
    }).unwrap_or_else(|| {
        eprintln!("duke: method '{method_name}' has no Code attribute");
        process::exit(1);
    });

    let instructions = decode(&code.code).unwrap_or_else(|e| {
        eprintln!("duke: decode error: {e}");
        process::exit(1);
    });

    match execute(&instructions, &cf.constant_pool, int_args,
                   code.max_stack, code.max_locals) {
        Ok(Some(result)) => println!("{result:?}"),
        Ok(None)         => println!("(void)"),
        Err(e)           => { eprintln!("duke: runtime error: {e}"); process::exit(1); }
    }
}
```

**Step 3: Smoke test the binary**

Run: `cargo run -p duke -- exec tests/fixtures/Arithmetic.class factorial 10`
Expected: `Int(3628800)`

Run: `cargo run -p duke -- exec tests/fixtures/Arithmetic.class fibonacci 10`
Expected: `Int(55)`

**Step 4: Run full workspace tests**

Run: `cargo test && cargo clippy`
Expected: all pass, no warnings.

**Step 5: Commit**

```bash
git add duke/ Cargo.toml Cargo.lock
git commit -m "feat: add 'duke exec' subcommand to run JVM methods from the command line"
```

---

## Verification Checklist

Phase 4 is complete when:

1. `cargo test` — all tests pass (35 existing + 5 duke-runtime + 3 duke-interpreter unit + 23 integration tests = ~66 total)
2. `cargo clippy -- -W clippy::pedantic` — no warnings in new crates
3. `duke exec tests/fixtures/Arithmetic.class factorial 10` prints `Int(3628800)`
4. `duke exec tests/fixtures/Arithmetic.class fibonacci 10` prints `Int(55)`
5. `duke dump tests/fixtures/HelloWorld.class` still works (regression)

**Known limitations carried forward to Phase 5:**
- `Areturn` not implemented (requires heap)
- Field access (`getstatic`, `getfield`, etc.) deferred to Phase 5
- Method invocation deferred to Phase 5
- `tableswitch` / `lookupswitch` deferred (not needed for arithmetic tests)
- Long/double locals occupy 1 slot, not 2 as JVM spec requires (will fix in Phase 5)
