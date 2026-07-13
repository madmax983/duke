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
//   * APP cleared logback's `COWArrayList.addIfAbsent` operand-stack-underflow
//     rung. Two changes did it: (1) lenient method dispatch is now honest — a
//     call to a method on an unresolvable class raises a catchable
//     `java/lang/NoSuchMethodError` instead of silently popping args and
//     continuing (which corrupted the operand stack for non-void descriptors);
//     and (2) a synthetic `java/util/concurrent/CopyOnWriteArrayList` (init/add/
//     addIfAbsent/get/size/contains/isEmpty/iterator; see java_util_concurrent.rs)
//     now models the type logback uses. With the honest throw in place the boot
//     then surfaced — and this lane cleared — several previously-swallowed rungs:
//     `java/lang/Record.<init>` (now registered), a synthetic
//     `java/util/concurrent/LinkedBlockingQueue` (FIFO queue with drainTo/clear),
//     and `java/lang/InheritableThreadLocal` (ThreadLocal subclass). The
//     `StringBuilder.append(Object)` rung is now CLEARED: a new
//     `native_sb_append_object` appends `String.valueOf(arg)` by rendering the
//     argument through `heap_object_to_string` — the same toString-dispatch
//     precedent `String.valueOf(Object)` and `PrintStream.println(Object)` use.
//     The app then climbed a run of shallow java.lang/java.util rungs, all
//     CLEARED this session: `String.indexOf(II)I`, `String.lastIndexOf(I)I` and
//     `String.lastIndexOf(II)I` (char-index natives); synthetic
//     `java/util/LinkedHashSet`, `java/util/WeakHashMap` and
//     `java/util/IdentityHashMap` (reuse the HashSet / HashMap native families —
//     Duke's map key comparison is already identity-based for general objects, so
//     IdentityHashMap is faithful); a synthetic `java/util/AbstractMap` with a
//     no-op `<init>` (jar-loaded map subclasses chain super() to it); the
//     initial-capacity `<init>(I)V`/`(IF)V` constructors on HashMap/HashSet/
//     LinkedHashSet (capacity hint ignored); `HashSet.addAll(Collection)`,
//     `Collections.addAll(Collection, Object[])` and
//     `Collections.newSetFromMap(Map)` (returns a fresh field-backed HashSet,
//     matching the empty-map contract); and a non-collecting
//     `java/lang/ref/ReferenceQueue` (no-op `<init>`, `poll()` always null) plus
//     the two-arg `WeakReference(referent, queue)` constructor. The app now boots
//     far enough to fail inside a *reflectively-invoked* method, so the visible
//     first blocker is an uncaught `java/lang/reflect/InvocationTargetException`
//     (reflection wraps the target throwable and discards its class). Instrumenting
//     the wrap shows the underlying cause is `NoClassDefFoundError:
//     java/util/EnumSet`; registering EnumSet only uncovers a deeper
//     `ExceptionInInitializerError` from an enum/config static initializer that
//     uses EnumSet's Class-typed factories (`noneOf`/`allOf`/`range`, which need
//     enum-constant reflection and ordinal bit-set storage). That is a real
//     subsystem, not a shallow method mirror, so the app is pinned here.
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
//     The ladder previously landed on `method not found:
//     org/apache/commons/logging/impl/LogFactoryImpl.objectId(...)`. That rung is
//     now CLEARED: `objectId` is a `public static` method declared on the abstract
//     superclass `LogFactory` and invoked via a Methodref bound to the subclass
//     `LogFactoryImpl`. The invokestatic slow-path resolver now walks the super
//     chain (JVMS §5.4.3.3) instead of flat-scanning the subclass, so the
//     inherited static body resolves. The frontier then advanced through a run of
//     in-lane rungs cleared this session (objectId → java/io/Serializable →
//     Logger.logp → Thread contextClassLoader → Class.getInterfaces):
//       (1) `class not found: java/io/Serializable` — registered as a synthetic
//           marker interface in stdlib.rs (like java/lang/AutoCloseable) so
//           reflective hierarchy walks / is_assignable_from resolve it. CLEARED.
//       (2) `method not found: java/util/logging/Logger.logp(...)` — commons-logging's
//           Jdk14Logger routes every call through the 4-arg `logp(Level, srcClass,
//           srcMethod, msg)` and 5-arg `logp(..., Throwable)`. Both natives added
//           (native_jul_logger_logp / _logp_throwable in java_util_logging.rs,
//           mirroring `log`/`log_throwable`). CLEARED.
//       (3) An uncaught `InvocationTargetException` wrapping a
//           `NullPointerException` inside the fixture's own reflectively-invoked
//           `LadderApplication.main`: `Thread.currentThread().getContextClassLoader()`
//           returned null because `currentThread()` allocates a throwaway Thread per
//           call, so the launcher's `setContextClassLoader(LaunchedClassLoader)` was
//           written to a discarded instance. Fixed by persisting the main thread's
//           context loader in a static slot on the synthetic java/lang/Thread
//           (MAIN_CONTEXT_CLASS_LOADER_FIELD): setContextClassLoader writes it,
//           getContextClassLoader falls back to it (then to the system loader). This
//           also cleared the follow-on `getResourceAsStream(...)` == null rung —
//           with the LaunchedClassLoader's real archive path restored, the lookup
//           resolves BOOT-INF/classes/META-INF/duke-ladder.properties. CLEARED.
//     The ladder now lands on `method not found: java/lang/Class.getInterfaces()`,
//     but that is only a SYMPTOM: commons-logging's `LogFactoryImpl.createLogFromClass`
//     evaluates `newLogger instanceof org/apache/commons/logging/Log` on the
//     reflectively-constructed `Jdk14Logger`. Duke's `instanceof` returns false — in
//     `is_assignable_from` (crates/duke-interpreter/src/native/common.rs) the class's
//     interface entries are plain internal names (`org/apache/commons/logging/Log`)
//     while `to_key` is loader-qualified (`org/apache/commons/logging/Log loader:NN`),
//     so `iface == to_key` never matches. That false negative diverts control into
//     `handleFlawedHierarchy`, which calls `Class.getInterfaces()` (and ultimately
//     throws a spurious LogConfigurationException). The real fix normalizes the
//     interface/`to_key` comparison in `is_assignable_from` (native/common.rs) or the
//     `Instanceof` opcode (execution.rs) — the interpreter class-identity lane —
//     so this rung is pinned, not patched here (adding getInterfaces alone would only
//     let commons-logging throw the false LogConfigurationException).
// If either boot advances past its pin, re-observe and update.
// See docs/findings/2026-07-10-spring-boot-real-app.md.
// Root cause (visible only via instrumentation of the reflection wrap):
// `NoClassDefFoundError: java/util/EnumSet`, then `ExceptionInInitializerError`.
const APP_BLOCKER: &str = "java exception: java/lang/reflect/InvocationTargetException";
const LADDER_BLOCKER: &str = "method not found: java/lang/Class.getInterfaces()[Ljava/lang/Class;";

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
#[ignore = "Blocked on an uncaught java/lang/reflect/InvocationTargetException whose underlying \
            cause (visible only via instrumentation of the reflection wrap) is \
            NoClassDefFoundError: java/util/EnumSet, which in turn uncovers an \
            ExceptionInInitializerError from an enum/config static initializer using EnumSet's \
            Class-typed factories (noneOf/allOf/range) — a real enum-reflection/ordinal-bitset \
            subsystem, not a shallow method mirror. The StringBuilder append(Object) rung plus a \
            run of shallow java.lang/java.util rungs (String.indexOf(II)/lastIndexOf(I)/(II); \
            synthetic LinkedHashSet/WeakHashMap/IdentityHashMap/AbstractMap; capacity ctors on \
            HashMap/HashSet; HashSet.addAll; Collections.addAll/newSetFromMap; non-collecting \
            ReferenceQueue) are now cleared; keep ignored until the Spring Boot app boot \
            completes. See docs/findings/2026-07-10-spring-boot-real-app.md"]
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
#[ignore = "Blocked on 'method not found: java/lang/Class.getInterfaces()' — a SYMPTOM of a \
            deeper instanceof bug. This session cleared a run of in-lane rungs \
            (objectId → java/io/Serializable synthetic marker → Logger.logp 4/5-arg natives → \
            Thread contextClassLoader persistence, which also restored getResourceAsStream \
            resolution of BOOT-INF/classes). The ladder now reaches \
            LogFactoryImpl.createLogFromClass, where 'newLogger instanceof \
            org/apache/commons/logging/Log' wrongly returns false for the reflectively-built \
            Jdk14Logger: in is_assignable_from (native/common.rs) a class's plain interface \
            names are compared against a loader-qualified to_key, so they never match. That \
            false negative diverts into handleFlawedHierarchy, which calls Class.getInterfaces(). \
            The real fix is the interface/to_key key-normalization in is_assignable_from \
            (native/common.rs) or the Instanceof opcode (execution.rs) — the interpreter \
            class-identity lane, out of this lane's scope; adding getInterfaces alone would only \
            surface a spurious LogConfigurationException. Keep ignored until the ladder completes. \
            See docs/findings/2026-07-10-spring-boot-real-app.md"]
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
