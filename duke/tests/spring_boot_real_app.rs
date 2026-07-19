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
//     `ConcurrentReferenceHashMap$SoftEntryReference.<init>` wall. The app then
//     booted PAST the enum/reference lane onto the URL resource-loading lane:
//     `java/net/URL.openConnection()`. That rung is now CLEARED too (2026-07-16):
//     a minimal synthetic `java/net/URLConnection` (1-slot spec-backed, mirroring
//     `java/net/URL`) with `getInputStream()`/`setUseCaches(Z)V`, plus
//     `URL.openConnection()` and a null-returning `URL.getUserInfo()`, carry
//     `org/springframework/core/io/UrlResource::getInputStream` end to end and hand
//     `PropertiesLoaderUtils::fillProperties` a `duke/io/ResourceInputStream` via the
//     existing `read_resource_bytes_from_url_spec`/`allocate_resource_input_stream`
//     machinery — no HttpURLConnection was required (getUserInfo returns null, so
//     the app never takes the Basic-auth path and the getInputStream call succeeds
//     before the IOException/HttpURLConnection.disconnect branch). The app now boots
//     PAST resource loading and the visible first blocker changed to a directly-
//     surfaced missing collections constructor: `java/util/ArrayDeque.<init>(I)V`
//     (the initial-capacity ArrayDeque ctor) — a java.util collections-lane rung,
//     so the app is pinned here.
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
// enum/reflection subsystem (EnumSet + Class.isEnum/getEnumConstants), the
// reference-type rungs (synthetic SoftReference), AND the whole URL resource-
// loading lane. The URLConnection rung is now CLEARED (2026-07-16): a minimal
// synthetic `java/net/URLConnection` (1-slot spec-backed, mirroring `java/net/URL`)
// with `getInputStream()`/`setUseCaches(Z)V`, plus `URL.openConnection()` and a
// null-returning `URL.getUserInfo()`, carry `UrlResource::getInputStream` end to
// end: `url.openConnection()` mints the connection, `customizeConnection` runs
// `useCachesIfNecessary`→`setUseCaches` (no-op) and `url.getUserInfo()` (null, so
// the Basic-auth header path is skipped), then `URLConnection.getInputStream()`
// reuses the existing `read_resource_bytes_from_url_spec` +
// `allocate_resource_input_stream` machinery to hand `PropertiesLoaderUtils::
// fillProperties` a `duke/io/ResourceInputStream`. The app now boots PAST resource
// loading and walls on a directly-surfaced missing collections constructor —
// `java/util/ArrayDeque.<init>(I)V` (the initial-capacity ArrayDeque ctor) — a
// java.util collections-lane rung, so the app is pinned here.
// APP now boots PAST the ArrayDeque, URLDecoder and SpringFactoriesLoader
// collections rungs (all cleared 2026-07-18, banner-climb wave):
//   * `java/util/ArrayDeque.<init>(I)V` — the initial-capacity ctor (capacity is a
//     sizing hint; reuses the empty-deque init native).
//   * `java/net/URLDecoder.decode(String, Charset)` — x-www-form-urlencoded decode
//     (`+`->space, `%XX`->byte, UTF-8), reached from UrlResource.getFilename.
//   * `java/util/Properties.forEach(BiConsumer)` — Properties has its OWN layout
//     (fields[0]=size, fields[1]=defaults, fields[2..]=entries), so it used to fall
//     through to the Hashtable/HashMap forEach, which read fields[1] (the defaults
//     slot, null) as the first key and handed SpringFactoriesLoader a null key — an
//     NPE on `name.trim()`. A Properties-specific forEach iterating the real entries
//     fixes it.
//   * `java/util/LinkedHashMap` Map-default family (computeIfAbsent + computeIfPresent
//     /compute/merge/replaceAll/putIfAbsent/replace/containsValue/putAll/clear),
//     reusing the HashMap natives (identical layout).
// The app then walls on a GC-relocation/root-completeness hazard in DEEPLY NESTED
// native callbacks: SpringFactoriesLoader runs
//   ConcurrentReferenceHashMap.computeIfAbsent(loader, k ->
//       Properties.forEach((name,value) ->
//           result.computeIfAbsent(name, k2 -> new ArrayList<>(names.length))))
// The innermost `new ArrayList<>` triggers a major GC while the `result` LinkedHashMap
// is reachable ONLY through outer interpreter frames that are not part of the nested
// `ops.invoke`/execute_class call-stack (and through native-held Rust locals, which are
// not GC roots). `result` is collected mid-callback, so the resumed
// `LinkedHashMap.computeIfAbsent` put dereferences a dangling ref and throws NPE, which
// the launcher's reflective `main.invoke` wraps as InvocationTargetException. Confirmed
// by forcing GC off: the NPE vanishes and boot climbs several rungs further (next visible
// gap `java/util/UnmodifiableMap.getOrDefault`, then Spring's own
// SpringFactoriesLoader$FailureHandler throwing an IllegalArgumentException while
// instantiating factory implementations — deep reflective bean-instantiation territory).
// The root cause is a GC-root-completeness gap in nested native-callback re-entrancy
// (interpreter/GC core — execution.rs `gather_roots`/`ops.invoke` + a missing native
// temp-root pin).
//
// RE-OBSERVED 2026-07-18 (native-root-handle lane): the missing native temp-root pin
// now exists — natives register the heap refs they hold across `ops.invoke` as GC local
// handles (`NativeRootScope`), which `gather_roots`/`patch_forwarded_slots` root and
// forward on EVERY collection. `HashMap.computeIfAbsent` (and the whole java.util
// callback family) adopted it, so the `LinkedHashMap.computeIfAbsent -> new ArrayList<>`
// collect no longer reclaims the live `result` map and the InvocationTargetException NPE
// is cleared.
//
// ADVANCED 2026-07-18 (banner-climb lane): `native_properties_for_each` was converted to
// the `NativeRootScope` pin API too, clearing the `invalid heap reference` wall inside
// `Properties.forEach`. Boot then climbed a run of reflective-bootstrap rungs, all cleared
// this session: `UnmodifiableMap.getOrDefault`; loader-suffix-safe `Class.isAssignableFrom`
// (so `SpringFactoriesLoader.instantiateFactory`'s factory-type assert passes);
// `Method`/`Constructor.getModifiers`, the whole `java.lang.reflect.Modifier` predicate
// set, and `AccessibleObject.isAccessible` (so `ReflectionUtils.makeAccessible` runs);
// inherited-method resolution for directed `ops.invoke` (so `OrderComparator.compare`
// dispatches through `AnnotationAwareOrderComparator`); `Arrays.hashCode(Object[])`; and
// `Class.getSuperclass`/`getInterfaces`.
//
// The Spring GENERICS-reflection wall is now CLEARED (2026-07-19,
// generics-reflection lane): real `Signature`-attribute parsing (JVMS §4.7.9.1),
// a synthetic `java.lang.reflect.Type`/`TypeVariable`/`ParameterizedType`/
// `GenericArrayType`/`WildcardType` hierarchy, and honest natives —
// `Class.getTypeParameters` (TypeVariable[] of the correct arity, so
// `ResolvableType.forClassWithGenerics`'s type-variable-count assert passes),
// `getGenericSuperclass`/`getGenericInterfaces` (ParameterizedType when
// parameterized), `Class.toGenericString`, and `Field.getGenericType`. Curated
// JDK generic signatures back synthetic generic types (Map = <K,V>, List = <E>,
// …) that carry no classfile Signature. A tiny `Boolean.getBoolean(String)`
// bootstrap read was also cleared.
//
// The app now advances PAST the generics wall and walls, nondeterministically
// (HashMap iteration order decides which surfaces first), on a cluster of
// downstream lanes that are ALL out of the generics-reflection lane.
//
// CLASS-IDENTITY LANE (2026-07-19): the former `ambiguous class name:
// org/springframework/context/ApplicationListener matches [..\0, ..\0]` outcome is
// now CLEARED. That wall was the SAME class registered under two loader-qualified
// keys differing only by a GC-promotion loader ref (a young-gen `154` vs an
// old-gen `9223372036854775895`), same jar / code source. The identity fix
// (`same_runtime_class` predicate + resolve collapse + assignability routing)
// makes `resolve_loaded_class_key` treat the pair as one runtime class, so it no
// longer raises `AmbiguousClassName`. Re-observed empirically: across 70 app
// launches on this fixture the string "ambiguous class name" appeared ZERO times
// (was one of the pre-fix nondeterministic outcomes). The `"ambiguous class name"`
// marker is therefore RETIRED from `APP_BLOCKERS` below — leaving it would silently
// tolerate a regression of this very fix. It survives only for THIS fixture; the
// `Error::AmbiguousClassName` variant itself is still exercised by the
// duke-runtime unit tests (crates/duke-runtime/src/lib.rs).
//
// The remaining post-fix frontier (re-observed 2026-07-19, N=70 launches):
//   * `class not found: [B` / `class not found: [Lorg/...ConcurrentReferenceHashMap$*;`
//     — primitive AND object array-class resolution (class-loader lane). DOMINANT
//     outcome (~65% direct `[B`, plus ~15% object-array `[L...;`). Surfaces as a
//     fatal `ClassNotFound` directly on the interpreter stack (never wrapped).
//   * `InvocationTargetException` — ~15-18% of runs. The reflective launcher's
//     `main.invoke` wraps a Java exception thrown inside the reflectively-invoked
//     `DukeApplication.main`. UNWRAPPED (throwaway instrumentation at the ITE
//     wrap-site, since Duke's output layer prints only the wrapper's class name):
//     the cause is UNIFORMLY `java/lang/NoClassDefFoundError` whose detail message
//     is UNIFORMLY `java/lang/StackWalker$Option` — i.e. Spring startup reaches an
//     unmodeled `java.lang.StackWalker` (the StackWalker$Option enum) and the
//     resulting linkage NoClassDefFoundError is re-wrapped by reflective invoke.
//     A class-resolution failure like the array-class lane, differing only in that
//     it lands inside reflective `main.invoke` and is caught as a LinkageError.
//     The marker stays `"InvocationTargetException"` because that is the only string
//     Duke emits for this outcome (the StackWalker cause is not surfaced in output).
//   * `class not found: $$Lambda$N` — LambdaMetafactory / invokedynamic lane
//     (~2-3%, rare but observed).
// The pin below accepts ANY of these de-flaking markers (see `APP_BLOCKERS`).
const APP_BLOCKERS: [&str; 3] = [
    // Primitive + object array-class resolution (`[B`, `[Lorg/...;`) — dominant
    // (~80% of runs across the two array shapes).
    "class not found: [",
    // Reflective `main.invoke` wrapping a linkage failure inside DukeApplication.main.
    // Unwrapped cause is uniformly `NoClassDefFoundError: java/lang/StackWalker$Option`
    // (unmodeled StackWalker enum), but Duke prints only this wrapper name. ~15-18%.
    "InvocationTargetException",
    // LambdaMetafactory / invokedynamic synthetic classes (~2-3%).
    "$$Lambda",
];
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
#[ignore = "The app boots PAST classpath scan, the SpringFactoriesLoader collections shape, the \
            forEach/computeIfAbsent GC callback family (both Properties.forEach and \
            HashMap.computeIfAbsent now pin native-held refs via NativeRootScope, clearing the \
            former `invalid heap reference` wall), and the whole reflective factory-instantiation \
            cluster cleared 2026-07-18 by the banner-climb lane: UnmodifiableMap.getOrDefault; \
            loader-suffix-safe Class.isAssignableFrom; Method/Constructor.getModifiers; \
            java.lang.reflect.Modifier; AccessibleObject.isAccessible; inherited-method resolution \
            for directed ops.invoke (OrderComparator.compare via AnnotationAwareOrderComparator); \
            Arrays.hashCode(Object[]); and Class.getSuperclass/getInterfaces. The Spring \
            annotation/generics reflection wall (AnnotationsScanner / ResolvableType) was then \
            CLEARED 2026-07-19 by the generics-reflection lane: real Signature-attribute parsing, \
            the java.lang.reflect.Type/TypeVariable/ParameterizedType hierarchy, and the \
            getTypeParameters/getGenericSuperclass/getGenericInterfaces/toGenericString natives. \
            The app now advances PAST the generics wall AND past the former loader class-key dedup \
            wall (`ambiguous class name` on ApplicationListener — CLEARED 2026-07-19 by the \
            class-identity fix; 0/70 launches). It now walls, nondeterministically, on the \
            array-class resolution lane (`class not found: [B` / `[Lorg/...;`, dominant), a \
            reflective `main.invoke` wrapping `NoClassDefFoundError: java/lang/StackWalker$Option` \
            (surfaced as `InvocationTargetException`), and the LambdaMetafactory lane (`$$Lambda`) — \
            all out of the generics lane. Keep ignored until the Spring Boot app boot completes. \
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
        APP_BLOCKERS.iter().any(|marker| combined.contains(marker)),
        "expected the app fixture to stay pinned at the current frontier cluster \
         (any of {APP_BLOCKERS:?} — the generics wall AND the loader class-key dedup \
         `ambiguous class name` wall are cleared; the app now walls nondeterministically \
         on the array-class / reflective-StackWalker-linkage / lambda lanes); \
         if it moved, re-observe and update this pin \
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
