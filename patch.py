import re

with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    content = f.read()

# I will apply the to_markdown_report method right after print_report
# in impl TelemetryStore

new_method = """
    /// Generate a Markdown report containing all telemetry data and a visualized Mermaid graph.
    ///
    /// This exporter provides a GitHub-flavored Markdown representation of the telemetry
    /// data, making it easy to paste into PRs or issues for performance analysis.
    #[must_use]
    pub fn to_markdown_report(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        writeln!(&mut out, "# Duke VM Telemetry Report\\n").unwrap();

        // Bytecode Cost
        writeln!(&mut out, "## Bytecode Cost (Top 10)\\n").unwrap();
        writeln!(&mut out, "| Opcode | Count | Time (ns) |").unwrap();
        writeln!(&mut out, "|--------|-------|-----------|").unwrap();
        let mut ops: Vec<_> = self.bytecode_cost.by_opcode.iter().collect();
        ops.sort_by(|a, b| b.1.count.cmp(&a.1.count));
        for (name, stat) in ops.iter().take(10) {
            writeln!(&mut out, "| `{name}` | {} | {} |", stat.count, stat.total_ns).unwrap();
        }
        writeln!(&mut out, "").unwrap();

        // Object Lineage
        writeln!(&mut out, "## Object Lineage (Top 10 Allocation Sites)\\n").unwrap();
        writeln!(&mut out, "| Location | Class Allocated | Count |").unwrap();
        writeln!(&mut out, "|----------|-----------------|-------|").unwrap();
        let mut sites: Vec<_> = self.object_lineage.sites.iter().collect();
        sites.sort_by(|a, b| b.1.count.cmp(&a.1.count));
        for ((class, method, pc), site) in sites.iter().take(10) {
            writeln!(&mut out, "| `{class}::{method}` @{pc} | `{}` | {} |", site.class_allocated, site.count).unwrap();
        }
        writeln!(&mut out, "").unwrap();

        // Class Init Dag Mermaid
        writeln!(&mut out, "## Class Initialization DAG\\n").unwrap();
        if self.class_init_dag.events.is_empty() {
            writeln!(&mut out, "No class initialization events recorded.\\n").unwrap();
        } else {
            writeln!(&mut out, "```mermaid\\n{}```\\n", self.class_init_dag.to_mermaid().trim()).unwrap();
        }

        // Exception Flow
        writeln!(&mut out, "## Exception Flow\\n").unwrap();
        writeln!(&mut out, "| Exception Class | Throw Site | Catch Site |").unwrap();
        writeln!(&mut out, "|-----------------|------------|------------|").unwrap();
        for ev in &self.exception_flow.events {
            let throw = format!("{}::{} @{}", ev.throw_site.0, ev.throw_site.1, ev.throw_site.2);
            let catch = ev.catch_site.as_ref().map_or_else(
                || "uncaught".to_string(),
                |(c, m, pc)| format!("{c}::{m} @{pc}"),
            );
            writeln!(&mut out, "| `{}` | `{throw}` | `{catch}` |", ev.exception_class).unwrap();
        }
        writeln!(&mut out, "").unwrap();

        // Dispatch Resolution
        writeln!(&mut out, "## Dispatch Resolution (Top 10 Virtual Call Sites)\\n").unwrap();
        writeln!(&mut out, "| Caller | Calls | Targets | Hierarchy Walks |").unwrap();
        writeln!(&mut out, "|--------|-------|---------|-----------------|").unwrap();
        let mut dsites: Vec<_> = self.dispatch_resolution.by_site.iter().collect();
        dsites.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));
        for ((class, cp), stat) in dsites.iter().take(10) {
            writeln!(&mut out, "| `{class}`[cp{cp}] | {} | {} | {} |", stat.calls, stat.unique_targets.len(), stat.hierarchy_walks).unwrap();
        }
        writeln!(&mut out, "").unwrap();

        // Native Boundary
        writeln!(&mut out, "## Native Boundary (Top 10 by Call Count)\\n").unwrap();
        writeln!(&mut out, "| Native Method | Calls | Errors | Time (ns) |").unwrap();
        writeln!(&mut out, "|---------------|-------|--------|-----------|").unwrap();
        let mut natives: Vec<_> = self.native_boundary.by_method.iter().collect();
        natives.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));
        for ((class, method), stat) in natives.iter().take(10) {
            writeln!(&mut out, "| `{class}.{method}` | {} | {} | {} |", stat.calls, stat.errors, stat.total_ns).unwrap();
        }
        writeln!(&mut out, "").unwrap();

        out
    }
}
"""

content = re.sub(r'        Ok\(\(\)\)\n    }\n\}', r'        Ok(())\n    }\n' + new_method, content)

test_code = """
    #[test]
    #[cfg(feature = "telemetry")]
    fn telemetry_store_to_markdown_report() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store.class_init_dag.record("java/lang/String", "java/lang/System", 500);

        let md = store.to_markdown_report();
        assert!(md.contains("# Duke VM Telemetry Report"));
        assert!(md.contains("| `iadd` | 1 | 100 |"));
        assert!(md.contains("```mermaid\\ngraph TD;\\n    \\\"java/lang/System\\\" -->|500ns| \\\"java/lang/String\\\";\\n```"));
    }
"""

content = re.sub(r'    fn native_boundary_records_errors\(\) \{([^}]+)\}\n\}', r'    fn native_boundary_records_errors() {\1}' + test_code + r'\n}', content)


with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(content)
