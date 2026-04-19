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

pub(crate) mod helpers;

pub mod bytecode_cost;
pub use bytecode_cost::*;
pub mod object_lineage;
pub use object_lineage::*;
pub mod class_init_dag;
pub use class_init_dag::*;
pub mod exception_flow;
pub use exception_flow::*;
pub mod dispatch_resolution;
pub use dispatch_resolution::*;
pub mod native_boundary;
pub use native_boundary::*;

// -- TelemetryStore --------------------------------------------------------------

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
/// let mut store = TelemetryStore::default();
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
    /// let mut store = TelemetryStore::default();
    /// #[cfg(feature = "telemetry")]
    /// let json = store.to_json();
    /// ```
    #[must_use]
    #[cfg(feature = "telemetry")]
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
    /// let mut store = TelemetryStore::default();
    /// let mut buf = Vec::<u8>::new();
    /// #[cfg(feature = "telemetry")]
    /// store.print_report(&mut buf).unwrap();
    /// ```
    pub fn print_report(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(w, "=== Duke VM Telemetry Report ===")?;

        writeln!(w, "\n-- bytecode_cost (top 10 by count) --")?;
        let mut ops: Vec<_> = self.bytecode_cost.by_opcode.iter().collect();
        ops.sort_by_key(|b| std::cmp::Reverse(b.1.count));
        for (name, stat) in ops.iter().take(10) {
            writeln!(w, "  {:20} count={:>10}", name, stat.count)?;
        }

        writeln!(
            w,
            "\n-- object_lineage (top 10 allocation sites by count) --"
        )?;
        let mut sites: Vec<_> = self.object_lineage.sites.iter().collect();
        sites.sort_by_key(|b| std::cmp::Reverse(b.1.count));
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
        dsites.sort_by_key(|b| std::cmp::Reverse(b.1.calls));
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
        natives.sort_by_key(|b| std::cmp::Reverse(b.1.calls));
        for ((class, method), stat) in natives.iter().take(10) {
            writeln!(
                w,
                "  {}.{} calls={} errors={}",
                class, method, stat.calls, stat.errors
            )?;
        }

        Ok(())
    }

    /// Generate a Markdown report containing all telemetry data and a visualized Mermaid graph.
    ///
    /// This exporter provides a GitHub-flavored Markdown representation of the telemetry
    /// data, making it easy to paste into PRs or issues for performance analysis.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn to_markdown_report(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        writeln!(
            &mut out,
            "# Duke VM Telemetry Report
"
        )
        .unwrap();

        // Bytecode Cost
        writeln!(
            &mut out,
            "## Bytecode Cost (Top 10)
"
        )
        .unwrap();
        writeln!(&mut out, "| Opcode | Count | Time (ns) |").unwrap();
        writeln!(&mut out, "|--------|-------|-----------|").unwrap();
        let mut ops: Vec<_> = self.bytecode_cost.by_opcode.iter().collect();
        ops.sort_by_key(|b| std::cmp::Reverse(b.1.count));
        for (name, stat) in ops.iter().take(10) {
            writeln!(
                &mut out,
                "| `{name}` | {} | {} |",
                stat.count, stat.total_ns
            )
            .unwrap();
        }
        writeln!(&mut out).unwrap();

        // Object Lineage
        writeln!(
            &mut out,
            "## Object Lineage (Top 10 Allocation Sites)
"
        )
        .unwrap();
        writeln!(&mut out, "| Location | Class Allocated | Count |").unwrap();
        writeln!(&mut out, "|----------|-----------------|-------|").unwrap();
        let mut sites: Vec<_> = self.object_lineage.sites.iter().collect();
        sites.sort_by_key(|b| std::cmp::Reverse(b.1.count));
        for ((class, method, pc), site) in sites.iter().take(10) {
            writeln!(
                &mut out,
                "| `{class}::{method}` @{pc} | `{}` | {} |",
                site.class_allocated, site.count
            )
            .unwrap();
        }
        writeln!(&mut out).unwrap();

        // Class Init Dag Mermaid
        writeln!(
            &mut out,
            "## Class Initialization DAG
"
        )
        .unwrap();
        if self.class_init_dag.events.is_empty() {
            writeln!(
                &mut out,
                "No class initialization events recorded.
"
            )
            .unwrap();
        } else {
            writeln!(
                &mut out,
                "```mermaid
{}```
",
                self.class_init_dag.to_mermaid().trim()
            )
            .unwrap();
        }

        // Exception Flow
        writeln!(
            &mut out,
            "## Exception Flow
"
        )
        .unwrap();
        writeln!(&mut out, "| Exception Class | Throw Site | Catch Site |").unwrap();
        writeln!(&mut out, "|-----------------|------------|------------|").unwrap();
        for ev in &self.exception_flow.events {
            let throw = format!(
                "{}::{} @{}",
                ev.throw_site.0, ev.throw_site.1, ev.throw_site.2
            );
            let catch = ev.catch_site.as_ref().map_or_else(
                || "uncaught".to_string(),
                |(c, m, pc)| format!("{c}::{m} @{pc}"),
            );
            writeln!(
                &mut out,
                "| `{}` | `{throw}` | `{catch}` |",
                ev.exception_class
            )
            .unwrap();
        }
        writeln!(&mut out).unwrap();

        // Dispatch Resolution
        writeln!(
            &mut out,
            "## Dispatch Resolution (Top 10 Virtual Call Sites)
"
        )
        .unwrap();
        writeln!(&mut out, "| Caller | Calls | Targets | Hierarchy Walks |").unwrap();
        writeln!(&mut out, "|--------|-------|---------|-----------------|").unwrap();
        let mut dsites: Vec<_> = self.dispatch_resolution.by_site.iter().collect();
        dsites.sort_by_key(|b| std::cmp::Reverse(b.1.calls));
        for ((class, cp), stat) in dsites.iter().take(10) {
            writeln!(
                &mut out,
                "| `{class}`[cp{cp}] | {} | {} | {} |",
                stat.calls,
                stat.unique_targets.len(),
                stat.hierarchy_walks
            )
            .unwrap();
        }
        writeln!(&mut out).unwrap();

        // Native Boundary
        writeln!(
            &mut out,
            "## Native Boundary (Top 10 by Call Count)
"
        )
        .unwrap();
        writeln!(&mut out, "| Native Method | Calls | Errors | Time (ns) |").unwrap();
        writeln!(&mut out, "|---------------|-------|--------|-----------|").unwrap();
        let mut natives: Vec<_> = self.native_boundary.by_method.iter().collect();
        natives.sort_by_key(|b| std::cmp::Reverse(b.1.calls));
        for ((class, method), stat) in natives.iter().take(10) {
            writeln!(
                &mut out,
                "| `{class}.{method}` | {} | {} | {} |",
                stat.calls, stat.errors, stat.total_ns
            )
            .unwrap();
        }
        writeln!(&mut out).unwrap();

        out
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "telemetry")]
    use crate::TelemetryStore;

    #[test]
    #[cfg(feature = "telemetry")]
    fn telemetry_store_to_markdown_report() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);
        store
            .object_lineage
            .record("com/Example", "method", 1, "java/lang/Object");
        let ev = store.exception_flow.record_throw(
            "java/lang/Exception",
            "ThrowClass",
            "ThrowMethod",
            1,
        );
        store
            .exception_flow
            .record_catch(ev, "CatchClass", "CatchMethod", 2);
        store
            .dispatch_resolution
            .record("CallerClass", 1, "CallerMethod", false);
        store
            .native_boundary
            .record_call("java/lang/System", "out", 500, false);

        let md = store.to_markdown_report();
        assert!(md.contains("# Duke VM Telemetry Report"));
        assert!(md.contains("| `iadd` | 1 | 100 |"));
        assert!(md.contains("```mermaid\n"));
        assert!(md.contains("graph TD;\n"));
        assert!(md.contains("\"java/lang/System\" -->|500ns| \"java/lang/String\";"));
        assert!(md.contains("```"));
        assert!(md.contains("com/Example::method @1"));
        assert!(md.contains("java/lang/Exception"));
        assert!(md.contains("CallerClass::CallerMethod"));
        assert!(md.contains("java/lang/System::out"));
    }

    #[test]
    #[cfg(feature = "telemetry")]
    fn telemetry_store_print_report() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .object_lineage
            .record("com/Example", "method", 1, "java/lang/Object");
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);
        let ev = store.exception_flow.record_throw(
            "java/lang/Exception",
            "ThrowClass",
            "ThrowMethod",
            1,
        );
        store
            .exception_flow
            .record_catch(ev, "CatchClass", "CatchMethod", 2);
        store
            .dispatch_resolution
            .record("CallerClass", 1, "CallerMethod", false);
        store
            .native_boundary
            .record_call("java/lang/System", "out", 500, false);
        let mut buf = Vec::new();
        store.print_report(&mut buf).unwrap();
        let report = String::from_utf8(buf).unwrap();
        assert!(report.contains("=== Duke VM Telemetry Report ==="));
        assert!(report.contains("iadd"));
        assert!(report.contains("com/Example"));
        assert!(report.contains("java/lang/String"));
        assert!(report.contains("java/lang/Exception"));
        assert!(report.contains("CallerClass"));
        assert!(report.contains("java/lang/System"));
    }

    #[test]
    fn test_print_report() {
        let mut store = crate::TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .object_lineage
            .record("com/Example", "method", 1, "java/lang/Object");
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);
        let ev = store.exception_flow.record_throw(
            "java/lang/Exception",
            "ThrowClass",
            "ThrowMethod",
            1,
        );
        store
            .exception_flow
            .record_catch(ev, "CatchClass", "CatchMethod", 2);
        store
            .dispatch_resolution
            .record("CallerClass", 1, "CallerMethod", false);
        store
            .native_boundary
            .record_call("java/lang/System", "out", 500, false);
        let mut buf = Vec::new();
        store.print_report(&mut buf).unwrap();
        let report = String::from_utf8(buf).unwrap();
        assert!(report.contains("=== Duke VM Telemetry Report ==="));
        assert!(report.contains("iadd"));
        assert!(report.contains("com/Example"));
        assert!(report.contains("java/lang/String"));
        assert!(report.contains("java/lang/Exception"));
        assert!(report.contains("CallerClass"));
        assert!(report.contains("java/lang/System"));
    }

    #[test]
    fn test_to_markdown_report() {
        let mut store = crate::TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .object_lineage
            .record("com/Example", "method", 1, "java/lang/Object");
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);
        let ev = store.exception_flow.record_throw(
            "java/lang/Exception",
            "ThrowClass",
            "ThrowMethod",
            1,
        );
        store
            .exception_flow
            .record_catch(ev, "CatchClass", "CatchMethod", 2);
        store
            .dispatch_resolution
            .record("CallerClass", 1, "CallerMethod", false);
        store
            .native_boundary
            .record_call("java/lang/System", "out", 500, false);
        let md = store.to_markdown_report();
        assert!(md.contains("# Duke VM Telemetry Report"));
        assert!(md.contains("iadd"));
        assert!(md.contains("com/Example"));
        assert!(md.contains("java/lang/String"));
        assert!(md.contains("java/lang/Exception"));
        assert!(md.contains("CallerClass"));
        assert!(md.contains("java/lang/System"));
    }

    #[test]
    fn test_print_report_empty() {
        let store = crate::TelemetryStore::default();
        let mut buf = Vec::new();
        store.print_report(&mut buf).unwrap();
        let report = String::from_utf8(buf).unwrap();
        assert!(report.contains("=== Duke VM Telemetry Report ==="));
    }

    #[test]
    fn test_to_markdown_report_empty() {
        let store = crate::TelemetryStore::default();
        let md = store.to_markdown_report();
        assert!(md.contains("# Duke VM Telemetry Report"));
    }





    #[test]
    #[cfg(feature = "telemetry")]
    fn test_to_json() {
        let mut store = crate::TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        let json = store.to_json();
        assert!(json.contains("iadd"));
        assert!(json.contains("bytecode_cost"));
    }

}