//! Duke VM Telemetry subsystem.
//!
//! This module provides the [`TelemetryStore`] and its constituent channels for
//! recording and analyzing the runtime behavior of the Duke Java Virtual Machine.
//!
//! Telemetry is divided into six independent channels, each focusing on a different
//! aspect of VM execution:
//!
//! - **Bytecode Cost** ([`BytecodeCostStore`]): Tracks execution frequency and cumulative
//!   time spent in each JVM opcode, both globally and per-call-site.
//! - **Object Lineage** ([`ObjectLineageStore`]): Records which methods are allocating
//!   which classes, providing insight into memory pressure hot-spots.
//! - **Class Initialization** ([`ClassInitDagStore`]): Builds a Directed Acyclic Graph (DAG)
//!   of `<clinit>` executions, tracking triggers and durations to diagnose slow startup.
//! - **Exception Flow** ([`ExceptionFlowStore`]): Traces the lifecycle of thrown exceptions,
//!   including their throw sites, catch sites, and the number of times they were rethrown.
//! - **Dispatch Resolution** ([`DispatchResolutionStore`]): Analyzes `invokevirtual` and
//!   `invokeinterface` calls to track receiver polymorphism (megamorphic call sites) and
//!   superclass hierarchy walk overhead.
//! - **Native Boundary** ([`NativeBoundaryStore`]): Measures the frequency, duration, and
//!   error rates of transitions into JNI or intrinsic native methods.
//!
//! # Examples
//!
//! ```
//! use duke_telemetry::TelemetryStore;
//!
//! let mut store = TelemetryStore::default();
//!
//! // Record a hypothetical allocation of java/lang/String
//! store.object_lineage.record("com/example/Main", "run", 42, "java/lang/String");
//!
//! // Serialize the entire telemetry dataset to JSON for external analysis
//! #[cfg(feature = "telemetry")]
//! let json = store.to_json();
//! ```

mod bytecode_cost;
mod class_init_dag;
mod dispatch_resolution;
mod exception_flow;
mod native_boundary;
mod object_lineage;
mod store;

pub use bytecode_cost::{BytecodeCostStore, OpcodeStat};
pub use class_init_dag::{ClassInitDagStore, ClinitEvent};
pub use dispatch_resolution::{DispatchResolutionStore, DispatchStat};
pub use exception_flow::{ExceptionEvent, ExceptionFlowStore};
pub use native_boundary::{NativeBoundaryStore, NativeStat};
pub use object_lineage::{AllocationSite, ObjectLineageStore};
pub use store::TelemetryStore;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytecode_cost_records_count_and_site() {
        let mut store = BytecodeCostStore::default();
        store.record("iadd", "Foo", "bar", 10, 100);
        store.record("iadd", "Foo", "bar", 10, 200);
        assert_eq!(store.by_opcode["iadd"].count, 2);
        assert_eq!(store.by_opcode["iadd"].total_ns, 300);
        assert_eq!(
            store.by_site[&("Foo".to_string(), "bar".to_string(), 10)].count,
            2
        );
    }

    #[test]
    fn object_lineage_records_allocation_site() {
        let mut store = ObjectLineageStore::default();
        store.record("Foo", "main", 5, "java/lang/Object");
        store.record("Foo", "main", 5, "java/lang/Object");
        let site = &store.sites[&("Foo".to_string(), "main".to_string(), 5)];
        assert_eq!(site.count, 2);
        assert_eq!(site.class_allocated, "java/lang/Object");
    }

    #[test]
    fn exception_flow_records_throw_and_catch() {
        let mut store = ExceptionFlowStore::default();
        let idx = store.record_throw("RuntimeException", "Foo", "bar", 10);
        store.record_catch(idx, "Foo", "bar", 20);
        assert_eq!(
            store.events[0].catch_site,
            Some(("Foo".to_string(), "bar".to_string(), 20))
        );
    }

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

    #[test]
    fn native_boundary_records_errors() {
        let mut store = NativeBoundaryStore::default();
        store.record_call("java/lang/System", "exit", 50, false);
        store.record_call("java/lang/System", "exit", 30, true);
        let stat = &store.by_method[&("java/lang/System".to_string(), "exit".to_string())];
        assert_eq!(stat.calls, 2);
        assert_eq!(stat.errors, 1);
        assert_eq!(stat.total_ns, 80);
    }
}
