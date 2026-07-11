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
// fixtures past the previous ClassLoader pins. The frontier has moved out of the
// ClassLoader lane on both:
//   * APP now lands on `getClass()` virtual dispatch failing to resolve the
//     inherited `java/lang/Object` native for a class loaded through the runtime
//     `loadClass` path (interpreter method-dispatch / class-identity lane, not the
//     ClassLoader-native lane).
//   * LADDER now runs deep into real commons-logging `LogFactory.getFactory` and
//     hits an `operand stack underflow` when `java/lang/ref/WeakReference.get()`
//     returns no value (java.lang.ref reference-object native lane).
// If either boot advances past its pin, re-observe and update.
// See docs/findings/2026-07-10-spring-boot-real-app.md.
const APP_BLOCKER: &str = "method not found: ch/qos/logback/classic/util/DefaultJoranConfigurator.getClass()Ljava/lang/Class;";
const LADDER_BLOCKER: &str = "operand stack underflow";

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
#[ignore = "Blocked on inherited java/lang/Object.getClass() virtual dispatch failing \
            for a class loaded through the runtime loadClass path (interpreter \
            method-dispatch lane; the ClassLoader getSystemClassLoader/loadClass rungs \
            are now cleared); keep ignored until the Spring Boot app boot completes. \
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
#[ignore = "Blocked on operand stack underflow when java/lang/ref/WeakReference.get() \
            returns no value deep in commons-logging LogFactory.getFactory \
            (java.lang.ref reference-object native lane; the ClassLoader \
            getSystemResources rung is now cleared); keep ignored until the ladder \
            fixture completes. See docs/findings/2026-07-10-spring-boot-real-app.md"]
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
