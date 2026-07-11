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

```
# app  (duke-spring-boot-app-3.5.12.jar) — new frontier:
duke: runtime error: method not found: ch/qos/logback/classic/util/DefaultJoranConfigurator.getClass()Ljava/lang/Class;

# ladder (duke-spring-boot-ladder-3.5.12.jar) — new frontier:
duke: runtime error: operand stack underflow
```

The **app** frontier is inherited `java/lang/Object.getClass()` virtual dispatch
failing to resolve for a class loaded through the runtime `loadClass` path — an
interpreter method-dispatch / class-identity issue (`execution.rs`), not a missing
ClassLoader native. The **ladder** frontier is an `operand stack underflow` deep
in real commons-logging `LogFactory.getFactory` at
`java/lang/ref/WeakReference.get()Ljava/lang/Object;` (offset 188 → `checkcast` at
191 underflows because `get()` returns no value) — a `java.lang.ref`
reference-object native that is unimplemented. Both are handoffs to other lanes.

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
| **1c** | `method not found: ch/qos/logback/classic/util/DefaultJoranConfigurator.getClass()Ljava/lang/Class;` — app, after 1b cleared. No banner yet. | Inherited `java/lang/Object.getClass()` virtual/interface dispatch fails to resolve for a class loaded through the runtime `loadClass` path — the hierarchy walk does not reach the `java/lang/Object` native (class-identity / method-dispatch). | **OBSERVED — current app pin. HANDOFF** to interpreter method-dispatch / class-identity lane (`execution.rs`). | execution.rs virtual/interface dispatch + class-key identity |
| **1d** | `operand stack underflow` — ladder, after `getSystemResources` cleared. Deep in real commons-logging `LogFactory.getFactory`. | `java/lang/ref/WeakReference.get()Ljava/lang/Object;` returns no value (offset 188), so the following `checkcast` (191) underflows. Reference-object native unimplemented. | **OBSERVED — current ladder pin. HANDOFF** to `java.lang.ref` reference-object native lane. | native / stdlib — `java.lang.ref` (`Reference`/`WeakReference`) |
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

## Lane ownership summary

| Blocker | Owning lane | Status |
| --- | --- | --- |
| #0 provenance-poison on bootstrap singletons | classloader / registry (`registry.rs`) | **DONE (this branch)** |
| #1 `ClassLoader.getSystemResources` missing | synthetic-stdlib `java_lang` — `ClassLoader` resource API (`stdlib.rs`) | **CLEARED (app #1320, ladder this branch)** |
| #1a `ClassLoader.getSystemClassLoader` missing | synthetic-stdlib `java_lang` — `ClassLoader` static/system-loader API (`stdlib.rs`) | **CLEARED (this branch)** |
| #1b `ClassLoader.loadClass` missing | synthetic-stdlib `java_lang` — `ClassLoader` (`stdlib.rs`) | **CLEARED (this branch)** |
| #1c `Object.getClass()` dispatch on loadClass-loaded class | execution.rs virtual/interface dispatch + class-identity | **HANDOFF — current app pin** |
| #1d `WeakReference.get()` underflow in commons-logging | native — `java.lang.ref` reference-object | **HANDOFF — current ladder pin** |
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
  two fixtures diverge, so the pins assert per-fixture constants. As of this
  branch (ClassLoader rungs cleared) they are:
  `APP_BLOCKER = "method not found: ch/qos/logback/classic/util/DefaultJoranConfigurator.getClass()Ljava/lang/Class;"`
  and
  `LADDER_BLOCKER = "operand stack underflow"` (the `WeakReference.get()` underflow
  in commons-logging `LogFactory.getFactory`).
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
- `cargo test --workspace --no-fail-fast` — **3039 passed / 0 failed / 6 ignored.**
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
