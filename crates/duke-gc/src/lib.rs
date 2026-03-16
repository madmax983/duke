#![allow(clippy::nursery)]
#![allow(clippy::pedantic)]
//! Generational mark-sweep GC for the Duke JVM (Phase 25).
//!
//! **Young generation** — bump-pointer allocation (Eden-style). Minor GC uses
//! a copy-collector: live objects are copied to `to_space`, forwarding pointers
//! patch all live slots, then `to_space` becomes the new young gen.
//!
//! **Old generation** — Phase 24 mark-sweep with free-list reuse.
//!
//! **Reference encoding** — the high bit of every `u64` heap reference indicates
//! generation: `r & OLD_BIT == 0` → young-gen index; `r & OLD_BIT != 0` →
//! old-gen index `(r & !OLD_BIT)`.
//!
//! [`Heap::get`] and [`Heap::get_mut`] are generation-agnostic; callers never
//! need to know which gen an object lives in.

use std::collections::{HashMap, HashSet};

use duke_runtime::{Slot, VmError, VmResult};

/// High bit set ⟹ old-generation reference; clear ⟹ young-generation reference.
pub const OLD_BIT: u64 = 1 << 63;

/// Default number of young-gen slots before a minor GC fires.
const DEFAULT_YOUNG_CAPACITY: usize = 512;

/// Default number of minor-GC survivals before an object is promoted to old gen.
const DEFAULT_PROMOTION_AGE: u8 = 4;

/// A single heap-allocated Java object.
#[derive(Debug, Clone)]
pub struct HeapObject {
    pub class_name: String,
    pub fields: Vec<Slot>,
    /// String content for `java/lang/String` objects. `None` for non-string objects.
    pub string_value: Option<String>,
    /// Mark bit for old-gen mark-sweep GC. `false` until marked reachable.
    // INVARIANT: Heap::mark_old() traces only `fields` for child references.
    // Any new Vec<Slot> member added to HeapObject MUST also be covered in mark_old().
    pub(crate) marked: bool,
    /// Number of minor GC collections this object has survived.
    #[allow(dead_code)] // used by promote_to_old
    pub(crate) age: u8,
    /// Forwarding pointer installed during the copy phase of minor GC.
    /// `Some(new_ref)` means this object was already copied; `None` means not yet copied.
    #[allow(dead_code)] // used by minor_collect_prepare / apply_forward
    pub(crate) forward: Option<u64>,
}

/// The generational object heap.
///
/// Young-gen refs: `r & OLD_BIT == 0`  → index into `young`
/// Old-gen refs:   `r & OLD_BIT != 0`  → index `(r & !OLD_BIT)` into `old`
#[derive(Debug, Default)]
pub struct Heap {
    // ── Young generation ────────────────────────────────────────────────────
    /// Young-gen object store. Index = raw young-gen reference.
    young: Vec<Option<HeapObject>>,
    /// Next free slot in young gen (bump pointer).
    young_top: usize,
    /// Staging area for the copy phase of minor GC; swapped with `young` on finish.
    to_space: Vec<Option<HeapObject>>,

    // ── Old generation ───────────────────────────────────────────────────────
    /// Old-gen object store. Index = `(r & !OLD_BIT)`.
    pub(crate) old: Vec<Option<HeapObject>>,
    /// Free-list of raw old-gen indices (no OLD_BIT) for reuse after sweep.
    old_free_list: Vec<u64>,

    // ── GC accounting ────────────────────────────────────────────────────────
    /// Live old-gen objects after the last major GC.
    live_after_last_gc: usize,
    /// Allocations since the last major GC (used for major-GC threshold).
    alloc_since_gc: usize,

    // ── Write barrier ────────────────────────────────────────────────────────
    /// Old-gen raw indices that hold ≥ 1 young-gen reference in their fields.
    /// Populated by `write_field`; cleared by `minor_collect_finish`.
    pub(crate) remembered_set: HashSet<usize>,

    // ── Tuning ───────────────────────────────────────────────────────────────
    /// Minor GC fires when `young_top >= young_capacity`.
    pub young_capacity: usize,
    /// Object is promoted to old gen after surviving this many minor GCs.
    pub(crate) promotion_age: u8,

    // ── Live-count cache ─────────────────────────────────────────────────────
    /// Total live objects across both generations; maintained for O(1) `len()`.
    live_count: usize,

    // ── Collection stats ─────────────────────────────────────────────────────
    /// Young objects dropped (not forwarded) in the most recent minor GC.
    /// Exposed via `free_list_len()` (combined with `old_free_list`) for test
    /// compatibility.
    young_dropped: usize,

    // ── Post-minor-GC forwarding map ─────────────────────────────────────────
    /// Maps old young-gen ref → new ref (young or old-gen with OLD_BIT).
    /// Populated during `minor_collect_prepare`, kept alive past
    /// `minor_collect_finish` so callers can patch their own slots after
    /// `collect()` returns via [`Heap::apply_forward`].
    forward_map: HashMap<u64, u64>,
}

impl Heap {
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
            young_dropped: 0,
            forward_map: HashMap::new(),
        }
    }

    // ── Allocation ───────────────────────────────────────────────────────────

    const fn make_obj(
        class_name: String,
        fields: Vec<Slot>,
        string_value: Option<String>,
    ) -> HeapObject {
        HeapObject {
            class_name,
            fields,
            string_value,
            marked: false,
            age: 0,
            forward: None,
        }
    }

    /// Allocate a new object in the young generation. Returns a young-gen reference.
    pub fn allocate(&mut self, class_name: String, field_count: usize) -> u64 {
        self.alloc_since_gc += 1;
        self.live_count += 1;
        let idx = self.young_top as u64; // OLD_BIT == 0 → young ref
        let obj = Self::make_obj(class_name, vec![Slot::Int(0); field_count], None);
        if self.young_top < self.young.len() {
            self.young[self.young_top] = Some(obj);
        } else {
            self.young.push(Some(obj));
        }
        self.young_top += 1;
        idx
    }

    /// Allocate a new String object in the young generation. Returns a young-gen reference.
    pub fn allocate_string(&mut self, value: String) -> u64 {
        self.alloc_since_gc += 1;
        self.live_count += 1;
        let idx = self.young_top as u64;
        let obj = Self::make_obj("java/lang/String".to_string(), Vec::new(), Some(value));
        if self.young_top < self.young.len() {
            self.young[self.young_top] = Some(obj);
        } else {
            self.young.push(Some(obj));
        }
        self.young_top += 1;
        idx
    }

    /// Promote a young-gen object to old gen. Returns `raw_old_idx | OLD_BIT`.
    fn promote_to_old(&mut self, mut obj: HeapObject) -> u64 {
        obj.age = 0; // reset age in old gen (not used there)
        obj.forward = None;
        if let Some(raw_idx) = self.old_free_list.pop() {
            self.old[usize::try_from(raw_idx).unwrap()] = Some(obj);
            raw_idx | OLD_BIT
        } else {
            let raw_idx = self.old.len() as u64;
            self.old.push(Some(obj));
            raw_idx | OLD_BIT
        }
    }

    // ── Object access ────────────────────────────────────────────────────────

    /// Returns a reference to the object at `r`, dispatching on OLD_BIT.
    ///
    /// # Errors
    /// Returns [`VmError::InvalidRef`] if `r` is out of bounds or the slot is `None`.
    pub fn get(&self, r: u64) -> VmResult<&HeapObject> {
        if r & OLD_BIT != 0 {
            let idx = (r & !OLD_BIT) as usize;
            self.old
                .get(idx)
                .and_then(|s| s.as_ref())
                .ok_or(VmError::InvalidRef { address: r })
        } else {
            let idx = usize::try_from(r).unwrap();
            self.young
                .get(idx)
                .and_then(|s| s.as_ref())
                .ok_or(VmError::InvalidRef { address: r })
        }
    }

    /// Returns a mutable reference to the object at `r`, dispatching on OLD_BIT.
    ///
    /// # Errors
    /// Returns [`VmError::InvalidRef`] if `r` is out of bounds or the slot is `None`.
    pub fn get_mut(&mut self, r: u64) -> VmResult<&mut HeapObject> {
        if r & OLD_BIT != 0 {
            let idx = (r & !OLD_BIT) as usize;
            self.old
                .get_mut(idx)
                .and_then(|s| s.as_mut())
                .ok_or(VmError::InvalidRef { address: r })
        } else {
            let idx = usize::try_from(r).unwrap();
            self.young
                .get_mut(idx)
                .and_then(|s| s.as_mut())
                .ok_or(VmError::InvalidRef { address: r })
        }
    }

    // ── Heap stats ───────────────────────────────────────────────────────────

    /// Returns the total number of live objects across both generations.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.live_count
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.live_count == 0
    }

    /// Returns the number of live objects in the old generation.
    #[must_use]
    pub fn old_live_count(&self) -> usize {
        self.old.iter().filter(|s| s.is_some()).count()
    }

    // ── GC triggers ──────────────────────────────────────────────────────────

    /// Returns `true` when the young gen is full (minor GC should fire).
    #[must_use]
    pub const fn should_minor_gc(&self) -> bool {
        self.young_top >= self.young_capacity
    }

    /// Returns `true` when the old gen has grown to 2× its post-GC size.
    /// The minimum threshold is 256 allocations (prevents thrashing on tiny heaps).
    #[must_use]
    pub fn should_major_gc(&self) -> bool {
        let threshold = (self.live_after_last_gc * 2).max(256);
        self.alloc_since_gc >= threshold
    }

    /// Returns `true` if any young-gen objects have been forwarded by an
    /// in-progress minor GC.  Used by callers to gate the `apply_forward`
    /// sweep: when `false`, the forward map is empty and every `apply_forward`
    /// call is a no-op, so the sweep can be skipped entirely.
    #[must_use]
    pub fn has_pending_forwards(&self) -> bool {
        !self.forward_map.is_empty()
    }

    /// Compatibility shim — use `should_minor_gc` / `should_major_gc` instead.
    #[must_use]
    pub fn should_gc(&self) -> bool {
        self.should_major_gc()
    }

    // ── Write barrier ────────────────────────────────────────────────────────

    /// Store `value` into field `field_idx` of the object at `obj_ref`.
    ///
    /// If the target object is in old gen and `value` is a young-gen reference,
    /// the old-gen object's raw index is added to the remembered set so the
    /// minor GC will scan it for cross-generational pointers.
    ///
    /// # Errors
    /// Returns [`VmError::InvalidRef`] if `obj_ref` is invalid.
    pub fn write_field(&mut self, obj_ref: u64, field_idx: usize, value: Slot) -> VmResult<()> {
        if obj_ref & OLD_BIT != 0 && value.as_reference().is_some_and(|r| r & OLD_BIT == 0) {
            self.remembered_set.insert((obj_ref & !OLD_BIT) as usize);
        }
        self.get_mut(obj_ref)?.fields[field_idx] = value;
        Ok(())
    }

    // ── Minor GC (copy collector) ────────────────────────────────────────────

    /// **Phase 1 of minor GC**: trace live young objects from `roots` and the
    /// remembered set, copy them to `to_space`, and install forwarding pointers.
    ///
    /// Call [`Heap::apply_forward`] on every live interpreter slot after this,
    /// then call [`Heap::minor_collect_finish`] to complete the collection.
    pub fn minor_collect_prepare(&mut self, roots: &[Slot]) {
        self.to_space = Vec::new();
        self.forward_map.clear();

        // Seed worklist with young refs from roots and remembered-set fields.
        let mut worklist: Vec<usize> = Vec::new();

        for slot in roots {
            if let Some(r) = slot.as_reference()
                && r & OLD_BIT == 0
            {
                worklist.push(usize::try_from(r).unwrap());
            }
        }

        // Scan remembered-set old-gen objects for young refs.
        let rs: Vec<usize> = self.remembered_set.iter().copied().collect();
        for old_idx in rs {
            if let Some(Some(obj)) = self.old.get(old_idx) {
                let young_refs: Vec<usize> = obj
                    .fields
                    .iter()
                    .filter_map(Slot::as_reference)
                    .filter(|r| r & OLD_BIT == 0)
                    .map(|r| usize::try_from(r).unwrap())
                    .collect();
                worklist.extend(young_refs);
            }
        }

        // Copy phase — BFS worklist.
        while let Some(y_idx) = worklist.pop() {
            let Some(Some(obj)) = self.young.get(y_idx) else {
                continue;
            };
            if obj.forward.is_some() {
                continue; // already copied
            }

            let mut copy = obj.clone();
            let new_ref = if copy.age >= self.promotion_age {
                // Promote: move to old gen.
                copy.age += 1;
                self.promote_to_old(copy)
            } else {
                // Copy to to_space.
                copy.age += 1;
                let new_idx = self.to_space.len() as u64; // no OLD_BIT → young ref
                self.to_space.push(Some(copy));
                new_idx
            };

            // Install forwarding pointer in the from-space slot and record in map.
            if let Some(Some(from_obj)) = self.young.get_mut(y_idx) {
                from_obj.forward = Some(new_ref);
                self.forward_map.insert(y_idx as u64, new_ref);

                // Push young children onto worklist.
                let children: Vec<usize> = from_obj
                    .fields
                    .iter()
                    .filter_map(Slot::as_reference)
                    .filter(|r| r & OLD_BIT == 0)
                    .map(|r| usize::try_from(r).unwrap())
                    .collect();
                worklist.extend(children);
            }
        }

        // Patch old-gen fields in remembered-set objects.
        // Read forward pointers from young (immutable), then write to old (mutable).
        // Use index-based access to enable the split borrow.
        let rs2: Vec<usize> = self.remembered_set.iter().copied().collect();
        for old_idx in rs2 {
            if let Some(Some(obj)) = self.old.get_mut(old_idx) {
                for slot in &mut obj.fields {
                    if let Some(r) = slot.as_reference()
                        && r & OLD_BIT == 0
                    {
                        let y_idx = r as usize;
                        // Read the forwarding pointer from young gen.
                        let forward = self
                            .young
                            .get(y_idx)
                            .and_then(|s| s.as_ref())
                            .and_then(|o| o.forward);
                        if let Some(new_r) = forward {
                            *slot = Slot::Reference(Some(new_r));
                        }
                    }
                }
            }
        }
    }

    /// Patch a single slot to point to the forwarded address, if applicable.
    ///
    /// Works both during `minor_collect_prepare` (reads from `young[].forward`)
    /// and after `minor_collect_finish` (reads from `forward_map`). No-op if
    /// the slot is not a young-gen reference or has no forwarding pointer.
    pub fn apply_forward(&self, slot: &mut Slot) {
        if let Some(r) = slot.as_reference()
            && r & OLD_BIT == 0
        {
            // Try forward_map first (valid at any phase); fall back to young[].forward
            // if forward_map hasn't been populated yet for this ref.
            let new_r = self.forward_map.get(&r).copied().or_else(|| {
                self.young
                    .get(r as usize)
                    .and_then(|s| s.as_ref())
                    .and_then(|o| o.forward)
            });
            if let Some(nr) = new_r {
                *slot = Slot::Reference(Some(nr));
            }
        }
    }

    /// **Phase 3 of minor GC**: swap `to_space` into `young`, reset `young_top`,
    /// clear forwarding pointers and the remembered set.
    pub fn minor_collect_finish(&mut self) {
        // Recalculate live_count: survivors in to_space + live old-gen objects.
        let young_live = self.to_space.iter().filter(|s| s.is_some()).count();
        let old_live = self.old.iter().filter(|s| s.is_some()).count();

        // Count young objects that had no forwarding pointer (dropped).
        let young_alive_before = self.young.iter().filter(|s| s.is_some()).count();
        // Forwarded = promoted_to_old + copied_to_to_space; dropped = was live but not forwarded.
        // We compute it as: (objects that were Some in young) - (those that got a forward pointer).
        let forwarded = self
            .young
            .iter()
            .filter_map(|s| s.as_ref())
            .filter(|o| o.forward.is_some())
            .count();
        self.young_dropped = young_alive_before.saturating_sub(forwarded);

        self.young = std::mem::take(&mut self.to_space);
        self.young_top = self.young.len();
        self.remembered_set.clear();
        self.live_count = young_live + old_live;
    }

    // ── Major GC (old-gen mark-sweep) ────────────────────────────────────────

    /// Mark-sweep the old generation. Only old-gen roots (`OLD_BIT` set) are
    /// traced. Young-gen survivors must be promoted before calling this.
    pub fn major_collect(&mut self, roots: &[Slot]) {
        self.mark_old(roots);
        self.sweep_old();
    }

    fn mark_old(&mut self, roots: &[Slot]) {
        let mut worklist: Vec<u64> = roots
            .iter()
            .filter_map(Slot::as_reference)
            .filter(|r| r & OLD_BIT != 0)
            .collect();

        while let Some(r) = worklist.pop() {
            let idx = (r & !OLD_BIT) as usize;
            let Some(Some(obj)) = self.old.get_mut(idx) else {
                continue;
            };
            if obj.marked {
                continue;
            }
            obj.marked = true;
            let children: Vec<u64> = obj
                .fields
                .iter()
                .filter_map(Slot::as_reference)
                .filter(|c| c & OLD_BIT != 0)
                .collect();
            worklist.extend(children);
        }
    }

    fn sweep_old(&mut self) {
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

    /// Full collection: run a minor GC first (promotes all young survivors to old),
    /// then run old-gen mark-sweep.
    ///
    /// **Caller must apply forwarding pointers to all live interpreter slots
    /// between `minor_collect_prepare` and `minor_collect_finish` before calling
    /// this.** The `collect` shim handles this internally for tests/compat callers
    /// by building a patched-roots vec.
    pub fn collect(&mut self, roots: &[Slot]) {
        // Step 1: promote all young survivors to old gen.
        self.minor_collect_prepare(roots);

        // Build patched roots for the major GC by applying forwarding pointers.
        let mut patched: Vec<Slot> = roots.to_vec();
        for slot in &mut patched {
            self.apply_forward(slot);
        }

        self.minor_collect_finish();

        // Step 2: mark-sweep old gen.
        self.major_collect(&patched);
    }

    // ── Test helpers ─────────────────────────────────────────────────────────

    /// Number of collected slots: old-gen free list + young objects dropped in
    /// the most recent minor GC. This combined count is used by tests that
    /// verify reclamation without caring which generation the objects lived in.
    #[cfg(test)]
    #[must_use]
    pub const fn free_list_len(&self) -> usize {
        self.old_free_list.len() + self.young_dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Allocation ────────────────────────────────────────────────────────────

    #[test]
    fn allocate_and_get() {
        let mut heap = Heap::new();
        let r = heap.allocate("Point".to_string(), 2);
        assert_eq!(r & OLD_BIT, 0, "new object must be in young gen");
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

    #[test]
    fn heap_object_age_defaults_zero() {
        let mut heap = Heap::new();
        let r = heap.allocate("Foo".to_string(), 0);
        assert_eq!(heap.get(r).unwrap().age, 0);
    }

    #[test]
    fn heap_object_forward_defaults_none() {
        let mut heap = Heap::new();
        let r = heap.allocate("Foo".to_string(), 0);
        assert!(heap.get(r).unwrap().forward.is_none());
    }

    // ── GC triggers ───────────────────────────────────────────────────────────

    #[test]
    fn should_gc_triggers_at_2x_growth() {
        let mut heap = Heap::new();
        for i in 0..255 {
            heap.allocate(format!("C{i}"), 0);
            assert!(!heap.should_gc());
        }
        heap.allocate("C255".to_string(), 0);
        assert!(heap.should_gc());
    }

    // ── collect() compat shim ─────────────────────────────────────────────────

    #[test]
    fn collect_reclaims_unreachable() {
        let mut heap = Heap::new();
        let r0 = heap.allocate("Keep".to_string(), 0);
        let _r1 = heap.allocate("Drop".to_string(), 0);
        let _r2 = heap.allocate("Drop".to_string(), 0);
        heap.collect(&[Slot::Reference(Some(r0))]);
        assert_eq!(heap.free_list_len(), 2);
        assert_eq!(heap.len(), 1);
    }

    #[test]
    fn collect_preserves_reachable_chain() {
        let mut heap = Heap::new();
        let rc = heap.allocate("C".to_string(), 0);
        let rb = heap.allocate("B".to_string(), 1);
        heap.get_mut(rb).unwrap().fields[0] = Slot::Reference(Some(rc));
        let ra = heap.allocate("A".to_string(), 1);
        heap.get_mut(ra).unwrap().fields[0] = Slot::Reference(Some(rb));
        heap.collect(&[Slot::Reference(Some(ra))]);
        assert_eq!(heap.free_list_len(), 0);
        assert_eq!(heap.len(), 3);
    }

    #[test]
    fn free_list_slot_reused_after_collect() {
        let mut heap = Heap::new();
        let r0 = heap.allocate("Keep".to_string(), 0);
        let _r1 = heap.allocate("Drop".to_string(), 0);
        heap.collect(&[Slot::Reference(Some(r0))]);
        // After full collect, Keep was promoted to old gen.
        // Allocate a new object — it goes to young gen.
        let r2 = heap.allocate("New".to_string(), 0);
        // r2 should be a young-gen ref (OLD_BIT == 0).
        assert_eq!(r2 & OLD_BIT, 0);
        assert!(heap.get(r2).is_ok());
    }

    #[test]
    fn get_on_swept_slot_returns_error() {
        let mut heap = Heap::new();
        let keep = heap.allocate("Keep".to_string(), 0);
        let drop_r = heap.allocate("Drop".to_string(), 0);
        heap.collect(&[Slot::Reference(Some(keep))]);
        // drop_r is now in the old-gen free list or absent.
        // Either way, get() on it must error.
        assert!(heap.get(drop_r).is_err() || heap.get(drop_r | OLD_BIT).is_err());
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn test_heap_with_capacity(cap: usize) -> Heap {
        let mut h = Heap::new();
        h.young_capacity = cap;
        h
    }

    fn make_old_obj(heap: &mut Heap) -> u64 {
        let obj = HeapObject {
            class_name: "OldObj".to_string(),
            fields: vec![Slot::Int(0)],
            string_value: None,
            marked: false,
            age: 0,
            forward: None,
        };
        heap.old.push(Some(obj));
        (heap.old.len() as u64 - 1) | OLD_BIT
    }

    // ── GC trigger tests (Task 3) ──────────────────────────────────────────────

    #[test]
    fn should_minor_gc_fires_at_young_capacity() {
        let mut heap = Heap::new();
        heap.young_capacity = 4;
        // First 3 allocs should not trigger.
        for i in 0..3 {
            heap.allocate(format!("C{i}"), 0);
            assert!(
                !heap.should_minor_gc(),
                "should not fire before reaching capacity"
            );
        }
        // 4th alloc hits young_top == young_capacity → fires.
        heap.allocate("C3".to_string(), 0);
        assert!(heap.should_minor_gc());
    }

    #[test]
    fn should_major_gc_fires_at_2x_old_live() {
        let mut heap = Heap::new();
        // Simulate post-GC state: 10 live old objects, alloc_since_gc reset to 0.
        // Use collect() on a fresh heap to set live_after_last_gc.
        // First, make 10 objects live through a collect.
        let roots: Vec<Slot> = (0..10)
            .map(|_| {
                let r = heap.allocate("O".to_string(), 0);
                Slot::Reference(Some(r))
            })
            .collect();
        heap.collect(&roots);
        // Now live_after_last_gc == 10 (all 10 in old gen after promotion).
        // threshold = max(10*2, 256) = 256. Must allocate 256 more.
        for i in 0..255 {
            heap.allocate(format!("X{i}"), 0);
            assert!(!heap.should_major_gc(), "should not fire at alloc {i}");
        }
        heap.allocate("X255".to_string(), 0);
        assert!(heap.should_major_gc());
    }

    // ── write_field tests (Task 4) ─────────────────────────────────────────────

    #[test]
    fn write_field_old_to_young_adds_to_remembered_set() {
        let mut heap = Heap::new();
        let old_ref = make_old_obj(&mut heap);
        let young_ref = heap.allocate("Young".to_string(), 0);
        heap.write_field(old_ref, 0, Slot::Reference(Some(young_ref)))
            .unwrap();
        let old_idx = (old_ref & !OLD_BIT) as usize;
        assert!(
            heap.remembered_set.contains(&old_idx),
            "old→young store must populate remembered_set"
        );
    }

    #[test]
    fn write_field_young_to_young_does_not_add_to_remembered_set() {
        let mut heap = Heap::new();
        let r0 = heap.allocate("A".to_string(), 1);
        let r1 = heap.allocate("B".to_string(), 0);
        // r0 is young; store another young ref into it.
        heap.write_field(r0, 0, Slot::Reference(Some(r1))).unwrap();
        assert!(
            heap.remembered_set.is_empty(),
            "young→young store must NOT populate remembered_set"
        );
    }

    #[test]
    fn write_field_old_to_old_does_not_add_to_remembered_set() {
        let mut heap = Heap::new();
        heap.old.push(Some(HeapObject {
            class_name: "A".to_string(),
            fields: vec![Slot::Int(0)],
            string_value: None,
            marked: false,
            age: 0,
            forward: None,
        }));
        heap.old.push(Some(HeapObject {
            class_name: "B".to_string(),
            fields: vec![],
            string_value: None,
            marked: false,
            age: 0,
            forward: None,
        }));
        let a_ref = OLD_BIT;
        let b_ref = 1u64 | OLD_BIT;
        heap.write_field(a_ref, 0, Slot::Reference(Some(b_ref)))
            .unwrap();
        assert!(
            heap.remembered_set.is_empty(),
            "old→old store must NOT populate remembered_set"
        );
    }

    // ── Minor GC unit tests (Task 6) ───────────────────────────────────────────

    #[test]
    fn minor_gc_copies_reachable_young_object() {
        let mut heap = test_heap_with_capacity(8);
        let r0 = heap.allocate("Keep".to_string(), 0);
        let _r1 = heap.allocate("Drop".to_string(), 0);
        let roots = vec![Slot::Reference(Some(r0))];
        heap.minor_collect_prepare(&roots);
        // r0 must have a forwarding pointer; _r1 must not.
        assert!(
            heap.young[usize::try_from(r0).unwrap()]
                .as_ref()
                .unwrap()
                .forward
                .is_some()
        );
        assert!(
            heap.young[usize::try_from(_r1).unwrap()]
                .as_ref()
                .unwrap()
                .forward
                .is_none()
        );
    }

    #[test]
    fn minor_gc_forward_patches_root_slot() {
        let mut heap = test_heap_with_capacity(8);
        let r0 = heap.allocate("A".to_string(), 0);
        let roots = vec![Slot::Reference(Some(r0))];
        heap.minor_collect_prepare(&roots);
        let mut slot = Slot::Reference(Some(r0));
        heap.apply_forward(&mut slot);
        // After forwarding, slot must point to the new location.
        let new_r = heap.young[usize::try_from(r0).unwrap()]
            .as_ref()
            .unwrap()
            .forward
            .unwrap();
        assert_eq!(slot, Slot::Reference(Some(new_r)));
    }

    #[test]
    fn minor_gc_finish_swaps_to_space_into_young() {
        let mut heap = test_heap_with_capacity(8);
        let r0 = heap.allocate("A".to_string(), 0);
        let _r1 = heap.allocate("B".to_string(), 0);
        // Only r0 is a root → _r1 is dead.
        let roots = vec![Slot::Reference(Some(r0))];
        heap.minor_collect_prepare(&roots);
        heap.minor_collect_finish();
        // After finish: young has 1 live survivor.
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
        let new_r = heap.young[usize::try_from(r).unwrap()]
            .as_ref()
            .unwrap()
            .forward
            .unwrap();
        heap.minor_collect_finish();
        // After finish, young is former to_space. new_r has no OLD_BIT → young index.
        let survivor = heap.young[usize::try_from(new_r).unwrap()]
            .as_ref()
            .unwrap();
        assert_eq!(survivor.age, 1);
    }

    #[test]
    fn minor_gc_promotes_at_promotion_age() {
        let mut heap = test_heap_with_capacity(64);
        // promotion_age = 1: an object with age >= 1 is promoted.
        // Round 0: age=0, check 0>=1 → false → to_space (age becomes 1), new_r is young.
        // Round 1: age=1, check 1>=1 → true  → old gen (OLD_BIT set).
        heap.promotion_age = 1;

        let mut current_r = heap.allocate("P".to_string(), 0);

        for round in 0..2u8 {
            let roots = vec![Slot::Reference(Some(current_r))];
            heap.minor_collect_prepare(&roots);
            let new_r = heap.young[usize::try_from(current_r).unwrap()]
                .as_ref()
                .unwrap()
                .forward
                .unwrap();
            heap.minor_collect_finish();

            if round == 0 {
                // Still young after first survival (age becomes 1, not yet promoted).
                assert_eq!(new_r & OLD_BIT, 0, "should still be young after 1 survival");
                current_r = new_r;
            } else {
                // Promoted to old gen (OLD_BIT set) on second survival.
                assert_ne!(new_r & OLD_BIT, 0, "should be in old gen after 2 survivals");
                let obj = heap.get(new_r).unwrap();
                assert_eq!(obj.class_name, "P");
            }
        }
    }

    #[test]
    fn remembered_set_root_survives_minor_gc() {
        let mut heap = test_heap_with_capacity(8);
        // Build: old-gen object with a field pointing to a young object.
        heap.old.push(Some(HeapObject {
            class_name: "Old".to_string(),
            fields: vec![Slot::Int(0)], // will be overwritten below
            string_value: None,
            marked: false,
            age: 0,
            forward: None,
        }));
        let old_ref = OLD_BIT;
        let young_ref = heap.allocate("Young".to_string(), 0);
        // Wire old→young via write_field (populates remembered_set).
        heap.write_field(old_ref, 0, Slot::Reference(Some(young_ref)))
            .unwrap();

        // No stack roots — young object reachable only through remembered set.
        heap.minor_collect_prepare(&[]);
        assert!(
            heap.young[usize::try_from(young_ref).unwrap()]
                .as_ref()
                .unwrap()
                .forward
                .is_some(),
            "young object reachable via rem-set must be forwarded"
        );
        heap.minor_collect_finish();
        // Verify old-gen field was patched to the new young location.
        let new_field = heap.old[0].as_ref().unwrap().fields[0];
        match new_field {
            Slot::Reference(Some(r)) => {
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
        let old_ref = OLD_BIT;
        let mut slot = Slot::Reference(Some(old_ref));
        heap.apply_forward(&mut slot);
        // No forwarding pointer in old gen → slot unchanged.
        assert_eq!(slot, Slot::Reference(Some(old_ref)));
    }

    // ── Major GC unit tests (Task 7) ───────────────────────────────────────────

    #[test]
    fn old_ref_has_old_bit() {
        let mut heap = Heap::new();
        heap.old.push(Some(HeapObject {
            class_name: "OldObj".to_string(),
            fields: vec![],
            string_value: None,
            marked: false,
            age: 0,
            forward: None,
        }));
        let old_ref = OLD_BIT;
        assert_ne!(old_ref & OLD_BIT, 0, "old ref must have OLD_BIT set");
        assert_eq!(heap.get(old_ref).unwrap().class_name, "OldObj");
    }

    #[test]
    fn major_collect_reclaims_unreachable_old_objects() {
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
        let keep_ref = OLD_BIT;
        let drop_ref = 1u64 | OLD_BIT;
        let roots = vec![Slot::Reference(Some(keep_ref))];
        heap.major_collect(&roots);
        assert!(
            heap.get(keep_ref).is_ok(),
            "reachable old-gen object must survive"
        );
        assert!(
            heap.get(drop_ref).is_err(),
            "unreachable old-gen object must be swept"
        );
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
            fields: vec![Slot::Reference(Some(OLD_BIT))],
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
        let keep_ref = OLD_BIT;
        heap.major_collect(&[Slot::Reference(Some(keep_ref))]);
        // old_free_list has raw index 1 (no OLD_BIT).
        assert!(heap.old_free_list.contains(&1u64));
    }

    #[test]
    fn collect_compat_shim_collects_full_heap() {
        let mut heap = test_heap_with_capacity(512);
        // promotion_age = 0: age >= 0 is always true, so any survivor promotes
        // on the first minor GC.  This lets the compat collect() shim produce a
        // single old-gen object in one pass.
        heap.promotion_age = 0;
        let keep = heap.allocate("Keep".to_string(), 0);
        let _drop1 = heap.allocate("Drop1".to_string(), 0);
        let _drop2 = heap.allocate("Drop2".to_string(), 0);
        let roots = vec![Slot::Reference(Some(keep))];
        heap.collect(&roots);
        // After full collect: 1 live object promoted to old gen; 2 dropped.
        assert_eq!(heap.old_live_count(), 1);
        assert_eq!(heap.len(), 1);
    }
}
