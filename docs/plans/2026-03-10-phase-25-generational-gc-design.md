# Phase 25 — Generational GC Design

**Date:** 2026-03-10
**Status:** Approved

## Problem

Phase 24's non-moving mark-sweep GC collects the entire heap on every collection.
Most objects die young (the generational hypothesis), so a full heap scan on every
GC is wasteful. Phase 25 adds a generational collector: a copying young gen
(Eden + S0/S1) and the existing mark-sweep old gen, with a remembered set write
barrier to enable young-only minor GC.

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Generation layout | Eden + S0/S1 (HotSpot-style) | Separates "just born" from "survived once" |
| Moving strategy | Forwarding pointers | Faithful to real JVM; no access-time overhead |
| Write barrier | Remembered set (`HashSet<usize>`) | Simple; interpreter routes all stores through `heap.write_field` |
| Reference encoding | High bit of `u64` = old gen | O(1) generation check; no separate type |

## Reference Encoding

```rust
const OLD_BIT: u64 = 1 << 63;

// Young-gen ref: r & OLD_BIT == 0  →  index into heap.young
// Old-gen ref:   r & OLD_BIT != 0  →  index (r & !OLD_BIT) into heap.old
```

`Slot::as_reference()` is unchanged. Callers that need generation use `r & OLD_BIT != 0`.

## Structural Changes — `duke-gc`

### `HeapObject`

```rust
pub struct HeapObject {
    pub class_name: String,
    pub fields: Vec<Slot>,
    pub string_value: Option<String>,
    pub(crate) marked: bool,         // old-gen mark bit (Phase 24, unchanged)
    pub(crate) age: u8,              // minor GC survivals; promote at threshold
    pub(crate) forward: Option<u64>, // forwarding pointer set during copy phase
}
```

### `Heap`

```rust
const DEFAULT_PROMOTION_AGE: u8 = 4;
const DEFAULT_YOUNG_CAPACITY: usize = 512;

pub struct Heap {
    // Young generation — bump-pointer allocation
    young: Vec<Option<HeapObject>>,
    young_top: usize,               // next free index in young

    // Old generation — Phase 24 mark-sweep (unchanged internals)
    old: Vec<Option<HeapObject>>,
    old_free_list: Vec<u64>,        // raw old-gen indices (no OLD_BIT)
    live_after_last_gc: usize,
    alloc_since_gc: usize,

    // Write barrier
    remembered_set: HashSet<usize>, // old-gen indices with ≥1 young ref in fields

    // Tuning
    young_capacity: usize,          // minor GC fires when young_top >= this
    promotion_age: u8,              // survivals before promotion
    live_count: usize,              // total live objects across both generations
}
```

## Allocation

All new objects land in young gen:

```rust
pub fn allocate(&mut self, class_name: String, field_count: usize) -> u64 {
    let idx = self.young_top as u64;  // OLD_BIT = 0 → young ref
    // grow young Vec if needed, insert Some(HeapObject { age: 0, forward: None, ... })
    self.young_top += 1;
    idx
}
```

`allocate_string` follows the same pattern.

## GC Triggers

```rust
pub fn should_minor_gc(&self) -> bool {
    self.young_top >= self.young_capacity
}

pub fn should_major_gc(&self) -> bool {
    let threshold = (self.live_after_last_gc * 2).max(256);
    self.alloc_since_gc >= threshold
}
```

## Write Barrier

Replaces direct `heap.get_mut(r)?.fields[i] = slot` at all field/array store sites:

```rust
pub fn write_field(&mut self, obj_ref: u64, field_idx: usize, value: Slot) -> VmResult<()> {
    // old-gen object receiving a young-gen reference → record in remembered set
    if obj_ref & OLD_BIT != 0 {
        if value.as_reference().is_some_and(|r| r & OLD_BIT == 0) {
            self.remembered_set.insert((obj_ref & !OLD_BIT) as usize);
        }
    }
    self.get_mut(obj_ref)?.fields[field_idx] = value;
    Ok(())
}
```

Interpreter sites requiring `write_field`: `putfield`, `putstatic`,
`aastore`, `iastore`, `lastore`, `fastore`, `dastore`, `bastore`, `castore`, `sastore`
(~10 sites total).

## Minor GC — Copy Collection

`pub fn minor_collect(&mut self, roots: &[Slot])`

### Phase 1: Copy

Build `to_space: Vec<Option<HeapObject>>`. Worklist seeds from:
- All `Slot::Reference(Some(r))` in roots where `r & OLD_BIT == 0`
- All young refs in fields of old-gen objects in `remembered_set`

For each live young object `r`:

```
if young[r].forward is Some(new_r) → already copied, skip
if young[r].age >= promotion_age   → promote: copy to old gen, new_r = old_idx | OLD_BIT
else                               → copy to to_space, new_r = to_space.len() as u64
                                     young[r].age += 1

young[r].forward = Some(new_r)  // install forwarding pointer
push all young child refs of young[r] onto worklist
```

Promotion appends to `self.old` (or reuses `old_free_list`), returns `idx | OLD_BIT`.

### Phase 2: Patch References

Walk every root slot and every field of old-gen objects in `remembered_set`.
For any `Slot::Reference(Some(r))` where `r & OLD_BIT == 0`:

```
new_r = young[r as usize].forward.unwrap()
replace slot with Slot::Reference(Some(new_r))
```

### Phase 3: Flip

```
self.young = to_space          // old from-space dropped (forwarding ptrs gone)
self.young_top = to_space.len()
self.remembered_set.clear()   // rebuilt lazily by write barrier
```

## Major GC — Full Collection

`pub fn collect(&mut self, roots: &[Slot])`

1. Run `minor_collect(roots)` first — promotes all young survivors to old gen.
2. Run old-gen mark-sweep (Phase 24 algorithm) on `self.old` with the same roots.
   After step 1, all live objects are in old gen, so the root scan is complete.
3. Update `live_after_last_gc`, reset `alloc_since_gc`.

## Public API

```rust
// Allocation — unchanged signatures
pub fn allocate(&mut self, class_name: String, field_count: usize) -> u64
pub fn allocate_string(&mut self, value: String) -> u64

// Write barrier — new, replaces direct field mutation
pub fn write_field(&mut self, obj_ref: u64, field_idx: usize, value: Slot) -> VmResult<()>

// GC triggers — replaces single should_gc()
pub fn should_minor_gc(&self) -> bool
pub fn should_major_gc(&self) -> bool

// Collection — minor_collect is new; collect signature unchanged
pub fn minor_collect(&mut self, roots: &[Slot])
pub fn collect(&mut self, roots: &[Slot])
```

## Interpreter Integration

### GC trigger sites (5 sites, unchanged locations)

```rust
// Before (Phase 24):
if heap.should_gc() { heap.collect(&roots); }

// After (Phase 25):
if heap.should_major_gc()      { heap.collect(&roots); }
else if heap.should_minor_gc() { heap.minor_collect(&roots); }
```

### Field/array store sites (~10 sites)

```rust
// Before:
heap.get_mut(r)?.fields[i] = slot;

// After:
heap.write_field(r, i, slot)?;
```

## Test Plan

### `duke-gc` unit tests

| Test | What it proves |
|------|----------------|
| `young_ref_has_no_old_bit` | allocated object index has OLD_BIT = 0 |
| `old_ref_has_old_bit` | promoted object index has OLD_BIT set |
| `minor_gc_copies_survivors` | 3 young allocs, 2 reachable → to_space has 2 |
| `minor_gc_clears_unreachable` | unreachable young objects not in to_space |
| `minor_gc_promotes_at_age` | object surviving `promotion_age` GCs lands in old gen |
| `forwarding_ptr_patches_roots` | root ref updated to new address after copy |
| `remembered_set_populated` | write_field old→young inserts into remembered_set |
| `remembered_set_root_on_minor_gc` | object reachable only via old→young ref survives |
| `major_gc_collects_old_gen` | unreachable old-gen objects swept after full collect |

### `duke-interpreter` integration test

`GcGenerationalStressTest.java` — allocates 5000 short-lived objects and 10
long-lived objects in a loop. Verifies correct output AND that long-lived objects
survive in old gen while short-lived ones are reclaimed by minor GC.

### Regression

All 349 existing tests pass unchanged.

## What Phase 26 Could Add

- Concurrent marking (don't stop the world for old-gen mark)
- Incremental minor GC (bounded pause time)
- `-Xmx` / `-Xms` style heap size configuration
