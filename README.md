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
| `gson` | `gson-2.11.0` | Blocked (`oss_jar_smoke` future-canary is `#[ignore]`) | [findings](docs/findings/2026-07-09-oss-smoke-widening.md) | Loads and begins `Gson().toJson(...)`; current blocker: internal `InvalidRef` from `java/lang/String.format("\u%04x", ...)` inside `com/google/gson/stream/JsonWriter.<clinit>` (building `REPLACEMENT_CHARS`). Inferred next: `java/lang/reflect/Field.getModifiers()I`, `Field.getGenericType()`, then reflective `sun/misc/Unsafe.allocateInstance` on `fromJson` |
| `commons-lang3` | `commons-lang3-3.17.0` | Blocked (`oss_jar_smoke` future-canary is `#[ignore]`) | [findings](docs/findings/2026-07-09-oss-smoke-widening.md) | Loads and enters `StringUtils`; current blocker: `java/util/regex/Pattern.compile("\p{InCombiningDiacriticalMarks}+")` in `StringUtils.<clinit>` throws `PatternSyntaxException` (unsupported Unicode block). Inferred next: `java/lang/reflect/Array.newInstance(Ljava/lang/Class;I)Ljava/lang/Object;` for `ArrayUtils.add(int[],int)` |
