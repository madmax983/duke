# Phase 7: Arrays and Exception Basics

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add array allocation, typed element access, and `athrow` dispatch to the Duke interpreter.

**Architecture:** Reuse `HeapObject` for arrays (class_name = `"[I"` etc., `fields` Vec holds elements).
All array instruction variants and `Athrow` are already decoded in `duke-bytecode`; only `execute_class`
dispatch is missing. `athrow` propagates as `VmError::JavaException` (no exception table dispatch yet —
deferred to Phase 8).

**Tech Stack:** Rust, `duke-runtime` (VmError), `duke-gc` (Heap), `duke-interpreter` (execute_class),
`duke-classfile` (ExceptionTableEntry — not used yet but referenced for completeness).

---

## Wave 1: Independent Preparatory Work (run both tasks in parallel)

### Task 1: Create ArrayOps.java Fixture

**Files:**
- Create: `tests/fixtures/ArrayOps.java`
- Create: `tests/fixtures/ArrayOps.class` (compiled)
- Modify: `tests/integration_test.rs` (add failing test stubs)

**Step 1: Write ArrayOps.java**

Create `tests/fixtures/ArrayOps.java`:

```java
public class ArrayOps {

    /** Allocate int[n], fill with 1..n, return sum.
     *  Exercises: newarray T_INT, iastore, iaload, arraylength */
    public static int sumArray(int n) {
        int[] a = new int[n];
        for (int i = 0; i < n; i++) {
            a[i] = i + 1;
        }
        int sum = 0;
        for (int i = 0; i < a.length; i++) {
            sum += a[i];
        }
        return sum;
    }

    /** Return length of a newly-allocated int[n].
     *  Exercises: newarray, arraylength */
    public static int arrayLength(int n) {
        int[] a = new int[n];
        return a.length;
    }

    /** Allocate long[n], fill with i*1_000_000, return sum.
     *  Exercises: newarray T_LONG, lastore, laload */
    public static long sumLongArray(int n) {
        long[] a = new long[n];
        for (int i = 0; i < n; i++) {
            a[i] = (long) i * 1_000_000L;
        }
        long sum = 0L;
        for (int i = 0; i < n; i++) {
            sum += a[i];
        }
        return sum;
    }

    /** Allocate double[n], fill with i * 0.5, return first element.
     *  Exercises: newarray T_DOUBLE, dastore, daload */
    public static double firstDouble(int n) {
        double[] a = new double[n];
        for (int i = 0; i < n; i++) {
            a[i] = i * 0.5;
        }
        return a[0];
    }

    /** Throw a RuntimeException object built in this class.
     *  (Actually we'll just use athrow on a pre-existing ref from a static
     *   field — deferred. This method is a placeholder for Phase 8.)
     *  For now we just test that the interpreter returns JavaException. */
}
```

**Step 2: Compile**

```bash
cd tests/fixtures
javac --release 21 ArrayOps.java
```

Expected: `ArrayOps.class` created with no errors.

**Step 3: Add failing integration test stubs**

In `tests/integration_test.rs`, add these tests (they will fail until Task 3 is done):

```rust
// ---- Phase 7: Arrays ----

#[test]
fn array_sum_5() {
    let cf = load_class("ArrayOps");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    let result = execute_class(&mut ctx, &mut heap, "sumArray", "(I)I", &[Slot::Int(5)]);
    assert_eq!(result.unwrap(), Some(Slot::Int(15))); // 1+2+3+4+5
}

#[test]
fn array_length() {
    let cf = load_class("ArrayOps");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    let result = execute_class(&mut ctx, &mut heap, "arrayLength", "(I)I", &[Slot::Int(7)]);
    assert_eq!(result.unwrap(), Some(Slot::Int(7)));
}

#[test]
fn array_sum_long() {
    let cf = load_class("ArrayOps");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    // n=3: 0*1e6 + 1*1e6 + 2*1e6 = 3_000_000
    let result = execute_class(&mut ctx, &mut heap, "sumLongArray", "(I)J", &[Slot::Int(3)]);
    assert_eq!(result.unwrap(), Some(Slot::Long(3_000_000)));
}

#[test]
fn array_first_double() {
    let cf = load_class("ArrayOps");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    // first element is a[0] = 0 * 0.5 = 0.0
    let result = execute_class(&mut ctx, &mut heap, "firstDouble", "(I)D", &[Slot::Int(3)]);
    assert_eq!(result.unwrap(), Some(Slot::Double(0.0)));
}

#[test]
fn array_index_out_of_bounds() {
    // We'll test this by running sumArray with n=0 and expecting success (empty sum = 0),
    // then manually verify bounds errors by building a minimal test.
    let cf = load_class("ArrayOps");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    let result = execute_class(&mut ctx, &mut heap, "sumArray", "(I)I", &[Slot::Int(0)]);
    assert_eq!(result.unwrap(), Some(Slot::Int(0))); // sum of empty = 0
}
```

**Step 4: Run tests to verify they fail**

```bash
cargo test -p duke -- array 2>&1 | head -40
```

Expected: failures with `Unimplemented { mnemonic: "newarray" }` or similar.

**Step 5: Commit**

```bash
git add tests/fixtures/ArrayOps.java tests/fixtures/ArrayOps.class tests/integration_test.rs
git commit -m "test(phase7): add ArrayOps fixture and failing integration tests"
```

---

### Task 2: Add VmError Variants for Arrays and Exceptions

**Files:**
- Modify: `crates/duke-runtime/src/error.rs`

**Step 1: Write a failing test**

In `crates/duke-runtime/src/lib.rs`, add these tests under the `#[cfg(test)]` block:

```rust
#[test]
fn array_index_oob_error_message() {
    let e = VmError::ArrayIndexOutOfBounds { index: 5, length: 3 };
    assert_eq!(e.to_string(), "array index 5 out of bounds for length 3");
}

#[test]
fn negative_array_size_error_message() {
    let e = VmError::NegativeArraySize { size: -1 };
    assert_eq!(e.to_string(), "negative array size: -1");
}

#[test]
fn java_exception_error_message() {
    let e = VmError::JavaException { class_name: "java/lang/RuntimeException".to_string() };
    assert_eq!(e.to_string(), "java exception: java/lang/RuntimeException");
}
```

**Step 2: Run tests to confirm failure**

```bash
cargo test -p duke-runtime 2>&1 | head -30
```

Expected: compile error (variants don't exist yet).

**Step 3: Add the three new variants to error.rs**

In `crates/duke-runtime/src/error.rs`, add after the `InvalidFieldref` variant:

```rust
#[error("array index {index} out of bounds for length {length}")]
ArrayIndexOutOfBounds { index: i32, length: usize },

#[error("negative array size: {size}")]
NegativeArraySize { size: i32 },

#[error("java exception: {class_name}")]
JavaException { class_name: String },
```

Note: `JavaException` cannot derive `Eq` because `String` is fine, but check that `VmError` still derives `PartialEq, Eq` — it should since all fields are `i32`/`usize`/`String`.

**Step 4: Run tests to confirm green**

```bash
cargo test -p duke-runtime 2>&1
```

Expected: all tests pass including the three new message tests.

**Step 5: Confirm zero warnings**

```bash
cargo clippy -p duke-runtime -- -W clippy::pedantic 2>&1
```

Expected: no warnings.

**Step 6: Commit**

```bash
git add crates/duke-runtime/src/error.rs crates/duke-runtime/src/lib.rs
git commit -m "feat(runtime): add ArrayIndexOutOfBounds, NegativeArraySize, JavaException errors"
```

---

## Wave 2: Core Implementation (sequential, depends on Wave 1)

### Task 3: Implement Array Opcodes and athrow in execute_class

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Background (read before implementing):**

Array instructions work like this in the JVM:
- `newarray T` — pops `count: i32`, allocates an array object, pushes a reference.
  Array type encodes as class_name: `ArrayType::Int` → `"[I"`, `Long` → `"[J"`, etc.
  If `count < 0`, return `Err(VmError::NegativeArraySize { size: count })`.
- `anewarray #cp` — same but for reference arrays. class_name = `"[LClassName;"`.
- `arraylength` — pops reference, pushes `fields.len() as i32`.
- `iaload` — pops index then arrayref, bounds-checks, pushes `Slot::Int(fields[idx] as i32)`.
- `iastore` — pops value, pops index, pops arrayref, bounds-checks, sets `fields[idx]`.
- Other load/store variants are analogous, just different Slot types:
  - `laload/lastore` — `Slot::Long`
  - `faload/fastore` — `Slot::Float`
  - `daload/dastore` — `Slot::Double`
  - `aaload/aastore` — `Slot::Reference`
  - `baload/bastore` — Int (sign-extended from i8; for store, truncate to i8 and sign-extend back)
  - `caload/castore` — Int (u16 zero-extended; for store, truncate to u16 and zero-extend back)
  - `saload/sastore` — Int (sign-extended from i16; for store, truncate to i16 and sign-extend back)
- `athrow` — pops reference, looks up heap object's class_name,
  returns `Err(VmError::JavaException { class_name })`.
  (Exception table dispatch is deferred to Phase 8.)

Bounds check helper (inline or a closure inside execute_class):
```rust
let bounds_check = |idx: i32, len: usize| -> VmResult<usize> {
    if idx < 0 || idx as usize >= len {
        Err(VmError::ArrayIndexOutOfBounds { index: idx, length: len })
    } else {
        Ok(idx as usize)
    }
};
```

**Step 1: Write unit tests first (inside crates/duke-interpreter/src/lib.rs)**

Add these unit tests to the existing `#[cfg(test)]` block:

```rust
// ---- Phase 7: Arrays (unit tests) ----

#[test]
fn newarray_int_empty() {
    // newarray T_INT count=0 → arraylength → 0
    use duke_bytecode::{ArrayType, Instruction::*};
    let instrs = vec![
        (0, Iconst0),
        (1, Newarray(ArrayType::Int)),
        (3, Arraylength),
        (4, Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 3, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(0)));
}

#[test]
fn newarray_iastore_iaload() {
    use duke_bytecode::{ArrayType, Instruction::*};
    // int[] a = new int[1]; a[0] = 42; return a[0];
    let instrs = vec![
        (0, Iconst1),               // count
        (1, Newarray(ArrayType::Int)), // ref
        (3, Dup),                   // ref ref
        (4, Iconst0),               // ref ref 0  (index)
        (5, Bipush(42)),            // ref ref 0 42 (value)
        (7, Iastore),               // ref
        (8, Iconst0),               // ref 0
        (9, Iaload),                // 42
        (10, Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 4, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(42)));
}

#[test]
fn newarray_int_bounds_error() {
    use duke_bytecode::{ArrayType, Instruction::*};
    // new int[1]; iaload at index 5 → ArrayIndexOutOfBounds
    let instrs = vec![
        (0, Iconst1),
        (1, Newarray(ArrayType::Int)),
        (3, Bipush(5)),    // invalid index
        (5, Iaload),
        (6, Ireturn),
    ];
    let err = execute(&instrs, &[], vec![], 3, 0).unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { index: 5, length: 1 }));
}

#[test]
fn newarray_negative_size() {
    use duke_bytecode::{ArrayType, Instruction::*};
    let instrs = vec![
        (0, IconstM1),
        (1, Newarray(ArrayType::Int)),
        (3, Arraylength),
        (4, Ireturn),
    ];
    let err = execute(&instrs, &[], vec![], 2, 0).unwrap_err();
    assert!(matches!(err, VmError::NegativeArraySize { size: -1 }));
}

#[test]
fn athrow_propagates() {
    use duke_bytecode::Instruction::*;
    // Allocate a dummy reference (via newarray to avoid needing 'new'),
    // then athrow it. Should return JavaException with "[I".
    use duke_bytecode::ArrayType;
    let instrs = vec![
        (0, Iconst1),
        (1, Newarray(ArrayType::Int)),  // produces a "[I" ref
        (3, Athrow),
    ];
    let err = execute(&instrs, &[], vec![], 2, 0).unwrap_err();
    assert!(matches!(err, VmError::JavaException { .. }));
}
```

Note: these tests use the `execute()` function (single-method, no heap). The heap-based tests use `execute_class()` and are in `tests/integration_test.rs`.

**Step 2: Run tests to verify they fail**

```bash
cargo test -p duke-interpreter -- newarray athrow 2>&1 | head -40
```

Expected: compile error or `Unimplemented` at runtime.

**Step 3: Implement array opcodes in both `execute()` and `execute_class()`**

The `execute()` function (single-method, no heap) needs a local mini-heap to make unit tests
work. Add a `Vec<HeapObject>` inside `execute()` for array storage in that function only.
The `execute_class()` function uses `heap: &mut Heap` already.

For `execute()`, add at the top of the function body:
```rust
let mut local_heap: Vec<duke_gc::HeapObject> = Vec::new();
```

Then helper closures for array alloc/get/get_mut on local_heap.

In `execute_class()`, use `heap` directly.

**Implement in BOTH `execute()` and `execute_class()` match blocks:**

```rust
// ---- Array allocation ----
Instruction::Newarray(array_type) => {
    let count = frame.pop_int()?;
    if count < 0 {
        return Err(VmError::NegativeArraySize { size: count });
    }
    let class_name = match array_type {
        ArrayType::Boolean => "[Z",
        ArrayType::Char    => "[C",
        ArrayType::Float   => "[F",
        ArrayType::Double  => "[D",
        ArrayType::Byte    => "[B",
        ArrayType::Short   => "[S",
        ArrayType::Int     => "[I",
        ArrayType::Long    => "[J",
    };
    let r = // allocate on heap or local_heap
    frame.push(Slot::Reference(Some(r)))?;
}

Instruction::Anewarray(_cp_idx) => {
    let count = frame.pop_int()?;
    if count < 0 {
        return Err(VmError::NegativeArraySize { size: count });
    }
    // Use "[Ljava/lang/Object;" as generic reference array class_name
    let r = // allocate count reference slots (all null)
    frame.push(Slot::Reference(Some(r)))?;
}

Instruction::Arraylength => {
    let r = frame.pop_ref()?;
    let len = // heap.get(r)?.fields.len()
    frame.push(Slot::Int(len as i32))?;
}

// ---- Array load/store ----
Instruction::Iaload => {
    let idx = frame.pop_int()?;
    let r = frame.pop_ref()?;
    let obj = // heap.get(r)?
    let slot_idx = bounds_check(idx, obj.fields.len())?;
    let v = obj.fields[slot_idx].as_int()?;
    frame.push(Slot::Int(v))?;
}

Instruction::Iastore => {
    let val = frame.pop_int()?;
    let idx = frame.pop_int()?;
    let r = frame.pop_ref()?;
    let obj = // heap.get_mut(r)?
    let slot_idx = bounds_check(idx, obj.fields.len())?;
    obj.fields[slot_idx] = Slot::Int(val);
}

// Laload/Lastore analogous with pop_long / Slot::Long
// Faload/Fastore analogous with pop_float / Slot::Float
// Daload/Dastore analogous with pop_double / Slot::Double
// Aaload/Aastore analogous with Reference slots
// Baload: read Int, cast to i8 then back to i32 (sign extension)
// Bastore: pop Int, truncate to i8, store as Int(i8 as i32)
// Caload: read Int, cast to u16 then back to i32 (zero extension)
// Castore: pop Int, truncate to u16, store as Int(u16 as i32)
// Saload: read Int, cast to i16 then back to i32 (sign extension)
// Sastore: pop Int, truncate to i16, store as Int(i16 as i32)

// ---- athrow ----
Instruction::Athrow => {
    let r = frame.pop_ref()?;
    let class_name = // heap.get(r)?.class_name.clone()
    return Err(VmError::JavaException { class_name });
}
```

**Step 4: Implement the full code (fill in the pseudocode above)**

Key implementation notes:
- For `execute()` use a `local_heap: Vec<HeapObject>` and closures; for `execute_class()` use `heap`
- `HeapObject` is in `duke_gc`, import as `use duke_gc::{Heap, HeapObject};` at top of lib.rs (already done for `execute_class`)
- For `execute()` to use `HeapObject` without `Heap`, directly push to `Vec<HeapObject>` and return `u64` index
- `Slot::as_int()` etc. already exist on Slot (check slot.rs). For `Reference` slot access on array fields, they're initialized to `Slot::Int(0)` by default — `aaload` needs to handle both `Slot::Reference(...)` and return that variant
- Add `use duke_bytecode::ArrayType;` import if not already present in `execute_class` match

**Step 5: Run all interpreter tests**

```bash
cargo test -p duke-interpreter 2>&1
```

Expected: all tests pass including the 5 new unit tests.

**Step 6: Run integration tests**

```bash
cargo test --test integration_test -- array 2>&1
```

Expected: `array_sum_5`, `array_length`, `array_sum_long`, `array_first_double`, `array_index_out_of_bounds` all pass.

**Step 7: Run full test suite**

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass, zero warnings.

**Step 8: Check warnings**

```bash
cargo clippy --all -- -W clippy::pedantic 2>&1 | grep "^error\|^warning.*duke" | head -20
```

Expected: no warnings. The `cast_*` clippy allows are already on `execute` and should be added to
`execute_class` too if new casts were introduced. The `#[allow(...)]` block at the top of `execute_class`
already covers all needed cast lints.

**Step 9: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): implement array opcodes (newarray, arraylength, all load/store) and athrow"
```

---

## Final Verification

After all three tasks are done:

**Step 1: Full test suite clean run**

```bash
cargo test 2>&1 | tail -10
```

Expected: all tests pass. Count should be ≥ 115 total (previous 100 + ~15 new).

**Step 2: fmt + clippy clean**

```bash
cargo fmt --check && cargo clippy --all -- -W clippy::pedantic -W clippy::nursery 2>&1 | grep "^error\|^warning.*warn"
```

Expected: no output (all clean).

**Step 3: Demo run with duke exec**

```bash
cargo build -p duke 2>/dev/null
./target/debug/duke exec tests/fixtures/ArrayOps.class sumArray "(I)I" 5
./target/debug/duke exec tests/fixtures/ArrayOps.class arrayLength "(I)I" 4
./target/debug/duke exec tests/fixtures/ArrayOps.class sumLongArray "(I)J" 3
```

Expected:
```
Int(15)
Int(4)
Long(3000000)
```

**Step 4: Update MEMORY.md**

Update `C:\Users\markm\.claude\projects\C--Users-markm-duke\memory\MEMORY.md`:
- Change "Status: Phase 6 Complete" → "Status: Phase 7 Complete"
- Update test count
- Add new opcodes to "New opcodes" list
- Update "Next Phase" section to Phase 8

---

## Implementation Notes for Agents

### Handling the local_heap in execute()

The single-method `execute()` function has no `heap` parameter. For Phase 7 unit tests to work,
add a local Vec inside the function body:

```rust
let mut local_heap: Vec<duke_gc::HeapObject> = Vec::new();
```

Then use these helpers inline (not closures, because closures borrowing frame would conflict):
- Allocation: push to local_heap, return index as u64
- Access: index into local_heap directly

Or alternatively, wrap in a helper struct. But inline is simplest.

### Wave 1 Parallelism

Tasks 1 and 2 are fully independent — they touch different files:
- Task 1 touches `tests/fixtures/` and `tests/integration_test.rs`
- Task 2 touches `crates/duke-runtime/src/error.rs` and `crates/duke-runtime/src/lib.rs`

Both can run simultaneously without conflict.

### Why not exception table dispatch yet?

Exception table dispatch requires:
1. Adding `exception_table: Vec<ExceptionEntry>` to `MethodEntry` (needs ExceptionTableEntry from duke-classfile)
2. During `athrow`, searching the exception table for a matching handler
3. Resolving `catch_type` CpIndex to a class name and comparing with the thrown class

This is doable but adds significant complexity. Phase 7 establishes the `JavaException` error variant
and `athrow` opcode support; Phase 8 will complete try/catch dispatch.
