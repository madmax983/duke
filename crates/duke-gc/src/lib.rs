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

pub mod host;
mod mermaid;
pub use host::{HostFileHandle, HostProcessHandle, SpawnedProcessIds};

/// High bit set ⟹ old-generation reference; clear ⟹ young-generation reference.
pub const OLD_BIT: u64 = 1 << 63;

/// Default number of young-gen slots before a minor GC fires.
const DEFAULT_YOUNG_CAPACITY: usize = 512;

/// Default number of minor-GC survivals before an object is promoted to old gen.
const DEFAULT_PROMOTION_AGE: u8 = 4;

/// A single heap-allocated Java object.
///
/// # Examples
///
/// ```
/// use duke_gc::HeapObject;
/// use duke_runtime::Slot;
///
/// // Usually created via `Heap::allocate`
/// let mut heap = duke_gc::Heap::new();
/// let obj_ref = heap.allocate("java/lang/Object".to_string(), 1);
///
/// let obj = heap.get_mut(obj_ref).unwrap();
/// obj.fields[0] = Slot::Int(42);
/// assert_eq!(obj.class_name, "java/lang/Object");
/// ```
#[derive(Debug, Clone)]
pub struct HeapObject {
    /// The runtime class name of this object (e.g. `"java/lang/String"`).
    pub class_name: String,
    /// Storage for all instance fields of this object.
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
///
/// # Examples
///
/// ```
/// use duke_gc::Heap;
/// use duke_runtime::Slot;
///
/// let mut heap = Heap::new();
/// let obj_ref = heap.allocate("MyClass".to_string(), 2);
///
/// let obj = heap.get_mut(obj_ref).unwrap();
/// obj.fields[0] = Slot::Int(42);
///
/// assert_eq!(heap.get(obj_ref).unwrap().fields[0], Slot::Int(42));
/// ```
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
    /// Free-list of raw old-gen indices (no `OLD_BIT`) for reuse after sweep.
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
    /// Maps old young-gen ref → new ref (young or old-gen with `OLD_BIT`).
    /// Populated during `minor_collect_prepare`, kept alive past
    /// `minor_collect_finish` so callers can patch their own slots after
    /// `collect()` returns via [`Heap::apply_forward`].
    forward_map: HashMap<u64, u64>,
    /// Host OS file handles keyed by small integer ids stored in Java objects.
    pub(crate) host_files: HashMap<i32, host::HostFileHandle>,
    pub(crate) next_host_file_id: i32,
}

impl Heap {
    /// Creates a new heap.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    /// let heap = Heap::new();
    /// assert!(heap.is_empty());
    /// ```
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
            host_files: HashMap::new(),
            next_host_file_id: 1,
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
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    ///
    /// let mut heap = Heap::new();
    /// let r = heap.allocate("java/lang/Object".to_string(), 0);
    /// assert_eq!(heap.get(r).unwrap().class_name, "java/lang/Object");
    /// ```
    pub fn allocate(&mut self, class_name: String, field_count: usize) -> u64 {
        self.alloc_since_gc += 1;
        self.live_count += 1;
        let idx = self.young_top as u64; // OLD_BIT == 0 → young ref
        let obj = Self::make_obj(class_name, vec![Slot::Int(0); field_count], None);
        self.young.push(Some(obj));
        self.young_top += 1;
        idx
    }

    /// Allocate a new String object in the young generation. Returns a young-gen reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    ///
    /// let mut heap = Heap::new();
    /// let r = heap.allocate_string("Hello".to_string());
    /// assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("Hello"));
    /// ```
    pub fn allocate_string(&mut self, value: String) -> u64 {
        self.alloc_since_gc += 1;
        self.live_count += 1;
        let idx = self.young_top as u64;
        let obj = Self::make_obj("java/lang/String".to_string(), Vec::new(), Some(value));
        self.young.push(Some(obj));
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

    /// Returns a reference to the object at `r`, dispatching on `OLD_BIT`.
    ///
    /// # Errors
    /// Returns [`VmError::InvalidRef`] if `r` is out of bounds or the slot is `None`.
    ///
    /// # Panics
    /// Panics if `r` (with `OLD_BIT` clear) cannot be converted to `usize`, which
    /// cannot happen on 64-bit targets since heap indices are always small.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    ///
    /// let mut heap = Heap::new();
    /// let obj_ref = heap.allocate("MyClass".to_string(), 0);
    /// assert_eq!(heap.get(obj_ref).unwrap().class_name, "MyClass");
    /// ```
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

    /// Returns a mutable reference to the object at `r`, dispatching on `OLD_BIT`.
    ///
    /// # Errors
    /// Returns [`VmError::InvalidRef`] if `r` is out of bounds or the slot is `None`.
    ///
    /// # Panics
    /// Panics if `r` (with `OLD_BIT` clear) cannot be converted to `usize`, which
    /// cannot happen on 64-bit targets since heap indices are always small.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    /// use duke_runtime::Slot;
    ///
    /// let mut heap = Heap::new();
    /// let obj_ref = heap.allocate("MyClass".to_string(), 1);
    /// heap.get_mut(obj_ref).unwrap().fields[0] = Slot::Int(123);
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    ///
    /// let mut heap = Heap::new();
    /// assert_eq!(heap.len(), 0);
    /// heap.allocate("MyClass".to_string(), 0);
    /// assert_eq!(heap.len(), 1);
    /// ```
    #[must_use]
    pub const fn len(&self) -> usize {
        self.live_count
    }

    /// Returns whether the heap is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    ///
    /// let heap = Heap::new();
    /// assert!(heap.is_empty());
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    /// use duke_runtime::Slot;
    ///
    /// let mut heap = Heap::new();
    /// let obj_ref = heap.allocate("Box".to_string(), 1);
    /// heap.write_field(obj_ref, 0, Slot::Int(42)).unwrap();
    /// assert_eq!(heap.get(obj_ref).unwrap().fields[0], Slot::Int(42));
    /// ```
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
    ///
    /// # Panics
    /// Panics if a young-gen reference in `roots` cannot be converted to `usize`,
    /// which cannot happen on 64-bit targets since heap indices are always small.
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
        // ⚡ Bolt: Avoid intermediate vector allocation by iterating directly
        for &old_idx in &self.remembered_set {
            if let Some(Some(obj)) = self.old.get(old_idx) {
                worklist.extend(
                    obj.fields
                        .iter()
                        .filter_map(Slot::as_reference)
                        .filter(|r| r & OLD_BIT == 0)
                        .map(|r| usize::try_from(r).unwrap()),
                );
            }
        }

        // Copy phase — BFS worklist.
        while let Some(y_idx) = worklist.pop() {
            let is_forwarded = self
                .young
                .get(y_idx)
                .and_then(|o| o.as_ref())
                .is_some_and(|o| o.forward.is_some());
            if is_forwarded {
                continue;
            }

            let Some(Some(mut copy)) = self.young.get_mut(y_idx).map(std::option::Option::take)
            else {
                continue;
            };

            // Push young children onto worklist *before* moving copy
            worklist.extend(
                copy.fields
                    .iter()
                    .filter_map(Slot::as_reference)
                    .filter(|r| r & OLD_BIT == 0)
                    .map(|r| usize::try_from(r).unwrap()),
            );

            let new_ref = if copy.age >= self.promotion_age {
                self.promote_to_old(copy)
            } else {
                copy.age += 1;
                let new_idx = self.to_space.len() as u64;
                self.to_space.push(Some(copy));
                new_idx
            };

            // Install dummy object with forwarding pointer
            self.young[y_idx] = Some(HeapObject {
                class_name: String::new(),
                fields: vec![],
                string_value: None,
                marked: false,
                age: 0,
                forward: Some(new_ref),
            });
            self.forward_map.insert(y_idx as u64, new_ref);
        }

        let forward_map = &self.forward_map;

        // Patch copied young survivors so their intra-young references point at
        // the forwarded children instead of stale from-space indices.
        let to_space = &mut self.to_space;
        for obj in to_space.iter_mut().flatten() {
            patch_forwarded_fields(&mut obj.fields, forward_map);
        }

        // Patch all old-gen objects and rebuild the remembered set so existing
        // old->young edges, including newly promoted objects, survive the next minor GC.
        let old = &mut self.old;
        let mut rebuilt_remembered_set = HashSet::new();
        for (old_idx, obj) in old.iter_mut().enumerate() {
            let Some(obj) = obj.as_mut() else {
                continue;
            };
            if patch_forwarded_fields(&mut obj.fields, forward_map) {
                rebuilt_remembered_set.insert(old_idx);
            }
        }
        self.remembered_set = rebuilt_remembered_set;
    }

    /// Patch a single slot to point to the forwarded address, if applicable.
    ///
    /// Works both during `minor_collect_prepare` (reads from `young[].forward`)
    /// and after `minor_collect_finish` (reads from `forward_map`). No-op if
    /// the slot is not a young-gen reference or has no forwarding pointer.
    ///
    /// # Panics
    /// Panics if the young-gen reference value cannot be converted to `usize`,
    /// which cannot happen on 64-bit targets since heap indices are always small.
    pub fn apply_forward(&self, slot: &mut Slot) {
        if let Some(r) = slot.as_reference()
            && r & OLD_BIT == 0
        {
            // Try forward_map first (valid at any phase); fall back to young[].forward
            // if forward_map hasn't been populated yet for this ref.
            let new_r = self.forward_map.get(&r).copied().or_else(|| {
                self.young
                    .get(usize::try_from(r).unwrap())
                    .and_then(|s| s.as_ref())
                    .and_then(|o| o.forward)
            });
            if let Some(nr) = new_r {
                *slot = Slot::Reference(Some(nr));
            }
        }
    }

    /// **Phase 3 of minor GC**: swap `to_space` into `young` and reset `young_top`.
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

    /// Generates a Mermaid JS graph of the heap.
    #[must_use]
    pub fn dump_mermaid(&self) -> String {
        mermaid::dump_mermaid(self)
    }
}

fn patch_forwarded_slot(slot: &mut Slot, forward_map: &HashMap<u64, u64>) {
    if let Some(r) = slot.as_reference()
        && r & OLD_BIT == 0
        && let Some(new_r) = forward_map.get(&r).copied()
    {
        *slot = Slot::Reference(Some(new_r));
    }
}

fn patch_forwarded_fields(fields: &mut [Slot], forward_map: &HashMap<u64, u64>) -> bool {
    let mut contains_young_ref = false;
    for slot in fields {
        patch_forwarded_slot(slot, forward_map);
        if slot.as_reference().is_some_and(|r| r & OLD_BIT == 0) {
            contains_young_ref = true;
        }
    }
    contains_young_ref
}

#[cfg(test)]
mod fuzz;
#[cfg(test)]
mod tests;
