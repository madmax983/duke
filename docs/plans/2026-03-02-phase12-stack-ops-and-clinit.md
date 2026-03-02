# Phase 12: Stack Manipulation Opcodes & Static Initializers Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the 5 missing stack manipulation opcodes (dup_x1, dup_x2, dup2, dup2_x1, dup2_x2) and `<clinit>` static initializer execution on class load, raising JVM compliance for real-world Java code.

**Architecture:** Stack ops are pure Frame operations added to the execute() and execute_class() match arms. Static initializers require tracking which classes have been initialized and running `<clinit>` on first use. A `HashSet<String>` of initialized class names on `ClassRegistry` prevents double-init. `<clinit>` is invoked via the existing call stack mechanism before the first method dispatch into a class.

**Tech Stack:** Rust, duke-interpreter crate, duke-runtime (Frame)

---

## Background

### Current Gaps
1. **Stack manipulation**: `dup_x1`, `dup_x2`, `dup2`, `dup2_x1`, `dup2_x2` are unimplemented (the simple `dup`, `pop`, `pop2`, `swap` already work)
2. **Static initializers**: `<clinit>` methods are silently skipped. Classes with `static int x = 42;` or `static { ... }` blocks won't initialize properly.

### JVM Stack Manipulation Semantics
```
dup_x1:    ..., v2, v1 → ..., v1, v2, v1
dup_x2:    ..., v3, v2, v1 → ..., v1, v3, v2, v1
dup2:      ..., v2, v1 → ..., v2, v1, v2, v1
dup2_x1:   ..., v3, v2, v1 → ..., v2, v1, v3, v2, v1
dup2_x2:   ..., v4, v3, v2, v1 → ..., v2, v1, v4, v3, v2, v1
```
(These are computational-type-1 semantics; we don't implement category-2 splitting since Long/Double are single slots in Duke.)

### Static Initializer Semantics
- `<clinit>` runs **once** per class, on first active use (new, getstatic, putstatic, invokestatic)
- Must complete before any other method in the class executes
- Circular dependencies: JVM spec says the class currently running `<clinit>` is considered initialized (prevents infinite loops)

---

## Task 1: Test Fixtures — StackOps.java and StaticInit.java

**Files:**
- Create: `tests/fixtures/StackOps.java`
- Create: `tests/fixtures/StackOps.class`
- Create: `tests/fixtures/StaticInit.java`
- Create: `tests/fixtures/StaticInit.class`

### Step 1: Write StackOps.java

```java
public class StackOps {
    /**
     * Uses dup_x1 pattern: swap + keep top.
     * Compiles to: iconst_1, iconst_2, dup_x1, iadd, iadd → 1+2+2 = 5
     * Actually javac may not emit dup_x1 for simple arithmetic, so we use
     * a pattern that forces it: array store with dup for index reuse.
     */

    /** Returns a + b + b (exercises dup via compiler pattern). */
    public static int dupAdd(int a, int b) {
        // Simple approach: rely on the compiler. Let's just test manually.
        return a + b;
    }

    /**
     * Create array, store at index 0 and return.
     * javac uses dup for array reference preservation during stores.
     */
    public static int arrayStoreDup() {
        int[] arr = new int[3];
        arr[0] = 10;
        arr[1] = 20;
        arr[2] = 30;
        return arr[0] + arr[1] + arr[2]; // 60
    }

    /**
     * String concatenation typically uses dup/dup_x1 for StringBuilder chains.
     * But we don't have StringBuilder yet, so use a pattern the compiler emits.
     */
    public static int multiAssign() {
        int[] a = new int[2];
        int[] b = new int[2];
        a[0] = 1;
        a[1] = 2;
        b[0] = 3;
        b[1] = 4;
        return a[0] + a[1] + b[0] + b[1]; // 10
    }
}
```

### Step 2: Write StaticInit.java

```java
public class StaticInit {
    static int x = 42;
    static int y = x + 8; // 50
    static int z;

    static {
        z = x + y; // 92
    }

    /** Returns the statically initialized value of x. */
    public static int getX() {
        return x; // 42
    }

    /** Returns the statically initialized value of y. */
    public static int getY() {
        return y; // 50
    }

    /** Returns the statically initialized value of z (from static block). */
    public static int getZ() {
        return z; // 92
    }

    /** Returns x + y + z. */
    public static int sum() {
        return x + y + z; // 184
    }
}
```

### Step 3: Compile both

```bash
cd tests/fixtures && javac --release 21 StackOps.java StaticInit.java
```

### Step 4: Verify bytecode

```bash
cargo run -- dump tests/fixtures/StackOps.class
cargo run -- dump tests/fixtures/StaticInit.class
```

For StaticInit, expect to see a `<clinit>` method with `putstatic` instructions.

### Step 5: Commit

```bash
git add tests/fixtures/StackOps.java tests/fixtures/StackOps.class tests/fixtures/StaticInit.java tests/fixtures/StaticInit.class
git commit -m "test(phase12): add StackOps and StaticInit fixtures"
```

---

## Task 2: Implement Stack Manipulation Opcodes

Add `dup_x1`, `dup_x2`, `dup2`, `dup2_x1`, `dup2_x2` to both `execute()` and `execute_class()`.

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Add helper methods to Frame

In `crates/duke-runtime/src/frame.rs`, add stack manipulation helpers:

```rust
/// Insert `val` at position `n` from the top of the stack.
/// `insert_at(0)` is equivalent to push.
/// `insert_at(1)` inserts below the top element.
pub fn insert_at(&mut self, n: usize, val: Slot) -> VmResult<()> {
    if self.operand_stack.len() >= self.max_stack {
        return Err(VmError::StackOverflow);
    }
    let pos = self.operand_stack.len().checked_sub(n)
        .ok_or(VmError::StackUnderflow)?;
    self.operand_stack.insert(pos, val);
    Ok(())
}

/// Peek at the top of the stack without removing.
pub fn peek(&self) -> VmResult<Slot> {
    self.operand_stack.last().copied().ok_or(VmError::StackUnderflow)
}

/// Peek at the nth element from the top (0 = top).
pub fn peek_at(&self, n: usize) -> VmResult<Slot> {
    let idx = self.operand_stack.len().checked_sub(n + 1)
        .ok_or(VmError::StackUnderflow)?;
    Ok(self.operand_stack[idx])
}
```

### Step 2: Write failing tests for stack ops

In the interpreter test module:

```rust
#[test]
fn stack_ops_dup_x1() {
    // dup_x1: ..., v2, v1 → ..., v1, v2, v1
    let instrs = vec![
        (0, Instruction::Iconst2),   // push 2 (v2)
        (1, Instruction::Iconst3),   // push 3 (v1)
        (2, Instruction::DupX1),     // → 3, 2, 3
        (3, Instruction::Iadd),      // → 3, 5
        (4, Instruction::Iadd),      // → 8
        (5, Instruction::Ireturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
    assert_eq!(r, Some(Slot::Int(8)));
}

#[test]
fn stack_ops_dup_x2() {
    // dup_x2: ..., v3, v2, v1 → ..., v1, v3, v2, v1
    let instrs = vec![
        (0, Instruction::Iconst1),   // push 1 (v3)
        (1, Instruction::Iconst2),   // push 2 (v2)
        (2, Instruction::Iconst3),   // push 3 (v1)
        (3, Instruction::DupX2),     // → 3, 1, 2, 3
        (4, Instruction::Iadd),      // → 3, 1, 5
        (5, Instruction::Iadd),      // → 3, 6
        (6, Instruction::Iadd),      // → 9
        (7, Instruction::Ireturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
    assert_eq!(r, Some(Slot::Int(9)));
}

#[test]
fn stack_ops_dup2() {
    // dup2: ..., v2, v1 → ..., v2, v1, v2, v1
    let instrs = vec![
        (0, Instruction::Iconst4),   // push 4 (v2)
        (1, Instruction::Iconst5),   // push 5 (v1)
        (2, Instruction::Dup2),      // → 4, 5, 4, 5
        (3, Instruction::Iadd),      // → 4, 5, 9
        (4, Instruction::Iadd),      // → 4, 14
        (5, Instruction::Iadd),      // → 18
        (6, Instruction::Ireturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
    assert_eq!(r, Some(Slot::Int(18)));
}

#[test]
fn stack_ops_dup2_x1() {
    // dup2_x1: ..., v3, v2, v1 → ..., v2, v1, v3, v2, v1
    let instrs = vec![
        (0, Instruction::Iconst1),   // 1 (v3)
        (1, Instruction::Iconst2),   // 2 (v2)
        (2, Instruction::Iconst3),   // 3 (v1)
        (3, Instruction::Dup2X1),    // → 2, 3, 1, 2, 3
        (4, Instruction::Iadd),      // → 2, 3, 1, 5
        (5, Instruction::Iadd),      // → 2, 3, 6
        (6, Instruction::Iadd),      // → 2, 9
        (7, Instruction::Iadd),      // → 11
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
    assert_eq!(r, Some(Slot::Int(11)));
}

#[test]
fn stack_ops_dup2_x2() {
    // dup2_x2: ..., v4, v3, v2, v1 → ..., v2, v1, v4, v3, v2, v1
    let instrs = vec![
        (0, Instruction::Iconst1),   // 1 (v4)
        (1, Instruction::Iconst2),   // 2 (v3)
        (2, Instruction::Iconst3),   // 3 (v2)
        (3, Instruction::Iconst4),   // 4 (v1)
        (4, Instruction::Dup2X2),    // → 3, 4, 1, 2, 3, 4
        (5, Instruction::Iadd),      // → 3, 4, 1, 2, 7
        (6, Instruction::Iadd),      // → 3, 4, 1, 9
        (7, Instruction::Iadd),      // → 3, 4, 10
        (8, Instruction::Iadd),      // → 3, 14
        (9, Instruction::Iadd),      // → 17
        (10, Instruction::Ireturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
    assert_eq!(r, Some(Slot::Int(17)));
}
```

### Step 3: Implement the opcodes

In `execute()`, find where `Dup`/`Pop`/`Swap` are handled and add:

```rust
Instruction::DupX1 => {
    let v1 = frame.pop()?;
    let v2 = frame.pop()?;
    frame.push(v1)?;
    frame.push(v2)?;
    frame.push(v1)?;
}
Instruction::DupX2 => {
    let v1 = frame.pop()?;
    let v2 = frame.pop()?;
    let v3 = frame.pop()?;
    frame.push(v1)?;
    frame.push(v3)?;
    frame.push(v2)?;
    frame.push(v1)?;
}
Instruction::Dup2 => {
    let v1 = frame.pop()?;
    let v2 = frame.pop()?;
    frame.push(v2)?;
    frame.push(v1)?;
    frame.push(v2)?;
    frame.push(v1)?;
}
Instruction::Dup2X1 => {
    let v1 = frame.pop()?;
    let v2 = frame.pop()?;
    let v3 = frame.pop()?;
    frame.push(v2)?;
    frame.push(v1)?;
    frame.push(v3)?;
    frame.push(v2)?;
    frame.push(v1)?;
}
Instruction::Dup2X2 => {
    let v1 = frame.pop()?;
    let v2 = frame.pop()?;
    let v3 = frame.pop()?;
    let v4 = frame.pop()?;
    frame.push(v2)?;
    frame.push(v1)?;
    frame.push(v4)?;
    frame.push(v3)?;
    frame.push(v2)?;
    frame.push(v1)?;
}
```

Add the same match arms in `execute_class()` (same logic — both execute loops need them).

### Step 4: Run tests

```bash
cargo test -p duke-interpreter -- stack_ops
```

Expected: 5 new tests pass.

### Step 5: Run full suite + clippy

```bash
cargo test && cargo clippy --workspace -- -W clippy::pedantic -W clippy::nursery
```

### Step 6: Commit

```bash
git add crates/duke-interpreter/src/lib.rs crates/duke-runtime/src/frame.rs
git commit -m "feat(interpreter): implement dup_x1, dup_x2, dup2, dup2_x1, dup2_x2 stack ops"
```

---

## Task 3: Implement `<clinit>` Static Initializer Execution

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Add `initialized` tracking to ClassRegistry

```rust
pub struct ClassRegistry {
    classes: HashMap<String, ClassContext>,
    natives: NativeRegistry,
    /// Tracks which classes have had their `<clinit>` run.
    initialized: HashSet<String>,
}
```

Update `new()`:
```rust
pub fn new() -> Self {
    Self {
        classes: HashMap::new(),
        natives: NativeRegistry::new(),
        initialized: HashSet::new(),
    }
}
```

Add methods:
```rust
/// Check if a class has been initialized (clinit has run).
pub fn is_initialized(&self, name: &str) -> bool {
    self.initialized.contains(name)
}

/// Mark a class as initialized.
pub fn mark_initialized(&mut self, name: &str) {
    self.initialized.insert(name.to_string());
}
```

### Step 2: Add `ensure_initialized` function

Add a helper that checks if a class needs initialization and runs `<clinit>` if present:

```rust
/// Ensure a class is initialized. Runs `<clinit>` if present and not yet run.
///
/// Must be called before first active use of a class (new, getstatic, putstatic, invokestatic).
fn ensure_initialized(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut Heap,
    stdout: &mut dyn Write,
    class_name: &str,
) -> VmResult<()> {
    if registry.is_initialized(class_name) {
        return Ok(());
    }
    // Mark as initialized BEFORE running clinit to prevent infinite recursion.
    registry.mark_initialized(class_name);

    // Check if the class has a <clinit> method.
    let clinit_idx = {
        let ctx = match registry.get(class_name) {
            Ok(ctx) => ctx,
            Err(_) => return Ok(()), // class not loaded yet, skip
        };
        ctx.methods.iter().position(|m| m.name == "<clinit>" && m.descriptor == "()V")
    };

    if let Some(_clinit_idx) = clinit_idx {
        // Run <clinit> by calling it through execute_class.
        execute_class(registry, loader, heap, stdout, class_name, "<clinit>", "()V", &[])?;
    }
    Ok(())
}
```

### Step 3: Call ensure_initialized at class-use points

Add `ensure_initialized` calls at these points in `execute_class()`:

1. **Before the main dispatch** (top of execute_class, after finding the entry method):
```rust
ensure_initialized(registry, loader, heap, stdout, class_name)?;
```

2. **After ensure_loaded succeeds for invokestatic** (before method lookup):
```rust
registry.ensure_loaded(&callee_class, loader)?;
ensure_initialized(registry, loader, heap, stdout, &callee_class)?;
```

3. **After ensure_loaded for getstatic/putstatic**:
```rust
ensure_initialized(registry, loader, heap, stdout, &target_class)?;
```

4. **After ensure_loaded for new**:
```rust
ensure_initialized(registry, loader, heap, stdout, &target_class)?;
```

### Step 4: Write failing integration tests

```rust
// ---- Phase 12: static initializer tests ----

fn load_static_init_class() -> ClassContext {
    let bytes = std::fs::read("tests/fixtures/StaticInit.class").expect("StaticInit.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn clinit_initializes_static_field_x() {
    let ctx = load_static_init_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut sink,
        "StaticInit", "getX", "()I", &[],
    ).unwrap();
    assert_eq!(result, Some(Slot::Int(42)));
}

#[test]
fn clinit_initializes_dependent_field_y() {
    let ctx = load_static_init_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut sink,
        "StaticInit", "getY", "()I", &[],
    ).unwrap();
    assert_eq!(result, Some(Slot::Int(50)));
}

#[test]
fn clinit_runs_static_block() {
    let ctx = load_static_init_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut sink,
        "StaticInit", "getZ", "()I", &[],
    ).unwrap();
    assert_eq!(result, Some(Slot::Int(92)));
}

#[test]
fn clinit_sum_all_statics() {
    let ctx = load_static_init_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut sink,
        "StaticInit", "sum", "()I", &[],
    ).unwrap();
    assert_eq!(result, Some(Slot::Int(184)));
}
```

### Step 5: Run tests

```bash
cargo test -p duke-interpreter -- clinit
```

Expected: 4 tests pass.

### Step 6: Run full suite + clippy

```bash
cargo test && cargo clippy --workspace -- -W clippy::pedantic -W clippy::nursery
```

### Step 7: Demo

```bash
cargo run -- exec tests/fixtures/StaticInit.class getX
# Expected: Int(42)

cargo run -- exec tests/fixtures/StaticInit.class sum
# Expected: Int(184)
```

### Step 8: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): implement <clinit> static initializer execution on class load"
```

---

## Task 4: Integration Tests for StackOps Fixture + Final Verification

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (add integration tests)

### Step 1: Add StackOps integration tests

```rust
fn load_stack_ops_class() -> ClassContext {
    let bytes = std::fs::read("tests/fixtures/StackOps.class").expect("StackOps.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn stack_ops_array_store_dup() {
    let ctx = load_stack_ops_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut sink,
        "StackOps", "arrayStoreDup", "()I", &[],
    ).unwrap();
    assert_eq!(result, Some(Slot::Int(60)));
}

#[test]
fn stack_ops_multi_assign() {
    let ctx = load_stack_ops_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut sink,
        "StackOps", "multiAssign", "()I", &[],
    ).unwrap();
    assert_eq!(result, Some(Slot::Int(10)));
}
```

### Step 2: Full verification

```bash
cargo test 2>&1
cargo clippy --workspace -- -W clippy::pedantic -W clippy::nursery 2>&1
cargo fmt --check 2>&1
```

Expected: ~164+ tests pass (153 existing + 5 stack op unit + 4 clinit + 2 stack integration).

### Step 3: Commit if not already committed with earlier tasks

---

## Wave Execution Strategy

**Wave 1 (parallel):**
- Agent A: Task 1 (fixtures — StackOps.java + StaticInit.java)
- Agent B: Task 2 (stack manipulation opcodes + unit tests) — can start with unit tests since they use `execute()` not integration fixtures

**Wave 2 (sequential):**
- Agent C: Task 3 (clinit implementation + integration tests) + Task 4 (StackOps integration tests + final verification)

**Wave 3 (leader):**
- Verify all tests, run demos, commit, update memory
