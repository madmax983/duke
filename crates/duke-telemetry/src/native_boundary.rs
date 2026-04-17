//! Tracks the cost and reliability of transitioning from JVM execution to native code.
//!
//! This module groups native method execution statistics by their defining class and method names.

#[cfg(feature = "telemetry")]
use crate::helpers::ser_helpers;
use std::collections::HashMap;

// -- native_boundary -------------------------------------------------------------

/// Statistics for a specific native method implementation.
///
/// Records the number of times the native method was called, the total wall-clock time
/// spent executing the native code, and how often it returned an error to the VM.
///
/// # Examples
///
/// ```
/// use duke_telemetry::NativeStat;
///
/// let mut stat = NativeStat::default();
/// stat.calls = 50;
/// stat.errors = 2;
/// stat.total_ns = 5000;
/// assert_eq!(stat.errors, 2);
/// ```
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct NativeStat {
    /// Total number of invocations of the native method.
    pub calls: u64,
    /// Number of times the native method execution resulted in an error or exception.
    pub errors: u64,
    /// Total duration spent executing the native method, in nanoseconds.
    pub total_ns: u64,
}

/// Tracks the cost and reliability of transitioning from JVM execution to native code.
///
/// Groups native method execution statistics by their defining class and method names.
///
/// # Examples
///
/// ```
/// use duke_telemetry::NativeBoundaryStore;
///
/// let mut store = NativeBoundaryStore::default();
/// store.record_call("java/lang/System", "arraycopy", 250, false);
///
/// let stat = &store.by_method[&("java/lang/System".to_string(), "arraycopy".to_string())];
/// assert_eq!(stat.calls, 1);
/// assert_eq!(stat.total_ns, 250);
/// assert_eq!(stat.errors, 0);
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct NativeBoundaryStore {
    /// Mapping from (`class_name`, `method_name`) to execution statistics.
    #[cfg_attr(feature = "telemetry", serde(serialize_with = "ser_helpers::pair_str"))]
    pub by_method: HashMap<(String, String), NativeStat>,
}

impl NativeBoundaryStore {
    /// Record a single execution of a native method.
    ///
    /// - `class`: The class on which the native method is defined.
    /// - `method`: The name of the native method.
    /// - `elapsed_ns`: How long the native execution took, in nanoseconds.
    /// - `is_err`: True if the method failed (e.g., returned a `VmError` or threw an exception).
    pub fn record_call(&mut self, class: &str, method: &str, elapsed_ns: u64, is_err: bool) {
        let stat = self
            .by_method
            .entry((class.to_string(), method.to_string()))
            .or_default();
        stat.calls += 1;
        stat.total_ns += elapsed_ns;
        if is_err {
            stat.errors += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
