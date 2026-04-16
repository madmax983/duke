//! Exception lifecycle and control flow tracking.
//!
//! This module traces the path of exceptions from their throw site to their catch site,
//! including unhandled exceptions that terminate execution.

// -- exception_flow --------------------------------------------------------------

/// Lifecycle event for an exception thrown by the VM.
///
/// Records the class of the exception, where it was thrown, and where it was
/// eventually caught (if at all). Also tracks the number of times it was re-thrown.
///
/// # Examples
///
/// ```
/// use duke_telemetry::ExceptionEvent;
///
/// let event = ExceptionEvent {
///     exception_class: "java/lang/RuntimeException".to_string(),
///     throw_site: ("com/example/Main".to_string(), "run".to_string(), 10),
///     catch_site: Some(("com/example/Main".to_string(), "run".to_string(), 20)),
///     rethrows: 0,
/// };
/// assert_eq!(event.exception_class, "java/lang/RuntimeException");
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ExceptionEvent {
    /// The class name of the thrown exception object.
    pub exception_class: String,
    /// (`class_name`, `method_name`, pc) of the throw site.
    pub throw_site: (String, String, usize),
    /// (`class_name`, `method_name`, `handler_pc`) of the catch site, or `None` if uncaught.
    pub catch_site: Option<(String, String, usize)>,
    /// How many times this exception object was rethrown before being caught or escaping.
    pub rethrows: u32,
}

/// A linear log of all thrown exceptions and their catch sites.
///
/// Tracks the paths taken by thrown exceptions through the VM's call stack,
/// providing insight into the error handling overhead of the application.
///
/// # Examples
///
/// ```
/// use duke_telemetry::ExceptionFlowStore;
///
/// let mut store = ExceptionFlowStore::default();
/// let idx = store.record_throw("java/lang/NullPointerException", "com/example/Main", "run", 5);
/// store.record_catch(idx, "com/example/Main", "run", 15);
///
/// assert_eq!(store.events[0].catch_site.as_ref().unwrap().2, 15);
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ExceptionFlowStore {
    /// Ordered list of exception lifecycle events.
    pub events: Vec<ExceptionEvent>,
}

impl ExceptionFlowStore {
    /// Record a new throw. Returns the index of this event for subsequent [`record_catch`](ExceptionFlowStore::record_catch).
    ///
    /// - `exception_class`: The type of the thrown exception.
    /// - `throw_class`: The class executing the `athrow` instruction.
    /// - `throw_method`: The method executing the `athrow` instruction.
    /// - `throw_pc`: Program counter of the `athrow` instruction.
    pub fn record_throw(
        &mut self,
        exception_class: &str,
        throw_class: &str,
        throw_method: &str,
        throw_pc: usize,
    ) -> usize {
        self.events.push(ExceptionEvent {
            exception_class: exception_class.to_string(),
            throw_site: (throw_class.to_string(), throw_method.to_string(), throw_pc),
            catch_site: None,
            rethrows: 0,
        });
        self.events.len() - 1
    }

    /// Record a catch event for a previously thrown exception.
    ///
    /// - `event_idx`: The index of the exception event returned by [`record_throw`](ExceptionFlowStore::record_throw).
    /// - `catch_class`: The class where the exception handler matched.
    /// - `catch_method`: The method where the exception handler matched.
    /// - `handler_pc`: The starting program counter of the exception handler block.
    pub fn record_catch(
        &mut self,
        event_idx: usize,
        catch_class: &str,
        catch_method: &str,
        handler_pc: usize,
    ) {
        if let Some(ev) = self.events.get_mut(event_idx) {
            ev.catch_site = Some((
                catch_class.to_string(),
                catch_method.to_string(),
                handler_pc,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
