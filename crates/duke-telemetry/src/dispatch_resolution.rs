#[cfg(feature = "telemetry")]
use crate::helpers::ser_helpers;
use std::collections::{HashMap, HashSet};

// -- dispatch_resolution ---------------------------------------------------------

/// Telemetry regarding a single virtual or interface dispatch site.
///
/// Tracks the total number of calls, the distinct receiver classes seen (polymorphism),
/// and the overhead of finding the appropriate method implementation within the class hierarchy.
///
/// # Examples
///
/// ```
/// use duke_telemetry::DispatchStat;
///
/// let mut stat = DispatchStat::default();
/// stat.calls = 100;
/// stat.hierarchy_walks = 50;
/// assert_eq!(stat.calls, 100);
/// ```
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct DispatchStat {
    /// Total number of method dispatches recorded at this call site.
    pub calls: u64,
    /// Distinct runtime receiver classes seen at this call site.
    #[cfg_attr(
        feature = "telemetry",
        serde(serialize_with = "ser_helpers::sorted_set")
    )]
    pub unique_targets: HashSet<String>,
    /// How many calls required a superclass hierarchy walk to find the method.
    pub hierarchy_walks: u64,
}

/// Statistics on dynamic method resolution (`invokevirtual` and `invokeinterface`).
///
/// Tracks polymorphism and dispatch overhead at every dynamic call site in the VM
/// to help identify opportunities for inline caching or other optimizations.
///
/// # Examples
///
/// ```
/// use duke_telemetry::DispatchResolutionStore;
///
/// let mut store = DispatchResolutionStore::default();
/// store.record("com/example/Main", 15, "java/lang/String", true);
///
/// let stat = &store.by_site[&("com/example/Main".to_string(), 15)];
/// assert_eq!(stat.calls, 1);
/// assert!(stat.unique_targets.contains("java/lang/String"));
/// assert_eq!(stat.hierarchy_walks, 1);
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct DispatchResolutionStore {
    /// Mapping from (`caller_class`, `cp_idx`) to dispatch statistics.
    #[cfg_attr(
        feature = "telemetry",
        serde(serialize_with = "ser_helpers::site2_u16")
    )]
    pub by_site: HashMap<(String, u16), DispatchStat>,
}

impl DispatchResolutionStore {
    /// Record the resolution of a virtual or interface method call.
    ///
    /// - `caller_class`: The class containing the `invoke*` instruction.
    /// - `cp_idx`: The constant pool index referenced by the instruction.
    /// - `resolved_class`: The actual runtime class of the receiver object.
    /// - `hierarchy_walk`: True if the method implementation was found by walking
    ///   up the superclass chain; false if it was found directly on `resolved_class`.
    pub fn record(
        &mut self,
        caller_class: &str,
        cp_idx: u16,
        resolved_class: &str,
        hierarchy_walk: bool,
    ) {
        let stat = self
            .by_site
            .entry((caller_class.to_string(), cp_idx))
            .or_default();
        stat.calls += 1;
        if !stat.unique_targets.contains(resolved_class) {
            stat.unique_targets.insert(resolved_class.to_string());
        }
        if hierarchy_walk {
            stat.hierarchy_walks += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_resolution_tracks_unique_targets() {
        let mut store = DispatchResolutionStore::default();
        store.record("Foo", 3, "SubA", false);
        store.record("Foo", 3, "SubB", true);
        let stat = &store.by_site[&("Foo".to_string(), 3)];
        assert_eq!(stat.calls, 2);
        assert_eq!(stat.unique_targets.len(), 2);
        assert_eq!(stat.hierarchy_walks, 1);
    }
}
