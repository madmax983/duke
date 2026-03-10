# Phase 25: Generational GC Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the non-moving mark-sweep GC with a full generational collector: copying young gen (Eden+S0/S1) with forwarding pointers and a remembered set write barrier, keeping the existing mark-sweep old gen.

**Architecture:** Young objects allocate via bump pointer into `young: Vec<Option<HeapObject>>`; minor GC copies survivors using forwarding pointers and patches all live interpreter slots; old gen is the existing Phase 24 mark-sweep Vec. References encode generation in the high bit: `r & OLD_BIT != 0` means old gen. Field stores go through `write_field()` which maintains a remembered set of old-gen objects holding young refs.

**Tech Stack:** Rust stable, no new deps (std::collections::HashSet already used in duke-gc indirectly via duke-interpreter). Changes span `duke-gc`, `duke-runtime`, `duke-interpreter`.

---

## Task 1: Add `age` and `forward` fields to `HeapObject`

**Files:**
- Modify: `crates/duke-gc/src/lib.rs`

### Step 1: Write the failing tests

Add to the `#[cfg(test)]` block in `crates/duke-gc/src/lib.rs`:

```rust
#[test]
fn heap_object_age_defaults_zero() {
    let mut heap = Heap::new();
    let r = heap.allocate("Foo".to_string(), 0);
    assert_eq!(heap.get(r).unwrap().age, 0);
}

#[test]
fn heap_object_forward_defaults_none() {
    let mut heap = Heap::new();
    let r = heap.allocate("Bar".to_string(), 1);
    assert!(heap.get(r).unwrap().forward.is_none());
}
```

### Step 2: Run to verify they fail

```
cargo test -p duke-gc heap_object_age_defaults_zero
cargo test -p duke-gc heap_object_forward_defaults_none
```

Expected: compile error — `age` and `forward` fields don't exist yet.

### Step 3: Implement

Replace the `HeapObject` struct definition in `crates/duke-gc/src/lib.rs`:

```rust
/// A single heap-allocated Java object.
#[derive(Debug, Clone)]
pub struct HeapObject {
    pub class_name: String,
    pub fields: Vec<Slot>,
    /// String content for `java/lang/String` objects. `None` for non-string objects.
    pub string_value: Option<String>,
    /// Mark bit for old-gen mark-sweep. `false` until marked reachable.
    // INVARIANT: Heap::mark() traces only `fields` for child references.
    // Any new Vec<Slot> member added to HeapObject MUST also be covered in mark().
    pub(crate) marked: bool,
    /// Number of minor GC cycles this object has survived. Promote at `promotion_age`.
    pub(crate) age: u8,
    /// Forwarding pointer installed during copy phase of minor GC.
    /// `Some(new_ref)` means this young-gen slot was copied to `new_ref`.
    pub(crate) forward: Option<u64>,
}
```

Update the two `allocate_with`-call sites in `allocate` and `allocate_string` to include the new fields. Update `allocate_with` itself (it constructs a `HeapObject` directly) — actually `allocate_with` receives a fully-constructed `HeapObject`, so update the callers:

In `allocate`:
```rust
pub fn allocate(&mut self, class_name: String, field_count: usize) -> u64 {
    self.allocate_with(HeapObject {
        class_name,
        fields: vec![Slot::Int(0); field_count],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    })
}
```

In `allocate_string`:
```rust
pub fn allocate_string(&mut self, value: String) -> u64 {
    self.allocate_with(HeapObject {
        class_name: "java/lang/String".to_string(),
        fields: Vec::new(),
        string_value: Some(value),
        marked: false,
        age: 0,
        forward: None,
    })
}
```

### Step 4: Run tests

```
cargo test -p duke-gc
```

Expected: all existing tests pass plus the two new ones.

### Step 5: Commit

```
git add crates/duke-gc/src/lib.rs
git commit -m "feat(gc): add age + forward fields to HeapObject for generational GC"
```

---

## Task 2: Split Heap into young/old generations; update allocate/get/get_mut

**Files:**
- Modify: `crates/duke-gc/src/lib.rs`

This is the largest structural change. The existing `objects` Vec becomes `old`; a new `young` Vec is added with bump-pointer allocation. All `allocate` calls now land in young gen; `get`/`get_mut` dispatch on the `OLD_BIT` high bit.

### Step 2a: Write the failing tests

Add to `#[cfg(test)]` in `crates/duke-gc/src/lib.rs`:

```rust
#[test]
fn young_ref_has_no_old_bit() {
    let mut heap = Heap::new();
    let r = heap.allocate("Young".to_string(), 0);
    assert_eq!(r & (1u64 << 63), 0, "young ref must not have OLD_BIT set");
}

#[test]
fn allocate_returns_sequential_young_indices() {
    let mut heap = Heap::new();
    let r0 = heap.allocate("A".to_string(), 0);
    let r1 = heap.allocate("B".to_string(), 0);
    assert_eq!(r0, 0);
    assert_eq!(r1, 1);
}

#[test]
fn get_on_young_ref_works() {
    let mut heap = Heap::new();
    let r = heap.allocate("Point".to_string(), 2);
    let obj = heap.get(r).unwrap();
    assert_eq!(obj.class_name, "Point");
    assert_eq!(obj.fields.len(), 2);
}

#[test]
fn old_gen_initially_empty() {
    let heap = Heap::new();
    assert_eq!(heap.old_live_count(), 0);
}
```

### Step 2b: Run to verify they fail

```
cargo test -p duke-gc young_ref_has_no_old_bit
```

Expected: compile error — `OLD_BIT`, `young`, `old_live_count` not defined.

### Step 2c: Implement

Replace the entire `duke-gc/src/lib.rs` with the generational layout. The full file content:

```rust
//! Generational GC for the Duke JVM (Phase 25).
//!
//! Young objects are allocated via bump pointer into `young: Vec<Option<HeapObject>>`.
//! Objects surviving `promotion_age` minor GCs are promoted to the old generation.
//! Old-gen uses the Phase 24 mark-sweep algorithm.
//!
//! Reference encoding:
//! - `r & OLD_BIT == 0`: index into `young`
//! - `r & OLD_BIT != 0`: index `(r & !OLD_BIT)` into `old`

use std::collections::HashSet;

use duke_runtime::{Slot, VmError, VmResult};

/// High bit of a `u64` heap reference marks an old-generation object.
pub const OLD_BIT: u64 = 1 << 63;

/// Number of minor GC survivals before an object is promoted to old gen.
const DEFAULT_PROMOTION_AGE: u8 = 4;

/// Minor GC fires when `young_top >= young_capacity`.
const DEFAULT_YOUNG_CAPACITY: usize = 512;

/// A single heap-allocated Java object.
#[derive(Debug, Clone)]
pub struct HeapObject {
    pub class_name: String,
    pub fields: Vec<Slot>,
    /// String content for `java/lang/String` objects. `None` for non-string objects.
    pub string_value: Option<String>,
    /// Mark bit for old-gen mark-sweep. `false` until marked reachable.
    // INVARIANT: major_collect() traces only `fields` for child references.
    // Any new Vec<Slot> member added to HeapObject MUST also be covered there.
    pub(crate) marked: bool,
    /// Number of minor GC cycles this object has survived. Promoted when >= promotion_age.
    pub(crate) age: u8,
    /// Forwarding pointer installed during the copy phase of minor GC.
    /// `Some(new_ref)` means this young-gen slot has been copied to `new_ref`.
    pub(crate) forward: Option<u64>,
}

/// The generational object heap.
///
/// Young-gen uses bump-pointer allocation into `young`.
/// Old-gen uses the Phase 24 mark-sweep Vec with a free-list.
/// References are `u64`; the high bit (`OLD_BIT`) distinguishes generations.
#[derive(Debug)]
pub struct Heap {
    // ---- Young generation ----
    /// Bump-pointer allocation space. Index is the raw young-gen reference (no OLD_BIT).
    young: Vec<Option<HeapObject>>,
    /// Next free slot index in `young`. Triggers minor GC when >= `young_capacity`.
    young_top: usize,
    /// Scratch space during minor GC copy phase. Empty when not collecting.
    to_space: Vec<Option<HeapObject>>,

    // ---- Old generation (Phase 24 mark-sweep) ----
    /// Old-gen objects. Reference = (raw_idx | OLD_BIT).
    old: Vec<Option<HeapObject>>,
    /// Freed old-gen indices available for reuse (raw, no OLD_BIT).
    old_free_list: Vec<u64>,
    /// Live count after the most recent major GC — used for the 2× threshold.
    live_after_last_gc: usize,
    /// Allocations since the last major GC — drives `should_major_gc`.
    alloc_since_gc: usize,

    // ---- Write barrier ----
    /// Old-gen indices (raw, no OLD_BIT) whose fields contain at least one young ref.
    /// Populated by `write_field`; cleared by `minor_collect_finish`.
    remembered_set: HashSet<usize>,

    // ---- Tuning ----
    /// Minor GC fires when `young_top >= young_capacity`.
    young_capacity: usize,
    /// Survivals before promotion to old gen.
    promotion_age: u8,
    /// Total live objects across both generations (maintained incrementally).
    live_count: usize,
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heap {
    /// Create a new heap with default young-gen capacity and promotion age.
    #[must_use]
    pub fn new() -> Self {
        Self {
            young: Vec::new(),
            young_top: 0,
            to_space: Vec::new(),
            old: Vec::new(),
            old_free_list: Vec::new(),
            live_after_last_gc: 0,
            alloc_since_gc: 0,
            remembered_set: HashSet::new(),
            young_capacity: DEFAULT_YOUNG_CAPACITY,
            promotion_age: DEFAULT_PROMOTION_AGE,
            live_count: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Allocation
    // -----------------------------------------------------------------------

    /// Allocate a new object in young gen. Returns a young-gen ref (no OLD_BIT).
    pub fn allocate(&mut self, class_name: String, field_count: usize) -> u64 {
        self.alloc_since_gc += 1;
        self.live_count += 1;
        let idx = self.young_top as u64;
        let obj = HeapObject {
            class_name,
            fields: vec![Slot::Int(0); field_count],
            string_value: None,
            marked: false,
            age: 0,
            forward: None,
        };
        if (idx as usize) < self.young.len() {
            self.young[idx as usize] = Some(obj);
        } else {
            self.young.push(Some(obj));
        }
        self.young_top += 1;
        idx // OLD_BIT == 0 → young ref
    }

    /// Allocate a new String object in young gen.
    pub fn allocate_string(&mut self, value: String) -> u64 {
        self.alloc_since_gc += 1;
        self.live_count += 1;
        let idx = self.young_top as u64;
        let obj = HeapObject {
            class_name: "java/lang/String".to_string(),
            fields: Vec::new(),
            string_value: Some(value),
            marked: false,
            age: 0,
            forward: None,
        };
        if (idx as usize) < self.young.len() {
            self.young[idx as usize] = Some(obj);
        } else {
            self.young.push(Some(obj));
        }
        self.young_top += 1;
        idx
    }

    /// Promote an object to old gen. Returns an old-gen ref (with OLD_BIT).
    fn promote_to_old(&mut self, obj: HeapObject) -> u64 {
        self.live_count += 1; // will be balanced by caller zeroing young count
        let raw_idx = if let Some(free) = self.old_free_list.pop() {
            self.old[free as usize] = Some(obj);
            free
        } else {
            let i = self.old.len() as u64;
            self.old.push(Some(obj));
            i
        };
        raw_idx | OLD_BIT
    }

    // -----------------------------------------------------------------------
    // Access
    // -----------------------------------------------------------------------

    /// Get a reference to a heap object. Dispatches on OLD_BIT.
    ///
    /// # Errors
    /// Returns [`VmError::InvalidRef`] if the slot is out of bounds or swept.
    pub fn get(&self, r: u64) -> VmResult<&HeapObject> {
        if r & OLD_BIT != 0 {
            let idx = (r & !OLD_BIT) as usize;
            self.old
                .get(idx)
                .and_then(|s| s.as_ref())
                .ok_or(VmError::InvalidRef { address: r })
        } else {
            let idx = r as usize;
            self.young
                .get(idx)
                .and_then(|s| s.as_ref())
                .ok_or(VmError::InvalidRef { address: r })
        }
    }

    /// Get a mutable reference to a heap object. Dispatches on OLD_BIT.
    ///
    /// # Errors
    /// Returns [`VmError::InvalidRef`] if the slot is out of bounds or swept.
    pub fn get_mut(&mut self, r: u64) -> VmResult<&mut HeapObject> {
        if r & OLD_BIT != 0 {
            let idx = (r & !OLD_BIT) as usize;
            self.old
                .get_mut(idx)
                .and_then(|s| s.as_mut())
                .ok_or(VmError::InvalidRef { address: r })
        } else {
            let idx = r as usize;
            self.young
                .get_mut(idx)
                .and_then(|s| s.as_mut())
                .ok_or(VmError::InvalidRef { address: r })
        }
    }

    // -----------------------------------------------------------------------
    // Counters
    // -----------------------------------------------------------------------

    /// Total live objects across both generations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.live_count
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.live_count == 0
    }

    /// Live count in old gen only — used in tests.
    #[must_use]
    pub fn old_live_count(&self) -> usize {
        self.old.iter().filter(|s| s.is_some()).count()
    }

    /// Number of slots currently on the old-gen free list (test-only helper).
    #[cfg(test)]
    pub fn free_list_len(&self) -> usize {
        self.old_free_list.len()
    }

    // -----------------------------------------------------------------------
    // GC triggers (Task 3 fills these in fully; stubs here so it compiles)
    // -----------------------------------------------------------------------

    /// `true` when young gen is full (`young_top >= young_capacity`).
    #[must_use]
    pub fn should_minor_gc(&self) -> bool {
        self.young_top >= self.young_capacity
    }

    /// `true` when old gen has grown 2× since last major GC.
    /// Minimum threshold is 256 allocations.
    #[must_use]
    pub fn should_major_gc(&self) -> bool {
        let threshold = (self.live_after_last_gc * 2).max(256);
        self.alloc_since_gc >= threshold
    }

    /// Compatibility shim: returns `should_major_gc()`.
    /// Removed in Phase 25 but kept as dead-code until interpreter is wired.
    #[must_use]
    pub fn should_gc(&self) -> bool {
        self.should_major_gc()
    }

    // -----------------------------------------------------------------------
    // Write barrier (Task 4)
    // -----------------------------------------------------------------------

    /// Store `value` into `fields[field_idx]` of the object at `obj_ref`.
    ///
    /// If `obj_ref` is in old gen and `value` is a young-gen reference,
    /// `obj_ref`'s raw old-gen index is inserted into the remembered set so
    /// minor GC will treat it as an additional root.
    ///
    /// # Errors
    /// Returns [`VmError::InvalidRef`] if `obj_ref` is invalid.
    pub fn write_field(
        &mut self,
        obj_ref: u64,
        field_idx: usize,
        value: Slot,
    ) -> VmResult<()> {
        if obj_ref & OLD_BIT != 0 {
            if value.as_reference().is_some_and(|r| r & OLD_BIT == 0) {
                self.remembered_set.insert((obj_ref & !OLD_BIT) as usize);
            }
        }
        self.get_mut(obj_ref)?.fields[field_idx] = value;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Minor GC (Tasks 5 & 6)
    // -----------------------------------------------------------------------

    /// Phase 1 of minor GC: copy live young objects into to_space / old gen
    /// and install forwarding pointers.
    ///
    /// After this call, every live young object in `self.young` has
    /// `forward = Some(new_ref)` pointing to its new location.
    /// Dead young objects have `forward = None`.
    ///
    /// The interpreter must call `apply_forward` on all live slots after this
    /// and before calling `minor_collect_finish`.
    pub fn minor_collect_prepare(&mut self, roots: &[Slot]) {
        // Seed worklist: young refs from roots + young refs held by remembered-set old objects.
        let mut worklist: Vec<u64> = roots
            .iter()
            .filter_map(|s| s.as_reference().filter(|&r| r & OLD_BIT == 0))
            .collect();

        // Collect remembered-set roots before iterating (avoid borrow conflict).
        let rs_indices: Vec<usize> = self.remembered_set.iter().copied().collect();
        for old_idx in rs_indices {
            if let Some(Some(obj)) = self.old.get(old_idx) {
                let young_children: Vec<u64> = obj
                    .fields
                    .iter()
                    .filter_map(|s| s.as_reference().filter(|&r| r & OLD_BIT == 0))
                    .collect();
                worklist.extend(young_children);
            }
        }

        while let Some(r) = worklist.pop() {
            let idx = r as usize;
            // Skip if out-of-bounds, already dead (None), or already forwarded.
            let Some(Some(ref obj)) = self.young.get(idx) else {
                continue;
            };
            if obj.forward.is_some() {
                continue;
            }

            // Collect children and age before moving (can't hold ref while mutating).
            let children: Vec<u64> = obj
                .fields
                .iter()
                .filter_map(|s| s.as_reference().filter(|&r| r & OLD_BIT == 0))
                .collect();
            let age = obj.age;

            // Clone the object for relocation; clear GC bookkeeping in the copy.
            let mut new_obj = self.young[idx].clone().unwrap();
            new_obj.forward = None;
            new_obj.marked = false;

            let new_ref = if age >= self.promotion_age {
                // Promote to old gen.
                new_obj.age = 0;
                // promote_to_old bumps live_count; we'll recount in finish().
                self.promote_to_old(new_obj)
            } else {
                // Copy to to_space (stays in young gen with NEW index).
                new_obj.age += 1;
                let new_idx = self.to_space.len() as u64; // no OLD_BIT → young ref
                self.to_space.push(Some(new_obj));
                new_idx
            };

            // Install forwarding pointer in the old young-gen slot.
            self.young[idx].as_mut().unwrap().forward = Some(new_ref);

            // Enqueue children for copying.
            worklist.extend(children);
        }

        // Patch fields of old-gen objects in the remembered set that still point into young.
        let rs_indices2: Vec<usize> = self.remembered_set.iter().copied().collect();
        for old_idx in rs_indices2 {
            if let Some(Some(obj)) = self.old.get_mut(old_idx) {
                for slot in obj.fields.iter_mut() {
                    if let Slot::Reference(Some(r)) = slot {
                        if *r & OLD_BIT == 0 {
                            if let Some(Some(fwd_obj)) = self.young.get(*r as usize) {
                                if let Some(new_r) = fwd_obj.forward {
                                    *r = new_r;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Apply the forwarding pointer for a single slot in-place.
    ///
    /// Call this on every local/stack slot in every live frame after
    /// `minor_collect_prepare` and before `minor_collect_finish`.
    pub fn apply_forward(&self, slot: &mut Slot) {
        if let Slot::Reference(Some(r)) = slot {
            if *r & OLD_BIT == 0 {
                if let Some(Some(obj)) = self.young.get(*r as usize) {
                    if let Some(new_r) = obj.forward {
                        *r = new_r;
                    }
                }
            }
        }
    }

    /// Phase 3 of minor GC: flip young ← to_space and reset bookkeeping.
    ///
    /// Must be called after the interpreter has applied forwarding pointers
    /// to all live frame slots and static fields.
    pub fn minor_collect_finish(&mut self) {
        // to_space was built densely (no Nones); flip it into young.
        std::mem::swap(&mut self.young, &mut self.to_space);
        self.to_space.clear();
        // young_top = number of survivors in to_space (all Some, densely packed).
        self.young_top = self.young.len();
        // Clear the remembered set — it will be rebuilt lazily by write_field.
        self.remembered_set.clear();
        // Recount live objects (young survivors + old-gen occupancy).
        let young_live = self.young.iter().filter(|s| s.is_some()).count();
        let old_live = self.old.iter().filter(|s| s.is_some()).count();
        self.live_count = young_live + old_live;
    }

    // -----------------------------------------------------------------------
    // Major GC (Task 7)
    // -----------------------------------------------------------------------

    /// Old-gen mark-sweep. Operates only on `self.old`; treats OLD_BIT refs.
    ///
    /// The interpreter must run `minor_collect_prepare/finish` first so all
    /// live young objects are already in old gen before calling this.
    pub fn major_collect(&mut self, roots: &[Slot]) {
        // Mark phase: walk old-gen refs from roots.
        let mut worklist: Vec<u64> = roots
            .iter()
            .filter_map(|s| s.as_reference().filter(|&r| r & OLD_BIT != 0))
            .map(|r| r & !OLD_BIT)
            .collect();

        while let Some(raw_idx) = worklist.pop() {
            let Some(Some(obj)) = self.old.get_mut(raw_idx as usize) else {
                continue;
            };
            if obj.marked {
                continue;
            }
            obj.marked = true;
            let children: Vec<u64> = obj
                .fields
                .iter()
                .filter_map(|s| s.as_reference().filter(|&r| r & OLD_BIT != 0))
                .map(|r| r & !OLD_BIT)
                .collect();
            worklist.extend(children);
        }

        // Sweep phase.
        let mut live = 0usize;
        for (idx, slot) in self.old.iter_mut().enumerate() {
            match slot {
                Some(obj) if obj.marked => {
                    obj.marked = false;
                    live += 1;
                }
                Some(_) => {
                    *slot = None;
                    self.old_free_list.push(idx as u64);
                }
                None => {}
            }
        }
        self.live_after_last_gc = live;
        self.alloc_since_gc = 0;
        let young_live = self.young.iter().filter(|s| s.is_some()).count();
        self.live_count = young_live + live;
    }

    /// Full collection: minor GC (promotes all young survivors) then old-gen mark-sweep.
    ///
    /// The interpreter must apply forwarding pointers to all live slots between
    /// the two phases. For simpler callers that don't need split phases, use this
    /// only if all slot patching is handled externally.
    ///
    /// Note: The interpreter wiring in Task 8 uses `minor_collect_prepare` +
    /// `apply_forward` + `minor_collect_finish` + `major_collect` directly.
    /// This method is kept for compatibility and testing.
    pub fn collect(&mut self, roots: &[Slot]) {
        // Minor GC first — promotes survivors, installs forwarding pointers.
        self.minor_collect_prepare(roots);
        // Patch all root slots using forwarding pointers.
        // (In tests there is no interpreter frame to patch, just the root Vec itself.)
        let mut patched_roots: Vec<Slot> = roots.to_vec();
        for slot in patched_roots.iter_mut() {
            self.apply_forward(slot);
        }
        self.minor_collect_finish();
        // Now all live objects are in old gen; run mark-sweep on old gen only.
        self.major_collect(&patched_roots);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Phase 24 regression tests (adapted for new field layout) ----

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

    #[test]
    fn get_on_invalid_ref_returns_error() {
        let heap = Heap::new();
        assert!(heap.get(0).is_err());
        assert!(heap.get(999).is_err());
    }

    #[test]
    fn allocate_string_stores_value() {
        let mut heap = Heap::new();
        let r = heap.allocate_string("hello".to_string());
        let obj = heap.get(r).unwrap();
        assert_eq!(obj.class_name, "java/lang/String");
        assert_eq!(obj.string_value, Some("hello".to_string()));
        assert!(obj.fields.is_empty());
    }

    #[test]
    fn heap_object_marked_defaults_false() {
        let mut heap = Heap::new();
        let r = heap.allocate("Foo".to_string(), 0);
        assert!(!heap.get(r).unwrap().marked);
    }

    // ---- Task 1 tests ----

    #[test]
    fn heap_object_age_defaults_zero() {
        let mut heap = Heap::new();
        let r = heap.allocate("Foo".to_string(), 0);
        assert_eq!(heap.get(r).unwrap().age, 0);
    }

    #[test]
    fn heap_object_forward_defaults_none() {
        let mut heap = Heap::new();
        let r = heap.allocate("Bar".to_string(), 1);
        assert!(heap.get(r).unwrap().forward.is_none());
    }

    // ---- Task 2 tests: generation encoding ----

    #[test]
    fn young_ref_has_no_old_bit() {
        let mut heap = Heap::new();
        let r = heap.allocate("Young".to_string(), 0);
        assert_eq!(r & OLD_BIT, 0, "young ref must not have OLD_BIT set");
    }

    #[test]
    fn allocate_returns_sequential_young_indices() {
        let mut heap = Heap::new();
        let r0 = heap.allocate("A".to_string(), 0);
        let r1 = heap.allocate("B".to_string(), 0);
        assert_eq!(r0, 0);
        assert_eq!(r1, 1);
    }

    #[test]
    fn get_on_young_ref_works() {
        let mut heap = Heap::new();
        let r = heap.allocate("Point".to_string(), 2);
        let obj = heap.get(r).unwrap();
        assert_eq!(obj.class_name, "Point");
        assert_eq!(obj.fields.len(), 2);
    }

    #[test]
    fn old_gen_initially_empty() {
        let heap = Heap::new();
        assert_eq!(heap.old_live_count(), 0);
    }
}
```

### Step 2d: Run tests

```
cargo test -p duke-gc
```

Expected: all tests pass. Also run full workspace to catch any breakage from the field renames:

```
cargo test --workspace
```

### Step 2e: Commit

```
git add crates/duke-gc/src/lib.rs
git commit -m "feat(gc): split Heap into young/old generations with OLD_BIT reference encoding"
```

---

## Task 3: GC triggers — `should_minor_gc` + `should_major_gc`

**Files:**
- Modify: `crates/duke-gc/src/lib.rs` (these were stubbed in Task 2; verify tests here)

The trigger methods are already written in Task 2. This task writes their tests and verifies the existing `should_gc` shim compiles.

### Step 3a: Write the failing tests

Add to `#[cfg(test)]` in `crates/duke-gc/src/lib.rs`:

```rust
#[test]
fn should_minor_gc_fires_at_capacity() {
    let mut heap = Heap::new();
    heap.young_capacity = 4; // override default for fast test
    // 3 allocs — below capacity
    for _ in 0..3 {
        heap.allocate("X".to_string(), 0);
        assert!(!heap.should_minor_gc());
    }
    // 4th alloc pushes young_top to 4 == young_capacity → fires
    heap.allocate("X".to_string(), 0);
    assert!(heap.should_minor_gc());
}

#[test]
fn should_major_gc_triggers_at_2x_growth() {
    let mut heap = Heap::new();
    // First 255 allocs: below floor threshold of 256
    for i in 0..255 {
        heap.allocate(format!("C{i}"), 0);
        assert!(!heap.should_major_gc());
    }
    // 256th alloc: alloc_since_gc == 256 == threshold → triggers
    heap.allocate("C255".to_string(), 0);
    assert!(heap.should_major_gc());
}

#[test]
fn should_gc_compat_matches_major() {
    let mut heap = Heap::new();
    for _ in 0..256 {
        heap.allocate("X".to_string(), 0);
    }
    assert_eq!(heap.should_gc(), heap.should_major_gc());
}
```

Note: `young_capacity` must be `pub(crate)` or temporarily `pub` for the test to set it. Mark it `pub(crate)` in the struct definition.

### Step 3b: Run to verify they fail

```
cargo test -p duke-gc should_minor_gc_fires_at_capacity
```

Expected: compile error on `heap.young_capacity = 4` (field not accessible). Change `young_capacity` from private to `pub(crate)` in the struct.

### Step 3c: Implement

In the `Heap` struct, change:
```rust
    young_capacity: usize,
```
to:
```rust
    pub(crate) young_capacity: usize,
```

### Step 3d: Run tests

```
cargo test -p duke-gc should_minor_gc_fires_at_capacity should_major_gc_triggers_at_2x_growth should_gc_compat_matches_major
```

Expected: all three pass.

```
cargo test -p duke-gc
```

Expected: all pass.

### Step 3e: Commit

```
git add crates/duke-gc/src/lib.rs
git commit -m "feat(gc): add should_minor_gc / should_major_gc triggers; pub(crate) young_capacity"
```

---

## Task 4: `write_field` write barrier

**Files:**
- Modify: `crates/duke-gc/src/lib.rs`

`write_field` is already written in Task 2 as part of the Heap impl. This task adds its tests.

### Step 4a: Write the failing tests

Add to `#[cfg(test)]`:

```rust
#[test]
fn write_field_does_not_populate_remembered_set_for_young_to_young() {
    let mut heap = Heap::new();
    // Two young objects; storing young ref into young object should NOT add to rem set.
    let r0 = heap.allocate("A".to_string(), 1);
    let r1 = heap.allocate("B".to_string(), 0);
    heap.write_field(r0, 0, Slot::Reference(Some(r1))).unwrap();
    // r0 has no OLD_BIT → not old-gen → remembered_set unchanged
    assert!(heap.remembered_set.is_empty());
    assert_eq!(heap.get(r0).unwrap().fields[0], Slot::Reference(Some(r1)));
}

#[test]
fn write_field_populates_remembered_set_for_old_to_young() {
    let mut heap = Heap::new();
    // Create a young object and promote it manually to old gen.
    let young_obj = HeapObject {
        class_name: "Y".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    };
    let old_obj = HeapObject {
        class_name: "O".to_string(),
        fields: vec![Slot::Int(0)],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    };
    // Insert into old gen directly (raw index 0).
    heap.old.push(Some(old_obj));
    let old_ref = 0u64 | OLD_BIT;

    // Allocate a fresh young object via the public API.
    let young_ref = heap.allocate("Y".to_string(), 0);
    let _ = young_obj; // unused now; we used allocate instead

    // Store young ref into old-gen object's field — should fire write barrier.
    heap.write_field(old_ref, 0, Slot::Reference(Some(young_ref))).unwrap();
    assert!(
        heap.remembered_set.contains(&0),
        "old-gen index 0 must be in remembered_set"
    );
}

#[test]
fn write_field_does_not_add_old_to_old_ref_to_remembered_set() {
    let mut heap = Heap::new();
    // Two old-gen objects.
    heap.old.push(Some(HeapObject {
        class_name: "O1".to_string(),
        fields: vec![Slot::Int(0)],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    heap.old.push(Some(HeapObject {
        class_name: "O2".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    let o1 = 0u64 | OLD_BIT;
    let o2 = 1u64 | OLD_BIT;
    heap.write_field(o1, 0, Slot::Reference(Some(o2))).unwrap();
    // Old→old ref: no entry in remembered_set.
    assert!(heap.remembered_set.is_empty());
}
```

Note: the tests above access `heap.old` directly. Make `old` `pub(crate)` for tests:

```rust
    pub(crate) old: Vec<Option<HeapObject>>,
```

### Step 4b: Run to verify they fail

```
cargo test -p duke-gc write_field_does_not_populate_remembered_set_for_young_to_young
```

Expected: compile error on `heap.old` / `heap.remembered_set` access. Update visibility.

### Step 4c: Implement

In the `Heap` struct:
- Change `old: Vec<Option<HeapObject>>` to `pub(crate) old: Vec<Option<HeapObject>>`
- Change `remembered_set: HashSet<usize>` to `pub(crate) remembered_set: HashSet<usize>`

`write_field` was already implemented in Task 2. It is correct as-is.

### Step 4d: Run tests

```
cargo test -p duke-gc
```

Expected: all pass.

### Step 4e: Commit

```
git add crates/duke-gc/src/lib.rs
git commit -m "feat(gc): write_field write barrier — remembered set for old→young refs"
```

---

## Task 5: `Frame::slots_mut()` and `ClassRegistry::all_classes_mut()`

**Files:**
- Modify: `crates/duke-runtime/src/frame.rs`
- Modify: `crates/duke-interpreter/src/lib.rs`

The interpreter's `run_minor_gc` helper (Task 8) needs to mutate every live slot to apply forwarding pointers. `Frame::slots_mut()` returns a mutable iterator over locals + stack. `ClassRegistry::all_classes_mut()` returns a mutable iterator over all registered `ClassContext` values (for patching static fields).

### Step 5a: Write the failing tests

In `crates/duke-runtime/src/frame.rs` `#[cfg(test)]`:

```rust
#[test]
fn slots_mut_patches_all_slots() {
    use crate::Slot;
    let locals = vec![Slot::Int(1), Slot::Int(2)];
    let mut f = Frame::from_pool_bufs(locals, Vec::new(), 4);
    f.push(Slot::Int(99)).unwrap();
    // Mutate all slots via slots_mut
    for slot in f.slots_mut() {
        if let Slot::Int(v) = slot {
            *v += 10;
        }
    }
    assert_eq!(f.load_local(0).unwrap(), Slot::Int(11));
    assert_eq!(f.load_local(1).unwrap(), Slot::Int(12));
    assert_eq!(f.pop_int().unwrap(), 109);
}
```

In `crates/duke-interpreter/src/lib.rs` (in the `ClassRegistry` `#[cfg(test)]` block or a new test):

```rust
#[test]
fn all_classes_mut_iterates_static_fields() {
    let mut registry = ClassRegistry::new();
    let ctx = ClassContext {
        class_name: "Foo".to_string(),
        super_class: None,
        constant_pool: vec![],
        methods: vec![],
        fields: vec![],
        static_fields: vec![Slot::Int(0)],
        instance_field_count: 0,
        bootstrap_methods: vec![],
    };
    registry.register_context(ctx);
    for c in registry.all_classes_mut() {
        for s in c.static_fields.iter_mut() {
            if let Slot::Int(v) = s {
                *v = 42;
            }
        }
    }
    let val = registry.get("Foo").unwrap().static_fields[0].clone();
    assert_eq!(val, Slot::Int(42));
}
```

### Step 5b: Run to verify they fail

```
cargo test -p duke-runtime slots_mut_patches_all_slots
cargo test -p duke-interpreter all_classes_mut_iterates_static_fields
```

Expected: compile error — methods don't exist yet.

### Step 5c: Implement `Frame::slots_mut`

In `crates/duke-runtime/src/frame.rs`, add after `slots()`:

```rust
/// Yields mutable references to all slots in locals and operand stack.
/// Used by GC to apply forwarding pointers after minor GC copy phase.
pub fn slots_mut(&mut self) -> impl Iterator<Item = &mut Slot> + '_ {
    self.locals.iter_mut().chain(self.stack.iter_mut())
}
```

### Step 5d: Implement `ClassRegistry::all_classes_mut`

In `crates/duke-interpreter/src/lib.rs`, add after `all_classes()`:

```rust
/// Iterate all registered class contexts mutably — used by GC to patch static fields.
pub fn all_classes_mut(&mut self) -> impl Iterator<Item = &mut ClassContext> {
    self.classes.values_mut()
}
```

Also add `register_context` if it doesn't exist (needed for the test):

```rust
/// Register a pre-built ClassContext directly (used in tests).
pub fn register_context(&mut self, ctx: ClassContext) {
    self.classes.insert(ctx.class_name.clone(), ctx);
}
```

### Step 5e: Run tests

```
cargo test -p duke-runtime slots_mut_patches_all_slots
cargo test -p duke-interpreter all_classes_mut_iterates_static_fields
cargo test --workspace
```

Expected: all pass.

### Step 5f: Commit

```
git add crates/duke-runtime/src/frame.rs crates/duke-interpreter/src/lib.rs
git commit -m "feat(gc): add Frame::slots_mut + ClassRegistry::all_classes_mut for GC slot patching"
```

---

## Task 6: `minor_collect_prepare`, `apply_forward`, `minor_collect_finish`

**Files:**
- Modify: `crates/duke-gc/src/lib.rs`

These methods were implemented as part of Task 2's full file rewrite. This task writes the comprehensive unit tests that verify the copy collector works correctly in isolation.

### Step 6a: Write the failing tests

Add to `#[cfg(test)]` in `crates/duke-gc/src/lib.rs`:

```rust
// Helper: build a small heap with a controlled young_capacity so tests can
// reason precisely about timing without setting it externally.
fn test_heap_with_capacity(cap: usize) -> Heap {
    let mut h = Heap::new();
    h.young_capacity = cap;
    h
}

#[test]
fn minor_gc_copies_reachable_young_object() {
    let mut heap = test_heap_with_capacity(8);
    let r0 = heap.allocate("Keep".to_string(), 0);
    let _r1 = heap.allocate("Drop".to_string(), 0);
    let roots = vec![Slot::Reference(Some(r0))];
    heap.minor_collect_prepare(&roots);
    // r0 must have a forwarding pointer; _r1 must not.
    assert!(heap.young[r0 as usize].as_ref().unwrap().forward.is_some());
    assert!(heap.young[_r1 as usize].as_ref().unwrap().forward.is_none());
}

#[test]
fn minor_gc_forward_patches_root_slot() {
    let mut heap = test_heap_with_capacity(8);
    let r0 = heap.allocate("A".to_string(), 0);
    let roots = vec![Slot::Reference(Some(r0))];
    heap.minor_collect_prepare(&roots);
    let mut slot = Slot::Reference(Some(r0));
    heap.apply_forward(&mut slot);
    // After forwarding, slot must point to the new location in to_space.
    let new_r = heap.young[r0 as usize].as_ref().unwrap().forward.unwrap();
    assert_eq!(slot, Slot::Reference(Some(new_r)));
}

#[test]
fn minor_gc_finish_swaps_to_space_into_young() {
    let mut heap = test_heap_with_capacity(8);
    let r0 = heap.allocate("A".to_string(), 0);
    let r1 = heap.allocate("B".to_string(), 0);
    // Only r0 is a root → r1 is dead.
    let roots = vec![Slot::Reference(Some(r0))];
    heap.minor_collect_prepare(&roots);
    heap.minor_collect_finish();
    // After finish: young has 1 live survivor; to_space is empty.
    assert_eq!(heap.young.iter().filter(|s| s.is_some()).count(), 1);
    assert!(heap.to_space.is_empty());
    assert!(heap.remembered_set.is_empty());
}

#[test]
fn minor_gc_increments_age_on_survival() {
    let mut heap = test_heap_with_capacity(8);
    let r = heap.allocate("Survivor".to_string(), 0);
    let roots = vec![Slot::Reference(Some(r))];
    heap.minor_collect_prepare(&roots);
    // Locate the copy in to_space (new_ref from forward pointer).
    let new_r = heap.young[r as usize].as_ref().unwrap().forward.unwrap();
    heap.minor_collect_finish();
    // After finish, young is former to_space.
    // new_r has no OLD_BIT → young index.
    let survivor = heap.young[new_r as usize].as_ref().unwrap();
    assert_eq!(survivor.age, 1);
}

#[test]
fn minor_gc_promotes_at_promotion_age() {
    let mut heap = test_heap_with_capacity(64);
    heap.promotion_age = 2; // promote after 2 survivals

    let mut current_r = heap.allocate("P".to_string(), 0);

    // Survive twice → promote on the second minor GC.
    for round in 0..2u8 {
        let roots = vec![Slot::Reference(Some(current_r))];
        heap.minor_collect_prepare(&roots);
        let new_r = heap.young[current_r as usize]
            .as_ref()
            .unwrap()
            .forward
            .unwrap();
        heap.minor_collect_finish();

        if round == 0 {
            // Still young (age == 1).
            assert_eq!(new_r & OLD_BIT, 0, "should still be young after 1 survival");
            current_r = new_r;
        } else {
            // Promoted to old gen (OLD_BIT set).
            assert_ne!(new_r & OLD_BIT, 0, "should be in old gen after {} survivals", round + 1);
            // Also verify get() works on the old-gen ref.
            let obj = heap.get(new_r).unwrap();
            assert_eq!(obj.class_name, "P");
        }
    }
}

#[test]
fn remembered_set_root_survives_minor_gc() {
    let mut heap = test_heap_with_capacity(8);
    // Build: old-gen object with a field pointing to a young object.
    let old_obj = HeapObject {
        class_name: "Old".to_string(),
        fields: vec![Slot::Int(0)], // will be overwritten below
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    };
    heap.old.push(Some(old_obj));
    let old_ref = 0u64 | OLD_BIT;

    let young_ref = heap.allocate("Young".to_string(), 0);
    // Wire old→young via write_field (populates remembered_set).
    heap.write_field(old_ref, 0, Slot::Reference(Some(young_ref))).unwrap();

    // No stack roots — young object reachable only through remembered set.
    heap.minor_collect_prepare(&[]);
    assert!(
        heap.young[young_ref as usize].as_ref().unwrap().forward.is_some(),
        "young object reachable via rem-set must be forwarded"
    );
    heap.minor_collect_finish();
    // Verify old-gen field was patched to the new young location.
    let new_field = heap.old[0].as_ref().unwrap().fields[0].clone();
    match new_field {
        Slot::Reference(Some(r)) => {
            // The new ref is either young (age < promotion_age) or old (promoted).
            // Either way, get() must succeed.
            heap.get(r).expect("patched old→young field must be valid");
        }
        other => panic!("expected Reference, got {other:?}"),
    }
}

#[test]
fn apply_forward_is_no_op_on_non_references() {
    let heap = Heap::new();
    let mut slot = Slot::Int(42);
    heap.apply_forward(&mut slot);
    assert_eq!(slot, Slot::Int(42));
}

#[test]
fn apply_forward_is_no_op_on_null_ref() {
    let heap = Heap::new();
    let mut slot = Slot::Reference(None);
    heap.apply_forward(&mut slot);
    assert_eq!(slot, Slot::Reference(None));
}

#[test]
fn apply_forward_is_no_op_on_old_gen_ref() {
    let heap = Heap::new();
    let old_ref = 0u64 | OLD_BIT;
    let mut slot = Slot::Reference(Some(old_ref));
    heap.apply_forward(&mut slot);
    // No forwarding pointer in old gen → slot unchanged.
    assert_eq!(slot, Slot::Reference(Some(old_ref)));
}
```

Also make `promotion_age` accessible in tests:

```rust
    pub(crate) promotion_age: u8,
```

### Step 6b: Run to verify they fail

```
cargo test -p duke-gc minor_gc_copies_reachable_young_object
```

Expected: compile error on `heap.promotion_age` access. Fix visibility.

### Step 6c: Implement

Change `promotion_age: u8` to `pub(crate) promotion_age: u8` in the `Heap` struct.

The three methods (`minor_collect_prepare`, `apply_forward`, `minor_collect_finish`) were already implemented in Task 2's file rewrite. No new code needed.

### Step 6d: Run tests

```
cargo test -p duke-gc
```

Expected: all tests pass, including all nine new minor-GC tests.

### Step 6e: Commit

```
git add crates/duke-gc/src/lib.rs
git commit -m "test(gc): comprehensive minor GC unit tests — copy, forward, promote, remembered-set"
```

---

## Task 7: `major_collect` — old-gen mark-sweep

**Files:**
- Modify: `crates/duke-gc/src/lib.rs`

`major_collect` was implemented in Task 2. This task writes the unit tests that prove it works correctly operating on old-gen objects only.

### Step 7a: Write the failing tests

Add to `#[cfg(test)]`:

```rust
#[test]
fn old_ref_has_old_bit() {
    let mut heap = Heap::new();
    // Manually push into old gen to get an old-gen ref.
    let obj = HeapObject {
        class_name: "OldObj".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    };
    heap.old.push(Some(obj));
    let old_ref = 0u64 | OLD_BIT;
    assert_ne!(old_ref & OLD_BIT, 0, "old ref must have OLD_BIT set");
    assert_eq!(heap.get(old_ref).unwrap().class_name, "OldObj");
}

#[test]
fn major_collect_reclaims_unreachable_old_objects() {
    let mut heap = Heap::new();
    // Manually populate old gen with two objects; keep one.
    heap.old.push(Some(HeapObject {
        class_name: "Keep".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    heap.old.push(Some(HeapObject {
        class_name: "Drop".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    let keep_ref = 0u64 | OLD_BIT;
    let drop_ref = 1u64 | OLD_BIT;
    let roots = vec![Slot::Reference(Some(keep_ref))];
    heap.major_collect(&roots);
    assert!(heap.get(keep_ref).is_ok(), "reachable old-gen object must survive");
    assert!(heap.get(drop_ref).is_err(), "unreachable old-gen object must be swept");
    assert_eq!(heap.old_free_list.len(), 1);
}

#[test]
fn major_collect_preserves_reachable_chain_in_old_gen() {
    let mut heap = Heap::new();
    heap.old.push(Some(HeapObject {
        class_name: "C".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    heap.old.push(Some(HeapObject {
        class_name: "B".to_string(),
        fields: vec![Slot::Reference(Some(0u64 | OLD_BIT))],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    heap.old.push(Some(HeapObject {
        class_name: "A".to_string(),
        fields: vec![Slot::Reference(Some(1u64 | OLD_BIT))],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    let a_ref = 2u64 | OLD_BIT;
    let roots = vec![Slot::Reference(Some(a_ref))];
    heap.major_collect(&roots);
    assert_eq!(heap.old_live_count(), 3);
    assert_eq!(heap.old_free_list.len(), 0);
}

#[test]
fn major_collect_reuses_freed_slot() {
    let mut heap = Heap::new();
    heap.old.push(Some(HeapObject {
        class_name: "Keep".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    heap.old.push(Some(HeapObject {
        class_name: "Drop".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    let keep_ref = 0u64 | OLD_BIT;
    heap.major_collect(&[Slot::Reference(Some(keep_ref))]);
    // old_free_list has raw index 1 (no OLD_BIT).
    assert!(heap.old_free_list.contains(&1u64));
}

#[test]
fn collect_compat_shim_collects_full_heap() {
    // Uses the public collect() method which chains minor + major.
    let mut heap = test_heap_with_capacity(512);
    let keep = heap.allocate("Keep".to_string(), 0);
    let _drop1 = heap.allocate("Drop1".to_string(), 0);
    let _drop2 = heap.allocate("Drop2".to_string(), 0);
    let roots = vec![Slot::Reference(Some(keep))];
    heap.collect(&roots);
    // After full collect: 1 live object (promoted to old gen).
    assert_eq!(heap.old_live_count(), 1);
    assert_eq!(heap.len(), 1);
}
```

### Step 7b: Run to verify they fail

```
cargo test -p duke-gc major_collect_reclaims_unreachable_old_objects
```

Expected: the test compiles but may fail if `major_collect` is wrong, OR passes if the Task 2 implementation is correct. Fix any failures found.

### Step 7c: Implement (if corrections needed)

The `major_collect` from Task 2 is the canonical implementation. If the test reveals bugs (e.g., `live_count` miscounting), fix `major_collect` in `crates/duke-gc/src/lib.rs`:

One subtle issue: when `collect()` calls `minor_collect_prepare` + `minor_collect_finish` before `major_collect`, the young survivors were promoted into old gen by `promote_to_old`, which bumps `live_count`. Then `minor_collect_finish` recounts `live_count` from scratch. Then `major_collect` recounts again. This is correct — both recounts are O(n) scans and the final value after `major_collect` is authoritative.

A second issue: the `collect()` compat shim builds `patched_roots` as a fresh Vec. After `minor_collect_finish`, all young objects are either in old gen (promoted) or in former to_space. The patched roots passed to `major_collect` must use OLD_BIT refs for promoted objects. The `apply_forward` call inside `collect()` handles this correctly because forwarding pointers install old-gen refs (`new_ref = raw_idx | OLD_BIT`) for promoted objects.

### Step 7d: Run tests

```
cargo test -p duke-gc
```

Expected: all tests pass including all major GC tests.

### Step 7e: Commit

```
git add crates/duke-gc/src/lib.rs
git commit -m "test(gc): major_collect unit tests — old-gen mark-sweep with OLD_BIT refs"
```

---

## Task 8: Interpreter wiring — `run_minor_gc`, GC trigger sites, `write_field` sites

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

This task wires the new GC into the interpreter. Three changes:
1. Add `run_minor_gc` helper function.
2. Replace 5 `should_gc` / `heap.collect` pairs with the new two-level trigger.
3. Replace 8 bytecode store sites (`putfield`, `aastore`, `iastore`, `lastore`, `fastore`, `dastore`, `bastore`, `castore`, `sastore`) with `heap.write_field(...)`.

Note: `putstatic` stores into `registry.static_fields`, not a heap object field, so it does NOT need `write_field`. `Putstatic` is excluded from the write barrier because static fields are roots themselves — the GC already scans all static fields in `gather_roots`. Only stores into heap-allocated object fields need the barrier.

### Step 8a: Write the failing test

This is an integration test that verifies the interpreter correctly routes through the new GC API. Add to the test block in `crates/duke-interpreter/src/lib.rs`:

```rust
#[test]
fn gc_minor_triggered_and_refs_stay_valid() {
    // Allocate many objects to trigger minor GC and verify references
    // in frames remain valid after forwarding.
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    // Set a tiny young_capacity so minor GC fires frequently.
    heap.young_capacity = 8;
    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    // Use an existing fixture that does allocation (e.g., SimpleObjects).
    // If not available, use a fixture that constructs strings in a loop.
    // For now, verify that execute_class with StringConcat (which allocates)
    // completes without panic.
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "HelloWorld",
        "main",
        "([Ljava/lang/String;)V",
        vec![Slot::Reference(None)],
    );
    // Should succeed (no panic, no InvalidRef).
    assert!(result.is_ok(), "GC must not corrupt refs: {result:?}");
}
```

### Step 8b: Run to verify it fails (or that old GC path still compiles)

```
cargo test -p duke-interpreter gc_minor_triggered_and_refs_stay_valid
```

Expected: test fails because the old `should_gc` path doesn't use `run_minor_gc`. (Or it passes because `should_gc` delegates to `should_major_gc` and the test heap never hits 256 allocations. Either way the wiring test in Step 8d will verify the new path.)

### Step 8c: Implement

**8c-1: Add `run_minor_gc` helper** — add after `gather_roots` in `crates/duke-interpreter/src/lib.rs`:

```rust
/// Run a minor GC: gather roots, copy live young objects, apply forwarding
/// pointers to all live interpreter slots, then flip young←to_space.
fn run_minor_gc(
    heap: &mut duke_gc::Heap,
    frame: &mut duke_runtime::Frame,
    call_stack: &mut Vec<CallFrame>,
    registry: &mut ClassRegistry,
) {
    let roots = gather_roots(frame, call_stack, registry);
    heap.minor_collect_prepare(&roots);
    // Patch current frame.
    for slot in frame.slots_mut() {
        heap.apply_forward(slot);
    }
    // Patch all suspended caller frames.
    for cf in call_stack.iter_mut() {
        for slot in cf.frame.slots_mut() {
            heap.apply_forward(slot);
        }
    }
    // Patch static fields in all loaded classes.
    for ctx in registry.all_classes_mut() {
        for slot in ctx.static_fields.iter_mut() {
            heap.apply_forward(slot);
        }
    }
    heap.minor_collect_finish();
}
```

**8c-2: Replace the 5 GC trigger sites** — find each of the 5 `if heap.should_gc()` blocks in `execute_class` and replace with:

```rust
if heap.should_major_gc() {
    run_minor_gc(heap, &mut frame, &mut call_stack, registry);
    let roots = gather_roots(&frame, &call_stack, registry);
    heap.major_collect(&roots);
} else if heap.should_minor_gc() {
    run_minor_gc(heap, &mut frame, &mut call_stack, registry);
}
```

The 5 locations (by instruction) are:
1. After `Instruction::New` — object allocation (`r = heap.allocate(target_class, field_count)`)
2. After `Instruction::Newarray` — primitive array allocation
3. After `Instruction::Anewarray` — reference array allocation
4. After `Instruction::Invokedynamic` — lambda proxy allocation
5. After `Instruction::Multianewarray` — multi-dimensional array allocation

**8c-3: Replace the 8 array/field store sites** — the bytecode store instructions that write into heap objects must use `write_field`. These are in the main `execute_class` dispatch loop (NOT in native handlers — native handler allocations are fresh young objects not yet reachable from old gen):

For `Instruction::Putfield` (line ~5928):
```rust
// Before:
heap.get_mut(r)?.fields[fidx] = val;
// After:
heap.write_field(r, fidx, val)?;
```

For `Instruction::Iastore` (line ~6368):
```rust
// Before (after bounds check):
let obj = heap.get_mut(r)?;
// ...bounds check...
obj.fields[idx_val as usize] = Slot::Int(val);
// After:
{
    let len = heap.get(r)?.fields.len();
    if idx_val < 0 || idx_val as usize >= len {
        return Err(VmError::ArrayIndexOutOfBounds { index: idx_val, length: len });
    }
}
heap.write_field(r, idx_val as usize, Slot::Int(val))?;
```

Apply the same pattern to `Lastore`, `Fastore`, `Dastore`, `Aastore`, `Bastore`, `Castore`, `Sastore`.

The full replacement for each of the 8 array store instructions follows this template (shown for `Aastore` since it stores References which are the ones that actually trigger the write barrier):

```rust
Instruction::Aastore => {
    let val = frame.pop()?;
    let idx_val = frame.pop_int()?;
    let r = frame.pop_ref()?;
    {
        let len = heap.get(r)?.fields.len();
        if idx_val < 0 || idx_val as usize >= len {
            return Err(VmError::ArrayIndexOutOfBounds {
                index: idx_val,
                length: len,
            });
        }
    }
    heap.write_field(r, idx_val as usize, val)?;
}
```

For primitive stores (Int/Long/Float/Double) where the value can never be a Reference, the write barrier will not fire (the `as_reference()` check in `write_field` returns `None`). The code is still correct — slightly more work at runtime but ensures all field mutations go through one path.

For `Bastore`, `Castore`, `Sastore` (which truncate the value before storing), extract the final `Slot::Int(val)` after truncation and pass that to `write_field`:

```rust
Instruction::Bastore => {
    let val = frame.pop_int()? as i8 as i32;
    let idx_val = frame.pop_int()?;
    let r = frame.pop_ref()?;
    {
        let len = heap.get(r)?.fields.len();
        if idx_val < 0 || idx_val as usize >= len {
            return Err(VmError::ArrayIndexOutOfBounds {
                index: idx_val,
                length: len,
            });
        }
    }
    heap.write_field(r, idx_val as usize, Slot::Int(val))?;
}
```

### Step 8d: Run tests

```
cargo test --workspace
```

Expected: all 327+ tests pass. Pay attention to any test that uses `heap.get_mut(r)?.fields[i] = ...` in the test body — those are in test setup code and do not need `write_field` (test objects are created fresh each test, no cross-generational pointers possible).

### Step 8e: Commit

```
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(gc): wire generational GC into interpreter — run_minor_gc, trigger sites, write_field"
```

---

## Task 9: `GcGenerationalStressTest` fixture + integration test

**Files:**
- Create: `tests/fixtures/GcGenerationalStressTest.java`
- Compile: `tests/fixtures/GcGenerationalStressTest.class`
- Modify: `crates/duke-interpreter/src/lib.rs` (add integration test)

This fixture exercises the full generational GC path: many short-lived objects (collected by minor GC) and a fixed number of long-lived objects (promoted to old gen).

### Step 9a: Write the Java fixture

Create `tests/fixtures/GcGenerationalStressTest.java`:

```java
public class GcGenerationalStressTest {
    public static void main(String[] args) {
        // 10 long-lived objects — must survive all minor GCs.
        String[] longLived = new String[10];
        for (int i = 0; i < 10; i++) {
            longLived[i] = "Survivor-" + i;
        }

        // 5000 short-lived string allocations — trigger multiple minor GCs.
        int sum = 0;
        for (int i = 0; i < 5000; i++) {
            String s = "tmp-" + i;
            sum += s.length();
        }

        // Verify long-lived objects are intact.
        for (int i = 0; i < 10; i++) {
            System.out.println(longLived[i]);
        }

        // Print sum to prevent dead-code elimination.
        System.out.println(sum);
    }
}
```

### Step 9b: Compile the fixture

```
javac --release 21 tests/fixtures/GcGenerationalStressTest.java -d tests/fixtures/
```

Verify `tests/fixtures/GcGenerationalStressTest.class` exists.

### Step 9c: Write the integration test (failing first)

Add to `#[cfg(test)]` in `crates/duke-interpreter/src/lib.rs`:

```rust
#[test]
fn gc_generational_stress_test() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // Small young_capacity to force frequent minor GCs.
    heap.young_capacity = 32;

    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
    heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(None);

    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "GcGenerationalStressTest",
        "main",
        "([Ljava/lang/String;)V",
        vec![Slot::Reference(Some(arr_ref))],
    );
    assert!(result.is_ok(), "generational GC stress test failed: {result:?}");
    let output = String::from_utf8(out).unwrap();
    // Verify long-lived objects survived all minor GCs.
    for i in 0..10 {
        assert!(
            output.contains(&format!("Survivor-{i}")),
            "long-lived object Survivor-{i} missing from output:\n{output}"
        );
    }
    // sum = 5000 * avg("tmp-N".length()) — "tmp-0" to "tmp-4999":
    // "tmp-0" to "tmp-9": 5 chars each → 10 * 5 = 50
    // "tmp-10" to "tmp-99": 6 chars each → 90 * 6 = 540
    // "tmp-100" to "tmp-999": 7 chars each → 900 * 7 = 6300
    // "tmp-1000" to "tmp-4999": 8 chars each → 4000 * 8 = 32000
    // total = 50 + 540 + 6300 + 32000 = 38890
    assert!(
        output.contains("38890"),
        "expected sum 38890 in output:\n{output}"
    );
}
```

### Step 9d: Run to verify the test fails before wiring (should now pass since Task 8 is done)

```
cargo test -p duke-interpreter gc_generational_stress_test
```

If `GcGenerationalStressTest.class` was compiled successfully this should pass. If it fails with a fixture-related error (class not found), verify the `.class` file path.

### Step 9e: Verify the sum

The expected sum `38890` is computed as:
- "tmp-0" through "tmp-9": length 5, count 10 → 50
- "tmp-10" through "tmp-99": length 6, count 90 → 540
- "tmp-100" through "tmp-999": length 7, count 900 → 6300
- "tmp-1000" through "tmp-4999": length 8, count 4000 → 32000
- Total: 38890

If the Duke string concatenation adds different formatting (e.g., invokedynamic produces "tmp-" + i differently), adjust the expected value based on what the test actually outputs. Run the fixture with HotSpot first to confirm:

```
java -cp tests/fixtures GcGenerationalStressTest
```

The last line of output should be `38890`.

### Step 9f: Commit

```
git add tests/fixtures/GcGenerationalStressTest.java tests/fixtures/GcGenerationalStressTest.class crates/duke-interpreter/src/lib.rs
git commit -m "test(gc): GcGenerationalStressTest fixture — 5000 short-lived + 10 long-lived objects"
```

---

## Task 10: Regression pass + memory update

**Files:**
- Modify: `C:\Users\markm\.claude\projects\C--Users-markm-duke\memory\MEMORY.md`
- Modify: `C:\Users\markm\.claude\projects\C--Users-markm-duke\memory\phase-history.md` (if it exists)

### Step 10a: Full regression pass

Run the complete test suite:

```
cargo test --workspace
```

Expected: all tests pass. Count the total.

Also run clippy to catch regressions:

```
cargo clippy --workspace -- -D warnings
```

Fix any new warnings introduced by Phase 25 (e.g., unused `should_gc` now that the trigger sites use `should_major_gc`/`should_minor_gc`). Mark `should_gc` with `#[deprecated]` or remove it if no external callers remain:

```rust
/// Compatibility shim. Use `should_minor_gc` / `should_major_gc` instead.
#[deprecated(since = "0.25.0", note = "use should_minor_gc / should_major_gc")]
#[must_use]
pub fn should_gc(&self) -> bool {
    self.should_major_gc()
}
```

And suppress the deprecation warning in the test:

```rust
#[test]
#[allow(deprecated)]
fn should_gc_compat_matches_major() { ... }
```

Run fmt:

```
cargo fmt --all
```

### Step 10b: Update MEMORY.md

Update the project memory at `C:\Users\markm\.claude\projects\C--Users-markm-duke\memory\MEMORY.md`:

- Change status line: `## Status: Phase 25 Complete (N tests passing)` (fill in actual count)
- Add Phase 25 section under "What's Built"
- Update test coverage numbers
- Add Phase 25 Additions section with the key changes

Key facts to record:

**Phase 25 Additions:**
- `HeapObject`: added `age: u8` and `forward: Option<u64>` for generational GC
- `Heap`: split into young gen (`young: Vec<Option<HeapObject>>`, bump-pointer) and old gen (`old: Vec<Option<HeapObject>>`, renamed from `objects`)
- Reference encoding: `r & OLD_BIT != 0` (where `OLD_BIT = 1 << 63`) = old-gen ref; otherwise young-gen ref
- `Heap::allocate` / `allocate_string`: now bump-allocate into young gen, returning young refs
- `Heap::promote_to_old`: private helper; appends to old gen or reuses `old_free_list`
- `Heap::write_field`: write barrier; adds old-gen index to `remembered_set` when old-gen object receives young ref
- `Heap::minor_collect_prepare(roots)`: copy phase — traces from roots + remembered set, copies survivors to `to_space`, installs forwarding pointers
- `Heap::apply_forward(slot)`: patches one slot to its new location after copy phase
- `Heap::minor_collect_finish()`: flips young ← to_space, resets `remembered_set` and `young_top`
- `Heap::major_collect(roots)`: old-gen mark-sweep (OLD_BIT-filtered roots only)
- `Heap::should_minor_gc()`: true when `young_top >= young_capacity` (default 512)
- `Heap::should_major_gc()`: true when `alloc_since_gc >= max(256, live_after_last_gc * 2)`
- `Heap::collect(roots)`: compat shim — minor + major
- `Frame::slots_mut()`: mutable iterator over locals + stack for GC slot patching
- `ClassRegistry::all_classes_mut()`: mutable iterator over all classes for static field patching
- Interpreter: `run_minor_gc` helper; 5 GC trigger sites updated; 8 field/array store bytecodes use `write_field`
- New fixture: `GcGenerationalStressTest.java` — 5000 short-lived + 10 long-lived, sum=38890
- Constants: `DEFAULT_PROMOTION_AGE = 4`, `DEFAULT_YOUNG_CAPACITY = 512`

### Step 10c: Final commit

```
git add .claude/projects/C--Users-markm-duke/memory/MEMORY.md
git commit -m "docs: update MEMORY.md for Phase 25 generational GC completion"
```

---

## Summary

| Task | Scope | Key Artifact |
|------|-------|--------------|
| 1 | `duke-gc`: add `age`/`forward` to `HeapObject` | 2 failing tests → compile fix |
| 2 | `duke-gc`: full generational Heap rewrite | `young`/`old` split, `OLD_BIT` encoding, all alloc/get/get_mut updated |
| 3 | `duke-gc`: `should_minor_gc` / `should_major_gc` triggers | `pub(crate) young_capacity` |
| 4 | `duke-gc`: `write_field` write barrier | `remembered_set` population |
| 5 | `duke-runtime`/`duke-interpreter`: `slots_mut` / `all_classes_mut` | Mutable GC slot patching |
| 6 | `duke-gc`: copy collector unit tests | 9 tests for prepare/forward/finish |
| 7 | `duke-gc`: major GC unit tests | 5 tests for old-gen mark-sweep |
| 8 | `duke-interpreter`: wiring | `run_minor_gc`, 5 trigger sites, 8 `write_field` sites |
| 9 | Fixture + integration test | `GcGenerationalStressTest.java` |
| 10 | Regression + MEMORY.md | `cargo test --workspace` all green |

**Invariants preserved throughout:**
- `Slot::as_reference()` is unchanged — returns `Option<u64>`, callers use `r & OLD_BIT` for generation check
- `Heap::get(r)` and `Heap::get_mut(r)` dispatch on `OLD_BIT` internally — callers are generation-agnostic
- All young-gen allocations (including string concat, lambda proxies, array init) land in young gen automatically
- Write barrier fires only at bytecode-level store instructions (`putfield`, `aastore`, etc.) — native handler internal allocations are exempt because freshly-allocated objects can't be referenced from old gen yet
- `gather_roots` is unchanged — it collects all `Slot::Reference(Some(_))` values from frames and static fields regardless of generation
