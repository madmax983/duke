# Phase 28 Threading Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add basic `java.lang.Thread` / `Runnable` support to Duke so fixture programs can spawn host threads, execute `run()` concurrently enough for `sleep()` / `join()`, and keep the VM alive until all non-daemon work finishes.

**Architecture:** Keep the existing interpreter core, but extract its mutable execution locals into a resumable `ExecutionState` so a Java thread can run until it either returns or requests a blocking thread action. Introduce a small `threading` runtime in `crates/duke-interpreter` that owns host thread records, shared output, and top-level wait-for-workers behavior. Thread natives do not perform blocking directly inside the native function; they set a pending thread action (`start`, `sleep`, `join`) that the interpreter loop handles outside the VM lock. To avoid lying about concurrent GC in the same phase, suspend collection while worker threads are live and document that limitation in an ADR.

**Tech Stack:** Rust 2024, `std::thread`, `std::sync::{Arc, Mutex, Condvar, atomic}`, `duke-interpreter`, `duke-loader`, checked-in Java fixtures under `tests/fixtures`, `javac --release 21`, `cargo test`.

---

## Background Notes

- `crates/duke-interpreter/src/lib.rs` currently assumes one mutable `ClassRegistry`, one mutable `Heap`, one mutable output sink, and one active call stack inside `execute_class`.
- `monitorenter` / `monitorexit` were intentionally implemented as no-ops in Phase 18 because Duke was single-threaded. Keep that behavior for Phase 28 and state clearly that real monitor semantics belong to a later locking phase.
- GC root gathering only scans the current frame, saved call frames, and static fields. That is insufficient once paused worker threads exist. Do not try to solve concurrent GC in this phase. Gate collection off while worker threads are live.
- File I/O already stores host-side resources in the runtime boundary. Threading should follow the same general principle: Java heap stores ids/state, Rust runtime stores host thread coordination.

### Task 1: Red Tests For Thread Lifecycle And Top-Level Completion

**Files:**
- Create: `tests/fixtures/ThreadingTest.java`
- Create: `tests/fixtures/ThreadingTest.class`
- Create: `tests/fixtures/ThreadingTest$Worker.class`
- Create: `tests/fixtures/ThreadingTest$DerivedThread.class`
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Write the fixture**

Create `tests/fixtures/ThreadingTest.java`:

```java
public final class ThreadingTest {
    static final class Worker implements Runnable {
        private final int id;

        Worker(int id) {
            this.id = id;
        }

        @Override
        public void run() {
            System.out.println(id);
            try {
                Thread.sleep(10L);
            } catch (Exception ignored) {
            }
            System.out.println(id + 100);
        }
    }

    static final class DerivedThread extends Thread {
        private final int value;

        DerivedThread(int value) {
            this.value = value;
        }

        @Override
        public void run() {
            System.out.println(value + 200);
        }
    }

    public static int spawnAndJoinTen() throws Exception {
        Thread[] threads = new Thread[10];
        for (int i = 0; i < threads.length; i++) {
            threads[i] = new Thread(new Worker(i));
            threads[i].start();
        }
        for (Thread thread : threads) {
            thread.join();
        }
        return threads.length;
    }

    public static int subclassRunWins() throws Exception {
        Thread thread = new DerivedThread(5);
        thread.start();
        thread.join();
        return 1;
    }

    public static int fireAndForgetStillFinishes() {
        for (int i = 0; i < 3; i++) {
            new Thread(new Worker(i)).start();
        }
        return 3;
    }
}
```

**Step 2: Compile the fixture**

Run: `javac --release 21 tests/fixtures/ThreadingTest.java -d tests/fixtures/`

Expected: `ThreadingTest.class`, `ThreadingTest$Worker.class`, and `ThreadingTest$DerivedThread.class` are created with no errors.

**Step 3: Add failing interpreter tests**

In `crates/duke-interpreter/src/lib.rs`, add a Phase 28 block with helpers that load `ThreadingTest.class`, capture stdout lines, and run the entry method through a new top-level helper name that does not exist yet, for example:

```rust
fn run_bootstrap_with_output(
    class_name: &str,
    method_name: &str,
    descriptor: &str,
) -> (Option<Slot>, Vec<String>)
```

Add tests:

```rust
#[test]
fn threading_spawn_and_join_ten_workers() { /* expect result = 10, output lines = 20 */ }

#[test]
fn threading_subclass_run_method_wins_over_base_thread() { /* expect output contains 205 */ }

#[test]
fn threading_fire_and_forget_still_waits_for_workers_before_returning() { /* expect six worker lines before helper returns */ }
```

Sort captured output lines before asserting so the test does not assume a scheduling order.

**Step 4: Run the tests to verify RED**

Run: `cargo test -p duke-interpreter threading_ -- --nocapture`

Expected: FAIL because `java/lang/Thread`, `java/lang/Runnable`, and the threaded entry helper do not exist yet.

**Step 5: Commit the red tests**

```bash
git add tests/fixtures/ThreadingTest.java tests/fixtures/ThreadingTest.class tests/fixtures/ThreadingTest$Worker.class tests/fixtures/ThreadingTest$DerivedThread.class crates/duke-interpreter/src/lib.rs
git commit -m "test: add red threading lifecycle fixtures"
```

### Task 2: Add Native Thread Control And Runtime Scaffolding

**Files:**
- Create: `crates/duke-interpreter/src/threading.rs`
- Modify: `crates/duke-interpreter/src/lib.rs`
- Modify: `crates/duke-interpreter/src/registry.rs`

**Step 1: Add a native side-channel for thread actions**

In `crates/duke-interpreter/src/registry.rs`, introduce a small control object shared by all native handlers:

```rust
#[derive(Debug)]
pub enum NativeThreadAction {
    Start { thread_ref: u64 },
    Sleep(std::time::Duration),
    Join { thread_id: i32 },
}

#[derive(Debug, Default)]
pub struct NativeControl {
    pending_thread_action: Option<NativeThreadAction>,
}

impl NativeControl {
    pub fn request(&mut self, action: NativeThreadAction) {
        self.pending_thread_action = Some(action);
    }

    pub fn take(&mut self) -> Option<NativeThreadAction> {
        self.pending_thread_action.take()
    }
}
```

Update `NativeHandler`, `CallbackNativeHandler`, and the registry call sites so every native function receives `&mut NativeControl`.

**Step 2: Add the shared threading runtime**

Create `crates/duke-interpreter/src/threading.rs` with:

```rust
pub(crate) enum ThreadPause {
    Sleep(std::time::Duration),
    Join { thread_id: i32 },
}

pub(crate) struct SharedOutput { /* Arc<Mutex<Vec<u8> or dyn Write + Send>> */ }

pub(crate) struct ThreadRecord {
    pub java_ref: u64,
    pub finished: bool,
    pub daemon: bool,
    pub join_cv: std::sync::Condvar,
}

pub(crate) struct ThreadRuntime {
    pub next_thread_id: std::sync::atomic::AtomicI32,
    pub live_workers: std::sync::atomic::AtomicUsize,
    pub records: std::sync::Mutex<std::collections::HashMap<i32, std::sync::Arc<std::sync::Mutex<ThreadRecord>>>>,
}
```

Keep this runtime interpreter-owned. Do not store `JoinHandle` or `Condvar` state in the GC heap.

**Step 3: Wire the module into the interpreter crate**

In `crates/duke-interpreter/src/lib.rs`:
- add `mod threading;`
- import `NativeControl` / `NativeThreadAction`
- add a new public top-level helper skeleton, for example:

```rust
pub fn execute_class_to_completion(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>> { todo!() }
```

This helper will become the threaded entry boundary. Keep the existing `execute_class` callable while the refactor is in progress.

**Step 4: Update registry unit tests**

Adjust the native registry tests near the bottom of `crates/duke-interpreter/src/lib.rs` / `registry.rs` so dummy handlers accept the new `NativeControl` parameter and still compile.

**Step 5: Run the smallest compile target**

Run: `cargo test -p duke-interpreter native_registry_ -- --nocapture`

Expected: PASS or compile-only GREEN for the registry surface. Threading tests should still fail.

**Step 6: Commit**

```bash
git add crates/duke-interpreter/src/registry.rs crates/duke-interpreter/src/threading.rs crates/duke-interpreter/src/lib.rs
git commit -m "refactor(interpreter): add native thread control scaffold"
```

### Task 3: Extract A Resumable Execution State

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`
- Modify: `crates/duke-interpreter/src/threading.rs`

**Step 1: Hoist `execute_class` locals into a struct**

In `crates/duke-interpreter/src/lib.rs`, extract the mutable execution locals currently declared near the top of `execute_class` into:

```rust
struct ExecutionState {
    current_class: String,
    method_idx: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: std::sync::Arc<[(usize, Instruction)]>,
    frame: Frame,
    call_stack: Vec<CallFrame>,
    frame_pool: FramePool,
    dispatch_cache: HashMap<String, HashMap<u16, (String, usize, usize)>>,
    string_intern: HashMap<usize, u64>,
    idx: usize,
}
```

Also add a small step outcome enum:

```rust
enum StepResult {
    Returned(Option<Slot>),
    Blocked(threading::ThreadPause),
}
```

**Step 2: Convert the loop to `run_until_blocked_or_returned`**

Refactor the current `execute_class` loop into a method like:

```rust
impl ExecutionState {
    fn run_until_blocked_or_returned(
        &mut self,
        registry: &mut ClassRegistry,
        loader: &dyn ClassLoader,
        heap: &mut duke_gc::Heap,
        stdout: &mut dyn Write,
    ) -> VmResult<StepResult> { /* existing loop moved here */ }
}
```

After every native handler call, inspect `control.take()`:
- `Start { thread_ref }` => ask the runtime to spawn a worker and continue running the current thread
- `Sleep(duration)` => return `StepResult::Blocked(ThreadPause::Sleep(duration))`
- `Join { thread_id }` => return `StepResult::Blocked(ThreadPause::Join { thread_id })`

Do not use `VmError` as control flow for blocking.

**Step 3: Preserve single-thread behavior**

Rebuild `execute_class` as a thin wrapper that:
1. builds an `ExecutionState`
2. repeatedly calls `run_until_blocked_or_returned`
3. errors if a block is requested without a threaded runtime

Existing non-threaded interpreter tests must continue to use `execute_class` unchanged.

**Step 4: Gate GC while workers are live**

In each `heap.should_gc()` site inside the interpreter loop, add:

```rust
if !thread_runtime.has_live_workers() && heap.should_gc() {
    let roots = gather_roots(&self.frame, &self.call_stack, registry);
    heap.collect(&roots);
    patch_forwarded_slots(&mut self.frame, &mut self.call_stack, registry, heap);
}
```

For this phase, do not attempt to patch roots belonging to paused worker threads.

**Step 5: Run regression tests before thread natives**

Run: `cargo test -p duke-interpreter monitor_ file_io_ native_registry_ -- --nocapture`

Expected: PASS. `threading_` tests still fail because the stdlib/natives are not implemented yet.

**Step 6: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs crates/duke-interpreter/src/threading.rs
git commit -m "refactor(interpreter): make execution resumable at thread block points"
```

### Task 4: Add Synthetic `Thread` / `Runnable` And Host Thread Management

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`
- Modify: `crates/duke-interpreter/src/threading.rs`

**Step 1: Bootstrap the minimal thread stdlib surface**

Extend `bootstrap_stdlib` in `crates/duke-interpreter/src/lib.rs` with:
- `java/lang/Runnable` as an empty interface context
- `java/lang/Thread` with instance fields:
  - `fields[0]` = target `Runnable` ref or null
  - `fields[1]` = host thread id `int`
  - `fields[2]` = state `int` (`0 = NEW`, `1 = RUNNABLE`, `2 = TERMINATED`)
  - `fields[3]` = daemon flag `int` (always `0` in this phase)
- `java/lang/IllegalThreadStateException` synthetic class

Register natives:
- `Thread.<init>()V`
- `Thread.<init>(Ljava/lang/Runnable;)V`
- `Thread.start()V`
- `Thread.start0()V`
- `Thread.join()V`
- `Thread.sleep(J)V`
- `Thread.isAlive()Z`

**Step 2: Implement constructors and state helpers**

Add helpers in `crates/duke-interpreter/src/lib.rs`:

```rust
fn thread_ref_from_this(args: &[Slot]) -> VmResult<u64> { ... }
fn thread_id_from_obj(obj: &duke_gc::HeapObject) -> VmResult<i32> { ... }
fn set_thread_state(obj: &mut duke_gc::HeapObject, state: i32) { ... }
```

Constructor rules:
- `<init>()V` => target = null, id = 0, state = NEW, daemon = 0
- `<init>(Runnable)` => same, but store target ref in field 0

**Step 3: Make thread natives request actions instead of blocking inline**

Implement:

```rust
fn native_thread_start0(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = thread_ref_from_this(args)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.get(2) {
        Some(Slot::Int(0)) => control.request(NativeThreadAction::Start { thread_ref: this_ref }),
        _ => return Err(VmError::JavaException { class_name: "java/lang/IllegalThreadStateException".to_string() }),
    }
    Ok(None)
}
```

`sleep(J)V` should validate non-negative duration and request `NativeThreadAction::Sleep`.

`join()V` should read `fields[1]` and request `NativeThreadAction::Join`.

**Step 4: Spawn and run Java workers in the runtime**

In `crates/duke-interpreter/src/threading.rs`, add runtime functions:

```rust
impl ThreadRuntime {
    fn spawn_thread(/* entry refs + shared runtime */) -> VmResult<()>;
    fn wait_for_thread(&self, thread_id: i32);
    fn wait_for_all_non_daemon(&self);
}
```

When handling `NativeThreadAction::Start { thread_ref }`:
1. allocate the next host thread id
2. write it and `RUNNABLE` into the Java `Thread` object
3. create a worker `ExecutionState`
4. spawn `std::thread::spawn` that:
   - resolves the actual entry target:
     - if `thread_ref.class_name != "java/lang/Thread"` => run the concrete thread subclass `run()`
     - else if target field is non-null => run target object's concrete `run()`
     - else => return immediately
   - loops `run_until_blocked_or_returned`
   - performs sleep/join waits outside the VM lock
   - marks the Java object `TERMINATED` and notifies join waiters on exit

Do not promise parallel bytecode mutation. The shared VM lock only needs to be released at blocking points in this phase.

**Step 5: Run the targeted thread tests**

Run: `cargo test -p duke-interpreter threading_ -- --nocapture`

Expected: PASS.

**Step 6: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs crates/duke-interpreter/src/threading.rs
git commit -m "feat(interpreter): add basic Thread and Runnable support"
```

### Task 5: Top-Level Completion, ADR, And Verification

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`
- Modify: `duke/src/main.rs`
- Create: `docs/adr/001-basic-threading-runtime.md`

**Step 1: Finish the top-level wrapper**

Implement `execute_class_to_completion` so it:
1. registers the current caller as the main Java thread in the runtime
2. runs the requested entry method
3. waits for all non-daemon workers before returning
4. preserves `SystemExit` semantics

Threading tests from Task 1 should call this wrapper instead of raw `execute_class`.

**Step 2: Update the CLI**

In `duke/src/main.rs`, switch both `exec_method` and `run_main` from `execute_class(...)` to `execute_class_to_completion(...)`.

Do not change CLI flags or telemetry output behavior.

**Step 3: Write the ADR**

Create `docs/adr/001-basic-threading-runtime.md` with:
- decision: Phase 28 uses host OS threads plus a single shared interpreter critical section
- rationale: smallest change that satisfies `start` / `sleep` / `join`
- explicit limitations:
  - no real monitor semantics yet
  - GC is suspended while worker threads are live
  - daemon / interrupt support deferred

**Step 4: Run full verification**

Run:

```bash
cargo test -p duke-interpreter threading_ -- --nocapture
cargo test -p duke-interpreter monitor_ file_io_ -- --nocapture
cargo test -p duke-interpreter gc_generational_stress_test -- --nocapture
cargo test -p duke-interpreter --lib
```

Expected: all commands PASS.

**Step 5: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs duke/src/main.rs docs/adr/001-basic-threading-runtime.md
git commit -m "feat(duke): wait for non-daemon Java threads before exit"
```

---

## Execution Notes

- Do not expand this phase into real locking semantics. `monitorenter` / `monitorexit` stay as no-ops until the dedicated synchronization phase.
- Do not try to fix concurrent GC here. The acceptance target is lifecycle correctness for `Thread` / `Runnable`, not a concurrent collector.
- If the resumable extraction gets too gnarly inside `lib.rs`, stop and split `ExecutionState` into a new `crates/duke-interpreter/src/execute.rs` module before proceeding further. Do not keep inflating a single 20k+ line file just because inertia asked nicely.

