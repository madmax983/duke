//! Simple bump-pointer heap for Duke Phase 6.
//!
//! No garbage collection yet — objects are allocated and never freed.
//! Phase 7 will add a generational collector here.

use duke_runtime::{Slot, VmError, VmResult};

/// A single heap-allocated Java object.
#[derive(Debug, Clone)]
pub struct HeapObject {
    pub class_name: String,
    pub fields: Vec<Slot>,
}

/// The object heap — a Vec-backed bump allocator.
///
/// `Slot::Reference(Some(u64))` values are indices into this Vec.
#[derive(Debug, Default)]
pub struct Heap {
    objects: Vec<HeapObject>,
}

impl Heap {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate a new object. Returns its heap index as `u64`.
    pub fn allocate(&mut self, class_name: String, field_count: usize) -> u64 {
        let idx = self.objects.len() as u64;
        self.objects.push(HeapObject {
            class_name,
            fields: vec![Slot::Int(0); field_count],
        });
        idx
    }

    /// # Errors
    /// Returns [`VmError::InvalidRef`] if out of bounds.
    pub fn get(&self, r: u64) -> VmResult<&HeapObject> {
        let idx = usize::try_from(r).map_err(|_| VmError::InvalidRef { address: r })?;
        self.objects
            .get(idx)
            .ok_or(VmError::InvalidRef { address: r })
    }

    /// # Errors
    /// Returns [`VmError::InvalidRef`] if out of bounds.
    pub fn get_mut(&mut self, r: u64) -> VmResult<&mut HeapObject> {
        let idx = usize::try_from(r).map_err(|_| VmError::InvalidRef { address: r })?;
        self.objects
            .get_mut(idx)
            .ok_or(VmError::InvalidRef { address: r })
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.objects.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.objects.is_empty()
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
}
