use std::collections::HashMap;

/// Statistics for a single bytecode opcode execution.
///
/// Records how many times an instruction was executed and the total wall-clock
/// time spent interpreting it.
///
/// # Examples
///
/// ```
/// use duke_telemetry::OpcodeStat;
///
/// let mut stat = OpcodeStat::default();
/// stat.count = 100;
/// stat.total_ns = 1500;
/// assert_eq!(stat.count, 100);
/// ```
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct OpcodeStat {
    /// Number of times this opcode was executed.
    pub count: u64,
    /// Cumulative execution time across all invocations, in nanoseconds.
    pub total_ns: u64,
}

/// Accumulates frequency and duration for bytecode execution.
///
/// Metrics are aggregated globally per opcode name (e.g., `"iadd"`) and locally
/// per specific call site (class, method, and instruction PC).
///
/// # Examples
///
/// ```
/// use duke_telemetry::BytecodeCostStore;
///
/// let mut store = BytecodeCostStore::default();
/// store.record("iadd", "Math", "add", 12, 45);
///
/// let stat = &store.by_opcode["iadd"];
/// assert_eq!(stat.count, 1);
/// assert_eq!(stat.total_ns, 45);
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct BytecodeCostStore {
    /// Global aggregation of costs keyed by opcode mnemonic (e.g., `"aload_0"`).
    pub by_opcode: HashMap<&'static str, OpcodeStat>,

    /// Call-site specific aggregation.
    ///
    /// Key: `(class_name, method_name, instruction_pc)`
    #[cfg_attr(feature = "telemetry", serde(serialize_with = "crate::store::ser_helpers::site3"))]
    pub by_site: HashMap<(String, String, usize), OpcodeStat>,
}

impl BytecodeCostStore {
    /// Record the execution of a single instruction.
    ///
    /// - `name`: The mnemonic of the instruction (e.g., `"iadd"`).
    /// - `class`: The name of the class being executed.
    /// - `method`: The name of the method being executed.
    /// - `pc`: The program counter (byte offset) of the instruction within the method.
    /// - `elapsed_ns`: The time taken to execute the instruction, in nanoseconds.
    pub fn record(&mut self, name: &'static str, class: &str, method: &str, pc: usize, elapsed_ns: u64) {
        let global = self.by_opcode.entry(name).or_default();
        global.count += 1;
        global.total_ns += elapsed_ns;

        let site = self
            .by_site
            .entry((class.to_string(), method.to_string(), pc))
            .or_default();
        site.count += 1;
        site.total_ns += elapsed_ns;
    }
}
