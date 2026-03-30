use crate::bytecode_cost::BytecodeCostStore;
use crate::class_init_dag::ClassInitDagStore;
use crate::dispatch_resolution::DispatchResolutionStore;
use crate::exception_flow::ExceptionFlowStore;
use crate::native_boundary::NativeBoundaryStore;
use crate::object_lineage::ObjectLineageStore;

#[cfg(feature = "telemetry")]
pub mod ser_helpers {
    use serde::Serialize;
    use std::collections::HashMap;

    /// Core helper: serialize any `HashMap<K, V>` by formatting each key with `key_fn`.
    pub fn keyed_map<K, V, S, F>(map: &HashMap<K, V>, ser: S, key_fn: F) -> Result<S::Ok, S::Error>
    where
        K: Eq + std::hash::Hash,
        V: Serialize,
        S: serde::Serializer,
        F: Fn(&K) -> String,
    {
        let string_map: HashMap<String, &V> = map.iter().map(|(k, v)| (key_fn(k), v)).collect();
        string_map.serialize(ser)
    }

    /// `HashMap<(class, method, pc), V>` → `"class::method@pc"`.
    pub fn site3<V: Serialize, S: serde::Serializer>(
        map: &HashMap<(String, String, usize), V>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        keyed_map(map, ser, |(c, m, pc)| format!("{c}::{m}@{pc}"))
    }

    /// `HashMap<(class, cp_idx), V>` → `"class@cp"`.
    pub fn site2_u16<V: Serialize, S: serde::Serializer>(
        map: &HashMap<(String, u16), V>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        keyed_map(map, ser, |(c, cp)| format!("{c}@{cp}"))
    }

    /// `HashMap<(class, method), V>` → `"class::method"`.
    pub fn pair_str<V: Serialize, S: serde::Serializer>(
        map: &HashMap<(String, String), V>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        keyed_map(map, ser, |(c, m)| format!("{c}::{m}"))
    }

    /// Serialize `HashSet<String>` as a sorted `Vec<String>` for deterministic output.
    pub fn sorted_set<S: serde::Serializer>(
        set: &std::collections::HashSet<String>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        let mut v: Vec<&String> = set.iter().collect();
        v.sort();
        v.serialize(ser)
    }
}

/// The root telemetry store.
///
/// Combines the six core telemetry channels into a single object, allowing
/// the VM to pass it around, update it during execution, and serialize
/// it into a unified report at shutdown.
///
/// # Examples
///
/// ```
/// use duke_telemetry::TelemetryStore;
///
/// let store = TelemetryStore::default();
/// assert_eq!(store.class_init_dag.events.len(), 0);
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct TelemetryStore {
    /// Metrics on bytecode execution costs.
    pub bytecode_cost: BytecodeCostStore,
    /// Memory profiling and allocation site tracing.
    pub object_lineage: ObjectLineageStore,
    /// Dependencies and durations of `<clinit>` executions.
    pub class_init_dag: ClassInitDagStore,
    /// Tracing for thrown exceptions and their catch blocks.
    pub exception_flow: ExceptionFlowStore,
    /// Analysis of virtual method calls and polymorphism overhead.
    pub dispatch_resolution: DispatchResolutionStore,
    /// Metrics on crossing the JNI/native boundary.
    pub native_boundary: NativeBoundaryStore,
}

#[cfg(feature = "telemetry")]
impl TelemetryStore {
    /// Serialize the store to pretty-printed JSON.
    ///
    /// # Panics
    ///
    /// Panics if telemetry serialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_telemetry::TelemetryStore;
    ///
    /// let store = TelemetryStore::default();
    /// #[cfg(feature = "telemetry")]
    /// let json = store.to_json();
    /// ```
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("telemetry serialization failed")
    }

    /// Print a human-readable top-10 summary per channel to `w`.
    ///
    /// Useful for displaying brief telemetry summaries to `stdout` or `stderr`
    /// at the end of a VM run.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to `w` fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_telemetry::TelemetryStore;
    ///
    /// let store = TelemetryStore::default();
    /// let mut buf = Vec::new();
    /// #[cfg(feature = "telemetry")]
    /// store.print_report(&mut buf).unwrap();
    /// ```
    pub fn print_report(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(w, "=== Duke VM Telemetry Report ===")?;

        writeln!(w, "\n-- bytecode_cost (top 10 by count) --")?;
        let mut ops: Vec<_> = self.bytecode_cost.by_opcode.iter().collect();
        ops.sort_by(|a, b| b.1.count.cmp(&a.1.count));
        for (name, stat) in ops.iter().take(10) {
            writeln!(w, "  {:20} count={:>10}", name, stat.count)?;
        }

        writeln!(
            w,
            "\n-- object_lineage (top 10 allocation sites by count) --"
        )?;
        let mut sites: Vec<_> = self.object_lineage.sites.iter().collect();
        sites.sort_by(|a, b| b.1.count.cmp(&a.1.count));
        for ((class, method, pc), site) in sites.iter().take(10) {
            writeln!(
                w,
                "  {}::{} @{} allocs {} of {}",
                class, method, pc, site.count, site.class_allocated
            )?;
        }

        writeln!(
            w,
            "\n-- class_init_dag ({} clinit events) --",
            self.class_init_dag.events.len()
        )?;
        for ev in &self.class_init_dag.events {
            writeln!(
                w,
                "  {} (triggered by: {}, {}ns)",
                ev.class, ev.triggered_by, ev.duration_ns
            )?;
        }

        writeln!(
            w,
            "\n-- exception_flow ({} throw events) --",
            self.exception_flow.events.len()
        )?;
        for ev in &self.exception_flow.events {
            let catch = ev.catch_site.as_ref().map_or_else(
                || "uncaught".to_string(),
                |(c, m, pc)| format!("{c}::{m} @{pc}"),
            );
            writeln!(
                w,
                "  {} thrown at {:?} caught at {}",
                ev.exception_class, ev.throw_site, catch
            )?;
        }

        writeln!(w, "\n-- dispatch_resolution (top 10 virtual call sites) --")?;
        let mut dsites: Vec<_> = self.dispatch_resolution.by_site.iter().collect();
        dsites.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));
        for ((class, cp), stat) in dsites.iter().take(10) {
            writeln!(
                w,
                "  {}[cp{}] calls={} targets={} walks={}",
                class,
                cp,
                stat.calls,
                stat.unique_targets.len(),
                stat.hierarchy_walks
            )?;
        }

        writeln!(w, "\n-- native_boundary (top 10 by call count) --")?;
        let mut natives: Vec<_> = self.native_boundary.by_method.iter().collect();
        natives.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));
        for ((class, method), stat) in natives.iter().take(10) {
            writeln!(
                w,
                "  {}.{} calls={} errors={}",
                class, method, stat.calls, stat.errors
            )?;
        }

        Ok(())
    }
}
