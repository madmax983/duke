#![deny(missing_docs)]
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

pub(crate) mod bytecode_cost;
pub use bytecode_cost::*;
pub(crate) mod object_lineage;
pub use object_lineage::*;
pub(crate) mod class_init_dag;
pub use class_init_dag::*;
pub(crate) mod exception_flow;
pub use exception_flow::*;
pub(crate) mod dispatch_resolution;
pub use dispatch_resolution::*;
pub(crate) mod native_boundary;
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
    /// Creates a new, empty telemetry store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Serialize the entire telemetry dataset into a JSON string.
    ///
    /// # Panics
    ///
    /// Panics if serialization to JSON fails (which should never happen for these basic types).
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
    /// store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
    /// let mut buf = Vec::<u8>::new();
    /// store.print_report(&mut buf).unwrap();
    ///
    /// let output = String::from_utf8(buf).unwrap();
    /// assert!(output.contains("=== Duke VM Telemetry Report ==="));
    /// assert!(output.contains("iadd"));
    /// ```
    pub fn print_report(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(w, "=== Duke VM Telemetry Report ===")?;
        self.print_bytecode_cost(w)?;
        self.print_object_lineage(w)?;
        self.print_class_init_dag(w)?;
        self.print_exception_flow(w)?;
        self.print_dispatch_resolution(w)?;
        self.print_native_boundary(w)?;
        Ok(())
    }

    fn print_bytecode_cost(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(w, "\n-- bytecode_cost (top 10 by count) --")?;
        let mut ops: Vec<_> = self.bytecode_cost.by_opcode.iter().collect();
        ops.sort_by_key(|b| std::cmp::Reverse(b.1.count));
        for (name, stat) in ops.iter().take(10) {
            writeln!(w, "  {:20} count={:>10}", name, stat.count)?;
        }
        Ok(())
    }

    fn print_object_lineage(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
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
        Ok(())
    }

    fn print_class_init_dag(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        if self.class_init_dag.events.is_empty() {
            writeln!(w, "\nNo class initialization events recorded.")?;
            return Ok(());
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
        Ok(())
    }

    fn print_exception_flow(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        if self.exception_flow.events.is_empty() {
            writeln!(w, "\nNo exception flow events recorded.")?;
            return Ok(());
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
        Ok(())
    }

    fn print_dispatch_resolution(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
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
        Ok(())
    }

    fn print_native_boundary(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
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
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_telemetry::TelemetryStore;
    ///
    /// let mut store = TelemetryStore::default();
    /// store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
    /// store.class_init_dag.record("java/lang/String", "java/lang/System", 500);
    ///
    /// let md = store.to_markdown_report();
    /// assert!(md.contains("# Duke VM Telemetry Report"));
    /// assert!(md.contains("java/lang/System"));
    /// ```
    #[must_use]
    pub fn to_markdown_report(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        writeln!(&mut out, "# Duke VM Telemetry Report\n").unwrap();

        self.markdown_bytecode_cost(&mut out);
        self.markdown_object_lineage(&mut out);
        self.markdown_class_init_dag(&mut out);
        self.markdown_exception_flow(&mut out);
        self.markdown_dispatch_resolution(&mut out);
        self.markdown_native_boundary(&mut out);

        out
    }

    fn markdown_bytecode_cost(&self, out: &mut String) {
        use std::fmt::Write;
        writeln!(out, "## Bytecode Cost (Top 10)\n").unwrap();
        writeln!(out, "| Opcode | Count | Time (ns) |").unwrap();
        writeln!(out, "|--------|-------|-----------|").unwrap();
        let mut ops: Vec<_> = self.bytecode_cost.by_opcode.iter().collect();
        ops.sort_by_key(|b| std::cmp::Reverse(b.1.count));
        for (name, stat) in ops.iter().take(10) {
            writeln!(out, "| `{name}` | {} | {} |", stat.count, stat.total_ns).unwrap();
        }
        writeln!(out).unwrap();
    }

    fn markdown_object_lineage(&self, out: &mut String) {
        use std::fmt::Write;
        writeln!(out, "## Object Lineage (Top 10 Allocation Sites)\n").unwrap();
        writeln!(out, "| Location | Class Allocated | Count |").unwrap();
        writeln!(out, "|----------|-----------------|-------|").unwrap();
        let mut sites: Vec<_> = self.object_lineage.sites.iter().collect();
        sites.sort_by_key(|b| std::cmp::Reverse(b.1.count));
        for ((class, method, pc), site) in sites.iter().take(10) {
            writeln!(
                out,
                "| `{class}::{method}` @{pc} | `{}` | {} |",
                site.class_allocated, site.count
            )
            .unwrap();
        }
        writeln!(out).unwrap();
    }

    fn markdown_class_init_dag(&self, out: &mut String) {
        use std::fmt::Write;
        writeln!(out, "## Class Initialization DAG\n").unwrap();
        if self.class_init_dag.events.is_empty() {
            writeln!(out, "No class initialization events recorded.\n").unwrap();
        } else {
            writeln!(
                out,
                "```mermaid\n{}```\n",
                self.class_init_dag.to_mermaid().trim()
            )
            .unwrap();
        }
    }

    fn markdown_exception_flow(&self, out: &mut String) {
        use std::fmt::Write;
        writeln!(out, "## Exception Flow\n").unwrap();
        if self.exception_flow.events.is_empty() {
            writeln!(out, "No exception flow events recorded.\n").unwrap();
            return;
        }
        writeln!(out, "| Exception Class | Throw Site | Catch Site |").unwrap();
        writeln!(out, "|-----------------|------------|------------|").unwrap();
        for ev in &self.exception_flow.events {
            let throw = format!(
                "{}::{} @{}",
                ev.throw_site.0, ev.throw_site.1, ev.throw_site.2
            );
            let catch = ev.catch_site.as_ref().map_or_else(
                || "uncaught".to_string(),
                |(c, m, pc)| format!("{c}::{m} @{pc}"),
            );
            writeln!(out, "| `{}` | `{throw}` | `{catch}` |", ev.exception_class).unwrap();
        }
        writeln!(out).unwrap();
    }

    fn markdown_dispatch_resolution(&self, out: &mut String) {
        use std::fmt::Write;
        writeln!(out, "## Dispatch Resolution (Top 10 Virtual Call Sites)\n").unwrap();
        writeln!(out, "| Caller | Calls | Targets | Hierarchy Walks |").unwrap();
        writeln!(out, "|--------|-------|---------|-----------------|").unwrap();
        let mut dsites: Vec<_> = self.dispatch_resolution.by_site.iter().collect();
        dsites.sort_by_key(|b| std::cmp::Reverse(b.1.calls));
        for ((class, cp), stat) in dsites.iter().take(10) {
            writeln!(
                out,
                "| `{class}`[cp{cp}] | {} | {} | {} |",
                stat.calls,
                stat.unique_targets.len(),
                stat.hierarchy_walks
            )
            .unwrap();
        }
        writeln!(out).unwrap();
    }

    fn markdown_native_boundary(&self, out: &mut String) {
        use std::fmt::Write;
        writeln!(out, "## Native Boundary (Top 10 by Call Count)\n").unwrap();
        writeln!(out, "| Native Method | Calls | Errors | Time (ns) |").unwrap();
        writeln!(out, "|---------------|-------|--------|-----------|").unwrap();
        let mut natives: Vec<_> = self.native_boundary.by_method.iter().collect();
        natives.sort_by_key(|b| std::cmp::Reverse(b.1.calls));
        for ((class, method), stat) in natives.iter().take(10) {
            writeln!(
                out,
                "| `{class}.{method}` | {} | {} | {} |",
                stat.calls, stat.errors, stat.total_ns
            )
            .unwrap();
        }
        writeln!(out).unwrap();
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

        let md = store.to_markdown_report();
        assert!(md.contains("# Duke VM Telemetry Report"));
        assert!(md.contains("| `iadd` | 1 | 100 |"));
        assert!(md.contains(
            "```mermaid
"
        ));
        assert!(md.contains(
            "graph TD;
"
        ));
        assert!(md.contains("\"java/lang/System\" -->|500ns| \"java/lang/String\";"));
        assert!(md.contains("```"));
    }

    #[test]
    #[cfg(feature = "telemetry")]
    fn should_indicate_empty_class_initialization_in_markdown_report() {
        let empty_store = TelemetryStore::default();
        let empty_md = empty_store.to_markdown_report();
        assert!(empty_md.contains("No class initialization events recorded."));
    }

    #[test]
    #[cfg(feature = "telemetry")]
    fn test_print_report_empty() {
        let store = TelemetryStore::default();
        let mut buf = Vec::new();
        store.print_report(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("=== Duke VM Telemetry Report ==="));
        assert!(s.contains("No class initialization events recorded."));
        assert!(s.contains("No exception flow events recorded."));
    }

    #[test]
    #[cfg(feature = "telemetry")]
    fn should_correctly_format_print_report_with_populated_data() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .object_lineage
            .record("java/lang/String", "Foo", 10, "bar");
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);
        store
            .exception_flow
            .record_throw("java/lang/Exception", "Foo", "bar", 10);
        store
            .dispatch_resolution
            .record("Foo", 42, "java/lang/String", true);
        store
            .dispatch_resolution
            .record("Foo", 42, "java/lang/String", true);
        store
            .dispatch_resolution
            .record("Foo", 42, "java/lang/String", true);
        store
            .native_boundary
            .record_call("java/lang/String", "intern", 100, true);

        let mut buf = Vec::new();
        store.print_report(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();

        assert!(s.contains("=== Duke VM Telemetry Report ==="));
        assert!(s.contains("iadd                 count=         1"));
        assert!(s.contains("allocs 1 of bar"));
        assert!(s.contains("java/lang/String (triggered by: java/lang/System, 500ns)"));
        assert!(
            s.contains("java/lang/Exception thrown at (\"Foo\", \"bar\", 10) caught at uncaught")
        );
        assert!(s.contains("Foo[cp42] calls=3 targets=1 walks=3"));
        assert!(s.contains("java/lang/String.intern calls=1 errors=1"));
    }

    #[test]
    #[cfg(feature = "telemetry")]
    fn should_correctly_serialize_telemetry_store_to_json() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .object_lineage
            .record("java/lang/String", "Foo", 10, "bar");
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);
        store
            .exception_flow
            .record_throw("java/lang/Exception", "Foo", "bar", 10);
        store
            .dispatch_resolution
            .record("Foo", 42, "java/lang/String", true);
        store
            .native_boundary
            .record_call("java/lang/String", "intern", 100, true);

        let json = store.to_json();
        assert!(json.contains("\"bytecode_cost\""));
        assert!(json.contains("\"iadd\""));
        assert!(json.contains("\"object_lineage\""));
        assert!(json.contains("\"java/lang/String::Foo@10\""));
        assert!(json.contains("\"class_init_dag\""));
        assert!(json.contains("\"java/lang/String\""));
        assert!(json.contains("\"exception_flow\""));
        assert!(json.contains("\"java/lang/Exception\""));
        assert!(json.contains("\"dispatch_resolution\""));
        assert!(json.contains("\"Foo@42\""));
        assert!(json.contains("\"native_boundary\""));
        assert!(json.contains("\"java/lang/String::intern\""));
    }

    #[test]
    #[cfg(feature = "telemetry")]
    fn test_print_report_io_error() {
        struct FailingWriter;
        impl std::io::Write for FailingWriter {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .object_lineage
            .record("java/lang/String", "Foo", 10, "bar");
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);
        store
            .exception_flow
            .record_throw("java/lang/Exception", "Foo", "bar", 10);
        store
            .dispatch_resolution
            .record("Foo", 42, "java/lang/String", true);
        store
            .native_boundary
            .record_call("java/lang/String", "intern", 100, true);

        let mut w = FailingWriter;
        let res = store.print_report(&mut w);
        assert!(res.is_err());
    }

    #[test]
    fn test_print_bytecode_cost_with_less_than_10() {
        let mut store = crate::TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        let mut buf = Vec::new();
        store.print_bytecode_cost(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("iadd"));
    }

    #[test]
    fn test_print_object_lineage_with_less_than_10() {
        let mut store = crate::TelemetryStore::default();
        store
            .object_lineage
            .record("java/lang/String", "Foo", 10, "bar");
        let mut buf = Vec::new();
        store.print_object_lineage(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("java/lang/String"));
    }

    #[test]
    fn test_print_class_init_dag_populated() {
        let mut store = crate::TelemetryStore::default();
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);
        let mut buf = Vec::new();
        store.print_class_init_dag(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("java/lang/String"));
    }

    #[test]
    fn test_print_exception_flow_populated() {
        let mut store = crate::TelemetryStore::default();
        store
            .exception_flow
            .record_throw("java/lang/Exception", "Foo", "bar", 10);
        let mut buf = Vec::new();
        store.print_exception_flow(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("java/lang/Exception"));
    }
}
