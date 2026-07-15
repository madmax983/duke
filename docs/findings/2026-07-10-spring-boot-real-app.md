# Spring Boot real-app stress test — blocker chain by owning lane

**Date:** 2026-07-10
**Branch:** `swarm/spring-boot-real-app` (off `origin/trunk` `42a1f58`)

## Mission

Flagship stress test: drive a **real** Spring Boot 3.5.12 fat jar through
`duke -jar` (the Rust JVM's application entry point) and map the **full blocker
chain** — every distinct capability gap between "process starts" and "Spring
Boot user code runs" — ranked and attributed to an owning implementation lane so
each gap can be handed to the lane that owns it.

This is deliberately an end-to-end integration probe against unmodified,
JVM-verified OSS bytecode, not a hand-written micro-fixture. The goal is not to
make Spring Boot boot in one branch — it is to *discover and order* the real
work, and to fix the one blocker that turned out to belong to this
(classloader/registry) lane.

## Fixtures

Both fat jars are vendored under `tests/fixtures/oss-jars/spring-boot/` and were
built with Spring Boot **3.5.12** and boot-verified on a real JVM before
vendoring.

| Fixture | What it is | Size | SHA-256 |
| --- | --- | --- | --- |
| `duke-spring-boot-app-3.5.12.jar` | A real Maven-built Spring Boot application (`Start-Class com.example.duke.DukeApplication`). Real `spring-boot-loader` `JarLauncher`, real auto-configuration, real nested-jar layout. | ~10.6 MB | `008bd724f19b5df6ba56b98655e389ce811942638d84c6f9952e916804c80366` |
| `duke-spring-boot-ladder-3.5.12.jar` | A commons-logging "ladder" fat jar (`Start-Class com.example.duke.ladder.LadderApplication`) that walks progressively harder logging/classpath rungs — a smaller, faster canary that exercises the same `LaunchedClassLoader` machinery without the full auto-config weight. | ~277 KB | `162689fc62f024257fc8b3c10cc091f814eb975ca9c6725d5f3c96d37881370a` |

**Real-JVM verification (pre-vendoring):**
- The app jar boots on a stock JVM to the Spring banner and prints
  `Started …` / `… in … seconds` (i.e. reaches `SpringApplication.run` user
  code and completes startup).
- The ladder jar boots on a stock JVM and prints a "completed all rungs"
  success line.

**Provenance & licensing:** both jars are recorded in the repo's `LICENSES.md`
(one row each, with SHA-256 and upstream/build provenance). A fully reproducible
build recipe (Maven coordinates, `Start-Class`, repackage invocation, expected
run output, and how to re-derive the SHA-256) lives at
`tests/fixtures/oss-jars/spring-boot/README.md`.

## How far boot gets today

With the loader-lane fix in this branch (`5860f6b`, see below), `duke -jar` on
**both** fixtures now:

1. Parses the fat jar and routes into Spring Boot's own
   `org.springframework.boot.loader.launch.JarLauncher`.
2. Constructs the `LaunchedClassLoader` (Spring's nested-jar URL classloader)
   and hands control to it.
3. Reaches Spring Boot's **classpath-scanning phase** — the point where the
   framework enumerates classpath resources to discover configuration.

It then stops. **Update 2026-07-11 (ClassLoader-lane rungs cleared):** three
synthetic `java/lang/ClassLoader` natives are now implemented —
`getSystemResources(String)` (mirrors `getResources` but resolves against the
system/bootstrap loader), `getSystemClassLoader()` (returns a single stable
synthetic system `ClassLoader` instance, cached in a static field), and base
`loadClass(String)` (reuses `native_url_class_loader_load_class`, which delegates
to the parent/default loader first). These advanced **both** fixtures several
rungs. The frontier has now moved **out of the ClassLoader-native lane** on both:

**Update 2026-07-11 (end of wave — 8 rungs cleared this wave):** this wave cleared
**eight** distinct rungs across four lanes. In order:
1. `ClassLoader.getSystemResources(String)` (ladder) — synthetic-stdlib `java_lang`.
2. `ClassLoader.getSystemClassLoader()` (app) — synthetic-stdlib `java_lang`.
3. `ClassLoader.loadClass(String)` (app) — synthetic-stdlib `java_lang`.
4. Inherited `Object.getClass()` through an interface-typed callsite (app) —
   execution.rs invokeinterface native fallback now walks the super chain.
5. Minimal non-collecting synthetic `java/lang/ref/WeakReference` (ladder) —
   clears the commons-logging `LogFactory.getFactory` underflow.
6. 3-arg `Class.forName(name, false, cl)` availability probe made catchable via a
   **lazy** identity-key computation (ladder) — native `java_lang`.
7. Minimal-for-boot synthetic `java/time/ZoneId` (app) — native/stdlib `java_time`.
8. Minimal-for-boot synthetic `java/util/Locale` (app) — native/stdlib `java_util`.

After all eight, the two fixtures are parked at these **final end-of-wave
frontiers**:

```
# app  (duke-spring-boot-app-3.5.12.jar) — final frontier:
duke: runtime error: method not found: java/time/format/DateTimeFormatter.ofPattern(Ljava/lang/String;)Ljava/time/format/DateTimeFormatter;

# ladder (duke-spring-boot-ladder-3.5.12.jar) — final frontier:
duke: runtime error: method not found: java/util/Hashtable.computeIfAbsent(Ljava/lang/Object;Ljava/util/function/Function;)Ljava/lang/Object;
```

The **app** frontier is now logback's `CachingDateFormatter` calling
`DateTimeFormatter.ofPattern(String)` — the synthetic `DateTimeFormatter` carries
only the ISO constant instances, not the `ofPattern` factory (java.time formatter
lane; handoff to a future wave).

**UPDATE (2026-07-12, linkage-error lane): the `MarkerManager` rung is CLEARED.**
The interpreter now models real JVM linkage-error semantics. The previous ladder
frontier — a missing `org/apache/logging/log4j/MarkerManager` reached while
initializing `Log4jApiLogFactory` (its `<clinit>` calls `MarkerManager.getMarker`)
— now raises a **catchable `java.lang.NoClassDefFoundError`** (a `LinkageError`)
at the failing `invokestatic`, instead of a fatal Rust `class not found` error.
commons-logging's `catch (LinkageError)` fires and falls through to
`LogFactoryImpl`, exactly as on a real JVM. The ladder now advances one rung
further and lands on `method not found: java/util/Hashtable.computeIfAbsent(...)`
— the `LogFactoryImpl` fallback uses `Hashtable.computeIfAbsent`, which the
synthetic `java/util/Hashtable` does not yet implement (collections lane; a
`NoSuchMethodError`-territory blocker, handoff to a future wave).

How the linkage-error lane works: synthetic `java/lang/LinkageError`,
`NoClassDefFoundError` and `ExceptionInInitializerError` are registered as `Error`
subtypes so `catch (LinkageError)`/`catch (Throwable)` match; execution-time
class-resolution failures at `invokestatic` / `get(field|static)` /
`put(field|static)` throw a catchable `NoClassDefFoundError` (internal-name detail
message) routed through the running method's exception table; and `<clinit>`
failures get JVMS 5.5 erroneous-class semantics (erroneous class re-throws
`NoClassDefFoundError`; an `Error`-typed clinit failure propagates unwrapped — the
ladder case; an `Exception`-typed failure is wrapped in
`ExceptionInInitializerError`). The `new` opcode deliberately keeps its legacy
"limp" for unmodelled classes (it never produced a fatal error), so real-jar boot
progress that relies on limping past unmodelled `java.util.concurrent` types is
preserved.

An intermediate frontier this wave (after the `WeakReference` rung, before the
`Class.forName` probe fix) was `class not found: org/apache/logging/slf4j/SLF4JProvider`
on the ladder; the lazy-key `Class.forName` fix turned that availability probe
catchable and advanced the ladder to the `MarkerManager` frontier above.

**UPDATE (2026-07-12, dispatch-honesty lane): the app `COWArrayList.addIfAbsent`
operand-stack-underflow rung is CLEARED, and the app frontier has advanced.**
Two changes did it:

1. **Lenient method dispatch is now honest.** The `invokespecial`/`invokevirtual`
   shared arm (`!class_was_loaded` branch) and the `invokeinterface`
   `None`/lambda-miss fallthrough previously popped the args (and `this`) and
   `continue`d **without** pushing a return value — corrupting the operand stack
   for non-void descriptors (this is exactly what surfaced downstream as the
   logback `COWArrayList.addIfAbsent` `operand stack underflow`). Both paths now
   throw a **catchable `java.lang.NoSuchMethodError`** whose detail message names
   `owner.method` + descriptor (e.g.
   `java/util/concurrent/CopyOnWriteArrayList.addIfAbsent(Ljava/lang/Object;)Z`).
   The `IncompatibleClassChangeError → NoSuchMethodError` / `NoSuchFieldError`
   hierarchy is registered under `LinkageError` so `catch (LinkageError)` /
   `catch (Throwable)` match, mirroring the linkage-error lane. (The `new` opcode
   limp and honest field resolution are untouched; the ladder's honest
   `Hashtable.computeIfAbsent` `MethodNotFound` pin is unaffected.)
2. **Synthetic `java/util/concurrent/CopyOnWriteArrayList`** (ArrayList storage
   layout; `init`/`add`/`addIfAbsent`/`get`/`size`/`contains`/`isEmpty`/`iterator`)
   now models the type logback's `COWArrayList` delegates to.

With the honest throw in place, the boot then surfaced — and this lane cleared —
several previously-swallowed super-constructor / method calls:

* `java/lang/Record.<init>()V` — every record chains `super()` to
  `java.lang.Record`, which was unregistered (so real record instantiation would
  now throw `NoSuchMethodError`). Registered with a no-op `<init>`.
* `java/util/concurrent/LinkedBlockingQueue` — synthetic unbounded FIFO queue
  (`init`/`add`/`offer`/`put`/`poll`/`take`/`peek`/`size`/`isEmpty`/
  `remainingCapacity`/`drainTo`/`clear`). Also cleared the slf4j-simple OSS canary,
  which resolves the queue honestly now instead of swallowing its ops.
* `java/lang/InheritableThreadLocal` — registered as a `ThreadLocal` subclass
  reusing its single-slot natives (inheritance is a no-op single-threaded).

New **app** frontier:

```
# app (duke-spring-boot-app-3.5.12.jar) — current frontier:
duke: runtime error: method not found: java/lang/StringBuilder.append(Ljava/lang/Object;)Ljava/lang/StringBuilder;
```

`StringBuilder.append(Object)` must `String.valueOf`/`toString` its argument, so
it needs a new `toString`-dispatching native in `native/java_lang.rs`. That is a
**java.lang lane** blocker, out of the j.u.c./java_util/dispatch scope of this
wave — handoff to a future wave.

For history, the prior (2026-07-11) frontiers were:

```
# app  — inherited Object.getClass() interface dispatch (FIXED this branch):
duke: runtime error: method not found: ch/qos/logback/classic/util/DefaultJoranConfigurator.getClass()Ljava/lang/Class;

# ladder — WeakReference.get() underflow (FIXED this branch):
duke: runtime error: operand stack underflow
```

The app rung was inherited `java/lang/Object.getClass()` dispatch failing to
resolve through an interface-typed callsite. The ladder rung was an
`operand stack underflow` deep in real commons-logging `LogFactory.getFactory` at
`java/lang/ref/WeakReference.get()Ljava/lang/Object;` (offset 188 → `checkcast` at
191 underflows because `get()` returned no value) — an unimplemented
`java.lang.ref` reference-object native.

Previous divergence (**Update 2026-07-10, after #1320**), retained for history —
the two fixtures diverged after #1320 advanced the app past `getSystemResources`:

```
# app  (duke-spring-boot-app-3.5.12.jar):
duke: runtime error: method not found: java/lang/ClassLoader.getSystemClassLoader()Ljava/lang/ClassLoader;

# ladder (duke-spring-boot-ladder-3.5.12.jar) — still parked at the old rung:
duke: runtime error: method not found: java/lang/ClassLoader.getSystemResources(Ljava/lang/String;)Ljava/util/Enumeration;
```

(exit 1). **Be precise about how far this is:** there is **no Spring banner
yet**, and the application does **not** reach `SpringApplication.run` user code.
Getting from "died on the first bootstrap class" to "inside Spring's classpath
scanner" is real forward progress, but the framework has not yet loaded a single
auto-configuration.

## Ranked blocker chain (core deliverable)

Ordered by where boot hits them. Status is honest about **OBSERVED** (actually
reproduced here) vs **INFERRED** (predicted from a `--real-jdk` probe plus
knowledge of Spring Boot's `SpringApplication.run` sequence — not yet reached
under the default synthetic-stdlib path).

| # | Blocker (symptom) | Root cause | Status | Owning lane |
| --- | --- | --- | --- | --- |
| **0** | `class not found: java/lang/System` (app) / `java/lang/ClassLoader` (ladder) — died before any framework code, at a loader-suffixed key (`java/lang/System\0loader:113`). | `ensure_loaded_inner` recorded per-loader/per-code-source provenance onto **plain synthetic bootstrap singletons**, flipping `is_plain_bootstrap_class` false and desyncing it from `class_key_from_provenance`, which then computed a loader-suffixed key nothing was stored under. | **FIXED here** (`5860f6b`) | classloader/registry (**this lane**) |
| **1** | `method not found: java/lang/ClassLoader.getSystemResources(Ljava/lang/String;)Ljava/util/Enumeration;` — boot reaches classpath scanning, no banner. | `getSystemResources` is a **missing synthetic method** on the synthetic `java/lang/ClassLoader` in `stdlib.rs`. The resource-enumeration API surface (`getResource(s)`, `getSystemResource(s)`, `getResourceAsStream`) is incomplete. | **CLEARED for app by #1320; CLEARED for ladder here** — `native_class_loader_get_system_resources` added (mirrors `getResources` with `None` loader). | synthetic-stdlib / native — `java_lang` (`ClassLoader` resource API) |
| **1a** | `method not found: java/lang/ClassLoader.getSystemClassLoader()Ljava/lang/ClassLoader;` — app fixture's first blocker after #1320. | `getSystemClassLoader` missing on synthetic `java/lang/ClassLoader`. | **CLEARED here** — `native_class_loader_get_system_class_loader` returns one stable synthetic `ClassLoader` cached in a new static field. | synthetic-stdlib / native — `java_lang` (`ClassLoader` static/system-loader API) |
| **1b** | `method not found: java/lang/ClassLoader.loadClass(Ljava/lang/String;)Ljava/lang/Class;` — app, after 1a cleared. | Base `ClassLoader.loadClass` missing on synthetic `java/lang/ClassLoader`. | **CLEARED here** — registered `native_url_class_loader_load_class` (parent-first delegation) for base `ClassLoader.loadClass`. | synthetic-stdlib / native — `java_lang` (`ClassLoader`) |
| **1c** | `method not found: ch/qos/logback/classic/util/DefaultJoranConfigurator.getClass()Ljava/lang/Class;` — app, after 1b cleared. | Inherited `java/lang/Object.getClass()` invoked through an **interface-typed** callsite (`Configurator.getClass()`): the invokeinterface native fallback probed only the receiver class and the interface directly — never the receiver's super chain — so the `java/lang/Object.getClass` native was never found. | **CLEARED here** — `lookup_native_kind_in_super_chain` added; invokeinterface native fallback now walks the super chain like invokevirtual. | execution.rs interface dispatch (native fallback) |
| **1d** | `operand stack underflow` — ladder, after `getSystemResources` cleared. Deep in real commons-logging `LogFactory.getFactory`. | `java/lang/ref/WeakReference.get()Ljava/lang/Object;` returned no value (offset 188), so the following `checkcast` (191) underflowed. Reference-object native unimplemented. | **CLEARED here** — minimal non-collecting synthetic `java/lang/ref/Reference`/`WeakReference` (strong-ref-backed `referent` slot; `<init>`/`get`/`clear`). | native / stdlib — `java.lang.ref` (`Reference`/`WeakReference`) |
| **1e** | `class not found: java/time/ZoneId` — app, after 1c cleared. Deeper in logback configuration. | Synthetic `java.time` surface incomplete — `java/time/ZoneId` is not registered. | **CLEARED here** — minimal synthetic `ZoneId` (UTC system default; `of`/`getId`/`toString`). | native / stdlib — `java_time` (`ZoneId`) |
| **1e′** | `class not found: java/util/Locale` — app, after 1e cleared. logback timestamp formatting. | Synthetic `java.util` surface incomplete — `java/util/Locale` is not registered. | **CLEARED here** — minimal synthetic `Locale` (fixed en-US default; `getDefault`/`getLanguage`/`getCountry`/`toString`). | native / stdlib — `java_util` (`Locale`) |
| **1e″** | `method not found: java/time/format/DateTimeFormatter.ofPattern(Ljava/lang/String;)Ljava/time/format/DateTimeFormatter;` — app, after 1e′ cleared. logback `CachingDateFormatter` builds a pattern formatter. | Synthetic `DateTimeFormatter` carries only the ISO constant instances, not the `ofPattern(String)` factory. | **OBSERVED — current app pin. HANDOFF** to java.time formatter lane (deferred to future wave). | native / stdlib — `java_time` (`DateTimeFormatter`) |
| **1f** | `class not found: org/apache/logging/slf4j/SLF4JProvider` — ladder, after 1d cleared. | The log4j-to-slf4j binding's `SLF4JProvider` is not resolvable — a logging-backend provider class discovered via the SLF4J `ServiceLoader`/provider mechanism is missing from the classpath resolution path. | **CLEARED here** — lazy-key 3-arg `Class.forName` makes the availability probe catchable. | class-loader — `Class.forName` availability probe (`java_lang`) |
| **1f′** | `class not found: org/apache/logging/log4j/MarkerManager` — ladder, after 1f cleared. `Log4jApiLogFactory.<clinit>` calls `MarkerManager.getMarker`. | A real JVM raises a catchable `NoClassDefFoundError` (`LinkageError`) here that commons-logging catches; duke had no synthetic `NoClassDefFoundError`/`LinkageError` and surfaced class-resolution failures during execution as fatal errors. | **CLEARED (2026-07-12, linkage-error lane)** — synthetic `LinkageError`/`NoClassDefFoundError`/`ExceptionInInitializerError` registered; execution-time class-resolution failures at `invokestatic`/`get(field\|static)`/`put(field\|static)` throw a catchable `NoClassDefFoundError`; `<clinit>` failures get JVMS 5.5 erroneous-class semantics. commons-logging's `catch (LinkageError)` now fires. | interpreter — class-resolution / `LinkageError` (**this lane**) |
| **1f″** | `method not found: java/util/Hashtable.computeIfAbsent(Ljava/lang/Object;Ljava/util/function/Function;)Ljava/lang/Object;` — ladder, after 1f′ cleared. The `LogFactoryImpl` fallback (reached via the `catch (LinkageError)` fall-through) calls `Hashtable.computeIfAbsent`. | Synthetic `java/util/Hashtable` does not implement the `computeIfAbsent(Object,Function)` default method. | **OBSERVED — current ladder pin. HANDOFF** to collections lane (`NoSuchMethodError` territory, deferred to future wave). | native / stdlib — `java_util` (`Hashtable`) |
| **2** | `--real-jdk` probe: getfield slot 10 out of bounds on an object allocated with 1 slot where real bytecode expects a larger layout (`crates/duke-interpreter/src/execution.rs`, Getfield handler `heap.get(r)?.fields[fidx]`; originally observed on `java/net/URL`). | Synthetic-vs-real **object-layout coherence** boundary: the object is allocated synthetically (few slots) but real bytecode indexes its full field layout; the raw `fields[fidx]` index panics with `layout_coherence_check` disabled. | **STILL INFERRED/OPEN** (seen only under `--real-jdk`, past blocker #1; re-confirmed pre-existing on the linkage-error branch — NOT a class/field-resolution failure, so out of the linkage-error lane) | native / stdlib object-layout — `java_net` (`URL`) / real-jdk shadow lane |
| **3** | Resource enumeration + `META-INF/spring.factories` and `META-INF/spring/…AutoConfiguration.imports` discovery returning empty/failing. | Spring's `SpringFactoriesLoader` / `ImportCandidates` walk **every** classpath entry via `ClassLoader.getResources`; requires working nested-jar resource enumeration (depends on #1). | **INFERRED** | java_util + classloader/registry (resource enumeration) |
| **4** | Heavy `java.lang.reflect` use during auto-configuration: `Constructor.newInstance`, `Method.invoke`, annotation reads, `Class.forName` fan-out. | `SpringApplication.run` instantiates and wires beans almost entirely reflectively; annotation metadata is read via reflection/ASM. | **INFERRED** | reflect (`java_lang_reflect`) + `java_lang` (`Class`) |
| **5** | `java.util` collections/streams volume: `Map`/`Set`/`List`, `Stream`, `Optional`, `ConcurrentHashMap`, `EnumMap` under the bean factory. | Spring's environment/bean machinery leans on the full collections + streams surface with real iteration/ordering semantics. | **INFERRED** | java_util |
| **6** | `System` properties/env + likely `sun.misc.Unsafe` / `AccessController` (doPrivileged) paths. | Spring reads `System.getProperties`/`getenv` extensively; parts of the loader and core touch privileged/Unsafe paths. | **INFERRED** | java_lang (`System`) + native (`Unsafe`/`AccessController`) |

Blockers 0 and 1 are directly reproduced on the default path. Blocker 2 was seen
once under a `--real-jdk` (jimage-backed) probe run specifically to look *past*
blocker 1. Blockers 3–6 are predicted from Spring Boot's known
`SpringApplication.run` sequence and are listed to scope the remaining work, not
because they have been individually reproduced here.

## What this lane fixed

**File:** `crates/duke-interpreter/src/registry.rs`, `ensure_loaded_inner`,
guard at **line 1219** (commit `5860f6b`).

The `self.classes.contains_key(&class_key)` fast-path branch was recording
per-loader/per-code-source provenance (`class_code_sources`,
`class_runtime_loaders`) onto **plain synthetic bootstrap classes** such as
`java/lang/System` and `java/lang/ClassLoader`. Those are loader-agnostic
singletons: `class_key_from_provenance` deliberately returns their *plain* key
regardless of code source / runtime loader. But once provenance was recorded,
`is_plain_bootstrap_class(name)` flipped `true → false`, so the *next*
`class_key_from_provenance` call began computing a **loader-suffixed** key
(`java/lang/System\0loader:113`) for the same singleton — a key nothing was
stored under → `ClassNotFound`.

The single-classloader HelloWorld synthetic-fat-jar test survived only by luck:
it touches `System` exactly once, so its key was computed *before* the poison was
recorded. Spring's multi-classloader boot (bootstrap → app → `LaunchedClassLoader`)
touches these singletons under multiple loaders and tripped it immediately.

**Why this is a classloader-machinery bug, not a stdlib/native one:** nothing
about `System` or `ClassLoader`'s *behavior* was wrong. The bug was purely in the
registry's **class-identity/key bookkeeping** — how provenance keys are derived
and when they are recorded. That is the classloader/registry lane's
responsibility.

**Fix:** guard the recording branch with
`if !self.is_plain_bootstrap_class(&internal_name)` so plain bootstrap singletons
never acquire provenance, keeping `is_plain_bootstrap_class` and
`class_key_from_provenance` in sync.

**Generality:** this is not a fixture-specific hack. It unblocks *any*
multi-classloader scenario that resolves a bootstrap singleton under more than one
loader — precisely the pattern Spring's `LaunchedClassLoader` (and any nested-jar
or custom-classloader app) creates. The two Spring Boot fixtures are the
regression witnesses, but the fix is generic.

## Update (this wave): two more rungs cleared

**Ladder — `SLF4JProvider` `Class.forName` availability probe (CLEARED).**
`crates/duke-interpreter/src/native/java_lang.rs`, `native_class_for_name_with_loader`.
commons-logging `LogFactory.newStandardFactory` probes for logging backends via
`isClassAvailable(name, cl)` → `Class.forName(name, false, cl)`, catching
`ClassNotFoundException`. Duke's 3-arg `Class.forName` native computed the class
identity key **eagerly** (with `?`) alongside the load attempt; for an absent
class the key lookup itself raises `ClassNotFound`, propagating a **fatal** error
before the not-found result could be mapped to a catchable
`ClassNotFoundException`. Fix: compute the key lazily, only on a successful load.
New ladder frontier (at the time): `class not found: org/apache/logging/log4j/MarkerManager`,
reached while initializing `Log4jApiLogFactory` (its `<clinit>` calls
`MarkerManager.getMarker`). A real JVM raises a **catchable `NoClassDefFoundError`**
(a `LinkageError`) that commons-logging catches and falls through on.

**Ladder — linkage-error semantics; `MarkerManager` rung CLEARED (2026-07-12).**
The interpreter now raises a catchable `NoClassDefFoundError` (a `LinkageError`)
at execution-time class-resolution failures (`invokestatic`/`get(field|static)`/
`put(field|static)`) and gives `<clinit>` failures JVMS 5.5 erroneous-class
semantics, so commons-logging's `catch (LinkageError)` around `Log4jApiLogFactory`
init fires and falls through to `LogFactoryImpl`. New ladder frontier:
`method not found: java/util/Hashtable.computeIfAbsent(...)` (collections lane
handoff — the `LogFactoryImpl` fallback path).

**App — minimal `java.time.ZoneId` for boot (CLEARED).**
`crates/duke-interpreter/src/native/java_time.rs` + registration in
`crates/duke-interpreter/src/stdlib.rs`. logback's timestamp formatting reaches
`ZoneId` during Spring Boot startup. Registered a thin synthetic `ZoneId`:
`systemDefault()` reports **UTC** (duke's `Instant`/`LocalDate*` clocks are all
epoch/UTC based, so a UTC default keeps timestamps self-consistent **without**
modelling `ZoneRules`/tzdb), plus `of(String)`, `getId()`, `toString()`. Clearing
`ZoneId` did **not** cascade into a chronology/tzdb chain — the app frontier moved
to `class not found: java/util/Locale` (a shallow, separate java.util rung).

**App — minimal `java.util.Locale` for boot (CLEARED).**
`crates/duke-interpreter/src/native/java_util.rs` + registration in
`crates/duke-interpreter/src/stdlib.rs`. logback's `CachingDateFormatter` calls
`Locale.getDefault()` during Spring Boot startup. Registered a thin synthetic
`Locale`: a holder carrying a language tag + country code, with `getDefault()`
reporting a **fixed en-US** locale (a fixed default keeps boot deterministic
**without** a CLDR/`ResourceBundle` build-out; the formatters duke models are
locale-insensitive, so the choice is inert beyond boot), plus
`getDefault(Locale$Category)`, `getLanguage()`, `getCountry()`, `toString()`.
Clearing `Locale` did **not** cascade into a locale/CLDR chain — the app frontier
moved to `method not found: java/time/format/DateTimeFormatter.ofPattern(String)`
(a java.time formatter rung, handoff to a future wave).

## Lane ownership summary

| Blocker | Owning lane | Status |
| --- | --- | --- |
| #0 provenance-poison on bootstrap singletons | classloader / registry (`registry.rs`) | **DONE (this branch)** |
| #1 `ClassLoader.getSystemResources` missing | synthetic-stdlib `java_lang` — `ClassLoader` resource API (`stdlib.rs`) | **CLEARED (app #1320, ladder this branch)** |
| #1a `ClassLoader.getSystemClassLoader` missing | synthetic-stdlib `java_lang` — `ClassLoader` static/system-loader API (`stdlib.rs`) | **CLEARED (this branch)** |
| #1b `ClassLoader.loadClass` missing | synthetic-stdlib `java_lang` — `ClassLoader` (`stdlib.rs`) | **CLEARED (this branch)** |
| #1c `Object.getClass()` interface-callsite dispatch | execution.rs interface dispatch (native fallback super-chain walk) | **CLEARED (this branch)** |
| #1d `WeakReference.get()` underflow in commons-logging | native — `java.lang.ref` reference-object | **CLEARED (this branch)** |
| #1e `class not found: java/time/ZoneId` | native / stdlib — `java_time` (`ZoneId`) | **CLEARED (this branch)** |
| #1e′ `class not found: java/util/Locale` | native / stdlib — `java_util` (`Locale`) | **CLEARED (this branch)** |
| #1e″ `method not found: DateTimeFormatter.ofPattern(String)` | native / stdlib — `java_time` (`DateTimeFormatter` factory) | **CLEARED (this branch)** |
| #1e‴ `operand stack underflow` in logback `COWArrayList.addIfAbsent` | java.util.concurrent (`CopyOnWriteArrayList`) + interpreter lenient method-dispatch | **CLEARED (2026-07-12, dispatch-honesty lane): lenient dispatch now throws catchable `NoSuchMethodError`; synthetic `CopyOnWriteArrayList` added** |
| #1e⁗ `method not found: java/lang/StringBuilder.append(Object)` | native — `java_lang` (`StringBuilder` `append(Object)`, needs `toString` dispatch) | **HANDOFF — current app pin (deferred to a java.lang wave)** |
| #1f `class not found: org/apache/logging/slf4j/SLF4JProvider` | class-loader — `Class.forName` availability probe (`java_lang`) | **CLEARED (this branch)** |
| #1f′ `class not found: org/apache/logging/log4j/MarkerManager` | interpreter — class-resolution / `NoClassDefFoundError` linkage during `<clinit>` (`execution.rs` + synthetic `LinkageError` in `stdlib.rs`) | **CLEARED (2026-07-12, linkage-error lane)** |
| #1f″ `method not found: java/util/Hashtable.computeIfAbsent(Object,Function)` | native / stdlib — `java_util` (`Hashtable`) | **HANDOFF — current ladder pin (deferred to future wave)** |
| #2 `java/net/URL` layout coherence (real-jdk getfield slot 10 panic) | native / stdlib object-layout — `java_net` (`URL`) / real-jdk shadow lane | inferred/open |
| #3 `spring.factories` / `AutoConfiguration.imports` resource enumeration | java_util + classloader/registry | inferred |
| #4 reflective bean instantiation / annotations | reflect (`java_lang_reflect`) + `java_lang` (`Class`) | inferred |
| #5 collections / streams volume | java_util | inferred |
| #6 `System` properties/env, `Unsafe`/`AccessController` | java_lang (`System`) + native | inferred |

## Test scaffolding

New file: `duke/tests/spring_boot_real_app.rs`. Mirrors the OSS-jar canary/pin
convention (`crates/duke-interpreter/tests/oss_jar_smoke.rs`). Four tests, two per
fixture:

- **2 `#[ignore]`d end-to-end CANARIES** — `spring_boot_app_boots_end_to_end`,
  `spring_boot_ladder_boots_end_to_end`. Each asserts a *successful* boot
  (banner + `Started … in … seconds` for the app; "completed all rungs" for the
  ladder). They stay ignored until boot actually reaches that point;
  **graduate them** (remove `#[ignore]`) when the happy path clears.
- **2 non-ignored PINS** —
  `spring_boot_app_surfaces_next_missing_capability_explicitly`,
  `spring_boot_ladder_surfaces_next_missing_capability_explicitly`. Each asserts
  the process still fails at **exactly** the current blocker. As of #1320 the
  two fixtures diverge, so the pins assert per-fixture constants. As of the end of
  this wave they are (updated 2026-07-12, dispatch-honesty lane: the
  `COWArrayList.addIfAbsent` rung cleared by honest `NoSuchMethodError` dispatch +
  synthetic `CopyOnWriteArrayList`/`LinkedBlockingQueue`/`InheritableThreadLocal`/
  `Record`):
  `APP_BLOCKER = "method not found: java/lang/StringBuilder.append(Ljava/lang/Object;)Ljava/lang/StringBuilder;"`
  and (unchanged; the `Hashtable.computeIfAbsent` ladder rung is another lane's)
  `LADDER_BLOCKER = "method not found: java/util/Hashtable.computeIfAbsent(Ljava/lang/Object;Ljava/util/function/Function;)Ljava/lang/Object;"`.
  If boot advances (or regresses) past those strings, the pin **trips**, forcing a
  re-observe and an update to this doc.

**Repro commands:**

```sh
# The two pins (run by default; assert we're still parked at blocker #1):
cargo test -p duke --test spring_boot_real_app

# The two ignored end-to-end canaries (currently expected to fail):
cargo test -p duke --test spring_boot_real_app -- --ignored

# Drive a single fixture by hand to eyeball the blocker:
cargo run -p duke -- -jar tests/fixtures/oss-jars/spring-boot/duke-spring-boot-app-3.5.12.jar
cargo run -p duke -- -jar tests/fixtures/oss-jars/spring-boot/duke-spring-boot-ladder-3.5.12.jar
```

## Verification

**LOCAL-ONLY gate — CI runners are permanently dead in this environment, so all
verification below was run locally.**

- `cargo build --workspace` — clean.
- `cargo test --workspace --no-fail-fast` — **3048 passed / 0 failed / 4 ignored**
  (end of this wave; +1 new `test_locale_get_default_is_en_us` unit test).
- `cargo fmt --check` — clean.
- `cargo clippy --workspace` — zero warnings.

Exact repro:

```sh
cargo build --workspace
cargo test --workspace --no-fail-fast
cargo fmt --check
cargo clippy --workspace --all-targets

# Focused:
cargo test -p duke --test spring_boot_real_app              # 2 passed / 2 ignored
cargo test -p duke --test spring_boot_real_app -- --ignored # canaries (expected fail today)
```

## Update (this wave): `DateTimeFormatter` cleared; app advances to a concurrent-collection underflow

**App — minimal `java.time.format.DateTimeFormatter` for boot (CLEARED).**
`crates/duke-interpreter/src/native/java_time.rs` + registration in
`crates/duke-interpreter/src/stdlib.rs`. logback's
`ch.qos.logback.core.util.CachingDateFormatter` builds a pattern-based formatter
during Spring Boot startup: `ofPattern(String)` in its ctor, then `withZone(ZoneId)`
and `withLocale(Locale)`, then per-format `Instant.ofEpochMilli(long)` +
`format(TemporalAccessor)`. The synthetic `DateTimeFormatter` previously carried
only the ISO constant instances (no factory), so `ofPattern` was `method not found`.

Implemented four natives on `java/time/format/DateTimeFormatter`:

- `ofPattern(String)` (static) — allocates a fresh synthetic formatter carrying the
  pattern string in its `string_value`.
- `withZone(ZoneId)` (instance) — returns `this` unchanged. **Honest** because duke
  is UTC-only (wave-5 `ZoneId`=UTC decision): a zone-bound and an unbound formatter
  produce identical output under UTC.
- `withLocale(Locale)` (instance) — returns `this` unchanged. **Honest** because
  duke's `Locale` is a fixed en-US default; there is nothing to re-bind.
- `format(TemporalAccessor)` (instance) — reads the formatter's pattern (or ISO
  constant name) from `string_value`, extracts the temporal's broken-down **UTC**
  fields (`Instant` via epoch-seconds/nanos, `LocalDateTime`, `LocalDate`), and
  renders them via a small pure pattern engine.

**Pattern engine** (`format_with_pattern`, unit-tested). Supported subset (en-US,
UTC): `y`/`u` (year, zero-padded to count); `M`/`L` (`M`/`MM` numeric, `MMM` short
name, `MMMM`+ full name); `d` (day); `H` (hour 0–23), `h` (clock-hour 1–12);
`m`/`s` (minute/second); `S…` (fraction-of-second, `nano / 10^(9-n)` padded to `n`,
so `SSS` = millis); `a` (AM/PM); `X`/`XX`/`XXX` (zone-offset → always `Z` under
duke's zero UTC offset); `z`/`zzzz` (zone name → `UTC`); single-quoted literals
(`'T'` → `T`, `''` → `'`); any non-letter char is a literal. Unrecognized letters
are emitted verbatim, never panicking.

**Simplifications (documented in code):** UTC-only offsets (`X`→`Z`), fixed en-US
month names / AM-PM, and the three ISO constant instances mapped to fixed
equivalent patterns — `ISO_LOCAL_DATE` → `yyyy-MM-dd`, `ISO_LOCAL_DATE_TIME` →
`yyyy-MM-dd'T'HH:mm:ss`, `ISO_INSTANT` → `yyyy-MM-dd'T'HH:mm:ss.SSSXXX` — rather
than the full ISO-8601 formatting rules.

**Unit tests** (`#[cfg(test)] mod datetimeformatter_pattern_tests` in
`native/java_time.rs`): epoch-0 → `"yyyy-MM-dd HH:mm:ss.SSS"` = `"1970-01-01
00:00:00.000"`; epoch-0 → `"yyyy-MM-dd'T'HH:mm:ss.SSSXXX"` = `"1970-01-01
T00:00:00.000Z"`; a non-zero 2026-07-11 14:05:09.123456789 covering `hh`/`a`
clock-hour + AM/PM and a multi-digit (`SSSSSS`) fraction; month names (`MMM`/`MMMM`);
quoted literals and `''`; midnight clock-hour = `12 AM`; unrecognized `QQ` verbatim;
and the ISO-constant → pattern mapping.

**Rung cleared:** `#1e″` — `method not found:
java/time/format/DateTimeFormatter.ofPattern(...)`.

**New app frontier (HANDOFF).** With the formatter working, boot advances and now
lands on `duke: runtime error: operand stack underflow` — the app's only output.
Traced (`DUKE_TRACE_EXEC=1`) to logback's
`ch/qos/logback/core/util/COWArrayList.addIfAbsent`: at offset 5 it calls
`java/util/concurrent/CopyOnWriteArrayList.addIfAbsent(Ljava/lang/Object;)Z` and at
offset 8 `pop`s the boolean. Duke has **no** synthetic `CopyOnWriteArrayList`; it
resolves the class leniently (the `new`/`invokespecial <init>()V` are silently
absorbed, and the `addIfAbsent` `invokevirtual` returns **void** instead of pushing
a boolean), so the following `pop` underflows the operand stack. Note this surfaces
as a generic interpreter runtime error, **not** a clean `method not found` — the
lenient dispatch swallows the missing method.

**Ownership assessment: owned-elsewhere (not java.time).** The gap is a synthetic
`java/util/concurrent/CopyOnWriteArrayList` (`addIfAbsent`/`add`/`remove` returning
their `boolean` results) and/or a stricter method-dispatch that raises
`method not found` rather than silently returning void for an unregistered method.
That is the **java.util.concurrent / interpreter lenient-dispatch lane**, outside
the java.time family, so it is pinned rather than fixed here.
`APP_BLOCKER` is now `"duke: runtime error: operand stack underflow"`.
`LADDER_BLOCKER` is unchanged (`"class not found: org/apache/logging/log4j/MarkerManager"`).

---

## Session 2026-07-13 — `StringBuilder.append(Object)` + a run of shallow java.lang/java.util APP rungs

**Mission:** implement `java/lang/StringBuilder.append(Ljava/lang/Object;)Ljava/lang/StringBuilder;`, then climb as many shallow APP rungs as possible.

**toString-dispatch mechanism (the crux of `append(Object)`).** Real
`StringBuilder.append(Object o)` == `append(String.valueOf(o))` == `"null"` when
`o == null`, else `o.toString()`. Calling a Java `toString()` from native code
would need interpreter re-entry — but Duke has an existing precedent that does
**not** re-enter: the free function `heap_object_to_string(&HeapObject, u64)` in
`native/common.rs`. It renders String / boxed primitives (Integer/Long/Double/
Float/Boolean/Character) / Class / UUID directly from the heap object, falling
back to `class@hex`. `String.valueOf(Object)` (`native_string_value_of_object`)
and `PrintStream.println(Object)`/`print(Object)` all use it. The new
`native_sb_append_object` **mirrors `native_string_value_of_object` exactly**:
null → append literal `"null"`; otherwise append `heap_object_to_string(...)`;
return `this`. No forbidden file was edited (all `native/*.rs` are `include!`d
into one module, so the helper is directly callable from `java_lang.rs`).

**Rungs cleared this session (each a small native mirroring existing ones):**
- `java/lang/StringBuilder.append(Ljava/lang/Object;)Ljava/lang/StringBuilder;`
  — `native_sb_append_object` (toString dispatch via `heap_object_to_string`).
- `java/lang/String.indexOf(II)I` — `native_string_index_of_char_from`.
- `java/lang/String.lastIndexOf(I)I` — `native_string_last_index_of_char`.
- `java/lang/String.lastIndexOf(II)I` — `native_string_last_index_of_char_from`.
  (All char-index natives mirror `native_string_index_of_char`; indices counted
  in chars, consistent with the existing char `indexOf`.)
- `java/util/LinkedHashSet` — synthetic class reusing the entire `HashSet` native
  family + the shared `duke/util/HashSetIterator`. Duke's HashSet already stores
  elements in insertion order (a flat field list), so insertion-ordered iteration
  is automatic. `<init>()V/(I)V/(IF)V`, add/addAll/contains/remove/size/isEmpty/
  iterator/toArray/stream all registered.
- `java/util/WeakHashMap` — synthetic class reusing the full `HashMap` native
  family (incl. the Map-default callback family), exactly like `Hashtable` does.
  Duke does not model GC-driven key eviction, so it behaves as a plain HashMap —
  the same documented simplification pattern.
- `java/util/IdentityHashMap` — synthetic class reusing the `HashMap` native
  family, incl. the `<init>(I)V` capacity ctor. This is *faithful*, not just a
  simplification: Duke's shared map key comparison (`slots_equal`) already
  compares general object keys by **identity** (`ra == rb`), only falling back to
  value equality for String/Class/UUID/boxed keys, which IdentityHashMap is not
  used with in these bootstraps.
- `java/util/AbstractMap` — synthetic class with a no-op `<init>()V`
  (`native_object_init`). A real map class loaded from the jar chains its `<init>`
  to `AbstractMap.<init>()V`; the synthetic super lets that `super()` resolve.
- `java/util/HashMap.<init>(I)V` and `(IF)V`, `java/util/HashSet.<init>(I)V` and
  `(IF)V`, `java/util/LinkedHashSet.<init>(I)V` and `(IF)V` — capacity/loadFactor
  hints ignored, all mapped to the existing no-arg init native.
- `java/util/HashSet.addAll(Ljava/util/Collection;)Z` — `native_hashset_add_all`
  (pulls elements via the source's `toArray()`, mirroring
  `native_hashset_init_from_collection`; routes each through `native_hashset_add`
  for dedup; returns whether the set changed). Also registered on LinkedHashSet.
- `java/util/Collections.addAll(Ljava/util/Collection;[Ljava/lang/Object;)Z` —
  `native_collections_add_all` (invokes `add(Object)` on the target collection
  for each array element, so any Collection impl works).
- `java/util/Collections.newSetFromMap(Ljava/util/Map;)Ljava/util/Set;` —
  `native_collections_new_set_from_map`. `newSetFromMap`'s contract requires the
  supplied map to be empty, so the returned set starts empty; Duke returns a
  fresh field-backed `HashSet` (same non-observable-backing-type simplification
  as WeakHashMap).
- `java/lang/ref/ReferenceQueue` — non-collecting stub: no-op `<init>()V`,
  `poll()` always returns null (Duke never enqueues, matching its existing
  non-collecting Reference model). Plus the two-arg
  `WeakReference(referent, ReferenceQueue)` constructor (queue accepted +
  ignored, referent stored via `native_reference_init`).

**New APP frontier (HANDOFF — a real subsystem, pinned not fixed).** After the
rungs above, the app boots far enough to fail *inside a reflectively-invoked
method*. `native_reflect_method_invoke` catches the target's `JavaException` and
re-throws it as `InvocationTargetException`, **discarding the cause's class name**,
so the only visible output is the generic
`duke: runtime error: java exception: java/lang/reflect/InvocationTargetException`.
Temporarily instrumenting that catch arm (peeking the uncaught-exception ref's
detail message via `take_uncaught_java_exception_ref`) revealed the underlying
cause: `NoClassDefFoundError: java/util/EnumSet`. Registering an empty synthetic
`java/util/EnumSet` only uncovers a deeper `ExceptionInInitializerError` from an
enum/config static initializer that uses EnumSet's Class-typed factories
(`noneOf`/`allOf`/`range`/`of`, which need enum-constant reflection and
ordinal-based bit-set storage). That is a genuine subsystem — enum reflection +
EnumSet ordinal semantics — not a one-function mirror, so the app is pinned here.
All instrumentation was reverted; `reflect.rs` is byte-for-byte unchanged.

`APP_BLOCKER` is now `"java exception: java/lang/reflect/InvocationTargetException"`
(root cause `NoClassDefFoundError: java/util/EnumSet`, then
`ExceptionInInitializerError`, both documented in the test).
`LADDER_BLOCKER` is unchanged
(`"method not found: org/apache/commons/logging/impl/LogFactoryImpl.objectId(Ljava/lang/Object;)Ljava/lang/String;"`).

## 2026-07-13 — LADDER climb: objectId → Serializable → Logger.logp → Thread CCL → getInterfaces (stacked branch `swarm/ladder-objectid`)

Continuing from the `objectId` fix (PR #1347, invokestatic super-walk, which advanced
the ladder to `class not found: java/io/Serializable`), three in-lane rungs were cleared
one at a time on a branch stacked off `swarm/invokestatic-super-walk`. Each rung was
observed directly from the freshly-built binary running
`duke -jar tests/fixtures/oss-jars/spring-boot/duke-spring-boot-ladder-3.5.12.jar`.

**Rung 1 — `class not found: java/io/Serializable` (CLEARED).** Registered as an empty
synthetic marker interface in `stdlib.rs`, immediately after the existing
`java/lang/AutoCloseable` block and using its exact `ClassContext` field set
(`super_class: None`, empty constant pool / methods / fields / interfaces, zero
`instance_field_count`, `load_source: ClassLoadSource::Synthetic`). Registered so
reflective hierarchy walks / `is_assignable_from` resolve it instead of raising
`ClassNotFound`.

**Rung 2 — `method not found: java/util/logging/Logger.logp(...)` (CLEARED).**
commons-logging's `Jdk14Logger` routes every log call through
`logp(Level, sourceClass, sourceMethod, msg)` and the 5-arg
`logp(..., Throwable)` overload. Added `native_jul_logger_logp` and
`native_jul_logger_logp_throwable` in `native/java_util_logging.rs`, mirroring the
existing `native_jul_logger_log` / `native_jul_logger_log_throwable` (level = arg 1,
message = arg 4, thrown = arg 5; source class/method are `LogRecord` metadata that
Duke's formatter does not render, so they are accepted and ignored). Registered both
descriptors on `java/util/logging/Logger` in `stdlib.rs`.

**Rung 3 — uncaught `InvocationTargetException` → `NullPointerException`, then a
follow-on `getResourceAsStream(...) == null` (BOTH CLEARED).** The visible blocker was
the generic `java exception: java/lang/reflect/InvocationTargetException`. Temporarily
instrumenting the two wrap sites in `native/reflect.rs` (printing the discarded cause +
the reflectively-invoked method; since reverted, `reflect.rs` byte-for-byte unchanged)
showed the target method was the fixture's OWN `com/example/duke/ladder/LadderApplication.main`
and the cause a `NullPointerException`. Bytecode + `DUKE_TRACE_EXEC` traced it to
`main@28 invokevirtual java/lang/ClassLoader.getResourceAsStream` with a null receiver:
`main@13` called `Thread.currentThread().getContextClassLoader()` which returned null.

Root cause: `native_thread_current_thread` allocates a fresh, throwaway
`java/lang/Thread` object on every call (with `contextClassLoader = null`). The Spring
Boot launcher (`Launcher.launch@0-4`) does call
`Thread.currentThread().setContextClassLoader(launchedLoader)`, but that write lands on a
discarded instance and is invisible to the next `currentThread().getContextClassLoader()`
in `main`.

Fix (in-lane, `native/java_lang.rs` + `stdlib.rs`): persist the main thread's context
loader in a new static slot `$dukeMainContextClassLoader` on the synthetic
`java/lang/Thread` (declared in the Thread `ClassContext`, same singleton pattern as
`$dukeSystemClassLoader` on `java/lang/ClassLoader`). `setContextClassLoader` (now a
`register_callback` native) writes both the instance field and the static slot;
`getContextClassLoader` (now `register_callback`) returns the instance field when set,
else the persisted static loader, else the system class loader — the JVM default for the
main thread (HotSpot installs it at VM startup). This also cleared the immediate
follow-on rung: with the real `LaunchedClassLoader` restored (it carries the fat-jar
archive path), `getResourceAsStream("META-INF/duke-ladder.properties")` resolves
`BOOT-INF/classes/META-INF/duke-ladder.properties`. Resource resolution keys off the
loader OBJECT's `runtime_loader_paths` (`native/common.rs`); a bare system loader has no
paths, so returning the launched loader — not just any system loader — was load-bearing.

**New LADDER frontier (HANDOFF — pinned, NOT patched; OUT OF LANE).** The ladder now
lands on `method not found: java/lang/Class.getInterfaces()[Ljava/lang/Class;`, but that
is only a SYMPTOM. `LogFactoryImpl.createLogFromClass@404` evaluates
`newLogger instanceof org/apache/commons/logging/Log` on the reflectively-constructed
`Jdk14Logger` (which genuinely `implements org.apache.commons.logging.Log,
java.io.Serializable`). Duke's `instanceof` returns FALSE, diverting control into
`handleFlawedHierarchy`, whose first act is `Class.getInterfaces()` and whose ultimate
act is to throw a spurious `LogConfigurationException`. The false negative is in
`is_assignable_from` (`crates/duke-interpreter/src/native/common.rs`, ~line 11465): a
class's interface entries are plain internal names (`org/apache/commons/logging/Log`)
while `to_key` is loader-qualified (`org/apache/commons/logging/Log loader:NN`), so the
`iface == to_key` comparison never matches (the super-class comparison one line up has
the same shape). The real fix is to normalize the interface/`to_key` comparison in
`is_assignable_from` (`native/common.rs`) or the `Instanceof` opcode (`execution.rs`) —
the interpreter class-identity/assignability lane, which is out of this lane's scope
(`registry.rs` / `native/common.rs` / `execution.rs` are owned elsewhere). Adding
`getInterfaces()` alone would be semantically WRONG: it would not boot the ladder, it
would only swap `method not found` for a false `LogConfigurationException`, so the rung
is pinned honestly rather than patched.

`LADDER_BLOCKER` is now
`"method not found: java/lang/Class.getInterfaces()[Ljava/lang/Class;"`
(root cause: `instanceof(Jdk14Logger, org/apache/commons/logging/Log)` false negative in
`is_assignable_from`, documented in the test). The ladder end-to-end canary stays
`#[ignore]`d. `APP_BLOCKER` and the app section are untouched. Full workspace suite stays
3096/0/4; fmt + clippy (1.97.0, pedantic + nursery) clean; HelloWorld class + jar modes
both print `Hello, World!`. No edits to `registry.rs`, `native/common.rs`, or
`execution.rs`.

---

## 2026-07-13 — instanceof loader-key fix clears the whole commons-logging wall; ladder now walls in its own `main` (wave-9 pin)

The pinned `Class.getInterfaces()` frontier above was only a symptom of a false-negative
`instanceof`. That is now **FIXED** in `is_assignable_from` (`native/common.rs`, branch
`swarm/instanceof-loader-keys`, cherry-picked onto `swarm/ladder-objectid`/PR #1353).

**Root cause (confirmed in code):** a `ClassContext`'s interface entries are stored either
as PLAIN internal names — for classes built reflectively/synthetically via
`build_class_context`, which never runs the loader-resolution pass — or as LOADER-QUALIFIED
keys (`name\0loader:N`), rewritten by `ensure_loaded_inner` (`registry.rs:1281-1292`). The
target `to_key` is loader-qualified, so the old `iface == to_key` equality silently failed
for the plain case. commons-logging's `LogFactoryImpl.createLogFromClass` evaluated
`newLogger instanceof org/apache/commons/logging/Log` on the reflectively-built
`Jdk14Logger` (which genuinely `implements Log, Serializable`) and Duke returned FALSE,
diverting into `handleFlawedHierarchy` → `Class.getInterfaces()`.

**Fix (one comparison site):** compare interface entries on the PLAIN internal name —
`class_internal_name_from_key(&iface) == to_internal` (loader separator is `\0`;
`to_internal` already holds the plain name of `to_key`). This strips the loader qualifier
from both sides, is loader-agnostic per Duke's interface-by-name identity model, and fixes
BOTH `Instanceof` and `Checkcast` (both opcodes route through `is_assignable_from` at
`execution.rs:2839`/`2871`). Regression test
`is_assignable_from_matches_loader_qualified_interface` in `tests.rs` (plain interface
entry vs loader-qualified target key = true; unrelated class = false) fails on trunk,
passes with the fix.

**Ladder end-to-end result:** the fix clears commons-logging entirely — `Jdk14Logger.log`
/`info` now execute and the ladder boots ALL THE WAY THROUGH logging into its own
`LadderApplication.main`.

**NEW WALL — pinned for wave 9:** an uncaught `java/lang/reflect/InvocationTargetException`.
Decoded from `main`'s bytecode (main@10-48) plus temporary instrumentation of the reflect
wrap (since reverted): main does
`new Properties().load(Thread.currentThread().getContextClassLoader().getResourceAsStream(name))`.
Duke's synthetic `java/util/Properties.load(Ljava/io/InputStream;)V` throws
`java/io/IOException` (it cannot read the resource `InputStream` Duke returns), which
propagates out of `main`; the Spring Boot launcher's reflective `main.invoke` wraps it into
the uncaught `InvocationTargetException` (the visible blocker). **Wave-9 lane:** java.util
Properties / resource-stream I/O — teach `Properties.load(InputStream)` to actually parse
the stream handed back by `getResourceAsStream`.

`LADDER_BLOCKER` is re-pinned to `"java exception: java/lang/reflect/InvocationTargetException"`
(the same visible string as `APP_BLOCKER`, though the two have different underlying causes —
LADDER = `Properties.load` IOException, APP = `NoClassDefFoundError: java/util/EnumSet`).
The ladder end-to-end canary stays `#[ignore]`d with an updated reason. `APP_BLOCKER` and
the app section are untouched. Part-1 gate: workspace 3097/0/4 (+1 = the new test, zero
instanceof/checkcast/reflection regressions), fmt + clippy (1.97.0 pedantic+nursery) clean,
HelloWorld class + jar both print `Hello, World!`. No edits to `registry.rs`.

---

## 2026-07-13 — LADDER: two more in-lane rungs cleared; walls at reflective same-class private invoke

The commons-logging ladder climbs all of commons-logging/Jdk14Logger into the fixture's own
`LadderApplication.main` (via PRs #1347/#1354/#1355/#1353 + the cherry-picked loader-key
`instanceof` fix). This session (branch `swarm/ladder-objectid`, PR #1353) cleared main's
first two rungs; it now walls on the third (out of lane). The ladder canary stays
`#[ignore]`d and the pin still asserts the (unchanged visible) `InvocationTargetException`.

**Rung 1 — CLEARED — `Properties.load(InputStream)` over a classpath resource.**
`main` does `new Properties().load(getContextClassLoader().getResourceAsStream(
"META-INF/duke-ladder.properties"))`. `native_properties_load` (`native/java_util.rs`)
delegated to `properties_stream_id_from_slot`, which only recognised host-file-backed
streams (`fields[0]` = positive fd) and raised `java/io/IOException` for anything else.
But `getResourceAsStream` returns a synthetic `duke/io/ResourceInputStream` that holds the
already-resolved resource bytes in a backing byte-array field (+ read cursor). Fix: a shared
`input_stream_drain_all_bytes` (`native/java_io.rs`) detects the `ResourceInputStream` by
class name, reads its remaining bytes directly (advancing the cursor to consume the stream),
else host-file drain; `properties_read_input_stream_bytes` delegates to it and feeds the
pre-existing `parse_properties_bytes`. The `.properties` (`duke.ladder.name`/`rung`/
`exercises`) now parses and `getProperty` returns real values.

**Rung 2 — CLEARED — character-stream stack (`InputStreamReader` + `BufferedReader`).**
`main` then does `new BufferedReader(new InputStreamReader(getResourceAsStream(
"greeting.txt"), StandardCharsets.UTF_8)).readLine()`. No `java.io` Reader classes existed,
so `InputStreamReader.<init>(InputStream, Charset)` raised `NoSuchMethodError`. Added minimal
synthetics in `stdlib.rs` — `java/io/InputStreamReader` (fields: underlying stream, charset
name) and `java/io/BufferedReader` (fields: fully-decoded content String, char cursor) — with
natives in `native/java_io.rs`. `decode_bytes_with_charset` maps the ISO-8859-1/US-ASCII
family 1:1 and everything else as UTF-8 (lossy); `readLine()` walks `\n`/`\r`/`\r\n`
terminators and returns null at end-of-input. Documented simplification: `BufferedReader`
eagerly drains the whole stream at construction (real `java.io` reads lazily) — fine for
finite classpath resources.

> **Regression trap (fixed during development):** both readers extend `java/lang/Object`, and
> Duke must **not** register a synthetic `java/io/Reader`. The OSS smoke harness runs some
> fixtures (notably gson) against the REAL JDK-21 modules via `BootstrapLoader`, where the real
> `java/io/Reader` (which declares a `lock` field) and its real subclasses (`StringReader`,
> `JsonReader`) load from classfiles. A 0-field synthetic `java/io/Reader` shadows the real one
> and shifts subclass field indices, which surfaced as `InvalidFieldref { index: 0 }` in the
> gson canary. Dropping the `Reader` registration (the `Ljava/io/Reader;` in
> `BufferedReader.<init>` is only a descriptor type, no class-load required) fixes it.

**Rung 3 — WALL (pinned for wave 9, OUT OF LANE) — reflective invoke of the class's OWN
private static method.** `main` does `LadderApplication.class.getDeclaredMethod("summarize",
List.class).invoke(null, List.of(...))` (main@399) with no `setAccessible(true)`. The real JVM
permits this: `Method.invoke`'s access check is **caller-sensitive** — a class may always
reflectively access its own (private/nestmate) members, and only CROSS-class access to a
non-accessible member throws `IllegalAccessException`. Duke's `native_reflect_method_invoke`
(`native/reflect.rs`) keeps the coarse `!is_public && !is_accessible → throw`, which is
CORRECT for the cross-class case that the gson canary and
`ReflectionTest.privateMethodRaisesIllegalAccess` (which invokes a *different* class's private
method) depend on — I initially relaxed this guard and it BROKE both, so it was reverted. Duke
cannot yet distinguish same-class from cross-class here: the invoking frame's class is not
available to this native. The stack snapshot in `native_control_for_call` /
`native_needs_stack_snapshot` (`native/common.rs`, FORBIDDEN in this lane) is captured only for
Throwable-init and `Reflection.getCallerClass`, not `Method.invoke`. So the spurious
`IllegalAccessException` propagates out of `main`, and the Spring Boot launcher's reflective
`main.invoke` wraps it into the uncaught `java/lang/reflect/InvocationTargetException` (same
visible string as `APP_BLOCKER`, different underlying cause). **Wave-9 lane:** interpreter
reflection / native-boundary — add `Method.invoke`/`Constructor.newInstance` to
`native_needs_stack_snapshot` so `control.stack_trace()` is populated, then allow the invoke
when the caller frame's class equals the method's declaring class (rejecting only real
cross-class violations). The public reflective `Math.sqrt(2809)` invoke (main@337) and the
`IntStream.rangeClosed(1,8).mapToObj(...).collect(toList())` /
`List.stream().mapToInt(...).sum()` pipeline already work unmodified.

Root causes for rungs 2 and 3 were decoded by temporarily adding an `eprintln!` to the
reflect-wrap arm in `native/reflect.rs` (which surfaced the wrapped cause as
`NoSuchMethodError` then `IllegalAccessException`); that instrumentation has been reverted
byte-for-byte.

**Known divergence (separate lane, follow-up — NOT fixed here):**
`Class.getResourceAsStream("greeting.txt")` resolves the class-relative name to
`com/example/duke/ladder/greeting.txt` correctly, but `lookup_class_resource`
(`native/common.rs`) looks it up against the class's runtime loader, which for the app class
resolves to the BOOTSTRAP loader (not the `LaunchedClassLoader` that carries the fat-jar
archive path). The lookup therefore misses `BOOT-INF/classes/...` and returns null; the
fixture tolerates the null (`greeting.txt -> null`) and continues. This is a Class-vs-
ClassLoader resource-loader-association gap, distinct from the `Properties.load` /
character-stream work above.

**Test / pin state.** `spring_boot_ladder_boots_end_to_end` stays `#[ignore]`d (reason updated
to the reflection wall) with a strengthened stable-substring assertion for when the wall clears
(starting banner, loaded props, `Math.sqrt(2809) = 53.0`, reflective `summarize`, both success
banners; the per-run heap-address `Squares ...@NN` line and the `greeting.txt -> null`
divergence are deliberately excluded). The ladder pin test + `LADDER_BLOCKER` const are kept
(re-pinned to the unchanged visible blocker). `APP_BLOCKER`, the app pin, and the app canary
are untouched (the app fixture is still blocked on `EnumSet` / `ExceptionInInitializerError`).
Gate: `cargo test --workspace --no-fail-fast` = 3097 passed / 0 failed / 4 ignored, fmt +
clippy (1.97.0 pedantic+nursery) clean, gson/slf4j/commons-lang3 canaries + both Spring Boot
pins green, HelloWorld class + jar both print `Hello, World!`. No edits to
`registry.rs` / `native/common.rs` / `execution.rs`.
