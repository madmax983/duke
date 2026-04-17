//! A linear log of all `<clinit>` executions, tracking their dependencies.
//!
// -- class_init_dag --------------------------------------------------------------

/// A single class initialization (`<clinit>`) event in the VM.
///
/// Records the class name, what triggered its initialization, and how long
/// the initialization took to execute.
///
/// # Examples
///
/// ```
/// use duke_telemetry::ClinitEvent;
///
/// let event = ClinitEvent {
///     class: "java/lang/String".to_string(),
///     triggered_by: "java/lang/System".to_string(),
///     duration_ns: 1000,
/// };
/// assert_eq!(event.duration_ns, 1000);
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ClinitEvent {
    /// The class whose `<clinit>` method was executed.
    pub class: String,
    /// Class that caused this `<clinit>` to fire, or empty string for entry point.
    pub triggered_by: String,
    /// Wall-clock time spent in the `<clinit>` block, in nanoseconds.
    pub duration_ns: u64,
}

/// A linear log of all `<clinit>` executions, tracking their dependencies.
///
/// Captures the directed acyclic graph (DAG) of class initializations, allowing
/// analysis of slow startup times due to heavy static initializers and dependency chains.
///
/// # Examples
///
/// ```
/// use duke_telemetry::ClassInitDagStore;
///
/// let mut store = ClassInitDagStore::default();
/// store.record("java/lang/String", "java/lang/System", 500);
///
/// assert_eq!(store.events.len(), 1);
/// assert_eq!(store.events[0].class, "java/lang/String");
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ClassInitDagStore {
    /// Ordered list of initialization events.
    pub events: Vec<ClinitEvent>,
}

impl ClassInitDagStore {
    /// Record the execution of a class `<clinit>` method.
    ///
    /// - `class`: The JVM name of the class that was initialized.
    /// - `triggered_by`: The name of the class whose execution triggered this initialization.
    /// - `duration_ns`: Total time spent executing the `<clinit>` method.
    pub fn record(&mut self, class: &str, triggered_by: &str, duration_ns: u64) {
        self.events.push(ClinitEvent {
            class: class.to_string(),
            triggered_by: triggered_by.to_string(),
            duration_ns,
        });
    }

    /// Export the initialization DAG to a Graphviz DOT format string.
    #[must_use]
    pub fn to_dot(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        writeln!(&mut out, "digraph ClassInitDag {{").unwrap();
        for ev in &self.events {
            let triggered_by = if ev.triggered_by.is_empty() {
                "<entry>"
            } else {
                &ev.triggered_by
            };
            writeln!(
                &mut out,
                "    \"{}\" -> \"{}\" [label=\"{}ns\"];",
                triggered_by, ev.class, ev.duration_ns
            )
            .unwrap();
        }
        writeln!(&mut out, "}}").unwrap();
        out
    }

    /// Export the initialization DAG to a Mermaid flowchart string.
    #[must_use]
    pub fn to_mermaid(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        writeln!(&mut out, "graph TD;").unwrap();
        for ev in &self.events {
            let triggered_by = if ev.triggered_by.is_empty() {
                "<entry>"
            } else {
                &ev.triggered_by
            };
            writeln!(
                &mut out,
                "    \"{}\" -->|{}ns| \"{}\";",
                triggered_by, ev.duration_ns, ev.class
            )
            .unwrap();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_init_dag_to_dot() {
        let mut store = ClassInitDagStore::default();
        store.record("java/lang/String", "java/lang/System", 500);
        store.record("java/lang/Object", "", 100);

        let dot = store.to_dot();
        assert!(dot.contains("digraph ClassInitDag {"));
        assert!(dot.contains("\"java/lang/System\" -> \"java/lang/String\" [label=\"500ns\"];"));
        assert!(dot.contains("\"<entry>\" -> \"java/lang/Object\" [label=\"100ns\"];"));
        assert!(dot.contains('}'));
    }

    #[test]
    fn class_init_dag_to_mermaid() {
        let mut store = ClassInitDagStore::default();
        store.record("java/lang/String", "java/lang/System", 500);
        store.record("java/lang/Object", "", 100);

        let mermaid = store.to_mermaid();
        assert!(mermaid.contains("graph TD;"));
        assert!(mermaid.contains("\"java/lang/System\" -->|500ns| \"java/lang/String\";"));
        assert!(mermaid.contains("\"<entry>\" -->|100ns| \"java/lang/Object\";"));
    }
}
