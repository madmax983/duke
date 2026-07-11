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
duke: runtime error: class not found: org/apache/logging/log4j/MarkerManager
```

The **app** frontier is now logback's `CachingDateFormatter` calling
`DateTimeFormatter.ofPattern(String)` — the synthetic `DateTimeFormatter` carries
only the ISO constant instances, not the `ofPattern` factory (java.time formatter
lane; handoff to a future wave). The **ladder** frontier is a missing
`org/apache/logging/log4j/MarkerManager` reached while initializing
`Log4jApiLogFactory` (its `<clinit>` calls `MarkerManager.getMarker`): a real JVM
raises a **catchable `NoClassDefFoundError`** (a `LinkageError`) that
commons-logging catches and falls through on. Duke has no synthetic
`NoClassDefFoundError`/`LinkageError` and surfaces class-resolution failures during
bytecode execution as fatal errors — a distinct interpreter class-resolution /
linkage-error rung **deferred to a future wave** (do not conflate with the missing
class itself). Both are handoffs to other lanes.

An intermediate frontier this wave (after the `WeakReference` rung, before the
`Class.forName` probe fix) was `class not found: org/apache/logging/slf4j/SLF4JProvider`
on the ladder; the lazy-key `Class.forName` fix turned that availability probe
catchable and advanced the ladder to the `MarkerManager` frontier above.

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
| **1f′** | `class not found: org/apache/logging/log4j/MarkerManager` — ladder, after 1f cleared. `Log4jApiLogFactory.<clinit>` calls `MarkerManager.getMarker`. | A real JVM raises a catchable `NoClassDefFoundError` (`LinkageError`) here that commons-logging catches; duke has no synthetic `NoClassDefFoundError`/`LinkageError` and surfaces class-resolution failures during execution as fatal errors. | **OBSERVED — current ladder pin. HANDOFF** to interpreter class-resolution / linkage-error lane (deferred to future wave). | interpreter — class-resolution / `LinkageError` |
| **2** | `--real-jdk` probe: getfield slot 10 out of bounds on a `java/net/URL` allocated with 1 slot where real `URL` bytecode expects 13 (`crates/duke-interpreter/src/execution.rs:1600`). | Synthetic-vs-real **object-layout coherence** boundary: `java/net/URL` is allocated synthetically (1 slot) but real `URL` bytecode indexes its full 13-field layout. | **INFERRED** (seen only under `--real-jdk`, past blocker #1) | native / stdlib object-layout — `java_net` (`URL`) |
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
New ladder frontier: `class not found: org/apache/logging/log4j/MarkerManager`,
reached while initializing `Log4jApiLogFactory` (its `<clinit>` calls
`MarkerManager.getMarker`). A real JVM raises a **catchable `NoClassDefFoundError`**
(a `LinkageError`) that commons-logging catches and falls through on; duke has no
synthetic `NoClassDefFoundError`/`LinkageError` and surfaces class-resolution
failures during bytecode execution as fatal errors. That is a distinct
interpreter class-resolution / linkage-error rung (handoff).

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
| #1e″ `method not found: DateTimeFormatter.ofPattern(String)` | native / stdlib — `java_time` (`DateTimeFormatter` factory) | **HANDOFF — current app pin (deferred to future wave)** |
| #1f `class not found: org/apache/logging/slf4j/SLF4JProvider` | class-loader — `Class.forName` availability probe (`java_lang`) | **CLEARED (this branch)** |
| #1f′ `class not found: org/apache/logging/log4j/MarkerManager` | interpreter — class-resolution / `NoClassDefFoundError` linkage during `<clinit>` (`execution.rs` + synthetic `LinkageError` in `stdlib.rs`) | **HANDOFF — current ladder pin (deferred to future wave)** |
| #2 `java/net/URL` layout coherence | native / stdlib object-layout — `java_net` (`URL`) | inferred |
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
  this wave (all eight rungs above cleared) they are:
  `APP_BLOCKER = "method not found: java/time/format/DateTimeFormatter.ofPattern(Ljava/lang/String;)Ljava/time/format/DateTimeFormatter;"`
  and
  `LADDER_BLOCKER = "class not found: org/apache/logging/log4j/MarkerManager"`.
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
