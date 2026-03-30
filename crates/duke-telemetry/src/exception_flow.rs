/// Represents the lifecycle of a single exception.
///
/// Records the exact bytecode location where an exception was originally thrown,
/// the class of the exception, and the site where it was eventually caught (if any).
///
/// # Examples
///
/// ```
/// use duke_telemetry::ExceptionEvent;
///
/// let ev = ExceptionEvent {
///     exception_class: "java/lang/NullPointerException".to_string(),
///     throw_site: ("Foo".to_string(), "bar".to_string(), 10),
///     catch_site: None,
///     rethrows: 0,
/// };
/// assert_eq!(ev.exception_class, "java/lang/NullPointerException");
/// assert!(ev.catch_site.is_none());
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ExceptionEvent {
    /// The class name of the thrown exception object.
    pub exception_class: String,
    /// Where the `athrow` or JVM-internal fault occurred (`class, method, pc`).
    pub throw_site: (String, String, usize),
    /// Where the exception was handled, if any (`class, method, pc`).
    /// `None` indicates the exception was uncaught and caused the thread to terminate.
    pub catch_site: Option<(String, String, usize)>,
    /// How many times this exception object was rethrown before being caught or escaping.
    pub rethrows: u32,
}

/// Tracks exception frequency and control flow.
///
/// Collects a chronological log of all exceptions thrown within the JVM,
/// allowing analysis of "exceptions used for control flow" anti-patterns.
///
/// # Examples
///
/// ```
/// use duke_telemetry::ExceptionFlowStore;
///
/// let mut store = ExceptionFlowStore::default();
///
/// // Record the initial throw
/// let id = store.record_throw("java/lang/Exception", "App", "main", 42);
///
/// // Update the record once the exception is caught
/// store.record_catch(id, "App", "main", 50);
///
/// assert_eq!(store.events[id].catch_site.as_ref().unwrap().2, 50);
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ExceptionFlowStore {
    /// A chronological log of every exception thrown during JVM execution.
    pub events: Vec<ExceptionEvent>,
}

impl ExceptionFlowStore {
    /// Log that a new exception has been thrown.
    ///
    /// - `exception_class`: The runtime class of the exception being thrown.
    /// - `class`: The class executing the throw.
    /// - `method`: The method executing the throw.
    /// - `pc`: The program counter of the throw instruction.
    ///
    /// Returns a unique index for this event, which must be passed to `record_catch`
    /// when the exception is finally handled.
    pub fn record_throw(
        &mut self,
        exception_class: &str,
        class: &str,
        method: &str,
        pc: usize,
    ) -> usize {
        let idx = self.events.len();
        self.events.push(ExceptionEvent {
            exception_class: exception_class.to_string(),
            throw_site: (class.to_string(), method.to_string(), pc),
            catch_site: None,
            rethrows: 0,
        });
        idx
    }

    /// Log that a previously thrown exception has been caught.
    ///
    /// - `idx`: The identifier returned by `record_throw`.
    /// - `class`: The class containing the `catch` block.
    /// - `method`: The method containing the `catch` block.
    /// - `pc`: The program counter of the first instruction in the catch handler.
    pub fn record_catch(&mut self, idx: usize, class: &str, method: &str, pc: usize) {
        if let Some(ev) = self.events.get_mut(idx) {
            ev.catch_site = Some((class.to_string(), method.to_string(), pc));
        }
    }
}
