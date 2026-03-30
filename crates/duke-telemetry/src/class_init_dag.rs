/// Represents the initialization of a single JVM class (`<clinit>`).
///
/// Records which class forced this initialization and how long it took, allowing
/// for offline reconstruction of the class initialization DAG.
///
/// # Examples
///
/// ```
/// use duke_telemetry::ClinitEvent;
///
/// let ev = ClinitEvent {
///     class: "Child".to_string(),
///     triggered_by: "Parent".to_string(),
///     duration_ns: 1000,
/// };
/// assert_eq!(ev.duration_ns, 1000);
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ClinitEvent {
    /// The class being initialized.
    pub class: String,
    /// The class whose execution or instantiation forced this initialization.
    pub triggered_by: String,
    /// The time taken to execute the `<clinit>` method, in nanoseconds.
    pub duration_ns: u64,
}

/// Tracks class initialization sequences and startup overhead.
///
/// Captures a chronologically ordered log of class initializations, useful for
/// identifying slow `static {}` blocks and diagnosing circular initialization
/// deadlocks.
///
/// # Examples
///
/// ```
/// use duke_telemetry::ClassInitDagStore;
///
/// let mut store = ClassInitDagStore::default();
/// store.record("java/lang/System", "java/lang/Object", 1500);
///
/// assert_eq!(store.events.len(), 1);
/// assert_eq!(store.events[0].class, "java/lang/System");
/// assert_eq!(store.events[0].duration_ns, 1500);
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ClassInitDagStore {
    /// Chronological list of class initialization events.
    pub events: Vec<ClinitEvent>,
}

impl ClassInitDagStore {
    /// Record a completed class initialization.
    ///
    /// - `class`: The name of the class that was initialized.
    /// - `triggered_by`: The context that caused the initialization (e.g., another class name).
    /// - `duration_ns`: The total time spent in the `<clinit>` block.
    pub fn record(&mut self, class: &str, triggered_by: &str, duration_ns: u64) {
        self.events.push(ClinitEvent {
            class: class.to_string(),
            triggered_by: triggered_by.to_string(),
            duration_ns,
        });
    }
}
