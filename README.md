# Duke JVM

Duke is an experimental Java Virtual Machine (JVM) implementation written in Rust. It is designed to be highly modular, focusing on correctness, safety, and a clean codebase.

## Architecture

Duke is structured as a Cargo Workspace containing several specialized crates. This modular architecture enforces separation of concerns and improves maintainability.

### Crates Overview

* **`duke-classfile`**: Parses raw JVM `.class` files according to the JVM Specification (SE 21). It decodes the constant pool, parses class hierarchies, fields, methods, and attributes (such as `Code` and `LineNumberTable`). It provides a robust, panic-free parsing mechanism using bounded cursors.

* **`duke-bytecode`**: Acts on the raw `Code` attributes extracted by `duke-classfile`. It decodes the raw byte streams into typed `Instruction` enums. It also provides structural bytecode verification (ensuring stack depths and local variable boundaries) and utilities for generating Control Flow Graphs (CFGs).

* **`duke-loader`**: Handles loading Java classes and resources from various sources. It supports reading from the filesystem (`DirectoryLoader`), from `MANIFEST.MF` JAR files and ZIP archives (`ZipLoader`), and from the modern Java 9+ `jimage` format (`JImageReader`), all orchestrated by the `BootstrapLoader`.

* **`duke-runtime`**: Provides the foundational building blocks for executing Java methods. It defines the `Frame` (which holds the operand stack and local variables) and the `Slot` (the core unit of data holding integers, floats, or object references).

* **`duke-gc`**: Implements the garbage collector and memory management for the JVM. It handles object allocation, generations (young/old), and memory reclamation strategies.

* **`duke-interpreter`**: The execution engine of the JVM. It orchestrates the flow of the application by interpreting decoded bytecode instructions, managing threads (`threading.rs`), handling class hierarchies (`registry.rs`), and bridging to native JNI-like functions (`native.rs` / `stdlib.rs`).

* **`duke-telemetry`**: A subsystem for gathering runtime metrics. It tracks bytecode execution costs, object lineage (allocation tracking), dispatch resolution performance, and exception flow, providing observability into the VM's behavior.

## Goals

1. **Modularity**: Strict crate boundaries ensure that the classfile parser has no dependency on the execution engine or garbage collector.
2. **Safety**: Written in Rust, it leverages the borrow checker to avoid memory leaks and data races common in C/C++ JVM implementations.
3. **Correctness**: Every module is tested rigorously against edge cases, with a focus on graceful error handling (e.g., returning `Err` rather than panicking on malformed input).

## Real-world JAR compatibility

Duke now vendors a hermetic OSS smoke harness under `tests/fixtures/oss-jars/` so third-party bytecode can run through CI instead of only duke-authored fixtures.

| JAR | Version(s) | Status | Last-tested phase | Notes |
| --- | --- | --- | --- | --- |
| `slf4j-simple` | `slf4j-simple-2.0.13` + `slf4j-api-2.0.13` | Passing (`oss_jar_smoke::slf4j_simple_smoke_runs_real_jar_bytecode` runs end-to-end) | [#1308](https://github.com/madmax983/duke/pull/1308) | Runs unmodified `slf4j-simple` bytecode end-to-end: loads `SimpleLogger`/`SimpleServiceProvider` from classfiles and emits `INFO duke-smoke - hello world`. Final blockers cleared in order: `ArrayList.<init>(I)V`, `StringBuilder.<init>(I)V`, `Thread.getName()Ljava/lang/String;`, `StringBuilder.append(Ljava/lang/CharSequence;II)Ljava/lang/StringBuilder;`, `Class.isArray()Z`, `PrintStream.flush()V` |
| `gson` | `gson-2.11.0` | Passing (`oss_jar_smoke::gson_smoke_runs_real_jar_bytecode` runs end-to-end) | [#1318](https://github.com/madmax983/duke/pull/1318) | Runs unmodified `gson` bytecode end-to-end: `Gson().toJson(new Pojo(7, "duke"))` then `gson.fromJson(...)` round-trips `{"count":7,"name":"duke"} -> count=7 name=duke`. Root cause was a GC nested-`<clinit>` stale-root bug (fixed in [#1318](https://github.com/madmax983/duke/pull/1318), branch `swarm/gc-nested-clinit-roots`); the 21-blocker capability chain cleared: `Type` marker interface, `Class` reflection natives, `Field.getModifiers`/`getGenericType`, `Unsafe.allocateInstance`, boxed numerics extend `Number`, and misc `String`/`StringBuffer`/`Map` natives. Known limitations: `Class.getModifiers`/`isInterface` report ACC_PUBLIC-only and `Field.getModifiers` omits transient/final (see [findings](docs/findings/2026-07-10-gson-canary-green.md)) |
| `commons-lang3` | `commons-lang3-3.17.0` | Passing (`oss_jar_smoke::commons_lang3_smoke_runs_real_jar_bytecode` runs end-to-end) | [findings](docs/findings/2026-07-09-oss-smoke-widening.md) | Runs unmodified `commons-lang3` bytecode end-to-end: `StringUtils.join`, `StringUtils.capitalize`, `ArrayUtils.add`/`contains`/`toObject` all execute, emitting `commons-lang3: join=duke-jvm-smoke capitalize=Hello added=1,2,3,4 contains4=true`. Blockers cleared in order: `Character.toTitleCase(I)I`/`(C)C`, `Character.charCount(I)I`, `String.<init>([III)V` (code-point ctor), `java/lang/reflect/Array.newInstance(Class,I)`/`getLength(Object)I`/`set(Object,I,Object)V`, `Class.getComponentType()Ljava/lang/Class;`, and `Arrays.setAll(Object[],IntFunction)V`; `Objects.toString(Object)` now unboxes wrapper types via `heap_object_to_string` |
| Spring Boot ladder (commons-logging) | `duke-spring-boot-ladder-3.5.12` | Passing (`spring_boot_real_app::spring_boot_ladder_boots_end_to_end` canary now un-ignored and runs end-to-end) | [findings](docs/findings/2026-07-10-spring-boot-real-app.md) | Runs a real Spring Boot fat JAR via `duke -jar`: the `JarLauncher` boots the whole commons-logging "ladder" (JCL `LogFactory`/`LogFactoryImpl` → `Jdk14Logger`) and reflectively invokes `LadderApplication.main`, which clears property load (`Properties.load` over a `ResourceInputStream`), classpath resource scan, the `BufferedReader(InputStreamReader(...))` character-stream read, the public reflective `Math.sqrt`, and — as of the same-class-reflection lane — main's reflective same-class private `summarize` invoke: `LadderApplication.class.getDeclaredMethod("summarize", List.class).invoke(null, …)` without `setAccessible` now succeeds because Duke's `Method.invoke`/`Constructor.newInstance` access check is caller-sensitive (JLS 6.6): `native_needs_stack_snapshot` captures the invoking frame so the natives can apply the JVM rule that a class may always reflectively access its OWN non-public members, while cross-class private invoke still throws `IllegalAccessException` (nest-mates not yet modelled). The ladder now prints `Reflective summarize() -> LOGGING,RESOURCE-SCAN,REFLECTION,STREAMS` and `Duke ladder fixture completed all rungs successfully.` Rung chain cleared across PRs [#1347](https://github.com/madmax983/duke/pull/1347)/[#1354](https://github.com/madmax983/duke/pull/1354)/[#1355](https://github.com/madmax983/duke/pull/1355)/[#1353](https://github.com/madmax983/duke/pull/1353): inherited-static super-walk on invokestatic, loader-key `instanceof`/`checkcast` fix, `Serializable`/`Logger.logp`/Thread context-loader persistence, `Properties.load(InputStream)` reading a `ResourceInputStream`, and synthetic `InputStreamReader`/`BufferedReader`. Known divergence: `Class.getResourceAsStream("greeting.txt")` resolves against the bootstrap loader (not the `LaunchedClassLoader`) and returns null; the fixture tolerates it |
