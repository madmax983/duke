# Method Dispatch Cache Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate two per-call allocations in the interpreter's invoke hot path: the `pc_to_idx` HashMap rebuilt from scratch on every method call, and the repeated CP 3-level walk + linear method search.

**Architecture:** (1) Store `pc_to_idx` as `Arc<HashMap<usize,usize>>` in `MethodInfo` — precomputed once at class registration, cloned O(1) on every invoke. (2) Add a per-`execute_class` dispatch cache keyed by `(current_class_name, cp_idx)` that maps to `(target_class_name, method_idx)`, turning repeat CP resolution + method search into a single HashMap lookup.

**Tech Stack:** Rust, `std::sync::Arc`, existing `ClassContext`/`MethodInfo` in `duke-interpreter`

---

## Background

### What's slow (from flamegraph + code reading)

Every `invokestatic` (and other invoke opcodes) currently does:

```rust
// 1. CP 3-level walk — 5 array lookups + 3 String::clone()
let (callee_class, callee_name, callee_desc) = {
    let ctx = registry.get(&current_class)?;
    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
};

// 2. Linear method search
let callee_idx = {
    let ctx = registry.get(&callee_class)?;
    ctx.methods.iter().position(|m| m.name == callee_name && m.descriptor == callee_desc)
};

// 3. Fresh HashMap construction from instructions list — EVERY CALL
let pci: HashMap<usize, usize> = ctx.methods[callee_idx]
    .instructions
    .iter()
    .enumerate()
    .map(|(i, &(pc, _))| (pc, i))
    .collect();                        // ← HashMap alloc + fill × 500k for benchFib
```

For `fib(25)` with ~500k recursive calls, all three fire on every call.

### The two fixes

**Fix A — Precompute `pc_to_idx` (Task 1)**

`pc_to_idx` maps bytecode PC offsets to instruction indices. It never changes — it's derived from the static instruction list of the method. Store it in `MethodInfo` as `Arc<HashMap<usize, usize>>`. Cloning an `Arc` is an atomic increment (O(1) with no allocation), vs building a fresh `HashMap` from ~10-20 instructions every call.

```rust
// Before: built at every invoke site (~500k times for fib)
let pci: HashMap<usize, usize> = ctx.methods[callee_idx].instructions.iter()...collect();

// After: precomputed once, Arc::clone on each invoke
let pci = Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
```

`CallFrame::pc_to_idx` also changes from `HashMap<usize, usize>` to `Arc<HashMap<usize, usize>>`.
The execute_class local `pc_to_idx` similarly becomes `Arc<HashMap<usize, usize>>`.

**Fix B — Dispatch cache (Task 2)**

Cache resolved `(target_class, method_idx)` by `(caller_class_name, cp_idx)`:

```rust
// In execute_class:
let mut dispatch_cache: HashMap<(String, u16), (String, usize)> = HashMap::new();

// At invoke site:
let cache_key = (current_class.clone(), cp_idx.0);
let (callee_class, callee_idx) = if let Some(hit) = dispatch_cache.get(&cache_key) {
    hit.clone()
} else {
    let ctx = registry.get(&current_class)?;
    let (class, name, desc) = resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?;
    let ctx2 = registry.get(&class)?;
    let idx = ctx2.methods.iter().position(|m| m.name == name && m.descriptor == desc)
        .ok_or_else(|| VmError::MethodNotFound { ... })?;
    dispatch_cache.insert(cache_key, (class.clone(), idx));
    (class, idx)
};
// Then use callee_class + callee_idx directly (skip resolve_methodref + position scan)
```

Applies to: **`invokestatic`** (fully static — same target every time) and **`invokespecial`** (also statically resolved).

Does NOT apply to `invokevirtual`/`invokeinterface` — target class varies by runtime object type.

---

## Task 1: Precompute `pc_to_idx` in `MethodInfo`

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` — `MethodInfo` struct, `build_class_context`, all invoke handlers, execute_class init, CallFrame

### Step 1: Find `MethodInfo` and `build_class_context`

```bash
grep -n "struct MethodInfo" /c/Users/markm/duke/crates/duke-interpreter/src/lib.rs
grep -n "pub fn build_class_context" /c/Users/markm/duke/crates/duke-interpreter/src/lib.rs
```

Note the line numbers. Also find where `pc_to_idx` is currently built:
```bash
grep -n "pc_to_idx" /c/Users/markm/duke/crates/duke-interpreter/src/lib.rs | head -30
```

### Step 2: Write the failing test

Add this test near other `run_bootstrap_int` tests in `lib.rs`:

```rust
#[test]
fn dispatch_cache_fib_correctness() {
    // After pc_to_idx precomputation, fib must still return the right answer.
    // Any regression in pc_to_idx sharing would cause wrong branch targets.
    let result = run_bootstrap_int("BenchmarkSuite", "benchFib", "()I");
    assert_eq!(result, 75025);
}
```

Run to verify it passes already (before any change):
```bash
cargo test -p duke-interpreter dispatch_cache_fib_correctness 2>&1 | tail -5
```

### Step 3: Add `pc_to_idx` field to `MethodInfo`

Find the `MethodInfo` struct. Add the field:

```rust
pub struct MethodInfo {
    // ... existing fields ...
    pub pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
}
```

### Step 4: Populate `pc_to_idx` in `build_class_context`

Find where `MethodInfo` is constructed inside `build_class_context`. After the `instructions` field is populated, add:

```rust
let pc_to_idx: std::collections::HashMap<usize, usize> = instructions
    .iter()
    .enumerate()
    .map(|(i, &(pc, _))| (pc, i))
    .collect();
// ... then in the MethodInfo literal:
MethodInfo {
    // ... existing fields ...,
    instructions,
    pc_to_idx: std::sync::Arc::new(pc_to_idx),
}
```

### Step 5: Update `CallFrame::pc_to_idx` type

Find `struct CallFrame`. Change:
```rust
pc_to_idx: HashMap<usize, usize>,
```
To:
```rust
pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
```

### Step 6: Update execute_class local `pc_to_idx`

The local variable `pc_to_idx` in `execute_class` tracks the current method's PC map. Change its type and initial assignment. Find where it's initialised for the first frame (search for the initial `pc_to_idx =` assignment near `execute_class`'s frame setup).

Change:
```rust
let mut pc_to_idx: HashMap<usize, usize> = ctx.methods[method_idx]
    .instructions
    .iter()
    .enumerate()
    .map(|(i, &(pc, _))| (pc, i))
    .collect();
```
To:
```rust
let mut pc_to_idx = std::sync::Arc::clone(&ctx.methods[method_idx].pc_to_idx);
```

### Step 7: Replace all inline `pc_to_idx` builds in invoke handlers

Search for every place that builds `pc_to_idx` inline:
```bash
grep -n "\.map(|(i, &(pc" /c/Users/markm/duke/crates/duke-interpreter/src/lib.rs
```

For each occurrence in an invoke handler (invokestatic, invokevirtual, invokespecial, invokeinterface), replace the inline build with:
```rust
let callee_pc_to_idx = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
```

Then where the callee's pc_to_idx is used in `call_stack.push(CallFrame { ... })`, it passes the Arc clone. Where the local variable `pc_to_idx` is updated after the push (i.e., `pc_to_idx = callee_pc_to_idx`), it assigns the Arc.

Also check the `do_return!` macro — when restoring the caller's `pc_to_idx` from `caller.pc_to_idx`, it now gets an `Arc` (no change needed structurally, but verify the type flows correctly).

### Step 8: Fix any pc_to_idx usage as map

Search for `pc_to_idx.get(` — these are the places that look up a PC offset to find the instruction index. These now work through the `Arc` deref transparently (`Arc<HashMap>` derefs to `HashMap`). No code changes needed at these call sites.

### Step 9: Run all tests

```bash
cargo test -p duke-interpreter 2>&1 | tail -20
```

Expected: all 272 tests pass.

### Step 10: Run clippy

```bash
cargo clippy -p duke-interpreter -- -W clippy::pedantic 2>&1 | grep "^error" | head -10
```

Fix errors. Common issue: `Arc::clone(&x)` preferred over `x.clone()` for Arcs per clippy.

### Step 11: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "perf(interpreter): precompute pc_to_idx as Arc<HashMap> in MethodInfo"
```

---

## Task 2: Dispatch Cache for CP Resolution

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` — `execute_class` function

### Step 1: Write a failing test (performance regression guard)

Add test (it won't fail functionally — but it ensures cache doesn't break dispatch):

```rust
#[test]
fn dispatch_cache_invokestatic_multiple_methods() {
    // Exercises the dispatch cache with two different static methods in the same class.
    // If the cache incorrectly returns the first method for the second lookup,
    // results will be wrong.
    let sum = run_bootstrap_int("BenchmarkSuite", "benchSum", "()I");
    let fib = run_bootstrap_int("BenchmarkSuite", "benchFib", "()I");
    assert_eq!(fib, 75025);
    // benchSum overflows i32: sum(0..499999) = 124999750000 → 445698416
    assert_eq!(sum, 445698416_i32 as i32);
}
```

Run to verify it passes now:
```bash
cargo test -p duke-interpreter dispatch_cache_invokestatic_multiple_methods 2>&1 | tail -5
```

### Step 2: Add dispatch cache to `execute_class`

In `execute_class`, find the line:
```rust
let mut frame_pool = FramePool::new();
```

Add immediately after:
```rust
// Dispatch cache: (class_name, cp_idx) -> (target_class_name, method_idx).
// Eliminates repeated CP 3-level walk + linear method search for repeat call sites.
// Only used for statically-resolved calls (invokestatic, invokespecial).
let mut dispatch_cache: std::collections::HashMap<(String, u16), (String, usize)> =
    std::collections::HashMap::new();
```

### Step 3: Wrap `invokestatic` resolution in cache check

Find `Instruction::Invokestatic(cp_idx) =>` handler. Currently it does:

```rust
let (callee_class, callee_name, callee_desc) = {
    let ctx = registry.get(&current_class)?;
    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
};
let callee_idx = {
    let ctx = registry.get(&callee_class)?;
    ctx.methods.iter().position(|m| m.name == callee_name && m.descriptor == callee_desc)
        ...
};
```

Replace with:

```rust
let cache_key = (current_class.clone(), cp_idx.0);
let (callee_class, callee_idx) = if let Some((cls, idx)) = dispatch_cache.get(&cache_key) {
    (cls.clone(), *idx)
} else {
    // Slow path: full resolution
    let (cls, name, desc) = {
        let ctx = registry.get(&current_class)?;
        resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
    };
    let idx = {
        let ctx = registry.get(&cls)?;
        ctx.methods
            .iter()
            .position(|m| m.name == name && m.descriptor == desc)
            .ok_or_else(|| VmError::MethodNotFound {
                class: cls.clone(),
                name: name.clone(),
                descriptor: desc.clone(),
            })?
    };
    dispatch_cache.insert(cache_key, (cls.clone(), idx));
    (cls, idx)
};
// Remove callee_name and callee_desc — no longer needed after this point
// (max_locals, max_stack, instructions all come from callee_idx)
```

**Note:** After this change, `callee_name` and `callee_desc` are no longer available below. Check whether they're used further down in the invokestatic handler:
- `parse_arg_count(&callee_desc)` — `arg_count` is still needed. Solution: compute `arg_count` from the cached method on the fast path. After getting `callee_idx`, do: `let arg_count = { let ctx = registry.get(&callee_class)?; parse_arg_count(&ctx.methods[callee_idx].descriptor) };`. OR: cache `arg_count` alongside `method_idx`. Simplest: include `arg_count` in cached value:

```rust
// Change cache value to include arg_count:
let mut dispatch_cache: HashMap<(String, u16), (String, usize, usize)> = HashMap::new();
//                                                         ^^^^^^ arg_count
```

So the cached value is `(callee_class: String, callee_idx: usize, arg_count: usize)`.

On miss path, compute `arg_count` from `callee_desc` (as before) and store it in the cache.
On hit path, extract `arg_count` directly from the cached tuple.

### Step 4: Apply same cache to `invokespecial`

Find `Instruction::Invokespecial(cp_idx) =>` handler. Apply the identical cache pattern (same `dispatch_cache`, same key structure). `invokespecial` is statically resolved (always calls the same class+method regardless of object type).

**Skip** `invokevirtual` and `invokeinterface` — these require runtime polymorphic dispatch and cannot be cached this way.

### Step 5: Verify no `callee_name`/`callee_desc` usage after the cache block

After applying the cache, search for remaining uses of `callee_name` and `callee_desc` in the invokestatic/invokespecial handlers. There should be none (they were only used for: `resolve_methodref` result, `position()` search, and `parse_arg_count` — all now replaced).

```bash
# Check for stray uses — only lambda/native dispatch paths should use them if any
grep -n "callee_name\|callee_desc" /c/Users/markm/duke/crates/duke-interpreter/src/lib.rs | head -20
```

### Step 6: Run all tests

```bash
cargo test -p duke-interpreter 2>&1 | tail -20
```

Expected: all tests pass.

### Step 7: Run clippy

```bash
cargo clippy -p duke-interpreter -- -W clippy::pedantic 2>&1 | grep "^error" | head -10
```

Fix errors.

### Step 8: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "perf(interpreter): add dispatch cache for invokestatic/invokespecial CP resolution"
```

---

## Task 3: Benchmark + Record Results

**Files:**
- Modify: `benchmarks/RESULTS.md`
- Modify: `benchmarks/criterion-output.txt`

### Step 1: Run Criterion benchmarks

```bash
cd /c/Users/markm/duke
cargo bench -p duke-interpreter 2>&1 | tee benchmarks/criterion-output.txt
```

Wait for completion. Record median times for all 5 benchmarks.

### Step 2: Update `benchmarks/RESULTS.md`

Add a new section after the frame-pool results:

```markdown
## Post-Dispatch-Cache Results (2026-03-08)

Duke version: Phase 23 + frame buffer pool + method dispatch cache
Changes:
- `pc_to_idx` precomputed as `Arc<HashMap>` in `MethodInfo` (eliminates per-call HashMap build)
- Dispatch cache in `execute_class` for `invokestatic`/`invokespecial` CP resolution

| Benchmark | Frame-pool (ms) | After cache (ms) | Speedup |
|-----------|-----------------|------------------|---------|
| benchSum  | 147.34 | X.XX | Xx |
| benchFib  | 213.98 | X.XX | Xx |
| benchArrayList | 8.68 | X.XX | Xx |
| benchHashMap | 2.01 | X.XX | Xx |
| bootstrap_stdlib only | 0.065 | X.XX | — |

[Analysis: which benchmark improved and why, what's the remaining bottleneck]
```

Fill in all real numbers. If benchFib drops below HotSpot -Xint (202ms baseline), note it.

### Step 3: Commit

```bash
git add benchmarks/RESULTS.md benchmarks/criterion-output.txt
git commit -m "bench: record post-dispatch-cache baseline results"
```

---

## Summary

| Task | Change | Target |
|------|--------|--------|
| 1 | `Arc<HashMap>` pc_to_idx in MethodInfo | Eliminate 500k HashMap builds for fib(25) |
| 2 | Dispatch cache in execute_class | Eliminate CP 3-level walk + method linear scan |
| 3 | Benchmark | Quantify improvement, update RESULTS.md |

**Expected outcome:** `benchFib` drops from 214ms toward or below HotSpot -Xint (202ms). If so, Duke's pure interpreter is competitive with HotSpot's pure interpreter for recursive arithmetic — a meaningful milestone.
