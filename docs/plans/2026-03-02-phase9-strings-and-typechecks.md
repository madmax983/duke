# Phase 9: String Constants + Type Checks + Reference Comparisons

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add string constant loading (`ldc` for `CpEntry::String`), type-checking opcodes
(`checkcast`, `instanceof`), and fix reference comparison stubs (`if_acmpeq`, `if_acmpne`).

**Architecture:** Extend `HeapObject` with `string_value: Option<String>` for string content.
String constants are allocated as heap objects with class_name `"java/lang/String"`. Type checks
use exact class name comparison (no subtype hierarchy yet). Reference comparisons compare the
`u64` heap addresses directly.

**Tech Stack:** Rust, `duke-gc` (HeapObject), `duke-interpreter` (execute, execute_class, ldc_push),
`duke-runtime` (VmError).

---

## Wave 1: Independent Preparatory Work (run both tasks in parallel)

### Task 1: Create StringAndTypes.java Fixture + Failing Tests

**Files:**
- Create: `tests/fixtures/StringAndTypes.java`
- Create: `tests/fixtures/StringAndTypes.class` (compiled)
- Modify: `crates/duke-interpreter/src/lib.rs` (add failing integration tests)

**Step 1: Write StringAndTypes.java**

Create `tests/fixtures/StringAndTypes.java`:

```java
public class StringAndTypes {

    /**
     * Load a string constant via ldc.
     * Returns 1 if the string reference is non-null, 0 otherwise.
     * Exercises: ldc for CpEntry::String, ifnonnull
     */
    public static int stringNonNull() {
        String s = "hello";
        if (s != null) {
            return 1;
        }
        return 0;
    }

    /**
     * Two ldc of the same string literal should return the same reference
     * (string interning). Returns 1 if same ref, 0 otherwise.
     * Exercises: ldc String, if_acmpeq
     */
    public static int stringIntern() {
        String a = "hello";
        String b = "hello";
        if (a == b) {
            return 1;
        }
        return 0;
    }

    /**
     * instanceof with exact class match.
     * Creates a RuntimeException, checks instanceof RuntimeException → true.
     * Exercises: new, instanceof
     */
    public static int instanceOfMatch() {
        Object o = new RuntimeException();
        if (o instanceof RuntimeException) {
            return 1;
        }
        return 0;
    }

    /**
     * instanceof with non-matching class.
     * Creates a RuntimeException, checks instanceof Error → false.
     * Exercises: instanceof with class mismatch
     */
    public static int instanceOfMismatch() {
        Object o = new RuntimeException();
        if (o instanceof Error) {
            return 1;
        }
        return 0;
    }

    /**
     * instanceof with null → always false.
     * Exercises: instanceof with null reference
     */
    public static int instanceOfNull() {
        Object o = null;
        if (o instanceof RuntimeException) {
            return 1;
        }
        return 0;
    }

    /**
     * checkcast that succeeds (exact match).
     * Exercises: checkcast with matching type
     */
    public static int checkcastOk() {
        Object o = new RuntimeException();
        RuntimeException e = (RuntimeException) o; // checkcast
        return 42;
    }

    /**
     * if_acmpeq: same reference comparison.
     * Exercises: if_acmpeq
     */
    public static int refEqual() {
        int[] a = new int[1];
        int[] b = a;
        if (a == b) {
            return 1;
        }
        return 0;
    }

    /**
     * if_acmpne: different reference comparison.
     * Exercises: if_acmpne
     */
    public static int refNotEqual() {
        int[] a = new int[1];
        int[] b = new int[1];
        if (a != b) {
            return 1;
        }
        return 0;
    }
}
```

**Step 2: Compile**

```bash
javac --release 21 tests/fixtures/StringAndTypes.java
```

**Step 3: Add failing integration tests**

Read `crates/duke-interpreter/src/lib.rs` (last ~200 lines) to find the test module and
existing test helpers (`run_class_int`, `load_class`, etc.).

Add these tests to the BOTTOM of the `#[cfg(test)]` module:

```rust
    // ---- Phase 9: String constants ----

    #[test]
    fn string_non_null() {
        let result = run_class_int("StringAndTypes", "stringNonNull", "()I", &[]);
        assert_eq!(result, 1);
    }

    #[test]
    fn string_intern() {
        let result = run_class_int("StringAndTypes", "stringIntern", "()I", &[]);
        assert_eq!(result, 1);
    }

    // ---- Phase 9: instanceof ----

    #[test]
    fn instanceof_match() {
        let result = run_class_int("StringAndTypes", "instanceOfMatch", "()I", &[]);
        assert_eq!(result, 1);
    }

    #[test]
    fn instanceof_mismatch() {
        let result = run_class_int("StringAndTypes", "instanceOfMismatch", "()I", &[]);
        assert_eq!(result, 0);
    }

    #[test]
    fn instanceof_null() {
        let result = run_class_int("StringAndTypes", "instanceOfNull", "()I", &[]);
        assert_eq!(result, 0);
    }

    // ---- Phase 9: checkcast ----

    #[test]
    fn checkcast_ok() {
        let result = run_class_int("StringAndTypes", "checkcastOk", "()I", &[]);
        assert_eq!(result, 42);
    }

    // ---- Phase 9: Reference comparison ----

    #[test]
    fn ref_equal() {
        let result = run_class_int("StringAndTypes", "refEqual", "()I", &[]);
        assert_eq!(result, 1);
    }

    #[test]
    fn ref_not_equal() {
        let result = run_class_int("StringAndTypes", "refNotEqual", "()I", &[]);
        assert_eq!(result, 1);
    }
```

**Step 4: Run tests to verify failure**

```bash
cargo test -p duke-interpreter -- string_ instanceof_ checkcast_ ref_equal ref_not_equal 2>&1 | head -40
```

Expected: failures (InvalidCpIndex for string tests, Unimplemented for checkcast/instanceof,
wrong results for reference comparison tests since if_acmpeq/if_acmpne are stubbed).

**Step 5: Commit**

```bash
git add tests/fixtures/StringAndTypes.java tests/fixtures/StringAndTypes.class crates/duke-interpreter/src/lib.rs
git commit -m "test(phase9): add StringAndTypes fixture with string/instanceof/checkcast/ref tests"
```

---

### Task 2: Add string_value to HeapObject + ClassCastException VmError

**Files:**
- Modify: `crates/duke-gc/src/lib.rs` (HeapObject struct + Heap methods)
- Modify: `crates/duke-runtime/src/error.rs` (new VmError variant)
- Modify: `crates/duke-runtime/src/lib.rs` (test for new error)

**Step 1: Add test for ClassCastException**

In `crates/duke-runtime/src/lib.rs`, add to the `#[cfg(test)]` block:

```rust
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
```

**Step 2: Add ClassCastException to VmError**

In `crates/duke-runtime/src/error.rs`, add after the `JavaException` variant:

```rust
#[error("class cast exception: {from} cannot be cast to {to}")]
ClassCastException { from: String, to: String },
```

**Step 3: Run runtime tests**

```bash
cargo test -p duke-runtime 2>&1
```

Expected: all pass including new test.

**Step 4: Add string_value to HeapObject**

In `crates/duke-gc/src/lib.rs`, update `HeapObject`:

```rust
pub struct HeapObject {
    pub class_name: String,
    pub fields: Vec<Slot>,
    /// String content for `java/lang/String` objects. `None` for non-string objects.
    pub string_value: Option<String>,
}
```

**Step 5: Update Heap::allocate() to set string_value to None**

In `Heap::allocate()`, add `string_value: None` to the HeapObject construction:

```rust
pub fn allocate(&mut self, class_name: String, field_count: usize) -> u64 {
    let idx = self.objects.len() as u64;
    self.objects.push(HeapObject {
        class_name,
        fields: vec![Slot::Int(0); field_count],
        string_value: None,
    });
    idx
}
```

**Step 6: Add a new method `allocate_string()` to Heap**

```rust
/// Allocate a new String object with the given content.
pub fn allocate_string(&mut self, value: String) -> u64 {
    let idx = self.objects.len() as u64;
    self.objects.push(HeapObject {
        class_name: "java/lang/String".to_string(),
        fields: Vec::new(),
        string_value: Some(value),
    });
    idx
}
```

**Step 7: Add test for allocate_string**

Add to the `#[cfg(test)]` block in `crates/duke-gc/src/lib.rs`:

```rust
#[test]
fn allocate_string_stores_value() {
    let mut heap = Heap::new();
    let r = heap.allocate_string("hello".to_string());
    let obj = heap.get(r).unwrap();
    assert_eq!(obj.class_name, "java/lang/String");
    assert_eq!(obj.string_value, Some("hello".to_string()));
    assert!(obj.fields.is_empty());
}
```

**Step 8: Run gc tests**

```bash
cargo test -p duke-gc 2>&1
```

Expected: all pass including the new string test.

**Step 9: Run full test suite (check for regressions)**

```bash
cargo test 2>&1 | tail -15
```

Expected: all 127 existing tests still pass.

**Step 10: fmt + clippy**

```bash
cargo fmt && cargo clippy --all -- -W clippy::pedantic 2>&1 | grep "^error" | head -5
```

**Step 11: Commit**

```bash
git add crates/duke-gc/src/lib.rs crates/duke-runtime/src/error.rs crates/duke-runtime/src/lib.rs
git commit -m "feat(gc,runtime): add string_value to HeapObject, allocate_string(), ClassCastException"
```

---

## Wave 2: Core Implementation (sequential, depends on Wave 1)

### Task 3: Implement String ldc, checkcast, instanceof, if_acmpeq/if_acmpne

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Background:**

**`ldc` for String:** The CP entry `CpEntry::String { string_index }` contains a `string_index`
pointing to a `CpEntry::Utf8(s)`. We need to resolve it, allocate a String object on the heap
(or local_heap for execute()), and push the reference.

For **string interning**: two `ldc "hello"` instructions should return the same reference.
Use a `HashMap<usize, u64>` mapping CP index → heap ref, checked before allocating.

**`checkcast`:** Pop objectref. If null, push it back (null passes any cast). If not null,
resolve the CP Class index to a class name, compare with `heap.get(ref)?.class_name`.
If match, push objectref back. If mismatch, return `Err(VmError::ClassCastException)`.

**`instanceof`:** Pop objectref. If null, push `Int(0)`. If not null, resolve CP Class index
to class name, compare with `heap.get(ref)?.class_name`. If match, push `Int(1)`. Else `Int(0)`.

**`if_acmpeq`/`if_acmpne`:** Pop two values. Compare as raw `Slot` equality. Branch if equal/not-equal.
In execute_class, these should compare `Slot::Reference` values (both `None` = null equals null;
both `Some(x)` where x matches = equal).

**Step 1: Write unit tests for ldc String, checkcast, instanceof**

Add to the `#[cfg(test)]` block:

```rust
    // ---- Phase 9: Unit tests ----

    #[test]
    fn if_acmpeq_same_ref() {
        use duke_bytecode::Instruction::*;
        use duke_bytecode::ArrayType;
        // new int[1]; dup; if_acmpeq → branch → return 1; else return 0
        let instrs = vec![
            (0, Iconst1),
            (1, Newarray(ArrayType::Int)),   // ref
            (3, Dup),                        // ref ref
            (4, IfAcmpeq(10)),               // same ref → jump to pc 14
            (7, Iconst0),
            (8, Ireturn),
            (14, Iconst1),
            (15, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 3, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn if_acmpne_different_refs() {
        use duke_bytecode::Instruction::*;
        use duke_bytecode::ArrayType;
        // two different arrays → if_acmpne → branch
        let instrs = vec![
            (0, Iconst1),
            (1, Newarray(ArrayType::Int)),   // ref_a
            (3, Iconst1),
            (4, Newarray(ArrayType::Int)),   // ref_b
            (6, IfAcmpne(10)),               // different → jump to pc 16
            (9, Iconst0),
            (10, Ireturn),
            (16, Iconst1),
            (17, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 3, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }
```

**Step 2: Run tests to verify failure (RED)**

```bash
cargo test -p duke-interpreter -- if_acmpeq_same if_acmpne_different 2>&1 | head -20
```

Expected: wrong results (stubs just pop without branching).

**Step 3: Fix if_acmpeq / if_acmpne in execute()**

Find the current stubbed implementation (around line 694-698) and replace:

```rust
// OLD (stub):
Instruction::IfAcmpeq(_) | Instruction::IfAcmpne(_) => {
    frame.pop()?;
    frame.pop()?;
}

// NEW:
Instruction::IfAcmpeq(offset) => {
    let b = frame.pop()?;
    let a = frame.pop()?;
    if a == b {
        jump!(*offset);
    }
}
Instruction::IfAcmpne(offset) => {
    let b = frame.pop()?;
    let a = frame.pop()?;
    if a != b {
        jump!(*offset);
    }
}
```

**Step 4: Fix if_acmpeq / if_acmpne in execute_class()**

Same replacement in the execute_class() match block (around line 1739-1742).

**Step 5: Run if_acmp unit tests (GREEN)**

```bash
cargo test -p duke-interpreter -- if_acmpeq if_acmpne 2>&1
```

Expected: pass.

**Step 6: Implement ldc for CpEntry::String in execute_class()**

In execute_class(), the `Instruction::Ldc` and `Instruction::LdcW | Instruction::Ldc2W`
arms currently call `ldc_push()`. We need to handle `CpEntry::String` before falling through
to `ldc_push()`.

Add a string intern cache at the top of execute_class(), after `let mut idx: usize = 0;`:

```rust
let mut string_intern: HashMap<usize, u64> = HashMap::new();
```

Then modify the ldc handling in execute_class():

```rust
Instruction::Ldc(raw_idx) => {
    let cp_idx = usize::from(*raw_idx);
    if let Some(CpEntry::String { string_index }) =
        ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
    {
        let si = string_index.0 as usize;
        let r = if let Some(&cached) = string_intern.get(&cp_idx) {
            cached
        } else {
            let s = match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => return Err(VmError::InvalidCpIndex { index: si }),
            };
            let r = heap.allocate_string(s);
            string_intern.insert(cp_idx, r);
            r
        };
        frame.push(Slot::Reference(Some(r)))?;
    } else {
        ldc_push(&mut frame, &ctx.constant_pool, cp_idx)?;
    }
}
Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
    let idx_val = usize::from(cp_idx.0);
    if let Some(CpEntry::String { string_index }) =
        ctx.constant_pool.get(idx_val).and_then(|e| e.as_ref())
    {
        let si = string_index.0 as usize;
        let r = if let Some(&cached) = string_intern.get(&idx_val) {
            cached
        } else {
            let s = match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => return Err(VmError::InvalidCpIndex { index: si }),
            };
            let r = heap.allocate_string(s);
            string_intern.insert(idx_val, r);
            r
        };
        frame.push(Slot::Reference(Some(r)))?;
    } else {
        ldc_push(&mut frame, &ctx.constant_pool, idx_val)?;
    }
}
```

**Step 7: Implement ldc for CpEntry::String in execute() (local_heap variant)**

In execute(), add a string intern cache and handle CpEntry::String similarly, but use
local_heap instead of heap:

```rust
// At top of execute(), after local_heap declaration:
let mut string_intern: HashMap<usize, u64> = HashMap::new();
```

Then update the ldc arm in execute():

```rust
Instruction::Ldc(raw_idx) => {
    let cp_idx = usize::from(*raw_idx);
    if let Some(CpEntry::String { string_index }) =
        cp.get(cp_idx).and_then(|e| e.as_ref())
    {
        let si = string_index.0 as usize;
        let r = if let Some(&cached) = string_intern.get(&cp_idx) {
            cached
        } else {
            let s = match cp.get(si).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => return Err(VmError::InvalidCpIndex { index: si }),
            };
            let r = local_heap.len() as u64;
            local_heap.push(("java/lang/String".to_string(), Vec::new(), Some(s)));
            string_intern.insert(cp_idx, r);
            r
        };
        frame.push(Slot::Reference(Some(r)))?;
    } else {
        ldc_push(&mut frame, cp, cp_idx)?;
    }
}
```

**IMPORTANT**: This requires changing the local_heap type from `Vec<(String, Vec<Slot>)>` to
`Vec<(String, Vec<Slot>, Option<String>)>`. Update ALL existing local_heap push calls to
include `, None` as the third element. Also update ALL existing local_heap get/get_mut accesses
to use `.0` for class_name and `.1` for fields (they should already work with tuples).

Search for `local_heap.push` and add `, None` to each existing call.
Search for `local_heap.get` and verify `.0` and `.1` accessors still work.

**Step 8: Implement checkcast in execute_class()**

Add a helper to resolve a CP index to a class name (reusable for checkcast and instanceof):

```rust
/// Resolve a CP Class entry to its name string.
fn resolve_class_name(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Class { name_index }) => {
            match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => Err(VmError::InvalidCpIndex { index: name_index.0 as usize }),
            }
        }
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}
```

Then in execute_class() match block:

```rust
Instruction::Checkcast(cp_idx) => {
    let slot = frame.pop()?;
    match &slot {
        Slot::Reference(None) => {
            // null passes any checkcast
            frame.push(slot)?;
        }
        Slot::Reference(Some(r)) => {
            let target = resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?;
            let actual = heap.get(*r)?.class_name.clone();
            if actual == target {
                frame.push(slot)?;
            } else {
                return Err(VmError::ClassCastException { from: actual, to: target });
            }
        }
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "reference",
                got: "non-reference",
            });
        }
    }
}
Instruction::Instanceof(cp_idx) => {
    let slot = frame.pop()?;
    match &slot {
        Slot::Reference(None) => {
            // null → 0
            frame.push(Slot::Int(0))?;
        }
        Slot::Reference(Some(r)) => {
            let target = resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?;
            let actual = heap.get(*r)?.class_name.clone();
            if actual == target {
                frame.push(Slot::Int(1))?;
            } else {
                frame.push(Slot::Int(0))?;
            }
        }
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "reference",
                got: "non-reference",
            });
        }
    }
}
```

**Step 9: Also implement checkcast/instanceof in execute() (using local_heap)**

Same logic but use `local_heap.get(*r as usize)...0` for class_name lookup instead of `heap.get()`.

**Step 10: Run all Phase 9 integration tests**

```bash
cargo test -p duke-interpreter -- string_ instanceof_ checkcast_ ref_equal ref_not_equal 2>&1
```

Expected: all 8 integration tests pass.

**Step 11: Run full test suite**

```bash
cargo test 2>&1 | tail -15
```

Expected: all tests pass, no regressions.

**Step 12: fmt + clippy**

```bash
cargo fmt && cargo clippy --all -- -W clippy::pedantic 2>&1 | grep "^error" | head -5
```

**Step 13: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): implement ldc String, checkcast, instanceof, if_acmpeq/if_acmpne

- ldc/ldc_w for CpEntry::String: allocate String HeapObject, intern cache
- checkcast: null passes, exact class name match or ClassCastException
- instanceof: null → 0, exact class name match → 1
- if_acmpeq/if_acmpne: fix stubs to actually compare Slot values
- resolve_class_name() helper for CP Class → name resolution"
```

---

## Final Verification

**Step 1: Full test suite**

```bash
cargo test 2>&1 | grep "test result"
```

Expected: all pass. Count should be ~140 (127 + ~10 new unit tests + ~8 integration tests).

**Step 2: Demo runs**

```bash
cargo build -p duke 2>/dev/null
./target/debug/duke exec tests/fixtures/StringAndTypes.class stringNonNull
./target/debug/duke exec tests/fixtures/StringAndTypes.class instanceOfMatch
./target/debug/duke exec tests/fixtures/StringAndTypes.class checkcastOk
./target/debug/duke exec tests/fixtures/StringAndTypes.class refEqual
```

Expected:
```
Int(1)
Int(1)
Int(42)
Int(1)
```

**Step 3: Update MEMORY.md**

Update status to Phase 9 Complete. Add test count, new opcodes, HeapObject changes.

---

## Implementation Notes for Agents

### local_heap type change in execute()

The `local_heap` type in `execute()` changes from `Vec<(String, Vec<Slot>)>` to
`Vec<(String, Vec<Slot>, Option<String>)>`. Every existing `local_heap.push(...)` call
needs a `, None` appended. There are ~10 such calls (newarray, anewarray variants).

### String interning

The `string_intern: HashMap<usize, u64>` maps CP index → heap reference. When two
`ldc` instructions reference the same CP String entry, they get the same heap reference.
This is important because Java specifies that identical string literals are interned.

Note: this only handles same-CP-index interning. Two different CP entries with the same
string content will NOT be interned together. Full interning requires a content-based
lookup, which we defer.

### Slot equality for if_acmpeq

`Slot` derives `PartialEq`, so `a == b` works for reference comparison:
- `Slot::Reference(None) == Slot::Reference(None)` → true (null == null)
- `Slot::Reference(Some(3)) == Slot::Reference(Some(3))` → true (same address)
- `Slot::Reference(Some(3)) == Slot::Reference(Some(5))` → false (different objects)

### checkcast vs instanceof

`checkcast` THROWS if the type doesn't match (unless null).
`instanceof` RETURNS 0 or 1 and never throws.
Both use exact class name comparison (no subtype hierarchy).

### Wave 1 Conflict Avoidance

Task 1 touches: `tests/fixtures/` + bottom of `crates/duke-interpreter/src/lib.rs` (test section)
Task 2 touches: `crates/duke-gc/src/lib.rs` + `crates/duke-runtime/src/error.rs` + `lib.rs`

No file overlap between tasks.
