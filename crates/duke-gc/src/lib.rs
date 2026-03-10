//! Non-moving mark-sweep GC for the Duke JVM (Phase 24).
//!
//! Objects are allocated via bump-pointer with free-list reuse of swept slots.
//! [`Heap::collect`] implements mark (iterative DFS from roots) + sweep (single pass).

use duke_runtime::{Slot, VmError, VmResult};

/// A single heap-allocated Java object.
#[derive(Debug, Clone)]
pub struct HeapObject {
    pub class_name: String,
    pub fields: Vec<Slot>,
    /// String content for `java/lang/String` objects. `None` for non-string objects.
    pub string_value: Option<String>,
    /// Mark bit for the mark phase of mark-sweep GC. `false` until marked reachable.
    // INVARIANT: Heap::mark() traces only `fields` for child references.
    // Any new Vec<Slot> member added to HeapObject MUST also be covered in mark().
    pub(crate) marked: bool,
    /// Number of minor GC cycles this object has survived. Promoted to old gen at threshold.
    /// Used by the minor collector added in Phase 25 Task 2.
    #[allow(dead_code)]
    pub(crate) age: u8,
    /// Forwarding pointer set during the copy phase of a copying/generational GC. `None` when
    /// the object has not been moved in the current collection cycle.
    /// Used by the minor collector added in Phase 25 Task 2.
    #[allow(dead_code)]
    pub(crate) forward: Option<u64>,
}

/// The object heap — a Vec-backed bump allocator with free-list support for GC.
///
/// `Slot::Reference(Some(u64))` values are indices into this Vec.
/// Swept slots become `None` and are tracked in `free_list` for reuse.
#[derive(Debug, Default)]
pub struct Heap {
    objects: Vec<Option<HeapObject>>,
    free_list: Vec<u64>,
    live_after_last_gc: usize,
    alloc_since_gc: usize,
    /// Number of live (Some) objects; maintained for O(1) `len()`.
    live_count: usize,
}

impl Heap {
    #[must_use]
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            free_list: Vec::new(),
            live_after_last_gc: 0,
            alloc_since_gc: 0,
            live_count: 0,
        }
    }

    /// Insert `obj` into the heap, reusing a free-list slot if available.
    /// Increments `alloc_since_gc`. Returns the heap index.
    fn allocate_with(&mut self, obj: HeapObject) -> u64 {
        self.alloc_since_gc += 1;
        self.live_count += 1;
        if let Some(idx) = self.free_list.pop() {
            self.objects[idx as usize] = Some(obj);
            return idx;
        }
        let idx = self.objects.len() as u64;
        self.objects.push(Some(obj));
        idx
    }

    /// Allocate a new object. Returns its heap index as `u64`.
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

    /// Allocate a new String object with the given content.
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

    /// # Errors
    /// Returns [`VmError::InvalidRef`] if out of bounds or slot is `None` (swept).
    pub fn get(&self, r: u64) -> VmResult<&HeapObject> {
        let idx = usize::try_from(r).map_err(|_| VmError::InvalidRef { address: r })?;
        self.objects
            .get(idx)
            .and_then(|slot| slot.as_ref())
            .ok_or(VmError::InvalidRef { address: r })
    }

    /// # Errors
    /// Returns [`VmError::InvalidRef`] if out of bounds or slot is `None` (swept).
    pub fn get_mut(&mut self, r: u64) -> VmResult<&mut HeapObject> {
        let idx = usize::try_from(r).map_err(|_| VmError::InvalidRef { address: r })?;
        self.objects
            .get_mut(idx)
            .and_then(|slot| slot.as_mut())
            .ok_or(VmError::InvalidRef { address: r })
    }

    /// Returns the number of live (non-swept) objects on the heap.
    #[must_use]
    pub fn len(&self) -> usize {
        self.live_count
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.live_count == 0
    }

    /// Returns `true` when the heap has grown to 2× its size after the last GC.
    /// The minimum threshold is 256 allocations (prevents thrashing on tiny heaps).
    #[must_use]
    pub fn should_gc(&self) -> bool {
        let threshold = (self.live_after_last_gc * 2).max(256);
        self.alloc_since_gc >= threshold
    }

    /// Collect garbage: mark all objects reachable from `roots`, then sweep the rest.
    pub fn collect(&mut self, roots: &[Slot]) {
        self.mark(roots);
        self.sweep();
    }

    fn mark(&mut self, roots: &[Slot]) {
        let mut worklist: Vec<u64> = roots.iter().filter_map(Slot::as_reference).collect();

        while let Some(r) = worklist.pop() {
            let Some(Some(obj)) = self.objects.get_mut(r as usize) else {
                continue;
            };
            if obj.marked {
                continue;
            }
            obj.marked = true;
            let children: Vec<u64> = obj.fields.iter().filter_map(Slot::as_reference).collect();
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
        self.live_count = live;
    }

    /// Number of slots currently on the free list (test-only helper).
    #[cfg(test)]
    pub fn free_list_len(&self) -> usize {
        self.free_list.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // No objects allocated — any ref is invalid
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
    fn should_gc_triggers_at_2x_growth() {
        let mut heap = Heap::new();
        // First 255 allocs don't trigger (floor threshold is 256)
        for i in 0..255 {
            heap.allocate(format!("C{i}"), 0);
            assert!(!heap.should_gc());
        }
        // 256th alloc pushes alloc_since_gc to 256, hitting threshold
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

    #[test]
    fn get_on_swept_slot_returns_error() {
        let mut heap = Heap::new();
        let _keep = heap.allocate("Keep".to_string(), 0);
        let drop = heap.allocate("Drop".to_string(), 0);
        // Collect with only _keep as root — drop gets swept
        heap.collect(&[Slot::Reference(Some(_keep))]);
        // The swept slot is now None — get() must return InvalidRef
        assert!(heap.get(drop).is_err());
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
}
