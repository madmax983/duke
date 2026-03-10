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

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct OpcodeStat {
    pub count: u64,
    pub total_ns: u64,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct BytecodeCostStore {
    /// Count/time per opcode name (e.g. "invokestatic", "iadd").
    pub by_opcode: HashMap<&'static str, OpcodeStat>,
    /// Count/time per bytecode site: (class_name, method_name, pc).
    #[cfg_attr(feature = "telemetry", serde(serialize_with = "ser_helpers::site3"))]
    pub by_site: HashMap<(String, String, usize), OpcodeStat>,
}

impl BytecodeCostStore {
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

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct AllocationSite {
    pub class_allocated: String,
    pub count: u64,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ObjectLineageStore {
    /// Key: (allocating_class, allocating_method, pc).
    #[cfg_attr(feature = "telemetry", serde(serialize_with = "ser_helpers::site3"))]
    pub sites: HashMap<(String, String, usize), AllocationSite>,
}

impl ObjectLineageStore {
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ClinitEvent {
    pub class: String,
    /// Class that caused this `<clinit>` to fire, or empty string for entry point.
    pub triggered_by: String,
    pub duration_ns: u64,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ClassInitDagStore {
    pub events: Vec<ClinitEvent>,
}

impl ClassInitDagStore {
    pub fn record(&mut self, class: &str, triggered_by: &str, duration_ns: u64) {
        self.events.push(ClinitEvent {
            class: class.to_string(),
            triggered_by: triggered_by.to_string(),
            duration_ns,
        });
    }
}

// -- exception_flow --------------------------------------------------------------

#[derive(Debug, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ExceptionEvent {
    pub exception_class: String,
    /// (class_name, method_name, pc) of the throw site.
    pub throw_site: (String, String, usize),
    /// (class_name, method_name, handler_pc) of the catch site, or None if uncaught.
    pub catch_site: Option<(String, String, usize)>,
    /// How many times this exception object was rethrown before being caught or escaping.
    pub rethrows: u32,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ExceptionFlowStore {
    pub events: Vec<ExceptionEvent>,
}

impl ExceptionFlowStore {
    /// Record a new throw. Returns the index of this event for subsequent `record_catch`.
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

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct DispatchStat {
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

#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct DispatchResolutionStore {
    /// Key: (caller_class, cp_idx).
    #[cfg_attr(
        feature = "telemetry",
        serde(serialize_with = "ser_helpers::site2_u16")
    )]
    pub by_site: HashMap<(String, u16), DispatchStat>,
}

impl DispatchResolutionStore {
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

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct NativeStat {
    pub calls: u64,
    pub errors: u64,
    pub total_ns: u64,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct NativeBoundaryStore {
    /// Key: (class_name, method_name).
    #[cfg_attr(
        feature = "telemetry",
        serde(serialize_with = "ser_helpers::pair_str")
    )]
    pub by_method: HashMap<(String, String), NativeStat>,
}

impl NativeBoundaryStore {
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

#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct TelemetryStore {
    pub bytecode_cost: BytecodeCostStore,
    pub object_lineage: ObjectLineageStore,
    pub class_init_dag: ClassInitDagStore,
    pub exception_flow: ExceptionFlowStore,
    pub dispatch_resolution: DispatchResolutionStore,
    pub native_boundary: NativeBoundaryStore,
}

#[cfg(feature = "telemetry")]
impl TelemetryStore {
    /// Serialize the store to pretty-printed JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("telemetry serialization failed")
    }

    /// Print a human-readable top-10 summary per channel to `w`.
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
            let catch = ev
                .catch_site
                .as_ref()
                .map(|(c, m, pc)| format!("{}::{} @{}", c, m, pc))
                .unwrap_or_else(|| "uncaught".to_string());
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
