//! CSV Exporter for `TelemetryStore`.

use crate::TelemetryStore;
use std::fmt::Write;

/// Exports the native boundary telemetry as CSV.
#[cfg(feature = "nova")]
#[must_use]
pub fn export_native_boundary_csv(store: &TelemetryStore) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "class_name,method_name,calls,errors,total_ns");

    let mut natives: Vec<_> = store.native_boundary.by_method.iter().collect();
    natives.sort_by_key(|b| std::cmp::Reverse(b.1.calls));

    for ((class, method), stat) in natives {
        let _ = writeln!(
            out,
            "{},{},{},{},{}",
            class, method, stat.calls, stat.errors, stat.total_ns
        );
    }
    out
}

/// Exports bytecode cost telemetry as CSV.
#[cfg(feature = "nova")]
#[must_use]
pub fn export_bytecode_cost_csv(store: &TelemetryStore) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "class_name,method_name,pc,count,total_ns");

    let mut sites: Vec<_> = store.bytecode_cost.by_site.iter().collect();
    sites.sort_by_key(|a| std::cmp::Reverse(a.1.count));

    for ((class, method, pc), stat) in sites {
        let _ = writeln!(
            out,
            "{},{},{},{},{}",
            class, method, pc, stat.count, stat.total_ns
        );
    }
    out
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_export_native_boundary_csv() {
        let mut store = TelemetryStore::default();
        store
            .native_boundary
            .record_call("java/lang/String", "intern", 100, true);
        store
            .native_boundary
            .record_call("java/lang/String", "intern", 200, false);

        let csv = export_native_boundary_csv(&store);
        assert!(csv.contains("class_name,method_name,calls,errors,total_ns\n"));
        assert!(csv.contains("java/lang/String,intern,2,1,300\n"));
    }

    #[test]
    fn test_export_bytecode_cost_csv() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Math", "add", 0, 150);
        let csv = export_bytecode_cost_csv(&store);
        assert!(csv.contains("class_name,method_name,pc,count,total_ns\n"));
        assert!(csv.contains("Math,add,0,1,150\n"));
    }
}
