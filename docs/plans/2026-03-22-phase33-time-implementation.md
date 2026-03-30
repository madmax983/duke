# Phase 33 Time Primitives Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add real `java/lang/System.currentTimeMillis()` and `java/lang/System.nanoTime()` support to Duke so Java fixtures can read host wall-clock time, observe advancing timestamps, and compute elapsed durations.

**Architecture:** Keep this slice inside `crates/duke-interpreter/src/lib.rs` by extending the existing synthetic `java/lang/System` bootstrap registrations with two new static natives. Implement wall-clock millis through `std::time::SystemTime`, implement monotonic nanos through a process-wide `std::time::Instant` origin cached behind `OnceLock`, and drive the work from fixture-backed interpreter tests first. Do not try to fake the whole `java.time` package in this phase; prove the primitive contracts directly and leave broader JDK time integration for a later loader-oriented follow-up.

**Tech Stack:** Rust 2024, `std::sync::OnceLock`, `std::time::{Duration, Instant, SystemTime, UNIX_EPOCH}`, `duke-interpreter`, checked-in Java fixtures under `tests/fixtures`, `javac --release 21`, `cargo test`, `cargo clippy`.

---

## Background Notes

- `crates/duke-interpreter/src/lib.rs` already synthesizes `java/lang/System`, but it currently exposes only `out` and `exit`; Phase 33 should extend that existing native bridge instead of inventing a new runtime layer.
- Fixture tests currently use `fixtures_loader()`, which is a `DirectoryLoader` rooted at `tests/fixtures`. That means real JDK `java/time/Instant` classfiles are not in play for the interpreter test harness. This phase should therefore prove the native primitives directly rather than pretending full `java.time` suddenly works.
- Reuse the existing Phase 28 `Thread.sleep(long)` support in fixtures to create deterministic elapsed-time windows instead of busy-spinning until the clock moves.
- Follow `@test-driven-development` literally: no runtime code before the targeted test fails. Follow `@verification-before-completion` before every completion claim or commit by rerunning the command listed in the task.

### Task 1: Red Tests For `System.currentTimeMillis()`

**Files:**
- Create: `tests/fixtures/TimePrimitivesTest.java`
- Create: `tests/fixtures/TimePrimitivesTest.class`
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Write the failing fixture source**

Create `tests/fixtures/TimePrimitivesTest.java`:

```java
public final class TimePrimitivesTest {
    private TimePrimitivesTest() {
    }

    public static long currentTimeMillisNow() {
        return System.currentTimeMillis();
    }

    public static int currentTimeMillisAdvancesAfterSleep() throws Exception {
        long before = System.currentTimeMillis();
        Thread.sleep(25L);
        long after = System.currentTimeMillis();
        return after > before ? 1 : 0;
    }
}
```

**Step 2: Compile the fixture**

Run: `javac --release 21 tests/fixtures/TimePrimitivesTest.java -d tests/fixtures/`

Expected: `tests/fixtures/TimePrimitivesTest.class` is created with no errors.

**Step 3: Add failing interpreter tests**

In `crates/duke-interpreter/src/lib.rs`, add a small helper near the other fixture runners:

```rust
fn run_bootstrap_long(class_name: &str, method_name: &str, descriptor: &str) -> i64 {
    let ctx = load_class_context(class_name);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        &entry_class,
        method_name,
        descriptor,
        &[],
    )
    .expect("fixture should execute");
    match result {
        Some(Slot::Long(value)) => value,
        other => panic!("expected long result, got {other:?}"),
    }
}
```

Add tests near the other phase integration blocks:

```rust
#[test]
fn time_current_time_millis_matches_host_wall_clock_window() {
    let host_before = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("host clock is after epoch")
            .as_millis(),
    )
    .expect("millis fit in i64");
    let value = run_bootstrap_long("TimePrimitivesTest.class", "currentTimeMillisNow", "()J");
    let host_after = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("host clock is after epoch")
            .as_millis(),
    )
    .expect("millis fit in i64");

    assert!(
        value >= host_before && value <= host_after,
        "expected {value} within [{host_before}, {host_after}]",
    );
}

#[test]
fn time_current_time_millis_advances_after_sleep() {
    assert_eq!(
        run_bootstrap_int("TimePrimitivesTest.class", "currentTimeMillisAdvancesAfterSleep", "()I"),
        1,
    );
}
```

**Step 4: Run the targeted tests to verify RED**

Run: `cargo test -p duke-interpreter time_current_time_millis -- --nocapture`

Expected: FAIL because `java/lang/System.currentTimeMillis:()J` is not registered yet.

**Step 5: Commit the red test changes**

```bash
git add tests/fixtures/TimePrimitivesTest.java tests/fixtures/TimePrimitivesTest.class crates/duke-interpreter/src/lib.rs
git commit -m "test: add red time primitive fixtures"
```

### Task 2: Green `System.currentTimeMillis()` And Wall-Clock Conversion

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Register the new static native**

In `bootstrap_stdlib`, add:

```rust
registry.natives_mut().register(
    "java/lang/System",
    "currentTimeMillis",
    "()J",
    native_system_current_time_millis,
);
```

Keep the registration next to the existing `System.exit` native so the `System` surface stays in one place.

**Step 2: Add a small wall-clock helper**

Add a helper near the other native utilities:

```rust
fn system_time_to_epoch_millis(now: std::time::SystemTime) -> i64 {
    match now.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_millis()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}
```

Rules:

- normal host clocks return epoch millis
- pre-epoch timestamps clamp to `0` instead of panicking
- pathological overflow clamps to `i64::MAX`

**Step 3: Implement the native**

Add:

```rust
fn native_system_current_time_millis(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn std::io::Write,
) -> VmResult<Option<Slot>> {
    Ok(Some(Slot::Long(system_time_to_epoch_millis(
        std::time::SystemTime::now(),
    ))))
}
```

**Step 4: Add helper unit tests for the conversion edge cases**

In the existing unit-test section of `crates/duke-interpreter/src/lib.rs`, add:

```rust
#[test]
fn time_system_time_to_epoch_millis_converts_forward_values() {
    let sample = std::time::UNIX_EPOCH + std::time::Duration::from_millis(1_234);
    assert_eq!(system_time_to_epoch_millis(sample), 1_234);
}

#[test]
fn time_system_time_to_epoch_millis_clamps_pre_epoch_to_zero() {
    let sample = std::time::UNIX_EPOCH - std::time::Duration::from_secs(1);
    assert_eq!(system_time_to_epoch_millis(sample), 0);
}
```

**Step 5: Run verification for the wall-clock slice**

Run: `cargo test -p duke-interpreter time_ -- --nocapture`

Expected: PASS for the two `currentTimeMillis` integration tests and the helper unit tests.

**Step 6: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat: add System.currentTimeMillis native"
```

### Task 3: Red Tests For `System.nanoTime()` Duration Measurement

**Files:**
- Modify: `tests/fixtures/TimePrimitivesTest.java`
- Create: `tests/fixtures/TimePrimitivesTest.class`
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Extend the fixture with monotonic-timer scenarios**

Append to `tests/fixtures/TimePrimitivesTest.java`:

```java
public static long nanoTimeDeltaAfterSleep() throws Exception {
    long before = System.nanoTime();
    Thread.sleep(10L);
    return System.nanoTime() - before;
}

public static int nanoTimeSupportsDurationMath() throws Exception {
    long start = System.nanoTime();
    Thread.sleep(10L);
    long elapsedNanos = System.nanoTime() - start;
    long elapsedMillis = elapsedNanos / 1_000_000L;
    return elapsedNanos > 0L && elapsedMillis >= 1L ? 1 : 0;
}
```

**Step 2: Recompile the fixture**

Run: `javac --release 21 tests/fixtures/TimePrimitivesTest.java -d tests/fixtures/`

Expected: updated `TimePrimitivesTest.class` is produced with no errors.

**Step 3: Add failing interpreter tests**

Add:

```rust
#[test]
fn time_nano_time_returns_positive_elapsed_duration() {
    let delta = run_bootstrap_long("TimePrimitivesTest.class", "nanoTimeDeltaAfterSleep", "()J");
    assert!(delta >= 1_000_000, "expected at least 1 ms in nanos, got {delta}");
}

#[test]
fn time_nano_time_supports_java_duration_math() {
    assert_eq!(
        run_bootstrap_int("TimePrimitivesTest.class", "nanoTimeSupportsDurationMath", "()I"),
        1,
    );
}
```

**Step 4: Run the targeted tests to verify RED**

Run: `cargo test -p duke-interpreter time_nano_ -- --nocapture`

Expected: FAIL because `java/lang/System.nanoTime:()J` is still missing.

**Step 5: Commit**

```bash
git add tests/fixtures/TimePrimitivesTest.java tests/fixtures/TimePrimitivesTest.class crates/duke-interpreter/src/lib.rs
git commit -m "test: add red nanoTime duration tests"
```

### Task 4: Green `System.nanoTime()` With A Monotonic Origin

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Add a process-wide monotonic origin**

Near the top-level imports / native helpers, add:

```rust
static NANO_TIME_ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn monotonic_nano_time_now() -> i64 {
    let origin = NANO_TIME_ORIGIN.get_or_init(std::time::Instant::now);
    i64::try_from(origin.elapsed().as_nanos()).unwrap_or(i64::MAX)
}
```

Use `OnceLock` so every call shares one origin without threading extra runtime state through the interpreter.

**Step 2: Register the native**

In `bootstrap_stdlib`, add:

```rust
registry
    .natives_mut()
    .register("java/lang/System", "nanoTime", "()J", native_system_nano_time);
```

**Step 3: Implement the native**

Add:

```rust
fn native_system_nano_time(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn std::io::Write,
) -> VmResult<Option<Slot>> {
    Ok(Some(Slot::Long(monotonic_nano_time_now())))
}
```

**Step 4: Add a small monotonic helper test**

Add:

```rust
#[test]
fn time_monotonic_nano_time_is_non_decreasing() {
    let first = monotonic_nano_time_now();
    std::thread::sleep(std::time::Duration::from_millis(1));
    let second = monotonic_nano_time_now();
    assert!(second >= first, "expected non-decreasing nanos: {first} -> {second}");
}
```

**Step 5: Run the full Phase 33 verification**

Run: `cargo test -p duke-interpreter time_ -- --nocapture`

Expected: PASS for all currentTimeMillis, nanoTime, and helper tests.

**Step 6: Run clippy on the touched crate**

Run: `cargo clippy -p duke-interpreter -- -D warnings`

Expected: PASS with no warnings.

**Step 7: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat: add System.nanoTime native"
```
