# Phase 6: Heap, Objects, and Field Access Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add object allocation (`new`), field access (`getfield`/`putfield`/`getstatic`/`putstatic`), and instance method dispatch (`invokespecial`/`invokevirtual`) so the interpreter can run programs that create and manipulate objects.

**Architecture:** A new `duke-gc` crate holds a simple bump-pointer heap (`Heap` + `HeapObject`) with no GC yet. `ClassContext` gains field metadata (`FieldEntry`, static field values, instance field count). `execute_class()` grows two new parameters: `&mut ClassContext` (for static fields) and `&mut Heap` (for objects). The fixture `Point.java` exercises the full object lifecycle: `new` → constructor → `getfield` → `invokevirtual` → `ireturn`.

**Tech Stack:** Rust, `duke-runtime` (Slot/Frame/VmError), `duke-bytecode` (Instruction), `duke-classfile` (ClassFile/FieldInfo/FieldAccessFlags), `duke-gc` (new crate)

**Dependency order after Phase 6:**
```
duke-classfile
duke-bytecode  → duke-classfile
duke-runtime   → duke-classfile
duke-gc        → duke-runtime          ← NEW
duke-interpreter → duke-runtime, duke-bytecode, duke-classfile, duke-gc
duke-loader    → duke-classfile
duke           → all crates
```

---

## Task 1: Create duke-gc crate (simple heap, no GC)

**Files:**
- Create: `crates/duke-gc/Cargo.toml`
- Create: `crates/duke-gc/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

### Step 1: Create Cargo.toml for duke-gc

```toml
# crates/duke-gc/Cargo.toml
[package]
name = "duke-gc"
version.workspace = true
edition.workspace = true
description = "Duke JVM - Heap allocator and garbage collector"

[dependencies]
duke-runtime = { path = "../duke-runtime" }
```

### Step 2: Create crates/duke-gc/src/lib.rs

```rust
//! Simple bump-pointer heap for Duke Phase 6.
//!
//! No garbage collection yet — objects are allocated and never freed.
//! Phase 7 will add a generational collector here.

use duke_runtime::{Slot, VmError, VmResult};

/// A single heap-allocated Java object.
///
/// Fields are stored as a flat `Vec<Slot>`, indexed by instance field position.
/// Static fields live on `ClassContext`, not here.
#[derive(Debug, Clone)]
pub struct HeapObject {
    /// Internal JVM class name (e.g. `"Point"`).
    pub class_name: String,
    /// Instance field values, zero-initialised at allocation.
    pub fields: Vec<Slot>,
}

/// The object heap.
///
/// Backed by a `Vec` — reference values (`Slot::Reference(Some(u64))`) are
/// indices into this vector.  Index 0 is always valid if any object has been
/// allocated; there is no reserved-slot convention (unlike the constant pool).
#[derive(Debug, Default)]
pub struct Heap {
    objects: Vec<HeapObject>,
}

impl Heap {
    /// Create an empty heap.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate a new object of the given class with `field_count` instance
    /// fields, all zero-initialised as `Slot::Int(0)`.
    ///
    /// Returns the heap reference (index) as a `u64`.
    pub fn allocate(&mut self, class_name: String, field_count: usize) -> u64 {
        let idx = self.objects.len() as u64;
        self.objects.push(HeapObject {
            class_name,
            fields: vec![Slot::Int(0); field_count],
        });
        idx
    }

    /// Borrow an object by heap reference.
    ///
    /// # Errors
    /// Returns [`VmError::InvalidRef`] if the reference is out of bounds.
    pub fn get(&self, r: u64) -> VmResult<&HeapObject> {
        self.objects.get(r as usize).ok_or(VmError::InvalidRef { address: r })
    }

    /// Mutably borrow an object by heap reference.
    ///
    /// # Errors
    /// Returns [`VmError::InvalidRef`] if the reference is out of bounds.
    pub fn get_mut(&mut self, r: u64) -> VmResult<&mut HeapObject> {
        self.objects.get_mut(r as usize).ok_or(VmError::InvalidRef { address: r })
    }

    /// Number of allocated objects (useful for testing).
    #[must_use]
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// True if no objects have been allocated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_and_get() {
        let mut heap = Heap::new();
        let r = heap.allocate("Point".to_string(), 2);
        assert_eq!(r, 0);
        let obj = heap.get(r).unwrap();
        assert_eq!(obj.class_name, "Point");
        assert_eq!(obj.fields.len(), 2);
        assert_eq!(obj.fields[0], Slot::Int(0));
    }

    #[test]
    fn allocate_multiple() {
        let mut heap = Heap::new();
        let r0 = heap.allocate("Point".to_string(), 2);
        let r1 = heap.allocate("Point".to_string(), 2);
        assert_eq!(r0, 0);
        assert_eq!(r1, 1);
        assert_eq!(heap.len(), 2);
    }

    #[test]
    fn get_mut_sets_field() {
        let mut heap = Heap::new();
        let r = heap.allocate("Point".to_string(), 2);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int(42);
        assert_eq!(heap.get(r).unwrap().fields[0], Slot::Int(42));
    }

    #[test]
    fn invalid_ref_returns_error() {
        let heap = Heap::new();
        let err = heap.get(999).unwrap_err();
        assert!(matches!(err, VmError::InvalidRef { address: 999 }));
    }
}
```

### Step 3: Add duke-gc to workspace members

In `Cargo.toml` (workspace root), add `"crates/duke-gc"` to the `members` array:

```toml
members = [
    "duke",
    "crates/duke-classfile",
    "crates/duke-bytecode",
    "crates/duke-loader",
    "crates/duke-runtime",
    "crates/duke-gc",
    "crates/duke-interpreter",
]
```

### Step 4: Run duke-gc tests

Run: `cargo test -p duke-gc`

Expected: `4 passed; 0 failed`

### Step 5: Run clippy on duke-gc

Run: `cargo clippy -p duke-gc -- -W clippy::pedantic -W clippy::nursery`

Expected: Zero warnings (the `#[must_use]` annotations and `Default` derive handle common nursery lints).

### Step 6: Commit

```bash
git add crates/duke-gc/ Cargo.toml
git commit -m "feat(gc): add duke-gc crate with simple bump-pointer heap"
```

---

## Task 2: Create Point.java fixture and compile

**Files:**
- Create: `tests/fixtures/Point.java`
- Compile: `tests/fixtures/Point.class`

### Step 1: Write Point.java

```java
// tests/fixtures/Point.java
public class Point {
    int x;
    int y;

    Point(int x, int y) {
        this.x = x;
        this.y = y;
    }

    int sum() {
        return x + y;
    }

    static int sumPoints(int ax, int ay, int bx, int by) {
        Point a = new Point(ax, ay);
        Point b = new Point(bx, by);
        return a.sum() + b.sum();
    }
}
```

This exercises: `new`, `dup`, `invokespecial <init>`, `astore`, `aload`, `invokevirtual sum`, `getfield x`, `getfield y`, `putfield x`, `putfield y`, `ireturn`, `return`.

### Step 2: Compile

Run: `javac tests/fixtures/Point.java`

Expected: `tests/fixtures/Point.class` created.

### Step 3: Verify with duke dump

Run: `cargo run --bin duke -- tests/fixtures/Point.class`

Expected: Shows `<init>`, `sum`, `sumPoints` methods with `invokespecial`, `invokevirtual`, `new`, `getfield`, `putfield` in the disassembly.

### Step 4: Commit

```bash
git add tests/fixtures/Point.java tests/fixtures/Point.class
git commit -m "test(fixtures): add Point.java for Phase 6 object lifecycle testing"
```

---

## Task 3: Add VmError variants and Frame::pop_ref helper

**Files:**
- Modify: `crates/duke-runtime/src/error.rs`
- Modify: `crates/duke-runtime/src/frame.rs`

### Step 1: Add new VmError variants to error.rs

Add these three variants after `InvalidMethodref`:

```rust
    #[error("null pointer dereference")]
    NullPointerException,

    #[error("invalid heap reference: address={address}")]
    InvalidRef { address: u64 },

    #[error("constant pool index {index} is not a valid Fieldref")]
    InvalidFieldref { index: usize },
```

### Step 2: Add pop_ref() to Frame (frame.rs)

Add after `pop_double()`:

```rust
    /// Pop and unwrap as a non-null heap reference (`u64`).
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
```

Note: `type_name()` is currently private in slot.rs. You need to make it `pub(crate)` or `pub`:

In `crates/duke-runtime/src/slot.rs`, change:
```rust
fn type_name(&self) -> &'static str {
```
to:
```rust
pub fn type_name(&self) -> &'static str {
```

### Step 3: Run tests

Run: `cargo test -p duke-runtime`

Expected: 5 existing tests pass, plus any new ones.

### Step 4: Add a test for pop_ref in frame.rs tests (inside `#[cfg(test)]`)

```rust
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
        assert!(matches!(f.pop_ref().unwrap_err(), VmError::NullPointerException));
    }
```

Run: `cargo test -p duke-runtime`

Expected: 7 tests pass.

### Step 5: Commit

```bash
git add crates/duke-runtime/src/error.rs crates/duke-runtime/src/frame.rs crates/duke-runtime/src/slot.rs
git commit -m "feat(runtime): add NullPointerException/InvalidRef/InvalidFieldref errors and Frame::pop_ref"
```

---

## Task 4: Extend ClassContext with field metadata and build_class_context()

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`
- Modify: `crates/duke-interpreter/Cargo.toml`

Tasks 1-3 must be complete before this task.

### Step 1: Add duke-gc dependency to duke-interpreter

In `crates/duke-interpreter/Cargo.toml`:

```toml
[dependencies]
duke-runtime = { path = "../duke-runtime" }
duke-bytecode = { path = "../duke-bytecode" }
duke-classfile = { path = "../duke-classfile" }
duke-gc = { path = "../duke-gc" }
```

### Step 2: Add FieldEntry to lib.rs and update ClassContext

Add `FieldEntry` near `MethodEntry` (before `ClassContext`):

```rust
/// A field declaration extracted from a parsed class.
pub struct FieldEntry {
    pub name: String,
    pub descriptor: String,
    /// True if the field is declared `static`.
    pub is_static: bool,
}
```

Update `ClassContext` to include field metadata:

```rust
/// A parsed class with all methods decoded — the unit of execution for Phase 5+.
pub struct ClassContext {
    /// Internal class name (e.g. `"Point"`).
    pub class_name: String,
    pub constant_pool: Vec<Option<CpEntry>>,
    pub methods: Vec<MethodEntry>,
    /// All field declarations (static and instance), in class file order.
    pub fields: Vec<FieldEntry>,
    /// Values of static fields, indexed by position among static-only fields.
    pub static_fields: Vec<Slot>,
    /// Number of instance (non-static) fields — used to size heap objects.
    pub instance_field_count: usize,
}
```

### Step 3: Add build_class_context() public function

Add this function after `execute_class()` and before `ldc_push()`:

```rust
/// Build a [`ClassContext`] from a parsed [`ClassFile`].
///
/// Decodes all methods with a Code attribute and extracts field metadata.
/// Methods without Code (abstract, native) are silently skipped.
///
/// # Panics
/// Does not panic — methods / fields with invalid CP references are silently
/// skipped (the interpreter will raise `MethodNotFound` at call time).
pub fn build_class_context(cf: &duke_classfile::ClassFile) -> ClassContext {
    use duke_bytecode::decode;
    use duke_classfile::types::{AttributeData, CpEntry};

    let class_name = {
        let class_entry = cf.constant_pool.get(cf.this_class.0 as usize)
            .and_then(|e| e.as_ref());
        if let Some(CpEntry::Class { name_index }) = class_entry {
            match cf.constant_pool.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => String::new(),
            }
        } else {
            String::new()
        }
    };

    let methods = cf
        .methods
        .iter()
        .filter_map(|m| {
            let name = match cf.constant_pool.get(m.name_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let descriptor = match cf.constant_pool.get(m.descriptor_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let code = m.attributes.iter().find_map(|a| {
                if let AttributeData::Code(c) = &a.data { Some(c) } else { None }
            })?;
            let instructions = decode(&code.code).ok()?;
            Some(MethodEntry {
                name,
                descriptor,
                instructions,
                max_stack: code.max_stack,
                max_locals: code.max_locals,
            })
        })
        .collect();

    let mut fields: Vec<FieldEntry> = Vec::new();
    let mut static_count = 0usize;
    let mut instance_count = 0usize;

    for f in &cf.fields {
        use duke_classfile::access_flags::FieldAccessFlags;
        let name = match cf.constant_pool.get(f.name_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s.clone(),
            _ => continue,
        };
        let descriptor = match cf.constant_pool.get(f.descriptor_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s.clone(),
            _ => continue,
        };
        let is_static = f.access_flags.contains(FieldAccessFlags::STATIC);
        if is_static { static_count += 1; } else { instance_count += 1; }
        fields.push(FieldEntry { name, descriptor, is_static });
    }

    ClassContext {
        class_name,
        constant_pool: cf.constant_pool.clone(),
        methods,
        fields,
        static_fields: vec![Slot::Int(0); static_count],
        instance_field_count: instance_count,
    }
}
```

### Step 4: Update existing tests to use build_class_context

In the test module, replace the `load_class_context()` helper with one that calls `build_class_context()`:

```rust
    fn load_class_context(class_name: &str) -> ClassContext {
        use duke_classfile::parse;
        let bytes = std::fs::read(fixture(class_name)).expect("fixture not found");
        let cf = parse(&bytes).expect("parse failed");
        build_class_context(&cf)
    }
```

Remove the old `load_class_context` implementation (the long filter_map one).

### Step 5: Update execute_class signature (stub the new params for now)

Change the signature of `execute_class()` to accept heap and make ctx mutable:

```rust
pub fn execute_class(
    ctx: &mut ClassContext,
    heap: &mut duke_gc::Heap,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>>
```

Update all test call sites: `execute_class(&mut ctx, &mut heap, ...)` where `heap` is a `duke_gc::Heap::new()` created at the start of each test.

Create a test helper:
```rust
    fn run_class_int(class_name: &str, method_name: &str, descriptor: &str, args: Vec<i32>) -> i32 {
        let ctx = load_class_context(class_name);
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
        let mut ctx = ctx;
        let mut heap = duke_gc::Heap::new();
        match execute_class(&mut ctx, &mut heap, method_name, descriptor, &slots)
            .expect("execute_class failed")
        {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }
```

### Step 6: Run existing tests (should all still pass)

Run: `cargo test -p duke-interpreter`

Expected: All 49 existing tests still pass.

### Step 7: Commit

```bash
git add crates/duke-interpreter/src/lib.rs crates/duke-interpreter/Cargo.toml
git commit -m "feat(interpreter): add FieldEntry to ClassContext, build_class_context(), update execute_class signature"
```

---

## Task 5: Implement heap opcodes + write integration tests

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

Tasks 1-4 must be complete before this task. This is the big one.

### Step 1: Write failing integration tests FIRST (RED)

Add these tests to the `#[cfg(test)]` block in lib.rs:

```rust
    // ---- Phase 6: object creation + field access ----

    #[test]
    fn point_sum_direct() {
        // Call sum() on a Point directly — requires new + invokespecial + invokevirtual + getfield
        assert_eq!(run_class_int("Point.class", "sumPoints", "(IIII)I", vec![1, 2, 3, 4]), 10);
    }

    #[test]
    fn point_sum_3_4() {
        assert_eq!(run_class_int("Point.class", "sumPoints", "(IIII)I", vec![3, 4, 0, 0]), 7);
    }

    #[test]
    fn point_sum_zeros() {
        assert_eq!(run_class_int("Point.class", "sumPoints", "(IIII)I", vec![0, 0, 0, 0]), 0);
    }

    #[test]
    fn point_sum_symmetry() {
        let a = run_class_int("Point.class", "sumPoints", "(IIII)I", vec![1, 2, 3, 4]);
        let b = run_class_int("Point.class", "sumPoints", "(IIII)I", vec![3, 4, 1, 2]);
        assert_eq!(a, b); // sumPoints is symmetric
    }
```

Run: `cargo test -p duke-interpreter point_`

Expected: All 4 tests FAIL with `Unimplemented { mnemonic: "new" }` or similar.

### Step 2: Add two new helper functions (resolve_fieldref + instance/static field lookup)

Add after `resolve_methodref()`:

```rust
/// Resolve a constant pool Fieldref to (field_name, descriptor).
fn resolve_fieldref(cp: &[Option<CpEntry>], idx: usize) -> VmResult<(String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Fieldref { name_and_type_index, .. }) => {
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType { name_index, descriptor_index }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    Ok((name, desc))
                }
                _ => Err(VmError::InvalidFieldref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidFieldref { index: idx }),
    }
}

/// Find the index of a named instance field within ctx.fields (non-static only).
fn instance_field_idx(ctx: &ClassContext, name: &str) -> VmResult<usize> {
    ctx.fields
        .iter()
        .filter(|f| !f.is_static)
        .position(|f| f.name == name)
        .ok_or_else(|| VmError::InvalidFieldref { index: 0 })
}

/// Find the index of a named static field within ctx.static_fields.
fn static_field_idx(ctx: &ClassContext, name: &str) -> VmResult<usize> {
    ctx.fields
        .iter()
        .filter(|f| f.is_static)
        .position(|f| f.name == name)
        .ok_or_else(|| VmError::InvalidFieldref { index: 0 })
}
```

### Step 3: Add heap opcode handlers to execute_class()

In the `match &instr { ... }` block of `execute_class()`, add these arms **before** the `other =>` catch-all:

```rust
            // ---- Object allocation ----
            Instruction::New(cp_idx) => {
                // Resolve class name from CP.
                let class_name = match ctx.constant_pool.get(usize::from(cp_idx.0))
                    .and_then(|e| e.as_ref())
                {
                    Some(CpEntry::Class { name_index }) => {
                        match ctx.constant_pool.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: usize::from(cp_idx.0) }),
                        }
                    }
                    _ => return Err(VmError::InvalidCpIndex { index: usize::from(cp_idx.0) }),
                };
                let field_count = ctx.instance_field_count;
                let r = heap.allocate(class_name, field_count);
                frame.push(Slot::Reference(Some(r)))?;
            }

            // ---- Field access ----
            Instruction::Getfield(cp_idx) => {
                let (field_name, _) = resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?;
                let r = frame.pop_ref()?;
                let fidx = instance_field_idx(ctx, &field_name)?;
                let val = heap.get(r)?.fields[fidx].clone();
                frame.push(val)?;
            }
            Instruction::Putfield(cp_idx) => {
                let (field_name, _) = resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?;
                let val = frame.pop()?;
                let r = frame.pop_ref()?;
                let fidx = instance_field_idx(ctx, &field_name)?;
                heap.get_mut(r)?.fields[fidx] = val;
            }
            Instruction::Getstatic(cp_idx) => {
                let (field_name, _) = resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?;
                let sidx = static_field_idx(ctx, &field_name)?;
                let val = ctx.static_fields[sidx].clone();
                frame.push(val)?;
            }
            Instruction::Putstatic(cp_idx) => {
                let (field_name, _) = resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?;
                let val = frame.pop()?;
                let sidx = static_field_idx(ctx, &field_name)?;
                ctx.static_fields[sidx] = val;
            }

            // ---- Instance method dispatch ----
            //
            // Both invokespecial and invokevirtual use the same dispatch in
            // Phase 6: resolve name+descriptor from CP, pop args + this ref,
            // push a new CallFrame.  The JVM distinction (vtable vs. direct
            // dispatch) is deferred to Phase 7 when we add inheritance.
            Instruction::Invokespecial(cp_idx) | Instruction::Invokevirtual(cp_idx) => {
                let (callee_name, callee_desc) =
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?;
                // Skip <clinit> silently (static initialiser, not needed yet).
                if callee_name == "<clinit>" {
                    idx += 1;
                    continue;
                }
                let callee_idx = ctx
                    .methods
                    .iter()
                    .position(|m| m.name == callee_name && m.descriptor == callee_desc)
                    .ok_or_else(|| VmError::MethodNotFound {
                        name: callee_name.clone(),
                        descriptor: callee_desc.clone(),
                    })?;
                // Pop explicit args (not including `this`).
                let arg_count = parse_arg_count(&callee_desc);
                let mut callee_args: Vec<Slot> =
                    (0..arg_count).map(|_| frame.pop()).collect::<VmResult<Vec<_>>>()?;
                callee_args.reverse();
                // Pop `this` reference and prepend as locals[0].
                let this_ref = frame.pop()?; // may be Reference(Some) or Reference(None)
                callee_args.insert(0, this_ref);
                let callee_pc_to_idx: HashMap<usize, usize> = ctx.methods[callee_idx]
                    .instructions
                    .iter()
                    .enumerate()
                    .map(|(i, &(pc, _))| (pc, i))
                    .collect();
                let callee_frame = Frame::new(
                    usize::from(ctx.methods[callee_idx].max_stack),
                    usize::from(ctx.methods[callee_idx].max_locals),
                    callee_args,
                )?;
                call_stack.push(CallFrame {
                    frame,
                    method_idx,
                    pc_to_idx,
                    resume_idx: idx + 1,
                });
                frame = callee_frame;
                method_idx = callee_idx;
                pc_to_idx = callee_pc_to_idx;
                idx = 0;
                continue;
            }

            // ---- Reference return ----
            Instruction::Areturn => {
                let v = frame.pop()?; // Slot::Reference(...)
                do_return!(Some(v));
            }
```

### Step 4: Run the failing tests

Run: `cargo test -p duke-interpreter point_`

Expected: All 4 `point_` tests pass.

Run: `cargo test -p duke-interpreter`

Expected: All 49 previous + 4 new = **53 tests pass**.

If `point_sum_direct` fails with `MethodNotFound` for `<init>`, check that `build_class_context` is decoding the constructor — its name in CP is `<init>`, check `cp_str` resolves it.

### Step 5: Add unit tests for resolve_fieldref

Add to the test block:

```rust
    // ---- Unit tests: resolve_fieldref ----

    #[test]
    fn resolve_fieldref_valid() {
        use duke_classfile::types::CpIndex;
        // CP: [1]=Fieldref{class=2, nat=3}, [2]=Class{name=4}, [3]=NameAndType{name=4, desc=5}
        // [4]=Utf8("x"), [5]=Utf8("I")
        let cp = make_cp(vec![
            Some(CpEntry::Fieldref {
                class_index: CpIndex(2),
                name_and_type_index: CpIndex(3),
            }),
            Some(CpEntry::Class { name_index: CpIndex(4) }),
            Some(CpEntry::NameAndType {
                name_index: CpIndex(4),
                descriptor_index: CpIndex(5),
            }),
            Some(CpEntry::Utf8("x".to_string())),
            Some(CpEntry::Utf8("I".to_string())),
        ]);
        let (name, desc) = resolve_fieldref(&cp, 1).unwrap();
        assert_eq!(name, "x");
        assert_eq!(desc, "I");
    }

    #[test]
    fn resolve_fieldref_invalid() {
        let cp = make_cp(vec![Some(CpEntry::Utf8("not a fieldref".to_string()))]);
        let err = resolve_fieldref(&cp, 1).unwrap_err();
        assert!(matches!(err, VmError::InvalidFieldref { .. }));
    }
```

Run: `cargo test -p duke-interpreter`

Expected: 55 tests pass.

### Step 6: Run clippy

Run: `cargo clippy -p duke-interpreter -- -W clippy::pedantic -W clippy::nursery`

Fix any new warnings. Common ones:
- `clippy::match_same_arms` on Invokespecial/Invokevirtual — they share an arm, which is correct.
- Any lint on the `do_return!` macro usage.

### Step 7: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): implement new/getfield/putfield/getstatic/putstatic/invokespecial/invokevirtual/areturn"
```

---

## Task 6: Update duke binary to use new execute_class signature

**Files:**
- Modify: `duke/Cargo.toml`
- Modify: `duke/src/main.rs`

### Step 1: Add duke-gc dependency to duke/Cargo.toml

```toml
[dependencies]
duke-classfile = { path = "../crates/duke-classfile" }
duke-bytecode = { path = "../crates/duke-bytecode" }
duke-loader = { path = "../crates/duke-loader" }
duke-runtime = { path = "../crates/duke-runtime" }
duke-gc = { path = "../crates/duke-gc" }
duke-interpreter = { path = "../crates/duke-interpreter" }
```

### Step 2: Update exec_method in main.rs

The function currently calls `execute_class(&ctx, ...)`. Change to:

1. Import `duke_gc::Heap` at the top of main.rs:
   ```rust
   use duke_gc::Heap;
   ```

2. Import `build_class_context` alongside the existing interpreter imports:
   ```rust
   use duke_interpreter::{ClassContext, MethodEntry, build_class_context, execute_class};
   ```
   Remove the old `MethodEntry` and `ClassContext` manual construction — replace with `build_class_context(&cf)`.

3. In `exec_method`, replace the manual `methods` vec construction + `ClassContext { ... }` with:
   ```rust
   let mut ctx = build_class_context(&cf);
   let mut heap = Heap::new();
   ```

4. Update the `execute_class` call:
   ```rust
   match execute_class(&mut ctx, &mut heap, method_name, &descriptor, &int_args) {
   ```

### Step 3: Build and smoke test

Run: `cargo build --bin duke`

```bash
./target/debug/duke exec tests/fixtures/Point.class sumPoints 1 2 3 4
# Expected: Int(10)

./target/debug/duke exec tests/fixtures/MathUtils.class sumOfSquares 3 4
# Expected: Int(25)

./target/debug/duke exec tests/fixtures/Arithmetic.class add 3 4
# Expected: Int(7)
```

### Step 4: Full test suite

Run: `cargo test`

Expected: All tests pass. Count: ~59+ total.

### Step 5: Clippy workspace

Run: `cargo clippy --workspace -- -W clippy::pedantic -W clippy::nursery`

Expected: No new warnings in changed files (pre-existing warnings in duke-bytecode/duke-loader are OK).

### Step 6: Format check

Run: `cargo fmt --check`

If it fails, run `cargo fmt` first, then re-verify tests pass.

### Step 7: Commit

```bash
git add duke/Cargo.toml duke/src/main.rs
git commit -m "feat(duke): update exec to use build_class_context and heap-aware execute_class"
```

---

## Parallelism Map

Tasks 1, 2, 3 are **fully independent** — no shared files, no ordering constraint.
Tasks 4, 5, 6 are **sequential** — each depends on the previous.

```
Wave 1 (parallel): Task 1 | Task 2 | Task 3
Wave 2 (sequential): Task 4 → Task 5 → Task 6
Wave 3 (lead): final verification
```

---

## Architecture Note: Why invokespecial ≈ invokevirtual in Phase 6

In the full JVM spec:
- `invokespecial`: Direct dispatch (no vtable lookup). Used for `<init>`, `private`, and `super` calls.
- `invokevirtual`: Virtual dispatch via vtable. Used for normal instance method calls.

In Phase 6, `ClassContext` has no vtable and no class hierarchy. Both instructions resolve to "find method by name+descriptor in current class". This is correct for single-class programs. Phase 7 (when we add a class registry + inheritance) will split these.

Document this decision in `docs/adr/006-simplified-virtual-dispatch.md` (optional, skip if time-constrained).
