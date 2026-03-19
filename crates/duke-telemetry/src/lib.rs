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

use std::collections::{HashMap, HashSet};

// -- Serialization helpers for tuple-keyed HashMaps ------------------------------
// serde_json requires map keys to be strings. These helpers format tuple keys
// as strings before serializing.

#[cfg(feature = "telemetry")]
mod ser_helpers {
    use std::collections::HashMap;

    use serde::Serialize;

    /// Core helper: serialize any `HashMap<K, V>` by formatting each key with `key_fn`.
    fn keyed_map<K, V, S, F>(map: &HashMap<K, V>, ser: S, key_fn: F) -> Result<S::Ok, S::Error>
    where
        K: Eq + std::hash::Hash,
        V: Serialize,
        S: serde::Serializer,
        F: Fn(&K) -> String,
    {
        let string_map: HashMap<String, &V> = map.iter().map(|(k, v)| (key_fn(k), v)).collect();
        string_map.serialize(ser)
    }

    /// `HashMap<(class, method, pc), V>` → `"class::method@pc"`.
    pub fn site3<V: Serialize, S: serde::Serializer>(
        map: &HashMap<(String, String, usize), V>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        keyed_map(map, ser, |(c, m, pc)| format!("{c}::{m}@{pc}"))
    }

    /// `HashMap<(class, cp_idx), V>` → `"class@cp"`.
    pub fn site2_u16<V: Serialize, S: serde::Serializer>(
        map: &HashMap<(String, u16), V>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        keyed_map(map, ser, |(c, cp)| format!("{c}@{cp}"))
    }

    /// `HashMap<(class, method), V>` → `"class::method"`.
    pub fn pair_str<V: Serialize, S: serde::Serializer>(
        map: &HashMap<(String, String), V>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        keyed_map(map, ser, |(c, m)| format!("{c}::{m}"))
    }

    /// Serialize `HashSet<String>` as a sorted `Vec<String>` for deterministic output.
    pub fn sorted_set<S: serde::Serializer>(
        set: &std::collections::HashSet<String>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        let mut v: Vec<&String> = set.iter().collect();
        v.sort();
        v.serialize(ser)
    }
}

// -- bytecode_cost ---------------------------------------------------------------

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

// -- object_lineage --------------------------------------------------------------

/// Details of a single allocation site and what it allocated.
///
/// Keeps track of the most recently allocated class at this site and the total
/// number of objects it created.
///
/// # Examples
///
/// ```
/// use duke_telemetry::AllocationSite;
///
/// let site = AllocationSite {
///     class_allocated: "java/lang/String".to_string(),
///     count: 42,
/// };
/// assert_eq!(site.count, 42);
/// ```
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct AllocationSite {
    /// The JVM class name of the allocated object.
    pub class_allocated: String,
    /// Number of times objects of `class_allocated` were instantiated at this site.
    pub count: u64,
}

/// Tracks where objects are allocated, by method and instruction offset.
///
/// Records the allocating class and method alongside the PC offset of the `new` instruction,
/// to trace the origin of high allocation rates back to the source code.
///
/// # Examples
///
/// ```
/// use duke_telemetry::ObjectLineageStore;
///
/// let mut store = ObjectLineageStore::default();
/// store.record("com/example/Main", "run", 10, "java/lang/String");
///
/// let site = &store.sites[&("com/example/Main".to_string(), "run".to_string(), 10)];
/// assert_eq!(site.count, 1);
/// assert_eq!(site.class_allocated, "java/lang/String");
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ObjectLineageStore {
    /// Mapping from (`allocating_class`, `allocating_method`, pc) to allocation statistics.
    #[cfg_attr(feature = "telemetry", serde(serialize_with = "ser_helpers::site3"))]
    pub sites: HashMap<(String, String, usize), AllocationSite>,
}

impl ObjectLineageStore {
    /// Record a single object allocation event.
    ///
    /// - `allocating_class`: The class executing the `new` instruction.
    /// - `method`: The method executing the `new` instruction.
    /// - `pc`: Program counter of the allocation site.
    /// - `class_allocated`: The class name of the instantiated object.
    pub fn record(
        &mut self,
        allocating_class: &str,
        method: &str,
        pc: usize,
        class_allocated: &str,
    ) {
        let site = self
            .sites
            .entry((allocating_class.to_string(), method.to_string(), pc))
            .or_insert_with(|| AllocationSite {
                class_allocated: class_allocated.to_string(),
                count: 0,
            });
        site.count += 1;
    }
}

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
}

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

// -- dispatch_resolution ---------------------------------------------------------

/// Telemetry regarding a single virtual or interface dispatch site.
///
/// Tracks the total number of calls, the distinct receiver classes seen (polymorphism),
/// and the overhead of finding the appropriate method implementation within the class hierarchy.
///
/// # Examples
///
/// ```
/// use duke_telemetry::DispatchStat;
///
/// let mut stat = DispatchStat::default();
/// stat.calls = 100;
/// stat.hierarchy_walks = 50;
/// assert_eq!(stat.calls, 100);
/// ```
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct DispatchStat {
    /// Total number of method dispatches recorded at this call site.
    pub calls: u64,
    /// Distinct runtime receiver classes seen at this call site.
    #[cfg_attr(
        feature = "telemetry",
        serde(serialize_with = "ser_helpers::sorted_set")
    )]
    pub unique_targets: HashSet<String>,
    /// How many calls required a superclass hierarchy walk to find the method.
    pub hierarchy_walks: u64,
}

/// Statistics on dynamic method resolution (`invokevirtual` and `invokeinterface`).
///
/// Tracks polymorphism and dispatch overhead at every dynamic call site in the VM
/// to help identify opportunities for inline caching or other optimizations.
///
/// # Examples
///
/// ```
/// use duke_telemetry::DispatchResolutionStore;
///
/// let mut store = DispatchResolutionStore::default();
/// store.record("com/example/Main", 15, "java/lang/String", true);
///
/// let stat = &store.by_site[&("com/example/Main".to_string(), 15)];
/// assert_eq!(stat.calls, 1);
/// assert!(stat.unique_targets.contains("java/lang/String"));
/// assert_eq!(stat.hierarchy_walks, 1);
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct DispatchResolutionStore {
    /// Mapping from (`caller_class`, `cp_idx`) to dispatch statistics.
    #[cfg_attr(
        feature = "telemetry",
        serde(serialize_with = "ser_helpers::site2_u16")
    )]
    pub by_site: HashMap<(String, u16), DispatchStat>,
}

impl DispatchResolutionStore {
    /// Record the resolution of a virtual or interface method call.
    ///
    /// - `caller_class`: The class containing the `invoke*` instruction.
    /// - `cp_idx`: The constant pool index referenced by the instruction.
    /// - `resolved_class`: The actual runtime class of the receiver object.
    /// - `hierarchy_walk`: True if the method implementation was found by walking
    ///   up the superclass chain; false if it was found directly on `resolved_class`.
    pub fn record(
        &mut self,
        caller_class: &str,
        cp_idx: u16,
        resolved_class: &str,
        hierarchy_walk: bool,
    ) {
        let stat = self
            .by_site
            .entry((caller_class.to_string(), cp_idx))
            .or_default();
        stat.calls += 1;
        if !stat.unique_targets.contains(resolved_class) {
            stat.unique_targets.insert(resolved_class.to_string());
        }
        if hierarchy_walk {
            stat.hierarchy_walks += 1;
        }
    }
}

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
/// let store = TelemetryStore::default();
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

#[cfg(feature = "telemetry")]
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
    /// let store = TelemetryStore::default();
    /// #[cfg(feature = "telemetry")]
    /// let json = store.to_json();
    /// ```
    #[must_use]
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
    /// let store = TelemetryStore::default();
    /// let mut buf = Vec::new();
    /// #[cfg(feature = "telemetry")]
    /// store.print_report(&mut buf).unwrap();
    /// ```
    pub fn print_report(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(w, "=== Duke VM Telemetry Report ===")?;

        writeln!(w, "\n-- bytecode_cost (top 10 by count) --")?;
        let mut ops: Vec<_> = self.bytecode_cost.by_opcode.iter().collect();
        ops.sort_by(|a, b| b.1.count.cmp(&a.1.count));
        for (name, stat) in ops.iter().take(10) {
            writeln!(w, "  {:20} count={:>10}", name, stat.count)?;
        }

        writeln!(
            w,
            "\n-- object_lineage (top 10 allocation sites by count) --"
        )?;
        let mut sites: Vec<_> = self.object_lineage.sites.iter().collect();
        sites.sort_by(|a, b| b.1.count.cmp(&a.1.count));
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
        dsites.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));
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
        natives.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));
        for ((class, method), stat) in natives.iter().take(10) {
            writeln!(
                w,
                "  {}.{} calls={} errors={}",
                class, method, stat.calls, stat.errors
            )?;
        }

        Ok(())
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

    #[test]
    fn object_lineage_records_allocation_site() {
        let mut store = ObjectLineageStore::default();
        store.record("Foo", "main", 5, "java/lang/Object");
        store.record("Foo", "main", 5, "java/lang/Object");
        let site = &store.sites[&("Foo".to_string(), "main".to_string(), 5)];
        assert_eq!(site.count, 2);
        assert_eq!(site.class_allocated, "java/lang/Object");
    }

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

    #[test]
    fn dispatch_resolution_tracks_unique_targets() {
        let mut store = DispatchResolutionStore::default();
        store.record("Foo", 3, "SubA", false);
        store.record("Foo", 3, "SubB", true);
        let stat = &store.by_site[&("Foo".to_string(), 3)];
        assert_eq!(stat.calls, 2);
        assert_eq!(stat.unique_targets.len(), 2);
        assert_eq!(stat.hierarchy_walks, 1);
    }

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
