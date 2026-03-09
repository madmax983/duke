# VM Telemetry Design

Date: 2026-03-09
Duke version: Phase 23 (327 tests, 76ms benchFib)

---

## Goal

Add structured diagnostic telemetry to the Duke interpreter: six channels covering
bytecode costs, object allocation sites, class initialization chains, exception flows,
virtual dispatch resolution, and native boundary crossings. Telemetry is zero-cost in
default builds and opt-in via a compile-time feature flag.

---

## Architecture

### Feature Flag

```toml
# crates/duke-telemetry/Cargo.toml
[features]
default = []
# No telemetry overhead in default builds.
# Enable with: cargo build --features duke-telemetry/telemetry
```

All instrumentation in `duke-interpreter` is gated with `#[cfg(feature = "telemetry")]`
blocks — no runtime branch, no data structures, no cost when the feature is absent.

### New Crate: `duke-telemetry`

```
crates/duke-telemetry/
├── Cargo.toml
├── src/
│   ├── lib.rs             # re-exports TelemetryStore + all channel types
│   ├── bytecode_cost.rs
│   ├── object_lineage.rs
│   ├── class_init_dag.rs
│   ├── exception_flow.rs
│   ├── dispatch_resolution.rs
│   └── native_boundary.rs
└── tests/
    ├── bytecode_cost.rs
    ├── object_lineage.rs
    ├── class_init_dag.rs
    ├── exception_flow.rs
    ├── dispatch_resolution.rs
    └── native_boundary.rs
```

`duke-interpreter` takes `duke-telemetry` as an optional dependency, activated only
when the `telemetry` feature is enabled.

### TelemetryStore Location

`TelemetryStore` lives as a field on `ClassRegistry`:

```rust
pub struct ClassRegistry {
    // ... existing fields ...
    #[cfg(feature = "telemetry")]
    pub telemetry: TelemetryStore,
}
```

`execute_class` already takes `&mut ClassRegistry`, so every instrumentation site has
access without any signature changes. Class initialization (`<clinit>`) also has
registry access, covering `class_init_dag`.

---

## The Six Channels

### 1. `bytecode_cost`

Time and invocation count per opcode and per bytecode site.

```rust
pub struct BytecodeCostStore {
    pub by_opcode: HashMap<&'static str, OpcodeStat>,
    pub by_site:   HashMap<(String, String, usize), OpcodeStat>,
    //                      class   method   pc
}

pub struct OpcodeStat {
    pub count:    u64,
    pub total_ns: u64,
}
```

**Instrumentation point:** Each `match instr { ... }` arm in `execute_class`.
Wrap with `std::time::Instant::now()` before dispatch and record elapsed ns after.
`by_site` key is `(current_class.clone(), current_method.clone(), pc)`.

---

### 2. `object_lineage`

Allocation site tracking. Death/lifetime deferred until GC has collection events.

```rust
pub struct ObjectLineageStore {
    pub sites: HashMap<(String, String, usize), AllocationSite>,
    //                  class   method   pc
}

pub struct AllocationSite {
    pub class_allocated: String,
    pub count:           u64,
}
```

**Instrumentation point:** `Instruction::New(cp_idx)` arm in `execute_class`.
Record allocating class, allocating method, PC, and the class being instantiated.

---

### 3. `class_init_dag`

`<clinit>` trigger chain with timings.

```rust
pub struct ClassInitDagStore {
    pub events: Vec<ClinitEvent>,
}

pub struct ClinitEvent {
    pub class:        String,
    pub triggered_by: Option<String>,  // class that caused this <clinit> to fire
    pub duration_ns:  u64,
}
```

**Instrumentation point:** The `initialized` HashSet check site before `<clinit>`
is invoked. Record the triggering class from `current_class`, wrap the `<clinit>`
execution with timing.

---

### 4. `exception_flow`

Throw site → catch site graph, including rethrows.

```rust
pub struct ExceptionFlowStore {
    pub events: Vec<ExceptionEvent>,
}

pub struct ExceptionEvent {
    pub exception_class: String,
    pub throw_site:      (String, String, usize),  // (class, method, pc)
    pub catch_site:      Option<(String, String, usize)>,
    pub rethrows:        u32,
}
```

**Instrumentation points:**
- `Instruction::Athrow` — record throw site.
- Exception handler dispatch loop — record catch site when a matching handler is found.
- `None` catch_site means the exception propagated uncaught to the top of the call stack.

---

### 5. `dispatch_resolution`

Virtual and interface call resolution: target class distribution and hierarchy walk frequency.

```rust
pub struct DispatchResolutionStore {
    pub by_site: HashMap<(String, u16), DispatchStat>,
    //                    class   cp_idx
}

pub struct DispatchStat {
    pub calls:           u64,
    pub unique_targets:  HashSet<String>,  // runtime classes seen at this call site
    pub hierarchy_walks: u64,              // how often a superclass walk was needed
}
```

**Instrumentation points:** `invokevirtual` and `invokeinterface` resolution paths.
Increment `hierarchy_walks` when the method is not found directly on the receiver's
class and a superclass search is required.

---

### 6. `native_boundary`

Per-native invocation count, error rate, and timing.

```rust
pub struct NativeBoundaryStore {
    pub by_method: HashMap<(String, String), NativeStat>,
    //                      class   name
}

pub struct NativeStat {
    pub calls:    u64,
    pub errors:   u64,
    pub total_ns: u64,
}
```

**Instrumentation point:** Native dispatch branch in all invoke handlers
(`invokestatic`, `invokevirtual`, `invokespecial`, `invokeinterface`). Wrap the
`native_handler(...)` call with timing; increment `errors` if the handler returns
`Err(_)`.

---

## Query API

`TelemetryStore` fields are `pub`. Tests and callers read directly:

```rust
#[cfg(feature = "telemetry")]
{
    let t = &registry.telemetry;
    assert_eq!(t.bytecode_cost.by_opcode["invokestatic"].count, 500_000);
    assert!(t.dispatch_resolution.by_site[&("BenchmarkSuite".into(), 3)].unique_targets.len() == 1);
    assert_eq!(t.native_boundary.by_method[&("java/lang/System".into(), "exit".into())].errors, 0);
}
```

---

## Output / Serialization

`serde` and `serde_json` are optional dependencies, activated only with the
`telemetry` feature:

```toml
[dependencies]
serde      = { version = "1", features = ["derive"], optional = true }
serde_json = { version = "1", optional = true }
```

`TelemetryStore` and all channel types derive `serde::Serialize`.

Two output methods:

```rust
impl TelemetryStore {
    /// Serialize to pretty-printed JSON.
    pub fn to_json(&self) -> String { serde_json::to_string_pretty(self).unwrap() }

    /// Human-readable top-N summary per channel.
    pub fn print_report(&self, w: &mut dyn Write);
}
```

`print_report` emits a top-10-by-count summary per channel — readable in the terminal.
JSON dump is for programmatic use (pipe to jq, diff across runs).

The `duke` binary gets a `--telemetry[=path]` flag: dumps JSON to `path` or stdout.

---

## Testing

Each channel has an integration test file in `crates/duke-telemetry/tests/`:

```rust
// tests/bytecode_cost.rs
#[cfg(feature = "telemetry")]
#[test]
fn counts_iadd_in_bench_sum() {
    let (_, registry) = run_fixture("BenchmarkSuite", "benchSum", "()I");
    let stat = &registry.telemetry.bytecode_cost.by_opcode["iadd"];
    assert_eq!(stat.count, 499_999);
}

// tests/exception_flow.rs
#[cfg(feature = "telemetry")]
#[test]
fn records_throw_and_catch() {
    let (_, registry) = run_fixture("ExceptionTest", "throwAndCatch", "()V");
    let events = &registry.telemetry.exception_flow.events;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].exception_class, "java/lang/RuntimeException");
    assert!(events[0].catch_site.is_some());
}
```

`run_fixture` returns `(Slot, ClassRegistry)` so tests inspect telemetry alongside
the return value.

New Java fixtures needed:
- `tests/fixtures/ExceptionTest.java` — throw + catch + rethrow
- `tests/fixtures/DispatchTest.java` — virtual call with multiple concrete receiver types

Existing fixtures (`BenchmarkSuite`, `ForEachTest`) cover `bytecode_cost`,
`object_lineage`, `class_init_dag`, and `native_boundary`.

CI default: `cargo test` — no telemetry feature, zero overhead.
Telemetry CI: `cargo test --features telemetry -p duke-telemetry -p duke-interpreter`.

---

## Implementation Order

1. Create `duke-telemetry` crate with all six channel structs + `TelemetryStore`
2. Add `TelemetryStore` field to `ClassRegistry` behind feature flag
3. Instrument `execute_class` — `bytecode_cost` first (simplest, validates the pattern)
4. Instrument `object_lineage` (`New` opcode), `native_boundary` (native dispatch)
5. Instrument `exception_flow` (`Athrow` + handler dispatch)
6. Instrument `dispatch_resolution` (`invokevirtual`/`invokeinterface`)
7. Instrument `class_init_dag` (`<clinit>` trigger site)
8. Add `serde` serialization + `to_json` + `print_report`
9. Wire `--telemetry` flag into the `duke` binary CLI
10. Write integration tests for all six channels
