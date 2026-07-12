//! CLI-level smoke tests for the vendored real Spring Boot fixtures.
//!
//! Drives the real `duke` binary via `duke -jar <fixture>` against the two
//! committed fat JARs under `tests/fixtures/oss-jars/spring-boot/`:
//!   * `duke-spring-boot-app-3.5.12.jar`    (real Spring Boot app,
//!     `Start-Class com.example.duke.DukeApplication`)
//!   * `duke-spring-boot-ladder-3.5.12.jar` (commons-logging ladder,
//!     `Start-Class com.example.duke.ladder.LadderApplication`)
//!
//! Mirrors the OSS-jar canary/pin convention (see
//! `crates/duke-interpreter/tests/oss_jar_smoke.rs`): for each fixture an
//! `#[ignore]`d end-to-end CANARY asserting a successful boot, plus a
//! non-ignored PIN test that asserts the process still fails with EXACTLY the
//! current first blocker. When boot advances past the pinned blocker the pin
//! trips, forcing a re-observe.
//!
//! See findings: docs/findings/2026-07-10-spring-boot-real-app.md

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at the `duke` crate; the workspace root is its parent.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn spring_boot_fixture(name: &str) -> PathBuf {
    repo_root()
        .join("tests/fixtures/oss-jars/spring-boot")
        .join(name)
}

const APP_JAR: &str = "duke-spring-boot-app-3.5.12.jar";
const LADDER_JAR: &str = "duke-spring-boot-ladder-3.5.12.jar";

// Per-fixture first blockers (re-observed 2026-07-10, after #1320 landed the gson
// natives and advanced the app fixture's boot). Both fixtures boot into the
// JarLauncher / Spring Boot classpath-scanning path and land on a missing synthetic
// `java/lang/ClassLoader` method that lives in crates/duke-interpreter/src/stdlib.rs
// (other lane), so both are pinned here rather than fixed.
//
// The synthetic `ClassLoader` natives `getSystemResources`, `getSystemClassLoader`
// and base `loadClass` are now implemented (ClassLoader lane), advancing BOTH
// fixtures past the previous ClassLoader pins. The interpreter method-dispatch
// lane then cleared the inherited `java/lang/Object.getClass()` virtual dispatch
// for interface-typed callsites, advancing the app fixture again. Current
// frontiers:
//   * APP cleared the logback timestamp `java/time/format/DateTimeFormatter`
//     rung: a minimal synthetic `DateTimeFormatter` now implements
//     `ofPattern`/`withZone`/`withLocale`/`format(TemporalAccessor)` with a small
//     pattern engine (UTC-only, fixed en-US; see native/java_time.rs). logback's
//     `CachingDateFormatter` now constructs and drives its formatter without a
//     `method not found`. It now lands on `duke: runtime error: operand stack
//     underflow`, raised inside logback's `COWArrayList.addIfAbsent`: that method
//     calls `java/util/concurrent/CopyOnWriteArrayList.addIfAbsent(Object)Z` and
//     `pop`s the boolean result, but duke resolves the unregistered
//     `CopyOnWriteArrayList` leniently (no-op `<init>`, and the `addIfAbsent`
//     invokevirtual returns void instead of a boolean), so the following `pop`
//     underflows the operand stack. The real gap is a synthetic
//     `java/util/concurrent/CopyOnWriteArrayList` (or a strict method-dispatch
//     that raises `method not found` instead of silently returning void)
//     (java.util.concurrent / interpreter lenient-dispatch lane).
//   * LADDER cleared the commons-logging `LogFactory.newStandardFactory`
//     `Class.forName("...SLF4JProvider", false, cl)` availability probe: the
//     3-arg `Class.forName` native was eagerly computing the class key (which
//     itself raises `ClassNotFound`) before the not-found result could be turned
//     into a catchable `ClassNotFoundException`, so the probe crashed instead of
//     returning "class absent". It then landed on missing
//     `org/apache/logging/log4j/MarkerManager` — reached while initializing
//     `Log4jApiLogFactory` (whose `<clinit>` calls `MarkerManager.getMarker`).
//     The interpreter linkage-error lane now models real JVM semantics: that
//     class-resolution failure raises a catchable `NoClassDefFoundError` (a
//     `LinkageError`) at the failing instruction instead of a fatal Rust error,
//     so commons-logging's `catch (LinkageError)` fires and falls through to
//     `LogFactoryImpl`. The MarkerManager rung is CLEARED. The ladder then landed
//     on `method not found: java/util/Hashtable.computeIfAbsent(...)`, which the
//     synthetic `java/util/Hashtable` now implements (the full Map-default family
//     — computeIfAbsent/computeIfPresent/compute/merge/forEach/replaceAll plus
//     putIfAbsent/getOrDefault/replace/containsValue/putAll — is registered on
//     `java/util/Hashtable`, reusing the HashMap natives). That rung is CLEARED.
//     The ladder now lands on `method not found:
//     org/apache/commons/logging/impl/LogFactoryImpl.objectId(...)` — the next
//     real blocker in commons-logging's LogFactoryImpl bootstrap (collections
//     lane cleared; this is a different missing-method rung).
// If either boot advances past its pin, re-observe and update.
// See docs/findings/2026-07-10-spring-boot-real-app.md.
const APP_BLOCKER: &str = "duke: runtime error: operand stack underflow";
const LADDER_BLOCKER: &str = "method not found: org/apache/commons/logging/impl/LogFactoryImpl.objectId(Ljava/lang/Object;)Ljava/lang/String;";

fn run_fixture(jar: &str) -> Output {
    let jar_path = spring_boot_fixture(jar);
    assert!(
        jar_path.exists(),
        "fixture jar should be committed at {}",
        jar_path.display()
    );
    Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("-jar")
        .arg(&jar_path)
        .output()
        .unwrap_or_else(|err| panic!("run duke -jar on {jar}: {err}"))
}

fn combined_output(output: &Output) -> String {
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    combined
}

// ---------------------------------------------------------------------------
// App fixture
// ---------------------------------------------------------------------------

/// End-to-end CANARY for the real Spring Boot app fixture. Ignored until boot
/// reaches the started-application line. Un-ignore when the happy path clears.
#[test]
#[ignore = "Blocked on a runtime 'operand stack underflow' raised inside logback's \
            COWArrayList.addIfAbsent — duke resolves java/util/concurrent/CopyOnWriteArrayList \
            leniently so its addIfAbsent(Object)Z returns void and the following pop underflows \
            (java.util.concurrent / interpreter lenient-dispatch lane; the DateTimeFormatter \
            ofPattern rung is now cleared); keep ignored until the Spring Boot app boot completes. \
            See docs/findings/2026-07-10-spring-boot-real-app.md"]
fn spring_boot_app_boots_end_to_end() {
    let output = run_fixture(APP_JAR);
    let combined = combined_output(&output);

    assert!(
        output.status.success(),
        "duke -jar on the Spring Boot app fixture should exit cleanly; output:\n{combined}"
    );
    assert!(
        combined.contains("Duke Spring Boot fixture application started successfully.")
            || combined.contains("Started ")
                && combined.contains(" in ")
                && combined.contains(" seconds"),
        "expected the Spring Boot app fixture to report a successful start; output:\n{combined}"
    );
}

/// PIN: the app fixture still fails at the current first blocker. When boot
/// advances past it this trips and the pin must be re-observed.
/// See docs/findings/2026-07-10-spring-boot-real-app.md
#[test]
fn spring_boot_app_surfaces_next_missing_capability_explicitly() {
    let output = run_fixture(APP_JAR);
    let combined = combined_output(&output);

    assert!(
        !output.status.success(),
        "app fixture is expected to still fail at the pinned blocker; output:\n{combined}"
    );
    assert!(
        combined.contains(APP_BLOCKER),
        "expected the app fixture to stay pinned at the current first blocker \
         ({APP_BLOCKER:?}); if it moved, re-observe and update this pin \
         (docs/findings/2026-07-10-spring-boot-real-app.md). Output:\n{combined}"
    );
}

// ---------------------------------------------------------------------------
// Ladder fixture
// ---------------------------------------------------------------------------

/// End-to-end CANARY for the commons-logging ladder fixture. Ignored until boot
/// completes all rungs. Un-ignore when the happy path clears.
#[test]
#[ignore = "Blocked on missing \
            org/apache/commons/logging/impl/LogFactoryImpl.objectId(Object)String in the \
            LogFactoryImpl bootstrap (a NoSuchMethodError-territory blocker). The \
            Hashtable.computeIfAbsent rung is now cleared: java/util/Hashtable registers the \
            full Map-default native family (computeIfAbsent/computeIfPresent/compute/merge/\
            forEach/replaceAll plus putIfAbsent/getOrDefault/replace/containsValue/putAll), so \
            the LogFactoryImpl fallback advances past it; keep ignored until the ladder fixture \
            completes. See docs/findings/2026-07-10-spring-boot-real-app.md"]
fn spring_boot_ladder_boots_end_to_end() {
    let output = run_fixture(LADDER_JAR);
    let combined = combined_output(&output);

    assert!(
        output.status.success(),
        "duke -jar on the ladder fixture should exit cleanly; output:\n{combined}"
    );
    assert!(
        combined.contains("Duke ladder fixture completed all rungs successfully.")
            || combined.contains("Duke ladder fixture application started successfully."),
        "expected the ladder fixture to report completion; output:\n{combined}"
    );
}

/// PIN: the ladder fixture still fails at the current first blocker. When boot
/// advances past it this trips and the pin must be re-observed.
/// See docs/findings/2026-07-10-spring-boot-real-app.md
#[test]
fn spring_boot_ladder_surfaces_next_missing_capability_explicitly() {
    let output = run_fixture(LADDER_JAR);
    let combined = combined_output(&output);

    assert!(
        !output.status.success(),
        "ladder fixture is expected to still fail at the pinned blocker; output:\n{combined}"
    );
    assert!(
        combined.contains(LADDER_BLOCKER),
        "expected the ladder fixture to stay pinned at the current first blocker \
         ({LADDER_BLOCKER:?}); if it moved, re-observe and update this pin \
         (docs/findings/2026-07-10-spring-boot-real-app.md). Output:\n{combined}"
    );
}
