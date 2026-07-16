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
//     the two-arg `WeakReference(referent, queue)` constructor. The EnumSet rung
//     is now CLEARED: the enum-reflection/ordinal-bitset subsystem
//     (`EnumSet.noneOf`/`allOf`/`range` over Class-typed factories) landed, clearing
//     the reflectively-wrapped `NoClassDefFoundError: java/util/EnumSet` /
//     `ExceptionInInitializerError` wall (PR #1368). The SoftReference rung is now
//     CLEARED too: a minimal non-collecting synthetic `java/lang/ref/SoftReference`
//     (super `Reference`, 1-arg + 2-arg `(Object, ReferenceQueue)` ctors reusing
//     `native_reference_init`, get/clear inherited) mirrors the WeakReference
//     precedent and clears Spring's
//     `ConcurrentReferenceHashMap$SoftEntryReference.<init>` wall. The app now boots
//     PAST the enum/reference lane and the visible first blocker changed to a
//     directly-surfaced (no reflection wrap) missing I/O/networking method:
//     `java/net/URL.openConnection()Ljava/net/URLConnection;` at
//     `org/springframework/core/io/UrlResource::getInputStream@4 invokevirtual`
//     (reached via `PropertiesLoaderUtils::fillProperties`). A `java/net/URL`
//     synthetic and `native_url_open_stream` already exist, but `openConnection()`
//     needs a NEW synthetic abstract `java/net/URLConnection` (getInputStream/
//     setUseCaches + an HttpURLConnection instanceof/disconnect path) — a dedicated
//     I/O-lane rung, so the app is pinned here.
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
//     The `getInterfaces()` rung was only a SYMPTOM of a deeper `instanceof` bug:
//     commons-logging's `LogFactoryImpl.createLogFromClass` evaluates `newLogger
//     instanceof org/apache/commons/logging/Log` on the reflectively-constructed
//     `Jdk14Logger`, and Duke returned false because in `is_assignable_from`
//     (crates/duke-interpreter/src/native/common.rs) the class's interface entries are
//     plain internal names (`org/apache/commons/logging/Log`) while `to_key` was
//     loader-qualified (`org/apache/commons/logging/Log\0loader:NN`), so `iface ==
//     to_key` never matched. The false negative diverted control into
//     `handleFlawedHierarchy` → `Class.getInterfaces()`. That is now CLEARED
//     (interpreter class-identity lane): `is_assignable_from` compares interface
//     entries against the target's PLAIN internal name (stripping any loader
//     qualifier from both sides), which is loader-agnostic and fixes both `Instanceof`
//     and `Checkcast`. commons-logging now resolves `Jdk14Logger` as a real `Log`,
//     initializes fully, and the ladder boots all the way through logging into its own
//     `LadderApplication.main`.
//     The ladder then ran into `LadderApplication.main` itself, which does a run of
//     final rungs. Two are CLEARED this session; the third is the current WALL:
//       (1) CLEARED — `new Properties().load(getContextClassLoader().getResourceAsStream(
//           name))` threw `java/io/IOException`: `Properties.load(InputStream)` only
//           understood host-file-backed streams (`fields[0]` = fd), but
//           `getResourceAsStream` returns a synthetic `duke/io/ResourceInputStream`
//           holding the resolved bytes in a backing byte-array. `native_properties_load`
//           (java_util.rs) now drains the ResourceInputStream (shared
//           `input_stream_drain_all_bytes` in java_io.rs) and parses via the existing
//           `parse_properties_bytes`.
//       (2) CLEARED — `new BufferedReader(new InputStreamReader(getResourceAsStream(
//           "greeting.txt"), UTF_8)).readLine()` — no character-stream stack existed, so
//           `InputStreamReader.<init>(InputStream, Charset)` raised NoSuchMethodError.
//           Added minimal synthetic `java/io/Reader`, `java/io/InputStreamReader` and
//           `java/io/BufferedReader` (stdlib.rs) with natives in java_io.rs: ISR stores
//           the stream + charset; BufferedReader eagerly drains the stream at
//           construction, decodes (UTF-8 / ISO-8859-1 family), and `readLine()` walks
//           `\n`/`\r`/`\r\n` terminators. (NOTE: the greeting stream itself resolves to
//           null — a *separate*, pre-existing divergence where Class.getResourceAsStream
//           looks the class-relative name up against the bootstrap loader instead of the
//           LaunchedClassLoader; the fixture tolerates the null and continues. Not this
//           lane; follow-up.)
//       (3) CLEARED (2026-07-15, same-class-reflection lane) —
//           `LadderApplication.class.getDeclaredMethod("summarize", List.class)
//           .invoke(null, ...)` invokes the class's OWN private static method via
//           reflection without setAccessible. The real JVM permits this: its
//           Method.invoke access check is CALLER-SENSITIVE — a class may always
//           reflectively access its own (private/nestmate) members, and only CROSS-class
//           access to a non-accessible member throws IllegalAccessException. Duke's
//           `native_reflect_method_invoke` (reflect.rs) previously kept the coarse
//           `!is_public && !is_accessible → throw`, WRONG for the same-class case, because
//           the invoking frame's class was not available to the native. FIX: `Method.invoke`
//           and `Constructor.newInstance` are now in `native_needs_stack_snapshot`
//           (common.rs), so `control.stack_trace()[0]` names the caller class; the reflect
//           natives allow the invoke when that caller class equals the member's declaring
//           class (`caller_is_same_class`), keeping the cross-class throw that the gson
//           canary + `ReflectionTest.privateMethodRaisesIllegalAccess` rely on. `setAccessible`
//           still bypasses. Nest-mates (JEP 181) are not yet modelled — cross-nest private
//           invoke still throws. The ladder now boots end-to-end; the canary is un-ignored.
// If the app boot advances past its pin, re-observe and update.
// See docs/findings/2026-07-10-spring-boot-real-app.md.
// APP frontier (now VISIBLE directly, no reflection wrap): the app clears the
// enum/reflection subsystem (EnumSet + Class.isEnum/getEnumConstants) and the
// reference-type rungs (synthetic SoftReference) and walls on a missing
// I/O/networking method — `java/net/URL.openConnection()Ljava/net/URLConnection;`
// at `org/springframework/core/io/UrlResource::getInputStream@4 invokevirtual`
// (reached via `PropertiesLoaderUtils::fillProperties`). A `java/net/URL`
// synthetic and `native_url_open_stream` already exist, but `openConnection()`
// needs a new synthetic abstract `java/net/URLConnection` — an I/O-lane rung.
const APP_BLOCKER: &str = "method not found: java/net/URL.openConnection()Ljava/net/URLConnection;";
// LADDER now boots END-TO-END (2026-07-15, same-class-reflection lane, trunk): main
// climbs into `LadderApplication.main`, clears Properties.load + the BufferedReader
// character-stream read, and its reflective same-class private `summarize` invoke now
// succeeds (caller-sensitive access check — see rung (3) above). The former
// `LADDER_BLOCKER` pin is repurposed into a no-regression guard asserting the ladder no
// longer emits a runtime error; the canary `spring_boot_ladder_boots_end_to_end` is
// un-ignored. This substring, if it reappears, means the reflective invoke regressed.
const LADDER_REGRESSED_MARKER: &str = "runtime error";

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
#[ignore = "The app now boots PAST the enum-reflection and reference-type lanes: the EnumSet \
            subsystem (noneOf/allOf/range Class-typed factories with enum-constant reflection + \
            ordinal bit-set storage) cleared the reflectively-wrapped NoClassDefFoundError: \
            java/util/EnumSet / ExceptionInInitializerError wall (PR #1368), and a minimal \
            non-collecting synthetic java/lang/ref/SoftReference (super Reference, 1-arg + 2-arg \
            (Object, ReferenceQueue) ctors reusing native_reference_init, get/clear inherited) \
            cleared Spring's ConcurrentReferenceHashMap$SoftEntryReference.<init> wall. The visible \
            first blocker is now a directly-surfaced missing I/O/networking method: \
            java/net/URL.openConnection()Ljava/net/URLConnection; at \
            org/springframework/core/io/UrlResource::getInputStream@4 invokevirtual (reached via \
            PropertiesLoaderUtils::fillProperties). A java/net/URL synthetic and \
            native_url_open_stream already exist, but openConnection() needs a NEW synthetic \
            abstract java/net/URLConnection (getInputStream/setUseCaches + an HttpURLConnection \
            instanceof/disconnect path) — a dedicated I/O-lane rung, not taken here. Keep ignored \
            until the Spring Boot app boot completes. \
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
///
/// The ladder now climbs all of commons-logging/Jdk14Logger into its own
/// `LadderApplication.main` and clears main's first two rungs — `Properties.load`
/// of the classpath `.properties` and the `BufferedReader(InputStreamReader(...))`
/// character-stream read — but WALLS on main's reflective same-class private
/// `summarize` invoke (an out-of-lane `IllegalAccessException`; see the LADDER
/// progression note above). When that wall clears, the assertion below should
/// pass end-to-end.
#[test]
fn spring_boot_ladder_boots_end_to_end() {
    let output = run_fixture(LADDER_JAR);
    let combined = combined_output(&output);

    assert!(
        output.status.success(),
        "duke -jar on the ladder fixture should exit cleanly; output:\n{combined}"
    );
    // Assert on STABLE substrings of the real boot output only (the per-run heap
    // address in the "Squares ...@NN sum=204" line and the known
    // Class.getResourceAsStream/bootstrap-loader "greeting.txt -> null"
    // divergence are deliberately excluded).
    for expected in [
        "Duke ladder fixture starting (commons-logging via JCL).",
        "Loaded resource props: name=duke-spring-boot-ladder rung=intermediate",
        "Reflection Math.sqrt(2809) = 53.0",
        "Reflective summarize() -> LOGGING,RESOURCE-SCAN,REFLECTION,STREAMS",
        "Duke ladder fixture completed all rungs successfully.",
        "Duke ladder fixture application started successfully.",
    ] {
        assert!(
            combined.contains(expected),
            "expected the ladder boot output to contain {expected:?}; output:\n{combined}"
        );
    }
}

/// REGRESSION GUARD: the ladder fixture boots end-to-end and emits no runtime
/// error. Repurposed from the former next-missing-capability pin once the
/// reflective same-class private invoke wall cleared (2026-07-15). If the
/// reflection access check (or any earlier rung) regresses, this trips with the
/// `runtime error` marker.
/// See docs/findings/2026-07-10-spring-boot-real-app.md
#[test]
fn spring_boot_ladder_surfaces_no_missing_capability() {
    let output = run_fixture(LADDER_JAR);
    let combined = combined_output(&output);

    assert!(
        output.status.success(),
        "ladder fixture is expected to boot cleanly; output:\n{combined}"
    );
    assert!(
        !combined.contains(LADDER_REGRESSED_MARKER),
        "expected the ladder fixture to boot with no runtime error \
         ({LADDER_REGRESSED_MARKER:?}); if it reappears the reflective same-class invoke (or an \
         earlier rung) regressed (docs/findings/2026-07-10-spring-boot-real-app.md). \
         Output:\n{combined}"
    );
}
