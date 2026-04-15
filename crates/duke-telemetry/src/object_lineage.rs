#[cfg(feature = "telemetry")]
use crate::helpers::ser_helpers;
use std::collections::HashMap;

// -- object_lineage --------------------------------------------------------------

/// Details of a single allocation site and what it allocated.
///
/// Keeps track of the most recently allocated class at this site and the total
/// number of objects it created.
///
/// # Examples
///
/// ```
/// use duke_telemetry::AllocationSite;
///
/// let site = AllocationSite {
///     class_allocated: "java/lang/String".to_string(),
///     count: 42,
/// };
/// assert_eq!(site.count, 42);
/// ```
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct AllocationSite {
    /// The JVM class name of the allocated object.
    pub class_allocated: String,
    /// Number of times objects of `class_allocated` were instantiated at this site.
    pub count: u64,
}

/// Tracks where objects are allocated, by method and instruction offset.
///
/// Records the allocating class and method alongside the PC offset of the `new` instruction,
/// to trace the origin of high allocation rates back to the source code.
///
/// # Examples
///
/// ```
/// use duke_telemetry::ObjectLineageStore;
///
/// let mut store = ObjectLineageStore::default();
/// store.record("com/example/Main", "run", 10, "java/lang/String");
///
/// let site = &store.sites[&("com/example/Main".to_string(), "run".to_string(), 10)];
/// assert_eq!(site.count, 1);
/// assert_eq!(site.class_allocated, "java/lang/String");
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ObjectLineageStore {
    /// Mapping from (`allocating_class`, `allocating_method`, pc) to allocation statistics.
    #[cfg_attr(feature = "telemetry", serde(serialize_with = "ser_helpers::site3"))]
    pub sites: HashMap<(String, String, usize), AllocationSite>,
}

impl ObjectLineageStore {
    /// Record a single object allocation event.
    ///
    /// - `allocating_class`: The class executing the `new` instruction.
    /// - `method`: The method executing the `new` instruction.
    /// - `pc`: Program counter of the allocation site.
    /// - `class_allocated`: The class name of the instantiated object.
    pub fn record(
        &mut self,
        allocating_class: &str,
        method: &str,
        pc: usize,
        class_allocated: &str,
    ) {
        let site = self
            .sites
            .entry((allocating_class.to_string(), method.to_string(), pc))
            .or_insert_with(|| AllocationSite {
                class_allocated: class_allocated.to_string(),
                count: 0,
            });
        site.count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_lineage_records_allocation_site() {
        let mut store = ObjectLineageStore::default();
        store.record("Foo", "main", 5, "java/lang/Object");
        store.record("Foo", "main", 5, "java/lang/Object");
        let site = &store.sites[&("Foo".to_string(), "main".to_string(), 5)];
        assert_eq!(site.count, 2);
        assert_eq!(site.class_allocated, "java/lang/Object");
    }
}
