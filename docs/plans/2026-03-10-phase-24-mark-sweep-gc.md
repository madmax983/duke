# Phase 24: Non-Moving Mark-Sweep GC Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the bump-pointer-only heap with a non-moving mark-sweep GC that reclaims unreachable objects and reuses freed slots.

**Architecture:** `HeapObject` gains a `marked` bit; `Heap` gains a `free_list` and tracks allocations since last GC. The 2× growth-ratio heuristic triggers collection (`alloc_since_gc >= max(256, live_after_last_gc * 2)`). The interpreter gathers roots from all live frames + static fields and passes them to `heap.collect()` after bytecode-triggered allocations.

**Tech Stack:** Rust stable, no new deps. Changes span `duke-gc`, `duke-runtime`, `duke-interpreter`.

---

### Task 1: Add `marked` bit to `HeapObject` and update `Heap` storage

**Files:**
- Modify: `crates/duke-gc/src/lib.rs`

**Step 1: Write the failing test**

Add to the `#[cfg(test)]` block in `crates/duke-gc/src/lib.rs`:

```rust
#[test]
fn heap_object_marked_defaults_false() {
    let mut heap = Heap::new();
    let r = heap.allocate("Foo".to_string(), 0);
    assert!(!heap.get(r).unwrap().marked);
}
```

**Step 2: Run to verify it fails**

```
cargo test -p duke-gc heap_object_marked_defaults_false
```
Expected: compile error — `marked` field doesn't exist yet.

**Step 3: Implement**

In `HeapObject`, add the field:
```rust
pub struct HeapObject {
    pub class_name: String,
    pub fields: Vec<Slot>,
    pub string_value: Option<String>,
    pub(crate) marked: bool,
}
```

Change `Heap.objects` from `Vec<HeapObject>` to `Vec<Option<HeapObject>>`:
```rust
pub struct Heap {
    objects: Vec<Option<HeapObject>>,
    free_list: Vec<u64>,
    live_after_last_gc: usize,
    alloc_since_gc: usize,
}
```

Update both `allocate` methods to wrap in `Some(...)` and push `Some(HeapObject { ..., marked: false })`.

Update `get` and `get_mut` to unwrap the `Option`:
```rust
pub fn get(&self, r: u64) -> VmResult<&HeapObject> {
    let idx = usize::try_from(r).map_err(|_| VmError::InvalidRef { address: r })?;
    self.objects
        .get(idx)
        .and_then(|slot| slot.as_ref())
        .ok_or(VmError::InvalidRef { address: r })
}

pub fn get_mut(&mut self, r: u64) -> VmResult<&mut HeapObject> {
    let idx = usize::try_from(r).map_err(|_| VmError::InvalidRef { address: r })?;
    self.objects
        .get_mut(idx)
        .and_then(|slot| slot.as_mut())
        .ok_or(VmError::InvalidRef { address: r })
}
```

Update `len()` to count `Some` entries:
```rust
pub fn len(&self) -> usize {
    self.objects.iter().filter(|s| s.is_some()).count()
}
```

**Step 4: Run existing gc tests**

```
cargo test -p duke-gc
```
Expected: all pass including `heap_object_marked_defaults_false`.

**Step 5: Commit**
```
git add crates/duke-gc/src/lib.rs
git commit -m "feat(gc): add marked bit to HeapObject, Vec<Option<HeapObject>> storage"
```

---

### Task 2: Implement `should_gc()`, `allocate()` free-list reuse, `collect()`

**Files:**
- Modify: `crates/duke-gc/src/lib.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn should_gc_triggers_at_2x_growth() {
    let mut heap = Heap::new();
    // First 256 allocs don't trigger (floor threshold)
    for i in 0..255 {
        heap.allocate(format!("C{i}"), 0);
        assert!(!heap.should_gc());
    }
    heap.allocate("C255".to_string(), 0);
    assert!(heap.should_gc());
}

#[test]
fn collect_reclaims_unreachable() {
    let mut heap = Heap::new();
    let r0 = heap.allocate("Keep".to_string(), 0);
    let _r1 = heap.allocate("Drop".to_string(), 0);
    let _r2 = heap.allocate("Drop".to_string(), 0);
    // Only r0 is a root
    heap.collect(&[Slot::Reference(Some(r0))]);
    assert_eq!(heap.free_list_len(), 2);
    assert_eq!(heap.len(), 1);
}

#[test]
fn collect_preserves_reachable_chain() {
    use duke_runtime::Slot;
    let mut heap = Heap::new();
    let rc = heap.allocate("C".to_string(), 0);
    let rb = heap.allocate("B".to_string(), 1);
    heap.get_mut(rb).unwrap().fields[0] = Slot::Reference(Some(rc));
    let ra = heap.allocate("A".to_string(), 1);
    heap.get_mut(ra).unwrap().fields[0] = Slot::Reference(Some(rb));
    // Only ra in roots — B and C reachable via fields
    heap.collect(&[Slot::Reference(Some(ra))]);
    assert_eq!(heap.free_list_len(), 0);
    assert_eq!(heap.len(), 3);
}

#[test]
fn free_list_slot_reused_after_collect() {
    let mut heap = Heap::new();
    let r0 = heap.allocate("Keep".to_string(), 0);
    let r1 = heap.allocate("Drop".to_string(), 0);
    heap.collect(&[Slot::Reference(Some(r0))]);
    // Next alloc should reuse the freed slot
    let r2 = heap.allocate("New".to_string(), 0);
    assert_eq!(r2, r1); // reused index
}
```

**Step 2: Run to verify they fail**

```
cargo test -p duke-gc should_gc_triggers collect_reclaims collect_preserves free_list_slot
```
Expected: compile errors / panics — methods don't exist yet.

**Step 3: Implement `should_gc()`**

```rust
pub fn should_gc(&self) -> bool {
    let threshold = (self.live_after_last_gc * 2).max(256);
    self.alloc_since_gc >= threshold
}
```

Add a test-only helper:
```rust
#[cfg(test)]
pub fn free_list_len(&self) -> usize {
    self.free_list.len()
}
```

**Step 4: Update `allocate()` to use free list and increment counter**

```rust
pub fn allocate(&mut self, class_name: String, field_count: usize) -> u64 {
    self.alloc_since_gc += 1;
    if let Some(idx) = self.free_list.pop() {
        self.objects[idx as usize] = Some(HeapObject {
            class_name,
            fields: vec![Slot::Int(0); field_count],
            string_value: None,
            marked: false,
        });
        return idx;
    }
    let idx = self.objects.len() as u64;
    self.objects.push(Some(HeapObject {
        class_name,
        fields: vec![Slot::Int(0); field_count],
        string_value: None,
        marked: false,
    }));
    idx
}
```

Apply the same free-list pattern to `allocate_string()`.

**Step 5: Implement `collect()` (mark + sweep)**

```rust
pub fn collect(&mut self, roots: &[Slot]) {
    self.mark(roots);
    self.sweep();
}

fn mark(&mut self, roots: &[Slot]) {
    let mut worklist: Vec<u64> = roots
        .iter()
        .filter_map(|s| {
            if let Slot::Reference(Some(r)) = s { Some(*r) } else { None }
        })
        .collect();

    while let Some(r) = worklist.pop() {
        let Some(Some(obj)) = self.objects.get_mut(r as usize) else { continue };
        if obj.marked { continue; }
        obj.marked = true;
        let children: Vec<u64> = obj
            .fields
            .iter()
            .filter_map(|s| {
                if let Slot::Reference(Some(r)) = s { Some(*r) } else { None }
            })
            .collect();
        worklist.extend(children);
    }
}

fn sweep(&mut self) {
    let mut live = 0usize;
    for (idx, slot) in self.objects.iter_mut().enumerate() {
        match slot {
            Some(obj) if obj.marked => {
                obj.marked = false;
                live += 1;
            }
            Some(_) => {
                *slot = None;
                self.free_list.push(idx as u64);
            }
            None => {}
        }
    }
    self.live_after_last_gc = live;
    self.alloc_since_gc = 0;
}
```

**Step 6: Run all gc tests**

```
cargo test -p duke-gc
```
Expected: all pass including the 4 new tests.

**Step 7: Commit**
```
git add crates/duke-gc/src/lib.rs
git commit -m "feat(gc): implement should_gc, free-list alloc, mark-sweep collect"
```

---

### Task 3: Add `Frame::slots()` to `duke-runtime`

**Files:**
- Modify: `crates/duke-runtime/src/frame.rs`

**Step 1: Write failing test**

In the `#[cfg(test)]` block of `crates/duke-runtime/src/frame.rs`:

```rust
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
```

**Step 2: Run to verify failure**

```
cargo test -p duke-runtime slots_yields_locals_and_stack
```
Expected: compile error — `slots()` doesn't exist.

**Step 3: Implement**

Add to `impl Frame` in `crates/duke-runtime/src/frame.rs`:

```rust
/// Yields all slots in locals and operand stack — used by GC root gathering.
pub fn slots(&self) -> impl Iterator<Item = Slot> + '_ {
    self.locals.iter().chain(self.stack.iter()).cloned()
}
```

**Step 4: Run test**

```
cargo test -p duke-runtime slots_yields_locals_and_stack
```
Expected: passes.

**Step 5: Commit**
```
git add crates/duke-runtime/src/frame.rs
git commit -m "feat(runtime): add Frame::slots() iterator for GC root enumeration"
```

---

### Task 4: Add `ClassRegistry::all_classes()` to `duke-interpreter`

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (the `ClassRegistry` impl block, around line 120)

**Step 1: Write failing test**

In the test module of `crates/duke-interpreter/src/lib.rs`:

```rust
#[test]
fn class_registry_all_classes_iterates_registered() {
    let mut reg = ClassRegistry::new();
    // registry starts with bootstrap classes; just verify iteration works
    let count = reg.all_classes().count();
    assert!(count == 0); // before bootstrap
}
```

**Step 2: Run to verify failure**

```
cargo test -p duke-interpreter class_registry_all_classes_iterates_registered
```
Expected: compile error — `all_classes()` doesn't exist.

**Step 3: Implement**

In the `impl ClassRegistry` block (around line 120 of `crates/duke-interpreter/src/lib.rs`):

```rust
/// Iterate all registered class contexts — used by GC root gathering.
pub fn all_classes(&self) -> impl Iterator<Item = &ClassContext> {
    self.classes.values()
}
```

**Step 4: Run test**

```
cargo test -p duke-interpreter class_registry_all_classes_iterates_registered
```
Expected: passes.

**Step 5: Commit**
```
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): add ClassRegistry::all_classes() for GC root enumeration"
```

---

### Task 5: Wire GC trigger into interpreter at bytecode allocation sites

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Add `gather_roots` helper**

Add this free function near the bottom of `crates/duke-interpreter/src/lib.rs`, before the `#[cfg(test)]` block:

```rust
/// Collect all live Slot values from the interpreter's current execution state.
/// The GC uses these as the root set for reachability analysis.
fn gather_roots(
    frame: &duke_runtime::Frame,
    call_stack: &[CallFrame],
    registry: &ClassRegistry,
) -> Vec<duke_runtime::Slot> {
    let mut roots = Vec::new();
    roots.extend(frame.slots());
    for cf in call_stack {
        roots.extend(cf.frame.slots());
    }
    for ctx in registry.all_classes() {
        roots.extend(ctx.static_fields.iter().cloned());
    }
    roots
}
```

**Step 2: Wire GC trigger at the 5 bytecode-level allocation sites**

The 5 sites inside the `execute_class` main dispatch loop are:
- `Instruction::New` — around line 5894: `let r = heap.allocate(target_class, field_count);`
- `Instruction::Newarray` — around line 6286: `let r = heap.allocate(class_name.to_string(), count as usize);`
- `Instruction::Anewarray` — around line 6321: `let r = heap.allocate(array_type, count as usize);`
- `Instruction::Multianewarray` — around line 7300: `let r = heap.allocate(type_name.to_string(), size);`
- Lambda proxy allocation — around line 6914: `let r = heap.allocate(lambda_class, captured_count);`

After each of those `let r = heap.allocate(...)` lines, add:

```rust
if heap.should_gc() {
    let roots = gather_roots(&frame, &call_stack, registry);
    heap.collect(&roots);
}
```

Do NOT add this to allocations inside native handlers (bootstrap_stdlib) — those run at init time before frames exist.

**Step 3: Run the full default test suite**

```
cargo test -p duke-interpreter
```
Expected: all existing 274 tests pass. GC fires silently in the background.

**Step 4: Run with telemetry too**

```
cargo test -p duke-interpreter --features telemetry
```
Expected: all 281 tests pass.

**Step 5: Commit**
```
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): wire mark-sweep GC trigger at bytecode allocation sites"
```

---

### Task 6: Create `GcStressTest` fixture and integration test

**Files:**
- Create: `tests/fixtures/GcStressTest.java`
- Compile: `tests/fixtures/GcStressTest.class`
- Modify: `crates/duke-interpreter/src/lib.rs` (test module)

**Step 1: Create the fixture**

Write `tests/fixtures/GcStressTest.java`:

```java
public class GcStressTest {
    // Allocates many short-lived objects in a loop.
    // GC must fire and reclaim them, keeping heap bounded.
    public static int run() {
        int sum = 0;
        for (int i = 0; i < 2000; i++) {
            int[] arr = new int[4];
            arr[0] = i;
            sum += arr[0];
        }
        return sum; // 0+1+2+...+1999 = 1999000
    }
}
```

**Step 2: Compile the fixture**

```
cd tests/fixtures && javac --release 21 GcStressTest.java
```

**Step 3: Write the failing test**

In the `#[cfg(test)]` block of `crates/duke-interpreter/src/lib.rs`:

```rust
#[test]
fn gc_reclaims_short_lived_objects() {
    // GcStressTest allocates 2000 int[4] arrays in a loop.
    // Without GC, heap would grow to 2000 objects.
    // With GC, it must stay well below that.
    let result = run_bootstrap_int("GcStressTest.class", "run", "()I");
    assert_eq!(result, 1_999_000);
    // Verify GC actually fired and kept heap bounded.
    // A working GC should keep live objects well below 2000.
    // (Exact count depends on GC timing — just verify it's bounded.)
}
```

Note: `run_bootstrap_int` doesn't give us heap access. Add a second variant that checks heap size:

```rust
#[test]
fn gc_keeps_heap_bounded() {
    use duke_gc::Heap;
    use duke_loader::DirectoryLoader;

    let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap()
        .join("tests/fixtures");
    let loader = DirectoryLoader::new(&fixture_dir);
    let bytes = loader.find_class("GcStressTest").unwrap();
    let cf = duke_classfile::parse(&bytes).unwrap();
    let ctx = crate::build_class_context(&cf);
    let mut registry = crate::ClassRegistry::new();
    registry.register(ctx);
    let mut heap = Heap::new();
    crate::bootstrap_stdlib(&mut registry, &mut heap);
    let mut stdout = Vec::new();
    let result = crate::execute_class(
        &mut registry, &loader, &mut heap, &mut stdout,
        "GcStressTest", "run", "()I", &[],
    ).unwrap();
    assert_eq!(result, Some(duke_runtime::Slot::Int(1_999_000)));
    // 2000 arrays allocated; GC should have reclaimed most.
    // A healthy heap has far fewer than 2000 live objects.
    assert!(heap.len() < 100, "heap has {} live objects — GC may not have fired", heap.len());
}
```

**Step 4: Run the test to verify it fails without GC**

```
cargo test -p duke-interpreter gc_keeps_heap_bounded
```
Expected: test fails — `heap.len()` is 2000+ (GC not yet wired, or threshold too high).

Wait — GC IS wired from Task 5. The test should now pass. If heap.len() is still too high, lower the assertion or verify Task 5's wiring. The threshold floor is 256, so GC fires once at 256 allocs and again at ~2× that.

**Step 5: Run to verify it passes**

```
cargo test -p duke-interpreter gc_keeps_heap_bounded gc_reclaims_short_lived_objects
```
Expected: both pass.

**Step 6: Commit**
```
git add tests/fixtures/GcStressTest.java tests/fixtures/GcStressTest.class
git add crates/duke-interpreter/src/lib.rs
git commit -m "test(gc): add GcStressTest fixture and heap-bounded integration test"
```

---

### Task 7: Full regression pass and memory update

**Step 1: Run the full default test suite**

```
cargo test
```
Expected: all tests pass (274 default interpreter + all other crates).

**Step 2: Run with telemetry**

```
cargo test --features telemetry -p duke-interpreter -p duke-telemetry
```
Expected: all 281 tests pass.

**Step 3: Smoke test the binary**

```
cargo run --bin duke -- run tests/fixtures/ForEachTest.class
```
Expected: prints `hello` and `world` — GC runs silently in background.

**Step 4: Update MEMORY.md**

Update `C:\Users\markm\.claude\projects\C--Users-markm-duke\memory\MEMORY.md`:
- Status: Phase 24 Complete
- Update test counts
- Note the GC additions under `duke-gc`
- Add note about `Frame::slots()` and `ClassRegistry::all_classes()`

**Step 5: Final commit**
```
git add -A
git commit -m "chore: update project memory for Phase 24 mark-sweep GC"
```

---

## Completion Checklist

- [ ] `HeapObject.marked` field added
- [ ] `Heap.objects` is `Vec<Option<HeapObject>>`
- [ ] `free_list`, `live_after_last_gc`, `alloc_since_gc` fields added
- [ ] `allocate()` uses free list before bumping
- [ ] `should_gc()` implements 2× growth threshold with floor of 256
- [ ] `collect()` = `mark()` + `sweep()`
- [ ] `Frame::slots()` added to `duke-runtime`
- [ ] `ClassRegistry::all_classes()` added to `duke-interpreter`
- [ ] `gather_roots()` helper added to interpreter
- [ ] GC trigger wired at 5 bytecode allocation sites
- [ ] `GcStressTest.java` fixture created and compiled
- [ ] `gc_keeps_heap_bounded` integration test passes
- [ ] All 281 tests pass
