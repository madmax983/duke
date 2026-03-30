use std::collections::{HashMap, HashSet};

/// Statistics for a specific virtual method call site.
///
/// Records the total number of invocations and the set of unique receiver types
/// observed at this `invokevirtual` or `invokeinterface` site, allowing
/// megamorphic sites to be identified for future optimization.
///
/// # Examples
///
/// ```
/// use duke_telemetry::DispatchStat;
///
/// let mut stat = DispatchStat::default();
/// stat.calls = 10;
/// stat.unique_targets.insert("java/lang/String".to_string());
/// assert_eq!(stat.unique_targets.len(), 1);
/// ```
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct DispatchStat {
    /// Total number of executions of this invoke instruction.
    pub calls: u64,
    /// The set of actual receiver classes observed at this site.
    #[cfg_attr(
        feature = "telemetry",
        serde(serialize_with = "crate::store::ser_helpers::sorted_set")
    )]
    pub unique_targets: HashSet<String>,
    /// How many times the JVM had to walk up the inheritance tree to find the method.
    pub hierarchy_walks: u64,
}

/// Tracks the cost of resolving dynamic method dispatch.
///
/// Correlates polymorphic method dispatch overhead to the exact `invokevirtual`
/// or `invokeinterface` instruction.
///
/// # Examples
///
/// ```
/// use duke_telemetry::DispatchResolutionStore;
///
/// let mut store = DispatchResolutionStore::default();
/// store.record("Foo", 5, "Bar", false);
///
/// let stat = &store.by_site[&("Foo".to_string(), 5)];
/// assert_eq!(stat.calls, 1);
/// assert!(stat.unique_targets.contains("Bar"));
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct DispatchResolutionStore {
    /// Mapping of call sites to their dynamic resolution statistics.
    ///
    /// Key: `(class_containing_the_invoke, constant_pool_index)`
    #[cfg_attr(
        feature = "telemetry",
        serde(serialize_with = "crate::store::ser_helpers::site2_u16")
    )]
    pub by_site: HashMap<(String, u16), DispatchStat>,
}

impl DispatchResolutionStore {
    /// Record the resolution of a virtual or interface method call.
    ///
    /// - `class`: The class containing the `invokevirtual` or `invokeinterface` instruction.
    /// - `cp_index`: The constant pool index of the method reference.
    /// - `target_class`: The actual runtime class of the receiver object.
    /// - `hierarchy_walked`: True if the method wasn't found directly on the receiver
    ///   and the JVM had to search its superclasses.
    pub fn record(
        &mut self,
        class: &str,
        cp_index: u16,
        target_class: &str,
        hierarchy_walked: bool,
    ) {
        let stat = self
            .by_site
            .entry((class.to_string(), cp_index))
            .or_default();
        stat.calls += 1;
        stat.unique_targets.insert(target_class.to_string());
        if hierarchy_walked {
            stat.hierarchy_walks += 1;
        }
    }
}
