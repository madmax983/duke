# Frame Buffer Pool Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate the two per-call `Vec<Slot>` allocations that dominate interpreter performance (23% of time in `Iterator::collect` per flamegraph) by reusing frame buffers in a pool.

**Architecture:** Add `Frame::from_pool_bufs` / `Frame::into_pool_bufs` to `duke-runtime` so Frame can be constructed from and decomposed into raw Vecs. Add `FramePool` to `duke-interpreter` — a simple `Vec<(Vec<Slot>, Vec<Slot>)>` free-list. Modify invoke opcodes to acquire from pool and populate locals directly (eliminating the temporary `callee_args` Vec), and modify the return macro to recycle the callee frame back to pool instead of dropping it.

**Tech Stack:** Rust, existing `duke-runtime` Frame, `duke-interpreter` execute loop

---

## Background

### The two hot allocations (from flamegraph, benchFib baseline 278ms)

```
invokestatic / invokevirtual / invokespecial:

  // Allocation 1: temporary callee_args Vec (23% of CPU time)
  let mut callee_args: Vec<Slot> = (0..arg_count)
      .map(|_| frame.pop())
      .collect::<VmResult<Vec<_>>>()?;   // ← collect() = alloc + fill + reverse
  callee_args.reverse();

  // Allocation 2: Frame locals Vec
  let new_frame = Frame::new(max_stack, max_locals, callee_args)?;
  //   inside Frame::new: vec![Slot::Int(0); max_locals]  ← alloc + zero-fill
  //   + Vec::new() for stack                             ← alloc
```

For `fib(25)` with ~500k recursive calls, both allocations happen 500k times.

### The fix

```
  // Pool hit: reuse previously-freed Vecs
  let (mut locals_buf, stack_buf) = frame_pool.acquire();
  locals_buf.resize(max_locals, Slot::Int(0));
  for i in (0..arg_count).rev() {
      locals_buf[i] = frame.pop()?;    // pop directly into locals, no temp Vec
  }
  let new_frame = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
```

On return, instead of dropping the frame (freeing the Vecs to the allocator), recycle them:
```
  let old = std::mem::replace(&mut frame, caller.frame);
  let (l, s) = old.into_pool_bufs();  // clears stack, returns Vecs
  frame_pool.release(l, s);           // back to free-list
```

After warmup (first N calls where N = max recursion depth), all subsequent frames are pool hits.

### Scope

Three invoke opcodes need updating: `invokestatic`, `invokevirtual`/`invokespecial` (share path), `invokeinterface`. All follow the same `callee_args.collect()` → `Frame::new()` pattern — search for the pattern, don't rely on line numbers.

---

## Task 1: Add Pool Support Methods to `Frame`

**Files:**
- Modify: `crates/duke-runtime/src/frame.rs`

### Step 1: Write failing tests

Add to the test module at the bottom of `crates/duke-runtime/src/frame.rs` (add `#[cfg(test)] mod tests { ... }` if absent):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_pool_bufs_initialises_correctly() {
        let locals = vec![Slot::Int(0); 3];
        let stack = Vec::new();
        let mut f = Frame::from_pool_bufs(locals, stack, 4);
        // locals[0] should be Int(0), locals[1] should be Int(0), etc.
        assert_eq!(f.load_local(0).unwrap(), Slot::Int(0));
        assert_eq!(f.load_local(1).unwrap(), Slot::Int(0));
        // can store and retrieve
        f.store_local(2, Slot::Int(99)).unwrap();
        assert_eq!(f.load_local(2).unwrap(), Slot::Int(99));
        // stack push/pop works
        f.push(Slot::Int(7)).unwrap();
        assert_eq!(f.pop_int().unwrap(), 7);
    }

    #[test]
    fn into_pool_bufs_clears_stack_preserves_capacity() {
        let mut f = Frame::new(4, 2, vec![Slot::Int(1), Slot::Int(2)]).unwrap();
        f.push(Slot::Int(10)).unwrap();
        f.push(Slot::Int(20)).unwrap();
        let (locals, stack) = f.into_pool_bufs();
        // locals holds the values that were in the frame
        assert_eq!(locals[0], Slot::Int(1));
        assert_eq!(locals[1], Slot::Int(2));
        // stack is cleared
        assert!(stack.is_empty());
        // but capacity is retained (>= 2 since we pushed 2 items)
        assert!(stack.capacity() >= 2);
    }

    #[test]
    fn pool_round_trip_reuses_allocation() {
        // Simulate: create frame, use it, extract bufs, reinit, verify
        let mut f = Frame::new(8, 3, vec![Slot::Int(42)]).unwrap();
        f.push(Slot::Int(1)).unwrap();
        let (mut locals_buf, stack_buf) = f.into_pool_bufs();
        // Reinitialise as a new frame (simulating pool reuse)
        locals_buf.resize(2, Slot::Int(0));
        locals_buf[0] = Slot::Int(99);
        let mut f2 = Frame::from_pool_bufs(locals_buf, stack_buf, 4);
        assert_eq!(f2.load_local(0).unwrap(), Slot::Int(99));
        assert_eq!(f2.load_local(1).unwrap(), Slot::Int(0));
        assert_eq!(f2.pop().unwrap_err(), VmError::StackUnderflow); // stack was cleared
    }
}
```

### Step 2: Run to verify failure

```bash
cd /c/Users/markm/duke
cargo test -p duke-runtime 2>&1 | tail -20
```

Expected: `from_pool_bufs_initialises_correctly` and others FAIL with "no method named `from_pool_bufs`".

### Step 3: Implement the two new methods in `frame.rs`

Add after the existing `Frame::new` method (around line 44):

```rust
/// Construct a frame from pre-allocated buffers obtained from a [`FramePool`].
///
/// The caller is responsible for:
/// - Sizing `locals` to `max_locals` elements and filling with default values.
/// - Ensuring `stack` is empty (the pool's `release` method guarantees this).
///
/// This is the zero-allocation fast path for method calls after pool warmup.
pub fn from_pool_bufs(locals: Vec<Slot>, stack: Vec<Slot>, max_stack: usize) -> Self {
    Self {
        locals,
        stack,
        max_stack,
    }
}

/// Decompose this frame into its backing Vecs for return to a [`FramePool`].
///
/// Clears the operand stack (retaining capacity). Locals are *not* cleared —
/// they will be resized and reinitialised by [`Frame::from_pool_bufs`] on reuse.
pub fn into_pool_bufs(mut self) -> (Vec<Slot>, Vec<Slot>) {
    self.stack.clear();
    (self.locals, self.stack)
}
```

### Step 4: Run tests

```bash
cargo test -p duke-runtime 2>&1 | tail -20
```

Expected: all duke-runtime tests pass including the 3 new ones.

### Step 5: Commit

```bash
git add crates/duke-runtime/src/frame.rs
git commit -m "perf(runtime): add Frame::from_pool_bufs and into_pool_bufs for frame pool"
```

---

## Task 2: Add `FramePool` and Wire Into `execute_class`

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Write a failing test

Add this test near the existing benchFib-style tests in `lib.rs`:

```rust
#[test]
fn frame_pool_does_not_change_fib_result() {
    // This test will pass only after FramePool is integrated correctly.
    // It's a regression guard: pool reuse must not corrupt frame state.
    let result = run_bootstrap_int("BenchmarkSuite", "benchFib", "()I");
    // fib(25) = 75025 — if frame data leaks between pool reuses, this fails
    assert_eq!(result, 75025);
}
```

**Note:** `run_bootstrap_int` loads from `tests/fixtures/`. You need `BenchmarkSuite.class` in `tests/fixtures/`. Copy it:

```bash
cp /c/Users/markm/duke/benchmarks/BenchmarkSuite.class /c/Users/markm/duke/tests/fixtures/
cp /c/Users/markm/duke/benchmarks/BenchmarkSuite.java /c/Users/markm/duke/tests/fixtures/
```

Run to verify it currently passes (before the pool change — it should already work via `Frame::new`):

```bash
cargo test -p duke-interpreter frame_pool_does_not_change_fib_result 2>&1
```

This test will serve as the regression guard throughout Tasks 2–3.

### Step 2: Add `FramePool` struct

In `crates/duke-interpreter/src/lib.rs`, find the top of the file (after the `use` imports). Add the `FramePool` struct before `pub fn execute_class`:

```rust
/// Reusable pool of frame backing buffers.
///
/// Eliminates per-call `Vec<Slot>` allocation for locals and operand stack.
/// On a recursive workload the pool reaches steady state after the first
/// call-depth number of calls; all subsequent frames are pool hits.
///
/// The pool is private to a single `execute_class` invocation — not shared
/// across threads or concurrent calls.
struct FramePool {
    free: Vec<(Vec<Slot>, Vec<Slot>)>,
}

impl FramePool {
    fn new() -> Self {
        Self { free: Vec::new() }
    }

    /// Acquire a `(locals_buf, stack_buf)` pair from the pool.
    /// Returns a pooled pair if one is available, otherwise allocates fresh Vecs.
    fn acquire(&mut self) -> (Vec<Slot>, Vec<Slot>) {
        self.free.pop().unwrap_or_default()
    }

    /// Return buffers to the pool.
    ///
    /// `stack` must already be empty (guaranteed by [`Frame::into_pool_bufs`]).
    /// Caps pool size at 256 to bound memory usage.
    fn release(&mut self, locals: Vec<Slot>, stack: Vec<Slot>) {
        if self.free.len() < 256 {
            self.free.push((locals, stack));
        }
    }
}
```

### Step 3: Initialise `FramePool` in `execute_class` and wire into `do_return!`

In `execute_class`, find the line `let mut call_stack: Vec<CallFrame> = Vec::new();` and add the pool initialisation immediately after:

```rust
let mut frame_pool = FramePool::new();
```

Find the `do_return!` macro definition (search for `macro_rules! do_return`). It has a `Some(caller)` arm that currently does `frame = caller.frame;`. Change that arm to recycle the callee frame before replacing it:

```rust
macro_rules! do_return {
    ($val:expr) => {{
        match call_stack.pop() {
            None => return Ok($val),
            Some(caller) => {
                let ret_val = $val;
                // Recycle callee frame buffers into pool before overwriting `frame`.
                let old = std::mem::replace(&mut frame, caller.frame);
                let (l, s) = old.into_pool_bufs();
                frame_pool.release(l, s);
                method_idx = caller.method_idx;
                pc_to_idx = caller.pc_to_idx;
                idx = caller.resume_idx;
                current_class = caller.class_name;
                if let Some(v) = ret_val {
                    frame.push(v)?;
                }
                continue;
            }
        }
    }};
}
```

**Important:** copy the full existing macro body exactly — only add the two `let old` / `let (l, s)` / `frame_pool.release(l, s)` lines and change `frame = caller.frame` to the `std::mem::replace` form. Do not change any other line.

### Step 4: Run tests

```bash
cargo test -p duke-interpreter 2>&1 | tail -30
```

Expected: all tests pass. The pool is now wired for recycling on return, but invoke opcodes still use `Frame::new` (pool is half-used — releases happen but no acquires yet, so it just discards). Correctness must still hold.

### Step 5: Commit

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/BenchmarkSuite.java tests/fixtures/BenchmarkSuite.class
git commit -m "perf(interpreter): add FramePool and recycle callee frames on return"
```

---

## Task 3: Eliminate `callee_args` in Invoke Opcodes

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

This is the high-impact change. Find every place that does the `callee_args.collect()` + `Frame::new()` pattern and replace with pool acquire + direct pop.

### Step 1: Find all call sites

```bash
grep -n "callee_args" /c/Users/markm/duke/crates/duke-interpreter/src/lib.rs | head -40
```

There will be several occurrences across `invokestatic`, `invokevirtual`/`invokespecial`, and `invokeinterface` opcodes.

### Step 2: Understand the two arg patterns

**Pattern A — static call (no `this`):**
```rust
// BEFORE (invokestatic)
let arg_count = parse_arg_count(&callee_desc);
let mut callee_args: Vec<Slot> = (0..arg_count)
    .map(|_| frame.pop())
    .collect::<VmResult<Vec<_>>>()?;
callee_args.reverse();
let new_frame = Frame::new(max_stack, max_locals, callee_args)?;
```

Replace with:
```rust
// AFTER (invokestatic)
let arg_count = parse_arg_count(&callee_desc);
let (mut locals_buf, stack_buf) = frame_pool.acquire();
locals_buf.resize(max_locals, Slot::Int(0));
for i in (0..arg_count).rev() {
    locals_buf[i] = frame.pop()?;
}
let new_frame = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
```

**Pattern B — virtual/special call (has `this` at locals[0]):**
```rust
// BEFORE (invokevirtual / invokespecial)
let arg_count = parse_arg_count(&callee_desc);
let mut callee_args: Vec<Slot> = (0..arg_count)
    .map(|_| frame.pop())
    .collect::<VmResult<Vec<_>>>()?;
callee_args.reverse();
let this_slot = frame.pop()?;
callee_args.insert(0, this_slot);
let new_frame = Frame::new(max_stack, max_locals, callee_args)?;
```

Replace with:
```rust
// AFTER (invokevirtual / invokespecial)
let arg_count = parse_arg_count(&callee_desc);
let (mut locals_buf, stack_buf) = frame_pool.acquire();
locals_buf.resize(max_locals, Slot::Int(0));
// Pop method args in reverse (stack top = last arg), into locals[1..=arg_count]
for i in (1..=arg_count).rev() {
    locals_buf[i] = frame.pop()?;
}
// Pop `this`, into locals[0]
locals_buf[0] = frame.pop()?;
let new_frame = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
```

**Apply to all occurrences** — `invokestatic`, `invokevirtual`, `invokespecial`, `invokeinterface`. Each will be one of the two patterns above. Some call sites are guarded by `if native { ... } else { ... }` — only change the non-native bytecode path (native handlers don't create Frames).

### Step 3: Check for remaining `callee_args` references

```bash
grep -n "callee_args" /c/Users/markm/duke/crates/duke-interpreter/src/lib.rs
```

Expected: no output. If any remain, fix them.

### Step 4: Run full test suite

```bash
cargo test -p duke-interpreter 2>&1 | tail -30
```

Expected: all 327 tests pass. Pay special attention to tests that exercise:
- `fib` recursion (frame state isolation)
- `invokevirtual` with `this` (arg ordering)
- `invokestatic` with multiple args (arg ordering)

If any test fails, the arg ordering in the new code is wrong. Double-check: the JVM operand stack has args pushed left-to-right, so the *rightmost* arg is at the top of the stack. When popping into `locals[i]` in reverse order (`for i in (0..n).rev()`), we pop the top of stack first into the highest local index — this is correct.

### Step 5: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "perf(interpreter): eliminate callee_args Vec in invoke opcodes using FramePool"
```

---

## Task 4: Benchmark and Record Results

**Files:**
- Modify: `benchmarks/RESULTS.md`
- Modify: `benchmarks/criterion-output.txt` (overwrite with new run)

### Step 1: Run Criterion benchmarks

```bash
cd /c/Users/markm/duke
cargo bench -p duke-interpreter 2>&1 | tee benchmarks/criterion-output.txt
```

Wait for completion. Record the median times.

### Step 2: Compare to baseline

Baseline (pre-pool):
| Benchmark | Before |
|-----------|--------|
| benchSum  | 141 ms |
| benchFib  | 278 ms |
| benchArrayList | 7.5 ms |
| benchHashMap | 1.5 ms |
| bootstrap_stdlib only | 41 µs |

Expected improvement for `benchFib` (most allocation-heavy due to deep recursion): **30–60% reduction** (from ~278ms to ~110–190ms). `benchSum` should also improve since its loop body doesn't call methods, but the initial frame creation matters less there.

### Step 3: Update `benchmarks/RESULTS.md`

Add a new section after the existing baseline:

```markdown
## Post-Frame-Pool Results (2026-03-08)

Duke version: Phase 23 + frame buffer pool
Change: Eliminated per-call `Vec<Slot>` allocation via FramePool;
        callee_args temporary Vec eliminated in all invoke opcodes.

| Benchmark | Before (ms) | After (ms) | Speedup |
|-----------|-------------|------------|---------|
| benchSum  | 141         | X          | Xx      |
| benchFib  | 278         | X          | Xx      |
| benchArrayList | 7.5    | X          | Xx      |
| benchHashMap | 1.5      | X          | Xx      |

### Analysis

[Write 3-5 bullet points based on actual numbers. If benchFib improved
by ~2x, note that this confirms frame allocation was the bottleneck.
If benchSum barely changed, that confirms the hot path was invoke,
not arithmetic. Note which benchmark improved most and why.]
```

### Step 4: Commit

```bash
git add benchmarks/RESULTS.md benchmarks/criterion-output.txt
git commit -m "bench: record post-frame-pool baseline results"
```

---

## Summary

| Task | Change | Expected impact |
|------|--------|----------------|
| 1 | `Frame::from_pool_bufs` + `into_pool_bufs` | Enables pool; no perf change yet |
| 2 | `FramePool` struct + recycle on return | Pool releases work; no perf change yet (still allocating on invoke) |
| 3 | Eliminate `callee_args` + use pool on invoke | **Main win**: ~23% hot path eliminated |
| 4 | Benchmark | Quantify actual improvement |

**Correctness note:** The arg ordering change (direct pop into locals vs collect+reverse) is equivalent:
- Old: collect in pop-order (reverse), then reverse → forward order in locals
- New: `for i in (0..n).rev()` pops top-of-stack (= last arg) into `locals[n-1]`, then next into `locals[n-2]`, etc → forward order in locals ✓
