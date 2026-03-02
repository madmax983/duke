# Phase 10: Multi-Class Method Dispatch Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable cross-class method calls by integrating class loading into the interpreter, so methods in one class can invoke methods, create objects, and access fields in other classes.

**Architecture:** Add a `ClassRegistry` (HashMap<String, ClassContext>) that holds multiple loaded classes. Refactor `execute_class()` to track a `current_class` per call frame and dispatch cross-class calls by loading the target class on demand via a `ClassLoader`. Unloadable classes (e.g., `java/lang/Object`) fall back to the existing no-op behavior.

**Tech Stack:** Rust, `duke-interpreter` (execute_class, ClassRegistry), `duke-loader` (ClassLoader trait, DirectoryLoader), `duke-runtime` (VmError), `duke-classfile` (parse).

---

## Background: Current Architecture

### What works (same-class only)
- `invokestatic` dispatches to methods within the same ClassContext
- `invokespecial`/`invokevirtual` dispatch within same class; cross-class calls are **no-ops** (pop args + this)
- `new` allocates using the current class's `instance_field_count` (wrong for other classes)
- `getstatic`/`putstatic` access current class's static fields only
- `getfield`/`putfield` access fields using current class's field index

### What needs to change
- `execute_class` takes `&mut ClassContext` → needs `&mut ClassRegistry`
- `CallFrame` lacks `class_name` → needs it to restore context on return
- `resolve_fieldref` returns `(field_name, desc)` → needs `(class_name, field_name, desc)`
- No dependency on `duke-loader` from `duke-interpreter`

### Key files
| File | Purpose |
|------|---------|
| `crates/duke-interpreter/src/lib.rs` | Main interpreter, 3783 lines |
| `crates/duke-interpreter/Cargo.toml` | Dependencies |
| `crates/duke-runtime/src/error.rs` | VmError enum |
| `crates/duke-loader/src/lib.rs` | ClassLoader trait |
| `duke/src/main.rs` | Binary crate |

---

## Wave 1: Independent Preparatory Work (run both tasks in parallel)

### Task 1: Create CrossCall + Pair Test Fixtures

**Files:**
- Create: `tests/fixtures/Callee.java`
- Create: `tests/fixtures/Callee.class`
- Create: `tests/fixtures/CrossCall.java`
- Create: `tests/fixtures/CrossCall.class`
- Create: `tests/fixtures/Pair.java`
- Create: `tests/fixtures/Pair.class`
- Create: `tests/fixtures/PairUser.java`
- Create: `tests/fixtures/PairUser.class`

**Step 1: Write Callee.java**

Create `tests/fixtures/Callee.java`:

```java
/**
 * Phase 10 fixture — a simple utility class called from CrossCall.
 * All methods are static for invokestatic testing.
 */
public class Callee {
    public static int add(int a, int b) {
        return a + b;
    }

    public static int doubleVal(int n) {
        return n * 2;
    }

    public static int negate(int n) {
        return -n;
    }
}
```

**Step 2: Write CrossCall.java**

Create `tests/fixtures/CrossCall.java`:

```java
/**
 * Phase 10 fixture — calls methods in Callee via cross-class invokestatic.
 */
public class CrossCall {
    /** Simple cross-class call: Callee.add(a, b). */
    public static int callAdd(int a, int b) {
        return Callee.add(a, b);
    }

    /** Cross-class with single arg: Callee.doubleVal(n). */
    public static int callDouble(int n) {
        return Callee.doubleVal(n);
    }

    /** Nested cross-class: Callee.add(Callee.doubleVal(n), n). */
    public static int chainCall(int n) {
        return Callee.add(Callee.doubleVal(n), n);
    }

    /** Cross-class call to negate, then add: Callee.add(n, Callee.negate(n)). */
    public static int addNegated(int n) {
        return Callee.add(n, Callee.negate(n));
    }
}
```

**Step 3: Write Pair.java**

Create `tests/fixtures/Pair.java`:

```java
/**
 * Phase 10 fixture — a class with instance fields, constructor, and methods.
 * Used by PairUser to test cross-class new/invokespecial/invokevirtual.
 */
public class Pair {
    private int x;
    private int y;

    public Pair(int x, int y) {
        this.x = x;
        this.y = y;
    }

    public int sum() {
        return x + y;
    }

    public int diff() {
        return x - y;
    }
}
```

**Step 4: Write PairUser.java**

Create `tests/fixtures/PairUser.java`:

```java
/**
 * Phase 10 fixture — creates Pair objects and calls their methods.
 * Tests cross-class new, invokespecial (<init>), and invokevirtual.
 */
public class PairUser {
    /** Create a Pair and return its sum. */
    public static int makePairSum(int x, int y) {
        Pair p = new Pair(x, y);
        return p.sum();
    }

    /** Create a Pair and return its diff. */
    public static int makePairDiff(int x, int y) {
        Pair p = new Pair(x, y);
        return p.diff();
    }

    /** Create two Pairs and return the sum of their sums. */
    public static int twoPairsSum(int a, int b, int c, int d) {
        Pair p1 = new Pair(a, b);
        Pair p2 = new Pair(c, d);
        return p1.sum() + p2.sum();
    }
}
```

**Step 5: Compile all fixtures**

```bash
cd tests/fixtures
javac --release 21 Callee.java CrossCall.java Pair.java PairUser.java
```

Verify all 4 `.class` files are created:

```bash
ls -la Callee.class CrossCall.class Pair.class PairUser.class
```

**Step 6: Commit**

```bash
git add tests/fixtures/Callee.java tests/fixtures/Callee.class \
        tests/fixtures/CrossCall.java tests/fixtures/CrossCall.class \
        tests/fixtures/Pair.java tests/fixtures/Pair.class \
        tests/fixtures/PairUser.java tests/fixtures/PairUser.class
git commit -m "test(phase10): add CrossCall/Callee + PairUser/Pair fixtures for cross-class dispatch"
```

---

### Task 2: Add ClassRegistry + ClassNotFound VmError

**Files:**
- Modify: `crates/duke-interpreter/Cargo.toml` (add duke-loader + duke-classfile deps)
- Modify: `crates/duke-interpreter/src/lib.rs` (add ClassRegistry struct)
- Modify: `crates/duke-runtime/src/error.rs` (add ClassNotFound variant)
- Modify: `crates/duke-runtime/src/lib.rs` (add test for ClassNotFound)

**Step 1: Add test for ClassNotFound VmError**

In `crates/duke-runtime/src/lib.rs`, add to the `#[cfg(test)]` block:

```rust
#[test]
fn class_not_found_error_message() {
    let e = VmError::ClassNotFound {
        name: "com/example/Missing".to_string(),
    };
    assert_eq!(
        e.to_string(),
        "class not found: com/example/Missing"
    );
}
```

**Step 2: Add ClassNotFound to VmError**

In `crates/duke-runtime/src/error.rs`, add after the `ClassCastException` variant:

```rust
#[error("class not found: {name}")]
ClassNotFound { name: String },
```

**Step 3: Run runtime tests**

```bash
cargo test -p duke-runtime 2>&1
```

Expected: all pass including new test.

**Step 4: Add duke-loader dependency to duke-interpreter**

In `crates/duke-interpreter/Cargo.toml`, add to `[dependencies]`:

```toml
duke-loader = { path = "../duke-loader" }
```

**Step 5: Add ClassRegistry struct to lib.rs**

At the top of `crates/duke-interpreter/src/lib.rs`, after the existing use statements (line 12),
add:

```rust
use duke_loader::ClassLoader;
```

Then after the `ClassContext` struct definition (after line 56), add:

```rust
/// Registry of loaded classes — maps class name to its ClassContext.
///
/// Used by `execute_class` for cross-class method dispatch.
pub struct ClassRegistry {
    classes: HashMap<String, ClassContext>,
}

impl ClassRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
        }
    }

    /// Register a pre-built ClassContext.
    pub fn register(&mut self, ctx: ClassContext) {
        self.classes.insert(ctx.class_name.clone(), ctx);
    }

    /// Get a reference to a loaded class.
    ///
    /// # Errors
    /// Returns [`VmError::ClassNotFound`] if the class is not loaded.
    pub fn get(&self, name: &str) -> VmResult<&ClassContext> {
        self.classes
            .get(name)
            .ok_or_else(|| VmError::ClassNotFound {
                name: name.to_string(),
            })
    }

    /// Get a mutable reference to a loaded class.
    ///
    /// # Errors
    /// Returns [`VmError::ClassNotFound`] if the class is not loaded.
    pub fn get_mut(&mut self, name: &str) -> VmResult<&mut ClassContext> {
        self.classes
            .get_mut(name)
            .ok_or_else(|| VmError::ClassNotFound {
                name: name.to_string(),
            })
    }

    /// Ensure a class is loaded. If not already present, loads it via the class
    /// loader, parses it, builds a ClassContext, and registers it.
    ///
    /// Returns `Ok(true)` if loaded, `Ok(false)` if the class could not be found
    /// (soft failure — for classes like `java/lang/Object` that we can't load yet).
    pub fn ensure_loaded(&mut self, name: &str, loader: &dyn ClassLoader) -> VmResult<bool> {
        if self.classes.contains_key(name) {
            return Ok(true);
        }
        let bytes = match loader.find_class(name) {
            Ok(b) => b,
            Err(_) => return Ok(false), // soft failure
        };
        let cf = match duke_classfile::parse(&bytes) {
            Ok(cf) => cf,
            Err(_) => return Ok(false), // soft failure
        };
        let ctx = build_class_context(&cf);
        self.classes.insert(name.to_string(), ctx);
        Ok(true)
    }

    /// Check if a class is loaded.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }
}

impl Default for ClassRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 6: Run full test suite (check for compile + no regressions)**

```bash
cargo test 2>&1 | tail -15
```

Expected: all 139 existing tests still pass. ClassRegistry is added but not yet used.

**Step 7: fmt + clippy**

```bash
cargo fmt && cargo clippy --all -- -W clippy::pedantic 2>&1 | grep "^error" | head -5
```

**Step 8: Commit**

```bash
git add crates/duke-runtime/src/error.rs crates/duke-runtime/src/lib.rs \
        crates/duke-interpreter/Cargo.toml crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter,runtime): add ClassRegistry struct and ClassNotFound VmError

- ClassRegistry: HashMap<String, ClassContext> with ensure_loaded() for lazy loading
- ensure_loaded() returns Ok(false) for unloadable classes (soft failure)
- ClassNotFound VmError variant for missing classes
- duke-loader added as duke-interpreter dependency"
```

---

## Wave 2: Core Implementation (sequential, depends on Wave 1)

### Task 3: Refactor execute_class for Cross-Class Dispatch

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (major refactoring)
- Modify: `duke/src/main.rs` (update exec_method)

**Background:**

This is the big refactoring task. The changes touch:
1. `CallFrame` struct — add `class_name` field
2. `execute_class` signature — `&mut ClassContext` → `&mut ClassRegistry` + `&dyn ClassLoader`
3. `execute_class` body — track `current_class`, resolve through registry
4. `resolve_fieldref` — return class name (3-tuple instead of 2-tuple)
5. All cross-class dispatch: invokestatic, invokespecial, invokevirtual, new, getstatic, putstatic
6. Test helpers — `run_class_int` etc. build a ClassRegistry
7. Binary — build ClassRegistry with DirectoryLoader

**CRITICAL PATTERN: Scoped borrows to satisfy the borrow checker.**

The current code clones the instruction at line 1237 (`let instr = instr.clone();`) to release
the borrow on `ctx` before the match body mutates state. With the registry, we extend this
pattern: read from registry in a scoped block, release the borrow, then mutate in a
separate block.

```rust
// PATTERN: Read from registry in a block
let (field_name, _desc) = {
    let ctx = registry.get(&current_class)?;
    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
};
// Borrow released — now safe to mutate
registry.get_mut(&current_class)?.static_fields[sidx] = val;
```

---

**Step 1: Add failing integration tests**

Add these tests to the bottom of the `#[cfg(test)]` module in
`crates/duke-interpreter/src/lib.rs`. They will fail to compile until the refactoring is done,
so comment them out with `/* ... */` for now — uncomment in Step 9 after the refactoring.

```rust
    // ---- Phase 10: Cross-class invokestatic ----

    fn run_cross_class_int(
        class_files: &[&str],
        entry_class: &str,
        method_name: &str,
        descriptor: &str,
        args: Vec<i32>,
    ) -> i32 {
        let mut registry = ClassRegistry::new();
        for name in class_files {
            let ctx = load_class_context(name);
            registry.register(ctx);
        }
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
        let mut heap = duke_gc::Heap::new();
        match execute_class(
            &mut registry,
            &loader,
            &mut heap,
            entry_class,
            method_name,
            descriptor,
            &slots,
        )
        .expect("execute_class failed")
        {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn cross_class_add() {
        assert_eq!(
            run_cross_class_int(
                &["CrossCall.class", "Callee.class"],
                "CrossCall",
                "callAdd",
                "(II)I",
                vec![3, 4],
            ),
            7
        );
    }

    #[test]
    fn cross_class_double() {
        assert_eq!(
            run_cross_class_int(
                &["CrossCall.class", "Callee.class"],
                "CrossCall",
                "callDouble",
                "(I)I",
                vec![5],
            ),
            10
        );
    }

    #[test]
    fn cross_class_chain() {
        // Callee.add(Callee.doubleVal(3), 3) = add(6, 3) = 9
        assert_eq!(
            run_cross_class_int(
                &["CrossCall.class", "Callee.class"],
                "CrossCall",
                "chainCall",
                "(I)I",
                vec![3],
            ),
            9
        );
    }

    #[test]
    fn cross_class_add_negated() {
        // Callee.add(5, Callee.negate(5)) = add(5, -5) = 0
        assert_eq!(
            run_cross_class_int(
                &["CrossCall.class", "Callee.class"],
                "CrossCall",
                "addNegated",
                "(I)I",
                vec![5],
            ),
            0
        );
    }

    // ---- Phase 10: Cross-class new + invokevirtual ----

    #[test]
    fn cross_class_pair_sum() {
        assert_eq!(
            run_cross_class_int(
                &["PairUser.class", "Pair.class"],
                "PairUser",
                "makePairSum",
                "(II)I",
                vec![3, 7],
            ),
            10
        );
    }

    #[test]
    fn cross_class_pair_diff() {
        assert_eq!(
            run_cross_class_int(
                &["PairUser.class", "Pair.class"],
                "PairUser",
                "makePairDiff",
                "(II)I",
                vec![10, 3],
            ),
            7
        );
    }

    #[test]
    fn cross_class_two_pairs() {
        // p1.sum() + p2.sum() = (1+2) + (3+4) = 10
        assert_eq!(
            run_cross_class_int(
                &["PairUser.class", "Pair.class"],
                "PairUser",
                "twoPairsSum",
                "(IIII)I",
                vec![1, 2, 3, 4],
            ),
            10
        );
    }
```

**Step 2: Update CallFrame**

Find the `CallFrame` struct (around line 2491):

```rust
// OLD:
struct CallFrame {
    frame: Frame,
    method_idx: usize,
    pc_to_idx: HashMap<usize, usize>,
    resume_idx: usize,
}

// NEW:
struct CallFrame {
    frame: Frame,
    method_idx: usize,
    class_name: String,
    pc_to_idx: HashMap<usize, usize>,
    resume_idx: usize,
}
```

**Step 3: Update resolve_fieldref to return class name**

Find `resolve_fieldref` (around line 2754). Change it to return a 3-tuple like `resolve_methodref`:

```rust
/// Resolve a constant pool Fieldref to (class_name, field_name, descriptor).
fn resolve_fieldref(cp: &[Option<CpEntry>], idx: usize) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Fieldref {
            class_index,
            name_and_type_index,
        }) => {
            let field_class = match cp.get(class_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    }
                }
                _ => return Err(VmError::InvalidFieldref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    Ok((field_class, name, desc))
                }
                _ => Err(VmError::InvalidFieldref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidFieldref { index: idx }),
    }
}
```

**Step 4: Update resolve_fieldref callers**

All callers currently destructure as `let (field_name, _)`. Update them:

- `getfield` (~line 1944): `let (field_name, _)` → `let (_field_class, field_name, _desc)`
- `putfield` (~line 1951): same
- `getstatic` (~line 1958): `let (field_name, _)` → `let (field_class, field_name, _desc)`
- `putstatic` (~line 1964): same

Also update the unit tests for resolve_fieldref (around line 3324):
- `resolve_fieldref_valid`: update the CP to include a Class entry with Utf8 for the class name.
  Change `let (name, desc) = resolve_fieldref(&cp, 1).unwrap();` to
  `let (class_name, name, desc) = resolve_fieldref(&cp, 1).unwrap();`
  Add assertion: `assert_eq!(class_name, "Point");` (or whatever class name the test uses).
- `resolve_fieldref_invalid`: no change needed (still returns error).

**Step 5: Refactor execute_class signature**

Change the signature from:

```rust
pub fn execute_class(
    ctx: &mut ClassContext,
    heap: &mut duke_gc::Heap,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>> {
```

to:

```rust
pub fn execute_class(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>> {
```

**Step 6: Refactor execute_class body — entry + main loop**

Replace the entry method lookup and state init (lines 1207-1237) with:

```rust
    // Find entry method in the specified class.
    let entry_idx = registry
        .get(class_name)?
        .methods
        .iter()
        .position(|m| m.name == method_name && m.descriptor == descriptor)
        .ok_or_else(|| VmError::MethodNotFound {
            name: method_name.to_string(),
            descriptor: descriptor.to_string(),
        })?;

    let mut call_stack: Vec<CallFrame> = Vec::new();
    let mut current_class = class_name.to_string();
    let mut method_idx = entry_idx;
    let mut pc_to_idx: HashMap<usize, usize> = {
        let ctx = registry.get(&current_class)?;
        ctx.methods[method_idx]
            .instructions
            .iter()
            .enumerate()
            .map(|(i, &(pc, _))| (pc, i))
            .collect()
    };
    let mut frame = {
        let ctx = registry.get(&current_class)?;
        Frame::new(
            usize::from(ctx.methods[method_idx].max_stack),
            usize::from(ctx.methods[method_idx].max_locals),
            args.to_vec(),
        )?
    };
    let mut idx: usize = 0;
    let mut string_intern: HashMap<usize, u64> = HashMap::new();
```

Replace the instruction read at the top of the loop (line 1233-1237):

```rust
    loop {
        let (pc, instr) = {
            let ctx = registry.get(&current_class)?;
            let Some(&(pc, ref instr)) = ctx.methods[method_idx].instructions.get(idx) else {
                return Err(VmError::FellOffEnd);
            };
            (pc, instr.clone())
        };
```

**Step 7: Refactor do_return! macro**

Update the `do_return!` macro to restore `current_class`:

```rust
        macro_rules! do_return {
            ($val:expr) => {{
                match call_stack.pop() {
                    None => return Ok($val),
                    Some(caller) => {
                        let ret_val = $val;
                        frame = caller.frame;
                        method_idx = caller.method_idx;
                        current_class = caller.class_name;
                        pc_to_idx = caller.pc_to_idx;
                        idx = caller.resume_idx;
                        if let Some(v) = ret_val {
                            frame.push(v)?;
                        }
                        continue;
                    }
                }
            }};
        }
```

**Step 8: Refactor invokestatic for cross-class dispatch**

Replace the current invokestatic handler (~line 1271-1309) with:

```rust
            Instruction::Invokestatic(cp_idx) => {
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };

                // Try to load the target class
                let loaded = registry.ensure_loaded(&callee_class, loader)?;

                // Find method in target class (fall back to current class for same-class calls)
                let target_class = if loaded || callee_class == current_class {
                    &callee_class
                } else {
                    &current_class
                };

                let callee_idx = registry
                    .get(target_class)?
                    .methods
                    .iter()
                    .position(|m| m.name == callee_name && m.descriptor == callee_desc)
                    .ok_or_else(|| VmError::MethodNotFound {
                        name: callee_name.clone(),
                        descriptor: callee_desc.clone(),
                    })?;

                let arg_count = parse_arg_count(&callee_desc);
                let mut callee_args: Vec<Slot> = (0..arg_count)
                    .map(|_| frame.pop())
                    .collect::<VmResult<Vec<_>>>()?;
                callee_args.reverse();

                let (callee_pc_to_idx, callee_frame) = {
                    let target_ctx = registry.get(target_class)?;
                    let callee_pc: HashMap<usize, usize> = target_ctx.methods[callee_idx]
                        .instructions
                        .iter()
                        .enumerate()
                        .map(|(i, &(pc, _))| (pc, i))
                        .collect();
                    let f = Frame::new(
                        usize::from(target_ctx.methods[callee_idx].max_stack),
                        usize::from(target_ctx.methods[callee_idx].max_locals),
                        callee_args,
                    )?;
                    (callee_pc, f)
                };

                call_stack.push(CallFrame {
                    frame,
                    method_idx,
                    class_name: current_class.clone(),
                    pc_to_idx,
                    resume_idx: idx + 1,
                });
                frame = callee_frame;
                method_idx = callee_idx;
                current_class = target_class.clone();
                pc_to_idx = callee_pc_to_idx;
                idx = 0;
                continue;
            }
```

**Step 9: Refactor invokespecial/invokevirtual for cross-class dispatch**

Replace the current handler (~line 1975-2034) with:

```rust
            Instruction::Invokespecial(cp_idx) | Instruction::Invokevirtual(cp_idx) => {
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };

                // <clinit> is not supported yet — skip silently.
                if callee_name == "<clinit>" {
                    idx += 1;
                    continue;
                }

                // Try to load the target class
                let loaded = registry.ensure_loaded(&callee_class, loader)?;

                // Look up method in target class; fall back to no-op if not found
                let callee_idx = if loaded || callee_class == current_class {
                    registry
                        .get(&callee_class)?
                        .methods
                        .iter()
                        .position(|m| m.name == callee_name && m.descriptor == callee_desc)
                } else {
                    None
                };

                let callee_idx = match callee_idx {
                    Some(i) => i,
                    None => {
                        // Unloadable class or missing method — no-op (pop args + this).
                        let arg_count = parse_arg_count(&callee_desc);
                        for _ in 0..arg_count {
                            frame.pop()?;
                        }
                        frame.pop()?; // pop `this`
                        idx += 1;
                        continue;
                    }
                };

                let arg_count = parse_arg_count(&callee_desc);
                let mut callee_args: Vec<Slot> = (0..arg_count)
                    .map(|_| frame.pop())
                    .collect::<VmResult<Vec<_>>>()?;
                callee_args.reverse();
                let this_slot = frame.pop()?;
                callee_args.insert(0, this_slot);

                let target_class_name = callee_class.clone();
                let (callee_pc_to_idx, callee_frame) = {
                    let target_ctx = registry.get(&target_class_name)?;
                    let callee_pc: HashMap<usize, usize> = target_ctx.methods[callee_idx]
                        .instructions
                        .iter()
                        .enumerate()
                        .map(|(i, &(pc, _))| (pc, i))
                        .collect();
                    let f = Frame::new(
                        usize::from(target_ctx.methods[callee_idx].max_stack),
                        usize::from(target_ctx.methods[callee_idx].max_locals),
                        callee_args,
                    )?;
                    (callee_pc, f)
                };

                call_stack.push(CallFrame {
                    frame,
                    method_idx,
                    class_name: current_class.clone(),
                    pc_to_idx,
                    resume_idx: idx + 1,
                });
                frame = callee_frame;
                method_idx = callee_idx;
                current_class = target_class_name;
                pc_to_idx = callee_pc_to_idx;
                idx = 0;
                continue;
            }
```

**Step 10: Refactor `new` for cross-class allocation**

Replace the current `Instruction::New` handler (~line 1920-1940) with:

```rust
            Instruction::New(cp_idx) => {
                let target_class = {
                    let ctx = registry.get(&current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                // Try to load the target class for its field count
                registry.ensure_loaded(&target_class, loader)?;
                let field_count = registry
                    .get(&target_class)
                    .map(|ctx| ctx.instance_field_count)
                    .unwrap_or(0);
                let r = heap.allocate(target_class, field_count);
                frame.push(Slot::Reference(Some(r)))?;
            }
```

**Step 11: Refactor getstatic/putstatic for cross-class dispatch**

Replace current getstatic/putstatic handlers with:

```rust
            Instruction::Getstatic(cp_idx) => {
                let (field_class, field_name, _desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                // Try loading the field's class; fall back to current class
                let target = if registry.ensure_loaded(&field_class, loader)?
                    || field_class == current_class
                {
                    &field_class
                } else {
                    &current_class
                };
                let sidx = {
                    let ctx = registry.get(target)?;
                    static_field_idx(ctx, &field_name)?
                };
                let val = registry.get(target)?.static_fields[sidx].clone();
                frame.push(val)?;
            }
            Instruction::Putstatic(cp_idx) => {
                let (field_class, field_name, _desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let val = frame.pop()?;
                let target = if registry.ensure_loaded(&field_class, loader)?
                    || field_class == current_class
                {
                    &field_class
                } else {
                    &current_class
                };
                let sidx = {
                    let ctx = registry.get(target)?;
                    static_field_idx(ctx, &field_name)?
                };
                registry.get_mut(target)?.static_fields[sidx] = val;
            }
```

**Step 12: Refactor getfield/putfield for cross-class dispatch**

Replace current getfield/putfield handlers with:

```rust
            Instruction::Getfield(cp_idx) => {
                let (field_class, field_name, _desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let r = frame.pop_ref()?;
                // Resolve field index from the declaring class
                registry.ensure_loaded(&field_class, loader)?;
                let target = if registry.contains(&field_class) {
                    &field_class
                } else {
                    &current_class
                };
                let fidx = {
                    let ctx = registry.get(target)?;
                    instance_field_idx(ctx, &field_name)?
                };
                let val = heap.get(r)?.fields[fidx].clone();
                frame.push(val)?;
            }
            Instruction::Putfield(cp_idx) => {
                let (field_class, field_name, _desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let val = frame.pop()?;
                let r = frame.pop_ref()?;
                registry.ensure_loaded(&field_class, loader)?;
                let target = if registry.contains(&field_class) {
                    &field_class
                } else {
                    &current_class
                };
                let fidx = {
                    let ctx = registry.get(target)?;
                    instance_field_idx(ctx, &field_name)?
                };
                heap.get_mut(r)?.fields[fidx] = val;
            }
```

**Step 13: Update all remaining `ctx.` references in execute_class**

Search for all remaining `ctx.` references in execute_class. Each one needs to go through the
registry. The main ones:

- `ctx.constant_pool` in ldc String handling → `registry.get(&current_class)?.constant_pool`
- `ctx.class_name` in athrow → `registry.get(&current_class)?.class_name.clone()` or just use `current_class`
- `ctx.methods[method_idx]` in exception handler search → through registry

**For ldc String handling (~line 1349-1380):**

```rust
            Instruction::Ldc(raw_idx) => {
                let cp_idx = usize::from(*raw_idx);
                let is_string = {
                    let ctx = registry.get(&current_class)?;
                    matches!(
                        ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref()),
                        Some(CpEntry::String { .. })
                    )
                };
                if is_string {
                    let r = if let Some(&cached) = string_intern.get(&cp_idx) {
                        cached
                    } else {
                        let s = {
                            let ctx = registry.get(&current_class)?;
                            if let Some(CpEntry::String { string_index }) =
                                ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
                            {
                                let si = string_index.0 as usize;
                                match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                                    Some(CpEntry::Utf8(s)) => s.clone(),
                                    _ => return Err(VmError::InvalidCpIndex { index: si }),
                                }
                            } else {
                                unreachable!()
                            }
                        };
                        let r = heap.allocate_string(s);
                        string_intern.insert(cp_idx, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    let ctx = registry.get(&current_class)?;
                    ldc_push(&mut frame, &ctx.constant_pool, cp_idx)?;
                }
            }
```

(Do the same for `LdcW`/`Ldc2W`.)

**For athrow** — find the athrow handler and replace `ctx.` references. The athrow handler
reads the exception class from heap, then searches `ctx.methods[method_idx].exception_table`.
Replace with registry lookups:

```rust
            Instruction::Athrow => {
                let r = frame.pop_ref()?;
                let ex_class = heap.get(r)?.class_name.clone();
                loop {
                    let handler_pc = {
                        let ctx = registry.get(&current_class)?;
                        find_exception_handler(
                            &ctx.methods[method_idx].exception_table,
                            pc,
                            &ex_class,
                        )
                    };
                    if let Some(handler) = handler_pc {
                        let target = handler as usize;
                        idx = *pc_to_idx
                            .get(&target)
                            .ok_or(VmError::InvalidBranchTarget { pc: target })?;
                        frame.clear_stack();
                        frame.push(Slot::Reference(Some(r)))?;
                        break;
                    }
                    match call_stack.pop() {
                        None => return Err(VmError::JavaException { class_name: ex_class }),
                        Some(caller) => {
                            frame = caller.frame;
                            method_idx = caller.method_idx;
                            current_class = caller.class_name;
                            pc_to_idx = caller.pc_to_idx;
                            // Don't update idx — we search the caller's exception table next
                        }
                    }
                }
                continue;
            }
```

**For checkcast/instanceof** — these use `heap.get(*r)?.class_name` which doesn't reference
`ctx` at all. And `resolve_class_name` reads from the CP, so:

```rust
            Instruction::Checkcast(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => frame.push(slot)?,
                    Slot::Reference(Some(r)) => {
                        let target = {
                            let ctx = registry.get(&current_class)?;
                            resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                        };
                        let actual = heap.get(*r)?.class_name.clone();
                        if actual == target {
                            frame.push(slot)?;
                        } else {
                            return Err(VmError::ClassCastException {
                                from: actual,
                                to: target,
                            });
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

(Same pattern for `instanceof`.)

**Step 14: Update the existing `run_class_int` test helper**

All existing tests use `run_class_int` which calls the old `execute_class` signature. Update it:

```rust
    fn run_class_int(class_name: &str, method_name: &str, descriptor: &str, args: Vec<i32>) -> i32 {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
        let mut heap = duke_gc::Heap::new();
        match execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &entry_class,
            method_name,
            descriptor,
            &slots,
        )
        .expect("execute_class failed")
        {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }
```

Do the same for `run_class_long` and `run_class_double` (if they exist).

**Step 15: Update binary crate**

In `duke/src/main.rs`, update `exec_method` (around line 85-140):

```rust
fn exec_method(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: duke exec <classfile.class> <method> [int-arg...]");
        process::exit(1);
    }
    let path = &args[0];
    let method_name = &args[1];
    let int_args: Vec<Slot> = args[2..]
        .iter()
        .map(|s| {
            Slot::Int(s.parse::<i32>().unwrap_or_else(|_| {
                eprintln!("duke: argument '{s}' is not an integer");
                process::exit(1);
            }))
        })
        .collect();

    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    // Find the target method's descriptor.
    let target = cf
        .methods
        .iter()
        .find(|m| {
            let Some(Some(CpEntry::Utf8(s))) = cf.constant_pool.get(m.name_index.0 as usize) else {
                return false;
            };
            s.as_str() == method_name.as_str()
        })
        .unwrap_or_else(|| {
            eprintln!("duke: method '{method_name}' not found");
            process::exit(1);
        });
    let descriptor = cp_str(&cf, target.descriptor_index)
        .unwrap_or("")
        .to_string();

    let ctx = build_class_context(&cf);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);

    // Create a loader from the class file's parent directory
    let class_dir = std::path::Path::new(path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let loader = duke_loader::DirectoryLoader::new(class_dir);
    let mut heap = Heap::new();

    match execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &entry_class,
        method_name,
        &descriptor,
        &int_args,
    ) {
        Ok(Some(result)) => println!("{result:?}"),
        Ok(None) => println!("(void)"),
        Err(e) => {
            eprintln!("duke: runtime error: {e}");
            process::exit(1);
        }
    }
}
```

Add to the imports at the top of main.rs (if not already present):

```rust
use duke_interpreter::ClassRegistry;
use duke_loader::DirectoryLoader;
```

Also update `duke/Cargo.toml` to add duke-loader as a dependency:

```toml
duke-loader = { path = "../crates/duke-loader" }
```

**Step 16: Uncomment the Phase 10 tests from Step 1**

Remove the `/* ... */` comment wrappers around the Phase 10 tests added in Step 1.

**Step 17: Run Phase 10 tests (RED → GREEN)**

```bash
cargo test -p duke-interpreter -- cross_class_ 2>&1
```

Expected: all 7 cross-class tests pass.

**Step 18: Run full test suite**

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass (139 existing + 7 new = ~146).

If any existing tests fail, they likely have signature issues. Fix them.

**Step 19: fmt + clippy**

```bash
cargo fmt && cargo clippy --all -- -W clippy::pedantic 2>&1 | grep "^error" | head -5
```

Fix any errors.

**Step 20: Demo runs**

```bash
cargo build -p duke 2>/dev/null
./target/debug/duke exec tests/fixtures/CrossCall.class callAdd 3 4
./target/debug/duke exec tests/fixtures/CrossCall.class chainCall 3
./target/debug/duke exec tests/fixtures/PairUser.class makePairSum 3 7
```

Expected:
```
Int(7)
Int(9)
Int(10)
```

**Step 21: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs duke/src/main.rs duke/Cargo.toml
git commit -m "feat(interpreter): implement cross-class method dispatch via ClassRegistry

- ClassRegistry wraps HashMap<String, ClassContext> for multi-class loading
- CallFrame tracks class_name for context restoration on return
- ensure_loaded() lazy-loads classes via ClassLoader (soft failure for stdlib)
- invokestatic dispatches across class boundaries
- invokespecial/invokevirtual dispatch cross-class with no-op fallback
- new uses target class's instance_field_count
- getstatic/putstatic/getfield/putfield resolve field's declaring class
- resolve_fieldref now returns (class_name, field_name, descriptor)
- Updated binary to create DirectoryLoader from class file's parent dir"
```

---

## Final Verification

**Step 1: Full test suite**

```bash
cargo test 2>&1 | grep "test result"
```

Expected: all pass. Count should be ~148 (139 + ~7 cross-class + ~2 new unit tests).

**Step 2: Demo runs**

```bash
./target/debug/duke exec tests/fixtures/CrossCall.class callAdd 3 4
./target/debug/duke exec tests/fixtures/CrossCall.class callDouble 5
./target/debug/duke exec tests/fixtures/CrossCall.class chainCall 3
./target/debug/duke exec tests/fixtures/CrossCall.class addNegated 5
./target/debug/duke exec tests/fixtures/PairUser.class makePairSum 3 7
./target/debug/duke exec tests/fixtures/PairUser.class makePairDiff 10 3
./target/debug/duke exec tests/fixtures/PairUser.class twoPairsSum 1 2 3 4
```

Expected:
```
Int(7)
Int(10)
Int(9)
Int(0)
Int(10)
Int(7)
Int(10)
```

**Step 3: Verify existing fixtures still work**

```bash
./target/debug/duke exec tests/fixtures/Arithmetic.class add 3 4
./target/debug/duke exec tests/fixtures/MathUtils.class square 7
./target/debug/duke exec tests/fixtures/Point.class sumPoints 1 2 3 4
./target/debug/duke exec tests/fixtures/ExceptionTest.class catchSimple
./target/debug/duke exec tests/fixtures/StringAndTypes.class stringNonNull
```

Expected: all return same values as before (7, 49, 10, 42, 1).

**Step 4: Update MEMORY.md**

Update status to Phase 10 Complete. Add:
- ClassRegistry, cross-class dispatch, resolve_fieldref 3-tuple
- Test count, fixture names
- Key commits

---

## Implementation Notes for Agents

### Borrow checker pattern
The critical pattern throughout this refactoring is **scoped borrows**:
```rust
let data = {
    let ctx = registry.get(&current_class)?;  // borrows registry
    extract_data_from(ctx)                      // compute result
};  // borrow released
registry.get_mut(&target)?.mutate(data);        // safe to mutate
```

### Soft failure for unloadable classes
`ensure_loaded()` returns `Ok(false)` when a class can't be found or parsed. This is
intentional — classes like `java/lang/Object` live in the JDK's jimage but we don't always
have access to them. Cross-class dispatch falls back to no-op when the target is unloadable.

### resolve_fieldref breaking change
`resolve_fieldref` changes from `(String, String)` to `(String, String, String)`. Update ALL
callers. Search for `resolve_fieldref(` to find them all.

### String interning scope
The `string_intern` HashMap is per-`execute_class` call (same as before). Cross-class string
interning (same string literal in two different classes) is not yet supported.

### Test helper pattern
`run_class_int` registers one class. `run_cross_class_int` registers multiple classes.
Both use `DirectoryLoader` pointing to `tests/fixtures/` so `ensure_loaded` can find peer
classes automatically.

### Binary loader
The binary creates a `DirectoryLoader` from the `.class` file's parent directory. This means
`duke exec tests/fixtures/CrossCall.class callAdd 3 4` will automatically find `Callee.class`
in the same directory.

### CallFrame.class_name
Each CallFrame now stores the class name. On `do_return!`, `current_class` is restored from
the popped frame's class_name. On cross-class dispatch, `current_class` switches to the
target class.
