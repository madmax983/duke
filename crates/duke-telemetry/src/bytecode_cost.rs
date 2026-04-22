//! Tracks execution frequency and duration per-opcode and per-bytecode site.
//!
//! This module provides structures to accumulate frequency and duration for bytecode execution,
//! keeping a running total of how many times a particular execution site or opcode was visited.

#[cfg(feature = "telemetry")]
use crate::helpers::ser_helpers;
use std::collections::HashMap;

/// Accumulates frequency and duration for bytecode execution.
///
/// Keeps a running total of how many times a particular execution site or opcode
/// was visited, and the total wall-clock time spent inside that instruction.
///
/// # Examples
///
/// ```
/// use duke_telemetry::OpcodeStat;
///
/// let mut stat = OpcodeStat::default();
/// stat.count += 1;
/// stat.total_ns += 1000;
/// assert_eq!(stat.count, 1);
/// assert_eq!(stat.total_ns, 1000);
/// ```
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct OpcodeStat {
    /// Number of times this opcode or instruction was executed.
    pub count: u64,
    /// Total duration spent executing this opcode or instruction, in nanoseconds.
    pub total_ns: u64,
}

/// Tracks execution frequency and duration per-opcode and per-bytecode site.
///
/// This store helps identify computationally expensive JVM instructions
/// and hot spots in specific methods by aggregating statistics globally
/// (by opcode name) and locally (by class, method, and instruction index).
///
/// # Examples
///
/// ```
/// use duke_telemetry::BytecodeCostStore;
///
/// let mut store = BytecodeCostStore::default();
///
/// store.record("iadd", "com/example/Math", "add", 42, 100);
/// store.record("iadd", "com/example/Math", "add", 42, 200);
///
/// assert_eq!(store.by_opcode["iadd"].count, 2);
/// assert_eq!(store.by_opcode["iadd"].total_ns, 300);
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct BytecodeCostStore {
    /// Count/time per opcode name (e.g. "invokestatic", "iadd").
    pub by_opcode: HashMap<&'static str, OpcodeStat>,
    /// Count/time per bytecode site: (`class_name`, `method_name`, pc).
    #[cfg_attr(feature = "telemetry", serde(serialize_with = "ser_helpers::site3"))]
    pub by_site: HashMap<(String, String, usize), OpcodeStat>,
}

impl BytecodeCostStore {
    /// Record the execution of a single bytecode instruction.
    ///
    /// - `name`: The mnemonic of the instruction (e.g. "`aload_0`", "`invokeinterface`").
    /// - `class`: The JVM name of the class currently executing.
    /// - `method`: The name of the method currently executing.
    /// - `pc`: The program counter (instruction index) within the method.
    /// - `elapsed_ns`: How long the instruction took to execute, in nanoseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_telemetry::BytecodeCostStore;
    ///
    /// let mut store = BytecodeCostStore::default();
    /// store.record("iadd", "Math", "add", 0, 150);
    ///
    /// assert_eq!(store.by_opcode["iadd"].count, 1);
    /// assert_eq!(store.by_opcode["iadd"].total_ns, 150);
    /// ```
    pub fn record(
        &mut self,
        name: &'static str,
        class: &str,
        method: &str,
        pc: usize,
        elapsed_ns: u64,
    ) {
        let op = self.by_opcode.entry(name).or_default();
        op.count += 1;
        op.total_ns += elapsed_ns;
        let site = self
            .by_site
            .entry((class.to_string(), method.to_string(), pc))
            .or_default();
        site.count += 1;
        site.total_ns += elapsed_ns;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytecode_cost_records_count_and_site() {
        let mut store = BytecodeCostStore::default();
        store.record("iadd", "Foo", "bar", 10, 100);
        store.record("iadd", "Foo", "bar", 10, 200);
        assert_eq!(store.by_opcode["iadd"].count, 2);
        assert_eq!(store.by_opcode["iadd"].total_ns, 300);
        assert_eq!(
            store.by_site[&("Foo".to_string(), "bar".to_string(), 10)].count,
            2
        );
    }
}
