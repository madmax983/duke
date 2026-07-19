//! CLI-level `--real-jdk` frontier pins for the vendored Spring Boot fat JARs.
//!
//! Companion to `spring_boot_real_app.rs` (which drives the same fixtures WITHOUT
//! real-JDK shadow). Under `--real-jdk`, non-allowlisted synthetic stdlib classes
//! load real JDK-21 bytecode. `java/net/URL` used to be shadowed while its instances
//! were still minted 1-slot by synthetic natives (`allocate_string_backed_object`),
//! so the real `URL.toExternalForm` indexed `handler` at slot 10 on a 1-slot object
//! and the interpreter panicked with a raw Rust `index out of bounds` at the
//! `getfield` opcode. `java/net/URL` is now on `KEEP_SYNTHETIC`, so the allocation
//! and the bytecode share one (synthetic) layout regime and the panic is gone; boot
//! advances to the next honest frontier.
//!
//! Each test asserts two things: (1) the process no longer aborts with a raw
//! `index out of bounds` panic from `execution.rs` (the guard-gap + coherence fix),
//! and (2) boot is pinned at the current verbatim first blocker. When boot advances
//! past the pin this trips, forcing a re-observe.
//!
//! Requires a real JDK jimage (`lib/modules`); without one the tests skip.

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

/// Locate a JDK jimage (`lib/modules`), preferring `JAVA_HOME`. `None` when absent, in
/// which case the `--real-jdk` pins skip (real-JDK shadow cannot be enabled without one).
fn jdk_modules_path() -> Option<PathBuf> {
    let from_java_home = std::env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .map(|path| path.join("lib/modules"));
    let fallbacks = [
        PathBuf::from("/usr/lib/jvm/java-21-openjdk-amd64/lib/modules"),
        PathBuf::from("/usr/lib/jvm/default-java/lib/modules"),
    ];
    from_java_home
        .into_iter()
        .chain(fallbacks)
        .find(|path| path.exists())
}

const APP_JAR: &str = "duke-spring-boot-app-3.5.12.jar";
const LADDER_JAR: &str = "duke-spring-boot-ladder-3.5.12.jar";

// Current `--real-jdk` frontier (re-observed 2026-07-12, after the properties-hashtable
// lane landed a synthetic `java/lang/System.getProperties()` native). With URL coherent
// (KEEP_SYNTHETIC) both fixtures boot past the former `java/net/URL` getfield layout panic;
// they then cleared the real-JDK system-properties bootstrap wall now that
// `System.getProperties()` materializes a live, real-layout `java/util/Properties` object
// graph (allocate + real `<init>` + real `setProperty`), so `StaticProperty.<clinit>`
// advances past it. With the synthetic String now declaring the real-JDK `COMPACT_STRINGS:Z`
// static (init true, mirroring String.<clinit> `iconst_1; putstatic COMPACT_STRINGS`), the
// former `getstatic java/lang/String.COMPACT_STRINGS` fieldref wall is cleared and real String
// bytecode advances into `String.coder()`. That instance method is now provided as a native
// reading real-layout slot1 (0=Latin1/1=UTF16), so the coder wall is cleared and real String
// bytecode advances into the `String.getBytes` copy helpers. Both package-private overloads —
// `getBytes(byte[] dst, int dstBegin, byte coder)` (`([BIB)V`) and
// `getBytes(byte[] dst, int srcBegin, int dstBegin, byte coder, int length)` (`([BIIBI)V`) —
// are now provided as coder-converting copy natives, so the getBytes wall is cleared and the
// real String encode path runs past it.
//
// The former `--real-jdk` frontier was the CLI runtime error
// `constant pool index 0 is not a valid Fieldref`: after String encode completed, bootstrap
// advanced past String into a `getstatic jdk/internal/misc/Unsafe.ARRAY_BOOLEAN_INDEX_SCALE`,
// a static field the synthetic `Unsafe` did not declare (a static miss reported as the
// hardcoded `InvalidFieldref { index: 0 }`). The synthetic `Unsafe` now declares the full
// 9-kind array-intrinsics surface — `ARRAY_<K>_BASE_OFFSET` (== 0) / `ARRAY_<K>_INDEX_SCALE`
// (== 1) for each element kind — so `getstatic Unsafe.ARRAY_*` resolves and the entire Unsafe
// intrinsics lane clears (see `register_unsafe_stdlib` in `crates/duke-interpreter/src/stdlib.rs`).
//
// The `String(StringBuilder)` / `String(StringBuffer)` constructors are now provided as
// real-layout natives (String stage 3a), so the former
// `method not found: java/lang/String.<init>(Ljava/lang/StringBuilder;)V` wall is cleared and
// both boots advance materially further — through the `String(StringBuilder)` mint and deep into
// the real JDK's own bootstrap.
//
// The new `--real-jdk` frontier is the CLI runtime error
// `java exception: java/lang/InternalError`: real-JDK `jdk/internal/util/StaticProperty.<clinit>`
// calls `getProperty(props, key)`, which returns null because the saved system-properties map Duke
// exposes lacks a required key (e.g. `java.home`), so the JDK constructs and throws `InternalError`.
// This is the SAME underlying blocker pinned by
// `crates/duke-interpreter/tests/classloader_bootstrap_frontier.rs` (rendered there as
// `JavaException { class_name: "java/lang/InternalError" }`), a distinct VM saved-system-properties
// bootstrap lane. Out of scope here. If either boot advances past this, re-observe and update.
const REAL_JDK_FRONTIER: &str = "java exception: java/lang/InternalError";

// Markers of the OLD raw panic that this fix eliminates. None of these must appear.
const RAW_PANIC_MARKERS: &[&str] = &["index out of bounds", "execution.rs", "panicked at"];

fn run_fixture_real_jdk(jar: &str) -> Output {
    let jar_path = spring_boot_fixture(jar);
    assert!(
        jar_path.exists(),
        "fixture jar should be committed at {}",
        jar_path.display()
    );
    Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("--real-jdk")
        .arg("-jar")
        .arg(&jar_path)
        // Guarantee the layout-coherence guard is in its default OFF mode: the graceful
        // outcome must hold WITHOUT opting into DUKE_LAYOUT_CHECK.
        .env_remove("DUKE_LAYOUT_CHECK")
        .output()
        .unwrap_or_else(|err| panic!("run duke --real-jdk -jar on {jar}: {err}"))
}

fn combined_output(output: &Output) -> String {
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    combined
}

/// Assert the shared invariant for a `--real-jdk` fixture run: no raw `index out of bounds`
/// panic, and boot is pinned at `REAL_JDK_FRONTIER`.
fn assert_no_url_panic_and_pinned(jar: &str) {
    if jdk_modules_path().is_none() {
        eprintln!("skipping {jar} --real-jdk pin: no JDK jimage available");
        return;
    }

    let output = run_fixture_real_jdk(jar);
    let combined = combined_output(&output);

    for marker in RAW_PANIC_MARKERS {
        assert!(
            !combined.contains(marker),
            "the java/net/URL layout fix must eliminate the raw Rust panic, but output for \
             {jar} still contains {marker:?}:\n{combined}"
        );
    }

    assert!(
        combined.contains(REAL_JDK_FRONTIER),
        "expected {jar} under --real-jdk to be pinned at the current first blocker \
         ({REAL_JDK_FRONTIER:?}); if it moved, re-observe and update this pin. Output:\n{combined}"
    );
}

/// PIN: the ladder fixture under `--real-jdk` no longer panics on the `java/net/URL`
/// layout, and stays at the `String.COMPACT_STRINGS` real-layout frontier (past the now-
/// cleared `System.getProperties()` system-properties wall).
#[test]
fn spring_boot_ladder_real_jdk_no_url_panic_and_pinned() {
    assert_no_url_panic_and_pinned(LADDER_JAR);
}

/// PIN: the app fixture under `--real-jdk` no longer panics on the `java/net/URL`
/// layout, and stays at the `String.COMPACT_STRINGS` real-layout frontier (past the now-
/// cleared `System.getProperties()` system-properties wall).
#[test]
fn spring_boot_app_real_jdk_no_url_panic_and_pinned() {
    assert_no_url_panic_and_pinned(APP_JAR);
}
