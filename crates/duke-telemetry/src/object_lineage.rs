use std::collections::HashMap;

/// Details about a specific bytecode instruction that allocated an object.
///
/// Tracks how many objects a single `new` or array allocation instruction created
/// during the JVM's lifetime.
///
/// # Examples
///
/// ```
/// use duke_telemetry::AllocationSite;
///
/// let mut site = AllocationSite::default();
/// site.class_allocated = "java/lang/String".to_string();
/// site.count = 42;
/// assert_eq!(site.count, 42);
/// ```
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct AllocationSite {
    /// Total number of objects allocated at this site.
    pub count: u64,
    /// The name of the class (or array type) being instantiated here.
    pub class_allocated: String,
}

/// Tracks memory allocation sites across the JVM.
///
/// Correlates memory pressure to the exact method and instruction PC responsible
/// for the allocations.
///
/// # Examples
///
/// ```
/// use duke_telemetry::ObjectLineageStore;
///
/// let mut store = ObjectLineageStore::default();
/// store.record("Main", "run", 5, "java/lang/Object");
///
/// let site = &store.sites[&("Main".to_string(), "run".to_string(), 5)];
/// assert_eq!(site.count, 1);
/// assert_eq!(site.class_allocated, "java/lang/Object");
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ObjectLineageStore {
    /// Mapping of allocation locations to statistics.
    ///
    /// Key: `(class_where_allocated, method_where_allocated, instruction_pc)`
    #[cfg_attr(
        feature = "telemetry",
        serde(serialize_with = "crate::store::ser_helpers::site3")
    )]
    pub sites: HashMap<(String, String, usize), AllocationSite>,
}

impl ObjectLineageStore {
    /// Record a single object or array allocation.
    ///
    /// - `class`: The name of the class performing the allocation.
    /// - `method`: The name of the method performing the allocation.
    /// - `pc`: The program counter of the allocation instruction.
    /// - `allocated_class`: The name of the class being created.
    pub fn record(&mut self, class: &str, method: &str, pc: usize, allocated_class: &str) {
        let site = self
            .sites
            .entry((class.to_string(), method.to_string(), pc))
            .or_default();
        site.count += 1;
        if site.class_allocated.is_empty() {
            site.class_allocated = allocated_class.to_string();
        }
    }
}
