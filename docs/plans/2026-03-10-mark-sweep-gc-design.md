# Phase 24 — Non-Moving Mark-Sweep GC

**Date:** 2026-03-10
**Status:** Approved

## Problem

The current heap is a bump-pointer allocator that never frees memory. Long-running programs or
stress workloads will exhaust memory. Phase 24 adds a non-moving mark-sweep collector as the
foundation for future GC work (Phase 25: compacting/copying).

## Scope

- Non-moving mark-sweep in `duke-gc`
- 2× growth-ratio trigger (same heuristic as Go's GC)
- Interpreter integration: root gathering at each allocation site
- Free-list reuse of swept slots
- Phase 25 will add compaction/copying (generational or semi-space)

## Structural Changes — `duke-gc`

### `HeapObject`

```rust
pub struct HeapObject {
    pub class_name: String,
    pub fields: Vec<Slot>,
    pub string_value: Option<String>,
    pub(crate) marked: bool,   // GC mark bit — reset to false after each sweep
}
```

### `Heap`

```rust
pub struct Heap {
    objects: Vec<Option<HeapObject>>,  // None = dead slot available for reuse
    free_list: Vec<u64>,               // recycled indices, popped on allocation
    live_after_last_gc: usize,         // live count after last sweep
    alloc_since_gc: usize,             // allocations since last collection
}
```

`objects` changes from `Vec<HeapObject>` to `Vec<Option<HeapObject>>`.
`get()` / `get_mut()` add one `.as_ref()`/`.as_mut()` unwrap — callers see no difference.

## GC Trigger

```
threshold = max(256, live_after_last_gc * 2)
should_gc() = alloc_since_gc >= threshold
```

The floor of 256 prevents collecting on cold-start programs before any GC baseline exists.

## Public API Additions

```rust
// Already-stable allocation API — return type unchanged.
pub fn allocate(&mut self, class_name: String, field_count: usize) -> u64
pub fn allocate_string(&mut self, value: String) -> u64

// New GC API.
pub fn should_gc(&self) -> bool
pub fn collect(&mut self, roots: &[Slot])
```

`allocate()` pops from `free_list` first; bumps the vec if empty. Increments `alloc_since_gc`.

## Mark Phase

Iterative DFS from roots to avoid call-stack overflow on deep object graphs:

```
worklist ← all Slot::Reference(Some(r)) in roots
while worklist not empty:
    r = worklist.pop()
    if objects[r] is None or already marked: skip
    mark objects[r]
    push all Slot::Reference children of objects[r] onto worklist
```

## Sweep Phase

Single pass over `objects`:

```
for each (idx, slot) in objects:
    Some(obj) where marked  → obj.marked = false, live++
    Some(obj) where !marked → *slot = None, free_list.push(idx)
    None                    → skip (already dead from prior cycle)

live_after_last_gc = live
alloc_since_gc = 0
```

## Interpreter Integration

At each `heap.allocate()` call site (~5 sites: `New`, `Newarray`, `Anewarray`,
`Multianewarray`, lambda proxy):

```rust
let r = heap.allocate(class_name, field_count);
if heap.should_gc() {
    heap.collect(&gather_roots(&frame, &call_stack, registry));
}
```

### `gather_roots`

Free function in `duke-interpreter/src/lib.rs`:

```rust
fn gather_roots(frame: &Frame, call_stack: &[CallFrame], registry: &ClassRegistry) -> Vec<Slot> {
    let mut roots = Vec::new();
    roots.extend(frame.slots());
    for cf in call_stack { roots.extend(cf.frame.slots()); }
    for ctx in registry.all_classes() { roots.extend(ctx.static_fields.iter().cloned()); }
    roots
}
```

### Required additions (both one-liners)

- `Frame::slots(&self) -> impl Iterator<Item = Slot>` — yields locals + stack values
- `ClassRegistry::all_classes() -> impl Iterator<Item = &ClassContext>` — iterates classes map

## Test Plan

### `duke-gc` unit tests

| Test | What it proves |
|------|----------------|
| `collect_reclaims_unreachable` | 3 allocs, 2 not in roots → free_list gains 2 entries |
| `collect_preserves_reachable` | A→B→C graph, only A in roots → all 3 survive |
| `free_list_reused_on_alloc` | post-collect alloc returns recycled index, not new bump slot |
| `should_gc_triggers_at_2x` | threshold fires, resets after collect |

### `duke-interpreter` integration test

`gc_reclaims_short_lived_objects` — `GcStressTest.java` allocates `new Object()` 2000× in
a loop, verifies correct output AND `heap.len()` stays well below 2000 (GC fired and reclaimed).

### Regression

All 281 existing tests pass unchanged.

## What Phase 25 Adds

- Semi-space copying for young objects (Eden + Survivor)
- Forwarding pointers to patch references after moving
- Promotion to old-gen after N survivals
