import re

with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    code = f.read()

# Add #[cfg(feature = "telemetry")] to all fields of the store structs.
# We will explicitly match the fields to avoid messing up.

code = code.replace("pub by_opcode: HashMap<&'static str, OpcodeStat>,", "#[cfg(feature = \"telemetry\")]\n    pub by_opcode: HashMap<&'static str, OpcodeStat>,")
code = code.replace("pub by_site: HashMap<(String, String, usize), OpcodeStat>,", "#[cfg(feature = \"telemetry\")]\n    pub by_site: HashMap<(String, String, usize), OpcodeStat>,")

code = code.replace("pub sites: HashMap<(String, String, usize), AllocationSite>,", "#[cfg(feature = \"telemetry\")]\n    pub sites: HashMap<(String, String, usize), AllocationSite>,")

code = code.replace("pub events: Vec<ClinitEvent>,", "#[cfg(feature = \"telemetry\")]\n    pub events: Vec<ClinitEvent>,")

code = code.replace("pub events: Vec<ExceptionEvent>,", "#[cfg(feature = \"telemetry\")]\n    pub events: Vec<ExceptionEvent>,")

code = code.replace("pub by_site: HashMap<(String, u16), DispatchStat>,", "#[cfg(feature = \"telemetry\")]\n    pub by_site: HashMap<(String, u16), DispatchStat>,")

code = code.replace("pub by_method: HashMap<(String, String), NativeStat>,", "#[cfg(feature = \"telemetry\")]\n    pub by_method: HashMap<(String, String), NativeStat>,")


# Now replace the functions. We will use a regex to match the body of the function.
# BytecodeCostStore.record
code = code.replace("""
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
""", """
    pub fn record(
        &mut self,
        #[allow(unused_variables)] name: &'static str,
        #[allow(unused_variables)] class: &str,
        #[allow(unused_variables)] method: &str,
        #[allow(unused_variables)] pc: usize,
        #[allow(unused_variables)] elapsed_ns: u64,
    ) {
        #[cfg(feature = "telemetry")]
        {
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
""")

# ObjectLineageStore.record
code = code.replace("""
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
""", """
    pub fn record(
        &mut self,
        #[allow(unused_variables)] allocating_class: &str,
        #[allow(unused_variables)] method: &str,
        #[allow(unused_variables)] pc: usize,
        #[allow(unused_variables)] class_allocated: &str,
    ) {
        #[cfg(feature = "telemetry")]
        {
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
""")

# ClassInitDagStore.record
code = code.replace("""
    pub fn record(&mut self, class: &str, triggered_by: &str, duration_ns: u64) {
        self.events.push(ClinitEvent {
            class: class.to_string(),
            triggered_by: triggered_by.to_string(),
            duration_ns,
        });
    }
""", """
    pub fn record(
        &mut self,
        #[allow(unused_variables)] class: &str,
        #[allow(unused_variables)] triggered_by: &str,
        #[allow(unused_variables)] duration_ns: u64,
    ) {
        #[cfg(feature = "telemetry")]
        {
            self.events.push(ClinitEvent {
                class: class.to_string(),
                triggered_by: triggered_by.to_string(),
                duration_ns,
            });
        }
    }
""")

# ExceptionFlowStore.record_throw
code = code.replace("""
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
""", """
    pub fn record_throw(
        &mut self,
        #[allow(unused_variables)] exception_class: &str,
        #[allow(unused_variables)] throw_class: &str,
        #[allow(unused_variables)] throw_method: &str,
        #[allow(unused_variables)] throw_pc: usize,
    ) -> usize {
        #[cfg(feature = "telemetry")]
        {
            self.events.push(ExceptionEvent {
                exception_class: exception_class.to_string(),
                throw_site: (throw_class.to_string(), throw_method.to_string(), throw_pc),
                catch_site: None,
                rethrows: 0,
            });
            self.events.len() - 1
        }
        #[cfg(not(feature = "telemetry"))]
        {
            0
        }
    }
""")

# ExceptionFlowStore.record_catch
code = code.replace("""
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
""", """
    pub fn record_catch(
        &mut self,
        #[allow(unused_variables)] event_idx: usize,
        #[allow(unused_variables)] catch_class: &str,
        #[allow(unused_variables)] catch_method: &str,
        #[allow(unused_variables)] handler_pc: usize,
    ) {
        #[cfg(feature = "telemetry")]
        {
            if let Some(ev) = self.events.get_mut(event_idx) {
                ev.catch_site = Some((
                    catch_class.to_string(),
                    catch_method.to_string(),
                    handler_pc,
                ));
            }
        }
    }
""")

# DispatchResolutionStore.record
code = code.replace("""
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
""", """
    pub fn record(
        &mut self,
        #[allow(unused_variables)] caller_class: &str,
        #[allow(unused_variables)] cp_idx: u16,
        #[allow(unused_variables)] resolved_class: &str,
        #[allow(unused_variables)] hierarchy_walk: bool,
    ) {
        #[cfg(feature = "telemetry")]
        {
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
""")

# NativeBoundaryStore.record_call
code = code.replace("""
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
""", """
    pub fn record_call(
        &mut self,
        #[allow(unused_variables)] class: &str,
        #[allow(unused_variables)] method: &str,
        #[allow(unused_variables)] elapsed_ns: u64,
        #[allow(unused_variables)] is_err: bool,
    ) {
        #[cfg(feature = "telemetry")]
        {
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
""")


# Ensure tests are under the feature
code = code.replace("mod tests {", "#[cfg(feature = \"telemetry\")]\nmod tests {")

with open("crates/duke-telemetry/src/lib.rs", "w") as f:
    f.write(code)
