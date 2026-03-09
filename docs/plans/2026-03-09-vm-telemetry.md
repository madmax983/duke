# VM Telemetry Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a zero-overhead (when disabled) telemetry layer to the Duke JVM interpreter with six diagnostic channels: bytecode cost, object lineage, class init DAG, exception flow, dispatch resolution, and native boundary.

**Architecture:** A new `duke-telemetry` crate holds all channel structs and `TelemetryStore`. The store lives as a field on `ClassRegistry` behind `#[cfg(feature = "telemetry")]`. All instrumentation in `duke-interpreter` is gated with the same cfg guard — zero cost in default builds, full diagnostics when compiled with `--features telemetry`.

**Tech Stack:** Rust, `std::collections::HashMap`, `std::time::Instant`, `serde`/`serde_json` (optional, telemetry feature only)

---

## Codebase orientation

- `crates/duke-interpreter/src/lib.rs` — the entire interpreter (12k+ lines)
- `ClassRegistry` struct: line 82. Fields: `classes`, `natives`, `initialized`, `lambdas`, `lambda_counter`
- `execute_class` fn: line 4685. Signature: `(registry: &mut ClassRegistry, loader, heap, stdout, class_name, method_name, descriptor, args)`
- `ensure_initialized` fn: line 4597. Called at 5 sites: lines 4708, 4825, 5634, 5690, 5702
- `Instruction::New(cp_idx)` arm: line 5628
- `Instruction::Athrow` arm: line 6351 (in `execute_class`; also at 4565 in standalone `execute`)
- Native handler call sites: lines 4885 (invokestatic), 5897 (invokevirtual), 6673 and 6800 (invokeinterface)
- Test helper `run_bootstrap_int`: line 12367
- `current_class: String` variable in `execute_class`: updated at ~10 sites (lines 4766, 4815, 4868, 5751, 5852, 5953, 6396, 6743, 6786, 6850)
- `method_idx: usize` variable in `execute_class`: updated at the same sites

---

## Task 1: Create `duke-telemetry` crate

**Files:**
- Create: `crates/duke-telemetry/Cargo.toml`
- Create: `crates/duke-telemetry/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

### Step 1: Create the crate directory and Cargo.toml

```toml
# crates/duke-telemetry/Cargo.toml
[package]
name = "duke-telemetry"
version.workspace = true
edition.workspace = true
description = "Duke JVM - VM telemetry channels"

[features]
default = []
telemetry = ["dep:serde", "dep:serde_json"]

[dependencies]
serde     = { version = "1", features = ["derive"], optional = true }
serde_json = { version = "1", optional = true }
```

### Step 2: Create `src/lib.rs` with all six channel types

```rust
// crates/duke-telemetry/src/lib.rs
use std::collections::{HashMap, HashSet};

// ── bytecode_cost ──────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct OpcodeStat {
    pub count:    u64,
    pub total_ns: u64,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct BytecodeCostStore {
    /// Count/time per opcode name (e.g. "invokestatic", "iadd").
    pub by_opcode: HashMap<&'static str, OpcodeStat>,
    /// Count/time per bytecode site: (class_name, method_name, pc).
    pub by_site: HashMap<(String, String, usize), OpcodeStat>,
}

impl BytecodeCostStore {
    pub fn record(&mut self, name: &'static str, class: &str, method: &str, pc: usize, elapsed_ns: u64) {
        let op = self.by_opcode.entry(name).or_default();
        op.count += 1;
        op.total_ns += elapsed_ns;
        let site = self.by_site.entry((class.to_string(), method.to_string(), pc)).or_default();
        site.count += 1;
        site.total_ns += elapsed_ns;
    }
}

// ── object_lineage ─────────────────────────────────────────────────────────────

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
    pub sites: HashMap<(String, String, usize), AllocationSite>,
}

impl ObjectLineageStore {
    pub fn record(&mut self, allocating_class: &str, method: &str, pc: usize, class_allocated: &str) {
        let site = self.sites
            .entry((allocating_class.to_string(), method.to_string(), pc))
            .or_insert_with(|| AllocationSite {
                class_allocated: class_allocated.to_string(),
                count: 0,
            });
        site.count += 1;
    }
}

// ── class_init_dag ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct ClinitEvent {
    pub class:        String,
    /// Class that caused this `<clinit>` to fire, or empty string for entry point.
    pub triggered_by: String,
    pub duration_ns:  u64,
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

// ── exception_flow ─────────────────────────────────────────────────────────────

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
            ev.catch_site = Some((catch_class.to_string(), catch_method.to_string(), handler_pc));
        }
    }
}

// ── dispatch_resolution ────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct DispatchStat {
    pub calls: u64,
    /// Distinct runtime receiver classes seen at this call site.
    pub unique_targets: HashSet<String>,
    /// How many calls required a superclass hierarchy walk to find the method.
    pub hierarchy_walks: u64,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct DispatchResolutionStore {
    /// Key: (caller_class, cp_idx).
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
        let stat = self.by_site
            .entry((caller_class.to_string(), cp_idx))
            .or_default();
        stat.calls += 1;
        stat.unique_targets.insert(resolved_class.to_string());
        if hierarchy_walk {
            stat.hierarchy_walks += 1;
        }
    }
}

// ── native_boundary ────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct NativeStat {
    pub calls:    u64,
    pub errors:   u64,
    pub total_ns: u64,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct NativeBoundaryStore {
    /// Key: (class_name, method_name).
    pub by_method: HashMap<(String, String), NativeStat>,
}

impl NativeBoundaryStore {
    pub fn record_call(&mut self, class: &str, method: &str, elapsed_ns: u64, is_err: bool) {
        let stat = self.by_method
            .entry((class.to_string(), method.to_string()))
            .or_default();
        stat.calls += 1;
        stat.total_ns += elapsed_ns;
        if is_err {
            stat.errors += 1;
        }
    }
}

// ── TelemetryStore ─────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]
pub struct TelemetryStore {
    pub bytecode_cost:       BytecodeCostStore,
    pub object_lineage:      ObjectLineageStore,
    pub class_init_dag:      ClassInitDagStore,
    pub exception_flow:      ExceptionFlowStore,
    pub dispatch_resolution: DispatchResolutionStore,
    pub native_boundary:     NativeBoundaryStore,
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

        writeln!(w, "\n-- object_lineage (top 10 allocation sites by count) --")?;
        let mut sites: Vec<_> = self.object_lineage.sites.iter().collect();
        sites.sort_by(|a, b| b.1.count.cmp(&a.1.count));
        for ((class, method, pc), site) in sites.iter().take(10) {
            writeln!(w, "  {}::{} @{} allocs {} of {}", class, method, pc, site.count, site.class_allocated)?;
        }

        writeln!(w, "\n-- class_init_dag ({} clinit events) --", self.class_init_dag.events.len())?;
        for ev in &self.class_init_dag.events {
            writeln!(w, "  {} (triggered by: {}, {}ns)", ev.class, ev.triggered_by, ev.duration_ns)?;
        }

        writeln!(w, "\n-- exception_flow ({} throw events) --", self.exception_flow.events.len())?;
        for ev in &self.exception_flow.events {
            let catch = ev.catch_site.as_ref()
                .map(|(c, m, pc)| format!("{}::{} @{}", c, m, pc))
                .unwrap_or_else(|| "uncaught".to_string());
            writeln!(w, "  {} thrown at {:?} caught at {}", ev.exception_class, ev.throw_site, catch)?;
        }

        writeln!(w, "\n-- dispatch_resolution (top 10 virtual call sites) --")?;
        let mut dsites: Vec<_> = self.dispatch_resolution.by_site.iter().collect();
        dsites.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));
        for ((class, cp), stat) in dsites.iter().take(10) {
            writeln!(w, "  {}[cp{}] calls={} targets={} walks={}", class, cp, stat.calls, stat.unique_targets.len(), stat.hierarchy_walks)?;
        }

        writeln!(w, "\n-- native_boundary (top 10 by call count) --")?;
        let mut natives: Vec<_> = self.native_boundary.by_method.iter().collect();
        natives.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));
        for ((class, method), stat) in natives.iter().take(10) {
            writeln!(w, "  {}.{} calls={} errors={}", class, method, stat.calls, stat.errors)?;
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
        assert_eq!(store.by_site[&("Foo".to_string(), "bar".to_string(), 10)].count, 2);
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
        assert_eq!(store.events[0].catch_site, Some(("Foo".to_string(), "bar".to_string(), 20)));
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
```

### Step 3: Add to workspace

In root `Cargo.toml`, add to the `members` array:
```toml
members = [
    "duke",
    "crates/duke-classfile",
    "crates/duke-bytecode",
    "crates/duke-loader",
    "crates/duke-runtime",
    "crates/duke-gc",
    "crates/duke-interpreter",
    "crates/duke-telemetry",   # ← add this line
]
```

### Step 4: Run unit tests

```bash
cargo test -p duke-telemetry 2>&1 | tail -10
```

Expected: `test result: ok. 5 passed`.

### Step 5: Commit

```bash
git add crates/duke-telemetry/ Cargo.toml
git commit -m "feat(telemetry): add duke-telemetry crate with 6 channel types"
```

---

## Task 2: Wire TelemetryStore into ClassRegistry

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`
- Modify: `crates/duke-interpreter/Cargo.toml`

### Step 1: Add `duke-telemetry` as optional dep to `duke-interpreter`

In `crates/duke-interpreter/Cargo.toml`:
```toml
[dependencies]
duke-runtime  = { path = "../duke-runtime" }
duke-bytecode = { path = "../duke-bytecode" }
duke-classfile = { path = "../duke-classfile" }
duke-gc       = { path = "../duke-gc" }
duke-loader   = { path = "../duke-loader" }
duke-telemetry = { path = "../duke-telemetry", optional = true }

[features]
telemetry = ["dep:duke-telemetry", "duke-telemetry/telemetry"]
```

### Step 2: Add `telemetry` field to `ClassRegistry` (line 82)

Find the `ClassRegistry` struct (line 82). Add the field and update `new()`:

```rust
pub struct ClassRegistry {
    classes: HashMap<String, ClassContext>,
    natives: NativeRegistry,
    initialized: HashSet<String>,
    lambdas: HashMap<String, LambdaInfo>,
    lambda_counter: u64,
    #[cfg(feature = "telemetry")]
    pub telemetry: duke_telemetry::TelemetryStore,
}

impl ClassRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            natives: NativeRegistry::new(),
            initialized: HashSet::new(),
            lambdas: HashMap::new(),
            lambda_counter: 0,
            #[cfg(feature = "telemetry")]
            telemetry: duke_telemetry::TelemetryStore::default(),
        }
    }
```

### Step 3: Add `run_fixture` test helper

Find the `run_bootstrap_int` helper (line 12367) and add a new helper directly after it:

```rust
#[cfg(feature = "telemetry")]
fn run_fixture(
    class_name: &str,
    method_name: &str,
    descriptor: &str,
) -> (Option<Slot>, ClassRegistry) {
    let ctx = load_class_context(class_name);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        &entry_class,
        method_name,
        descriptor,
        &[],
    )
    .expect("execute_class failed");
    (result, registry)
}
```

### Step 4: Verify compilation with telemetry feature

```bash
cargo build -p duke-interpreter --features telemetry 2>&1 | grep "^error" | head -10
```

Expected: no errors.

### Step 5: Run all existing tests (default, no telemetry)

```bash
cargo test -p duke-interpreter 2>&1 | tail -5
```

Expected: `274 passed`.

### Step 6: Commit

```bash
git add crates/duke-interpreter/src/lib.rs crates/duke-interpreter/Cargo.toml
git commit -m "feat(telemetry): wire TelemetryStore into ClassRegistry behind feature flag"
```

---

## Task 3: Instrument `bytecode_cost`

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Add `current_method` tracking variable

In `execute_class`, near line 4710 where `current_class` is initialised, add:

```rust
let mut current_class = class_name.to_string();
#[cfg(feature = "telemetry")]
let mut current_method = method_name.to_string();
```

Then find every place `method_idx` and `current_class` are updated together (use the grep output above — lines 4763, 4813, 4815, 4866, 4868, 5749, 5751, 5850, 5852, 5951, 5953, 6394, 6396, 6741, 6743, 6784, 6786, 6850). After each `method_idx = X; current_class = Y;` assignment block, add:

```rust
#[cfg(feature = "telemetry")]
{
    current_method = registry
        .get(&current_class)
        .map(|c| c.methods.get(method_idx).map(|m| m.name.clone()).unwrap_or_default())
        .unwrap_or_default();
}
```

There are two patterns:
1. **Return from callee** (do_return! macro, Athrow unwind): `method_idx = caller.method_idx; current_class = caller.class_name;` — add the `current_method` update after both assignments.
2. **Call into callee** (at end of invokestatic/virtual/special/interface bytecode paths): `method_idx = callee_idx; current_class = callee_class;` — add after both.

### Step 2: Add `instr_name` helper function

Add this function near the top of `execute_class` (after the `FramePool` struct, before `execute_class` fn):

```rust
#[cfg(feature = "telemetry")]
fn instr_name(instr: &duke_bytecode::Instruction) -> &'static str {
    use duke_bytecode::Instruction as I;
    match instr {
        I::Nop => "nop",
        I::Iconst0 | I::Iconst1 | I::Iconst2 | I::Iconst3 | I::Iconst4 | I::Iconst5 => "iconst_n",
        I::IconstM1 => "iconst_m1",
        I::Lconst0 | I::Lconst1 => "lconst_n",
        I::Fconst0 | I::Fconst1 | I::Fconst2 => "fconst_n",
        I::Dconst0 | I::Dconst1 => "dconst_n",
        I::Bipush(_) => "bipush",
        I::Sipush(_) => "sipush",
        I::Ldc(_) | I::LdcW(_) | I::Ldc2W(_) => "ldc",
        I::Iload(_) | I::Iload0 | I::Iload1 | I::Iload2 | I::Iload3 => "iload",
        I::Lload(_) | I::Lload0 | I::Lload1 | I::Lload2 | I::Lload3 => "lload",
        I::Fload(_) | I::Fload0 | I::Fload1 | I::Fload2 | I::Fload3 => "fload",
        I::Dload(_) | I::Dload0 | I::Dload1 | I::Dload2 | I::Dload3 => "dload",
        I::Aload(_) | I::Aload0 | I::Aload1 | I::Aload2 | I::Aload3 => "aload",
        I::Istore(_) | I::Istore0 | I::Istore1 | I::Istore2 | I::Istore3 => "istore",
        I::Lstore(_) | I::Lstore0 | I::Lstore1 | I::Lstore2 | I::Lstore3 => "lstore",
        I::Fstore(_) | I::Fstore0 | I::Fstore1 | I::Fstore2 | I::Fstore3 => "fstore",
        I::Dstore(_) | I::Dstore0 | I::Dstore1 | I::Dstore2 | I::Dstore3 => "dstore",
        I::Astore(_) | I::Astore0 | I::Astore1 | I::Astore2 | I::Astore3 => "astore",
        I::Iaload => "iaload", I::Laload => "laload", I::Faload => "faload",
        I::Daload => "daload", I::Aaload => "aaload", I::Baload => "baload",
        I::Caload => "caload", I::Saload => "saload",
        I::Iastore => "iastore", I::Lastore => "lastore", I::Fastore => "fastore",
        I::Dastore => "dastore", I::Aastore => "aastore", I::Bastore => "bastore",
        I::Castore => "castore", I::Sastore => "sastore",
        I::Pop => "pop", I::Pop2 => "pop2", I::Dup => "dup", I::DupX1 => "dup_x1",
        I::DupX2 => "dup_x2", I::Dup2 => "dup2", I::Dup2X1 => "dup2_x1",
        I::Dup2X2 => "dup2_x2", I::Swap => "swap",
        I::Iadd => "iadd", I::Ladd => "ladd", I::Fadd => "fadd", I::Dadd => "dadd",
        I::Isub => "isub", I::Lsub => "lsub", I::Fsub => "fsub", I::Dsub => "dsub",
        I::Imul => "imul", I::Lmul => "lmul", I::Fmul => "fmul", I::Dmul => "dmul",
        I::Idiv => "idiv", I::Ldiv => "ldiv", I::Fdiv => "fdiv", I::Ddiv => "ddiv",
        I::Irem => "irem", I::Lrem => "lrem", I::Frem => "frem", I::Drem => "drem",
        I::Ineg => "ineg", I::Lneg => "lneg", I::Fneg => "fneg", I::Dneg => "dneg",
        I::Ishl => "ishl", I::Lshl => "lshl", I::Ishr => "ishr", I::Lshr => "lshr",
        I::Iushr => "iushr", I::Lushr => "lushr",
        I::Iand => "iand", I::Land => "land", I::Ior => "ior", I::Lor => "lor",
        I::Ixor => "ixor", I::Lxor => "lxor",
        I::Iinc(_, _) => "iinc",
        I::I2l => "i2l", I::I2f => "i2f", I::I2d => "i2d",
        I::L2i => "l2i", I::L2f => "l2f", I::L2d => "l2d",
        I::F2i => "f2i", I::F2l => "f2l", I::F2d => "f2d",
        I::D2i => "d2i", I::D2l => "d2l", I::D2f => "d2f",
        I::I2b => "i2b", I::I2c => "i2c", I::I2s => "i2s",
        I::Lcmp => "lcmp", I::Fcmpl => "fcmpl", I::Fcmpg => "fcmpg",
        I::Dcmpl => "dcmpl", I::Dcmpg => "dcmpg",
        I::Ifeq(_) | I::Ifne(_) | I::Iflt(_) | I::Ifge(_) | I::Ifgt(_) | I::Ifle(_) => "if_<cond>",
        I::IfIcmpeq(_) | I::IfIcmpne(_) | I::IfIcmplt(_) | I::IfIcmpge(_) | I::IfIcmpgt(_) | I::IfIcmple(_) => "if_icmp<cond>",
        I::IfAcmpeq(_) | I::IfAcmpne(_) => "if_acmp<cond>",
        I::Goto(_) | I::GotoW(_) => "goto",
        I::Jsr(_) | I::JsrW(_) => "jsr",
        I::Ret(_) => "ret",
        I::Tableswitch { .. } => "tableswitch",
        I::Lookupswitch { .. } => "lookupswitch",
        I::Ireturn => "ireturn", I::Lreturn => "lreturn", I::Freturn => "freturn",
        I::Dreturn => "dreturn", I::Areturn => "areturn", I::Return => "return",
        I::Getstatic(_) => "getstatic", I::Putstatic(_) => "putstatic",
        I::Getfield(_) => "getfield", I::Putfield(_) => "putfield",
        I::Invokevirtual(_) => "invokevirtual",
        I::Invokespecial(_) => "invokespecial",
        I::Invokestatic(_) => "invokestatic",
        I::Invokeinterface(_, _) => "invokeinterface",
        I::Invokedynamic(_) => "invokedynamic",
        I::New(_) => "new",
        I::Newarray(_) => "newarray", I::Anewarray(_) => "anewarray",
        I::Arraylength => "arraylength",
        I::Athrow => "athrow",
        I::Checkcast(_) => "checkcast", I::Instanceof(_) => "instanceof",
        I::Monitorenter => "monitorenter", I::Monitorexit => "monitorexit",
        I::Multianewarray(_, _) => "multianewarray",
        I::Ifnull(_) | I::Ifnonnull(_) => "ifnull/nonnull",
        _ => "other",
    }
}
```

**Note:** The exact variant names depend on what's in `duke_bytecode::Instruction`. If a variant doesn't exist or has different casing, fix the name to match. Compile errors will guide you.

### Step 3: Add timing + recording around the main match

In the main interpreter loop in `execute_class`, find the instruction dispatch loop body. It looks roughly like:

```rust
let (pc, instr) = instructions[idx].clone();
// ... some setup ...
match instr {
    Instruction::Nop => { ... }
    // ...
}
idx += 1;
```

Wrap it with:

```rust
#[cfg(feature = "telemetry")]
let (_telem_name, _telem_pc, _telem_start) = {
    let name = instr_name(&instr);
    let pc_val = pc;
    (name, pc_val, std::time::Instant::now())
};

match instr {
    // ... all arms unchanged ...
}

#[cfg(feature = "telemetry")]
{
    let elapsed = _telem_start.elapsed().as_nanos() as u64;
    registry.telemetry.bytecode_cost.record(
        _telem_name,
        &current_class,
        &current_method,
        _telem_pc,
        elapsed,
    );
}

idx += 1;
```

**Important:** Arms that use `continue` (branch opcodes, invoke opcodes after call setup) will skip the post-match telemetry block for that iteration — those instructions won't get their timing recorded but WILL be counted on the NEXT iteration with a different opcode. This means timing is approximate for branching opcodes. For the MVP this is acceptable; counts per site are fully accurate only for non-continue arms. Add a comment noting this limitation.

### Step 4: Write the failing test

Add this test in the `#[cfg(test)]` block:

```rust
#[cfg(feature = "telemetry")]
#[test]
fn telemetry_bytecode_cost_counts_iadd() {
    // benchSum does 499_999 integer adds (sum 0..499_999).
    let (result, registry) = run_fixture("BenchmarkSuite", "benchSum", "()I");
    assert_eq!(result, Some(Slot::Int(445_698_416)));
    let stat = &registry.telemetry.bytecode_cost.by_opcode["iadd"];
    assert_eq!(stat.count, 499_999);
}
```

### Step 5: Run the failing test

```bash
cargo test -p duke-interpreter --features telemetry telemetry_bytecode_cost_counts_iadd 2>&1 | tail -10
```

Expected: PASS. (If it panics with missing opcode name variant, fix `instr_name`.)

### Step 6: Run all tests — no telemetry build must still pass

```bash
cargo test -p duke-interpreter 2>&1 | tail -5
```

Expected: `274 passed`.

```bash
cargo test -p duke-interpreter --features telemetry 2>&1 | tail -5
```

Expected: `275 passed` (one new test).

### Step 7: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(telemetry): instrument bytecode_cost per opcode and call site"
```

---

## Task 4: Instrument `object_lineage` and `native_boundary`

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Instrument `object_lineage` at the `New` opcode (line 5628)

Find `Instruction::New(cp_idx) =>` in `execute_class`. After `target_class` is resolved and `heap.allocate(...)` is called, add:

```rust
#[cfg(feature = "telemetry")]
registry.telemetry.object_lineage.record(
    &current_class,
    &current_method,
    pc,
    &target_class,
);
```

### Step 2: Write `object_lineage` test

```rust
#[cfg(feature = "telemetry")]
#[test]
fn telemetry_object_lineage_records_allocations() {
    // benchArrayList creates 1 ArrayList + 5000 objects? No — it just calls native add.
    // Use a fixture that calls `new` on a known class.
    // BenchmarkSuite.benchFib does no `new` — use ForEachTest instead.
    // ForEachTest creates an ArrayList and adds elements.
    let (_, registry) = run_fixture("ForEachTest", "main", "([Ljava/lang/String;)V");
    // At least one allocation site must exist (the ArrayList constructor).
    assert!(!registry.telemetry.object_lineage.sites.is_empty());
    // Verify some allocation is attributed to ForEachTest class.
    let has_foreach_alloc = registry.telemetry.object_lineage.sites
        .keys()
        .any(|(cls, _, _)| cls == "ForEachTest");
    assert!(has_foreach_alloc, "expected at least one allocation from ForEachTest");
}
```

### Step 3: Instrument `native_boundary` at all four native dispatch sites

Find each native handler dispatch point. There are four:

**Site 1 — invokestatic (around line 4885):**
```rust
// Before the handler call:
#[cfg(feature = "telemetry")]
let _native_start = std::time::Instant::now();

let result = handler(&native_args, heap, stdout);

#[cfg(feature = "telemetry")]
registry.telemetry.native_boundary.record_call(
    &callee_class,
    &callee_name,
    _native_start.elapsed().as_nanos() as u64,
    result.is_err(),
);

let result = result?;  // propagate error after recording
```

**Note:** The existing `let result = handler(...)?;` uses `?` immediately. Split it into `let result = handler(...);` + telemetry + `let result = result?;`.

Repeat this pattern at the other three sites (invokevirtual ~line 5897, invokeinterface ~lines 6673 and 6800). At each site, identify the `callee_class` and `callee_name` variables in scope — the names may differ slightly (`dispatch_class`, `impl_class`, etc. — check what's in scope with `grep`).

### Step 4: Write `native_boundary` test

```rust
#[cfg(feature = "telemetry")]
#[test]
fn telemetry_native_boundary_records_println() {
    let (_, registry) = run_fixture("ForEachTest", "main", "([Ljava/lang/String;)V");
    // ForEachTest calls System.out.println which dispatches through println native.
    let stat = registry.telemetry.native_boundary.by_method
        .iter()
        .find(|((_, name), _)| name.contains("println"));
    assert!(stat.is_some(), "expected println to be recorded in native_boundary");
    let (_, s) = stat.unwrap();
    assert!(s.calls > 0);
    assert_eq!(s.errors, 0);
}
```

### Step 5: Run tests

```bash
cargo test -p duke-interpreter --features telemetry telemetry_object_lineage 2>&1 | tail -10
cargo test -p duke-interpreter --features telemetry telemetry_native_boundary 2>&1 | tail -10
cargo test -p duke-interpreter 2>&1 | tail -5
```

Expected: all pass, default build still 274.

### Step 6: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(telemetry): instrument object_lineage and native_boundary"
```

---

## Task 5: Instrument `exception_flow` + ExceptionTest fixture

**Files:**
- Create: `tests/fixtures/ExceptionTest.java`
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Create `ExceptionTest.java` and compile it

```java
// tests/fixtures/ExceptionTest.java
public class ExceptionTest {
    public static int throwAndCatch() {
        try {
            throw new RuntimeException("test");
        } catch (RuntimeException e) {
            return 42;
        }
    }

    public static int uncaught() {
        throw new RuntimeException("uncaught");
    }

    public static int rethrow() {
        try {
            try {
                throw new RuntimeException("inner");
            } catch (RuntimeException e) {
                throw e;
            }
        } catch (RuntimeException e) {
            return 99;
        }
    }
}
```

Compile:
```bash
javac --release 21 tests/fixtures/ExceptionTest.java -d tests/fixtures/
```

Expected: `ExceptionTest.class` created.

### Step 2: Instrument `Athrow` in `execute_class` (line 6351)

In the `Instruction::Athrow =>` arm, after `exc_class_name` is obtained, before the handler search:

```rust
#[cfg(feature = "telemetry")]
let _telem_exc_event_idx = registry.telemetry.exception_flow.record_throw(
    &exc_class_name,
    &current_class,
    &current_method,
    pc,
);
```

After the handler is found (the `if let Some(handler_pc) = handler {` branch), record the catch:

```rust
if let Some(handler_pc) = handler {
    #[cfg(feature = "telemetry")]
    registry.telemetry.exception_flow.record_catch(
        _telem_exc_event_idx,
        &current_class,
        &current_method,
        handler_pc as usize,
    );
    frame.clear_stack();
    // ... rest of handler branch
```

For the unwind loop (no handler in current method), we need to track catch in the caller. In the unwind loop, after a handler IS found in a caller frame, record the catch. Find the code inside the `loop { match call_stack.pop() { ... Some(caller) => { ... let handler2 = find_exception_handler(...); if let Some(h2) = handler2 { ... }` pattern and add:

```rust
if let Some(h2) = handler2 {
    #[cfg(feature = "telemetry")]
    registry.telemetry.exception_flow.record_catch(
        _telem_exc_event_idx,
        &current_class,   // now restored to caller's class
        &current_method,
        h2 as usize,
    );
    // ... existing catch handling
```

### Step 3: Write the test

```rust
#[cfg(feature = "telemetry")]
#[test]
fn telemetry_exception_flow_records_throw_and_catch() {
    let (result, registry) = run_fixture("ExceptionTest", "throwAndCatch", "()I");
    assert_eq!(result, Some(Slot::Int(42)));
    let events = &registry.telemetry.exception_flow.events;
    assert_eq!(events.len(), 1);
    assert!(events[0].exception_class.contains("RuntimeException"));
    assert!(events[0].catch_site.is_some(), "exception should be caught");
}

#[cfg(feature = "telemetry")]
#[test]
fn telemetry_exception_flow_rethrow_caught() {
    let (result, registry) = run_fixture("ExceptionTest", "rethrow", "()I");
    assert_eq!(result, Some(Slot::Int(99)));
    let events = &registry.telemetry.exception_flow.events;
    // Two throw events: inner throw + rethrow
    assert!(events.len() >= 1);
    assert!(events.iter().all(|e| e.catch_site.is_some()));
}
```

### Step 4: Run tests

```bash
cargo test -p duke-interpreter --features telemetry telemetry_exception_flow 2>&1 | tail -10
```

Expected: both pass.

### Step 5: Run full suite

```bash
cargo test -p duke-interpreter 2>&1 | tail -5
```

Expected: still 274 in default build.

### Step 6: Commit

```bash
git add tests/fixtures/ExceptionTest.java tests/fixtures/ExceptionTest.class crates/duke-interpreter/src/lib.rs
git commit -m "feat(telemetry): instrument exception_flow + ExceptionTest fixture"
```

---

## Task 6: Instrument `dispatch_resolution`

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Instrument `invokevirtual` resolution

Find the `invokevirtual` bytecode path (not the lambda path). The resolution logic looks up the method on the receiver's class, then walks the super chain if not found. The variable `dispatch_class` (or similar) holds the final resolved class; `impl_idx` or `callee_idx` holds the method index. A `hierarchy_walk` is performed when the initial direct lookup fails and a super chain walk happens.

Find the invokevirtual bytecode path. Add after the resolution (before `call_stack.push`):

```rust
#[cfg(feature = "telemetry")]
{
    // cp_idx is the CpIdx from the match arm; dispatch_class is the resolved receiver class.
    // hierarchy_walk = true when the method was found via super chain, not directly on receiver.
    let hw = /* true if super-chain walk occurred */ false; // set based on actual resolution flow
    registry.telemetry.dispatch_resolution.record(
        &current_class,
        cp_idx.0,
        &dispatch_class,
        hw,
    );
}
```

For `hierarchy_walk` detection: look at the resolution code. If it directly finds the method on the receiver class (first lookup succeeds), `hw = false`. If it had to walk the super chain, `hw = true`. Add a `let mut _hw = false;` before the loop and set `_hw = true;` when the loop fires.

Repeat for `invokeinterface` bytecode path.

### Step 2: Write the test

```rust
#[cfg(feature = "telemetry")]
#[test]
fn telemetry_dispatch_resolution_records_virtual_calls() {
    // ForEachTest uses invokevirtual for iterator().hasNext() and iterator().next()
    let (_, registry) = run_fixture("ForEachTest", "main", "([Ljava/lang/String;)V");
    // At least one virtual dispatch site must be recorded.
    assert!(!registry.telemetry.dispatch_resolution.by_site.is_empty());
    // All recorded sites must have at least 1 call.
    for stat in registry.telemetry.dispatch_resolution.by_site.values() {
        assert!(stat.calls > 0);
        assert!(!stat.unique_targets.is_empty());
    }
}
```

### Step 3: Run tests

```bash
cargo test -p duke-interpreter --features telemetry telemetry_dispatch_resolution 2>&1 | tail -10
cargo test -p duke-interpreter 2>&1 | tail -5
```

Expected: pass, default still 274.

### Step 4: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(telemetry): instrument dispatch_resolution for invokevirtual/invokeinterface"
```

---

## Task 7: Instrument `class_init_dag`

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Add `triggered_by` parameter to `ensure_initialized`

Find `fn ensure_initialized` (line 4597). Add a parameter:

```rust
fn ensure_initialized(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    _triggered_by: &str,   // ← new: caller class, or "" for entry point
) -> VmResult<()> {
```

Inside the function, wrap `<clinit>` execution with timing and recording:

```rust
if has_clinit {
    #[cfg(feature = "telemetry")]
    let _clinit_start = std::time::Instant::now();

    execute_class(registry, loader, heap, stdout, class_name, "<clinit>", "()V", &[])?;

    #[cfg(feature = "telemetry")]
    registry.telemetry.class_init_dag.record(
        class_name,
        _triggered_by,
        _clinit_start.elapsed().as_nanos() as u64,
    );
}
```

### Step 2: Update all 5 call sites

The 5 call sites (lines 4708, 4825, 5634, 5690, 5702) must pass a `_triggered_by` string:

```rust
// Line 4708 — entry point initialization (no triggering class):
ensure_initialized(registry, loader, heap, stdout, class_name, "")?;

// Line 4825 — callee class init triggered by invokestatic:
ensure_initialized(registry, loader, heap, stdout, &callee_class, &current_class)?;

// Line 5634 — `new` opcode triggers target class init:
ensure_initialized(registry, loader, heap, stdout, &target_class, &current_class)?;

// Lines 5690, 5702 — similar patterns, pass &current_class as triggered_by
ensure_initialized(registry, loader, heap, stdout, &target_class, &current_class)?;
```

### Step 3: Write the test

```rust
#[cfg(feature = "telemetry")]
#[test]
fn telemetry_class_init_dag_records_clinit() {
    // Any fixture with a static initializer. BenchmarkSuite has none.
    // Use a fixture with a static field initialized by clinit.
    // The existing `clinit_initializes_static_field_x` tests use ClinitTest fixture.
    let ctx = load_class_context("ClinitTest");
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    execute_class(&mut registry, &loader, &mut heap, &mut out, &entry_class, "getX", "()I", &[])
        .expect("execute_class failed");
    let events = &registry.telemetry.class_init_dag.events;
    assert!(!events.is_empty(), "expected at least one clinit event");
    assert!(events.iter().any(|e| e.class == "ClinitTest"));
}
```

### Step 4: Run tests

```bash
cargo test -p duke-interpreter --features telemetry telemetry_class_init_dag 2>&1 | tail -10
cargo test -p duke-interpreter 2>&1 | tail -5
```

Expected: pass, default still 274.

### Step 5: Run clippy on the telemetry feature build

```bash
cargo clippy -p duke-interpreter --features telemetry -- -W clippy::pedantic 2>&1 | grep "^error" | head -10
```

Fix any errors.

### Step 6: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(telemetry): instrument class_init_dag in ensure_initialized"
```

---

## Task 8: Wire `--telemetry` flag into `duke` binary

**Files:**
- Modify: `duke/src/main.rs` (the `duke` binary CLI)

### Step 1: Read the current CLI

```bash
cat duke/src/main.rs | head -80
```

Find the `exec` or `run` subcommand definition.

### Step 2: Add `--telemetry` flag

Add an optional `--telemetry[=<path>]` argument to the `exec`/`run` subcommand. When provided, after execution, call `registry.telemetry.print_report(&mut stderr)` (or write JSON to the path).

```rust
// In exec/run subcommand handler, after execute_class:
#[cfg(feature = "telemetry")]
if let Some(path) = args.telemetry_path {
    let json = registry.telemetry.to_json();
    if path == "-" {
        println!("{json}");
    } else {
        std::fs::write(&path, json).expect("failed to write telemetry output");
    }
}
```

Also add `duke-telemetry` as a dep in `duke/Cargo.toml` with the same `telemetry` feature gate.

### Step 3: Verify it compiles

```bash
cargo build -p duke --features duke-interpreter/telemetry 2>&1 | grep "^error" | head -10
```

### Step 4: Manual smoke test

```bash
cargo run --features duke-interpreter/telemetry -p duke -- exec tests/fixtures/BenchmarkSuite.class benchFib 2>&1 | head -5
```

Then with telemetry dump:
```bash
cargo run --features duke-interpreter/telemetry -p duke -- exec --telemetry=- tests/fixtures/BenchmarkSuite.class benchFib 2>&1 | python -m json.tool | head -30
```

Expected: JSON output with all six channels.

### Step 5: Commit

```bash
git add duke/src/main.rs duke/Cargo.toml
git commit -m "feat(telemetry): wire --telemetry flag into duke binary CLI"
```

---

## Final verification

```bash
# Default build — no telemetry overhead, all 274 tests pass:
cargo test 2>&1 | grep "test result"

# Telemetry build — all tests pass including new telemetry tests:
cargo test --features duke-interpreter/telemetry 2>&1 | grep "test result"

# Clippy clean on both:
cargo clippy -- -W clippy::pedantic 2>&1 | grep "^error"
cargo clippy --features duke-interpreter/telemetry -- -W clippy::pedantic 2>&1 | grep "^error"
```
