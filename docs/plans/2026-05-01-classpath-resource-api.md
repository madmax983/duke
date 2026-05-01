# Classpath Resource API Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement `Class.getResource*`, `ClassLoader.getResource*`, synthetic `URL.openStream`, resource-backed `InputStream`, and `ClassLoader.getResources` so real classpath resources work across directory and JAR classpaths.

**Architecture:** Add a shared resource-resolution substrate that can return resource bytes plus a stable synthetic URL for both bootstrap and runtime `URLClassLoader` searches. Reuse that substrate for the user-facing natives first, then refactor `ServiceLoader` to dogfood `ClassLoader.getResources` plus `URL.openStream` instead of calling Rust-only service resource helpers directly.

**Tech Stack:** Rust 2024, `duke-loader`, `duke-interpreter`, existing synthetic stdlib/native registry, Java fixture harness in `tests/fixtures`.

---

### Task 1: Resource lookup substrate

**Files:**
- Modify: `crates/duke-loader/src/lib.rs`
- Modify: `crates/duke-loader/src/directory.rs`
- Modify: `crates/duke-loader/src/bootstrap.rs`
- Modify: `crates/duke-loader/src/zip.rs`
- Test: `crates/duke-loader/src/bootstrap.rs`
- Test: `crates/duke-loader/src/directory.rs`

**Step 1: Write the failing tests**

Add loader-level tests that prove:
- directory resources expose a stable `file:` URL
- JAR resources expose a stable `jar:file:` URL
- bootstrap resource enumeration preserves classpath order and returns per-hit URL metadata

**Step 2: Run tests to verify they fail**

Run: `cargo test -p duke-loader resource_url -- --nocapture`
Expected: FAIL because the loader trait does not expose resource URL metadata yet.

**Step 3: Write minimal implementation**

Add a small public resource descriptor in `duke-loader` and trait methods that return resource bytes plus a stable URL string for first-hit and plural-hit lookups. Implement the methods in `DirectoryLoader`, `ZipLoader`, `ClasspathEntry`, and `BootstrapLoader`, preserving existing traversal guards and classpath ordering.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p duke-loader resource_url -- --nocapture`
Expected: PASS.

### Task 2: Interpreter resource resolver and synthetic classes

**Files:**
- Modify: `crates/duke-interpreter/src/registry.rs`
- Modify: `crates/duke-interpreter/src/native.rs`
- Modify: `crates/duke-interpreter/src/stdlib.rs`
- Test: `crates/duke-interpreter/src/tests.rs`

**Step 1: Write the failing tests**

Add native-level tests for:
- resource name resolution from `Class` relative vs absolute names
- traversal rejection returning `null`
- `URL.toString`, `URL.toExternalForm`, and `URL.getPath`
- `ClassLoader.getResources` returning an `Enumeration`

**Step 2: Run tests to verify they fail**

Run: `cargo test -p duke-interpreter resource_resolution -- --nocapture`
Expected: FAIL because the natives and helper plumbing do not exist.

**Step 3: Write minimal implementation**

Add interpreter helpers that:
- resolve class-relative resource names
- search either the bootstrap loader or runtime loader paths
- convert loader resource hits into Java `URL` objects and `Enumeration<URL>`

Register new natives and synthetic helper classes in `bootstrap_stdlib`:
- `java/lang/Class.getResourceAsStream`
- `java/lang/ClassLoader.getResourceAsStream`
- `java/lang/Class.getResource`
- `java/lang/ClassLoader.getResource`
- `java/lang/ClassLoader.getResources`
- `java/net/URL.openStream`
- `java/net/URL.toExternalForm`
- `java/net/URL.getPath`
- synthetic `duke/util/ResourceEnumeration`

**Step 4: Run tests to verify they pass**

Run: `cargo test -p duke-interpreter resource_resolution -- --nocapture`
Expected: PASS.

### Task 3: Resource-backed InputStream

**Files:**
- Modify: `crates/duke-interpreter/src/native.rs`
- Modify: `crates/duke-interpreter/src/stdlib.rs`
- Test: `crates/duke-interpreter/src/tests.rs`

**Step 1: Write the failing tests**

Add tests for a dedicated synthetic resource stream covering:
- `read()`
- `read([B)`
- `read([BII)`
- `available()`
- `skip(J)`
- `close()` and post-close `IOException("Stream closed")`
- bad offset/length throwing `IndexOutOfBoundsException`

**Step 2: Run tests to verify they fail**

Run: `cargo test -p duke-interpreter resource_input_stream -- --nocapture`
Expected: FAIL because the stream class and natives do not exist.

**Step 3: Write minimal implementation**

Add a small synthetic subclass such as `duke/io/ResourceInputStream` backed by host byte-buffer handles plus a closed flag. Reuse the host byte-buffer support already used by ZIP streams where possible, and add the missing slice-read / available / skip / closed-stream behavior in natives.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p duke-interpreter resource_input_stream -- --nocapture`
Expected: PASS.

### Task 4: Fixture-driven Java acceptance tests

**Files:**
- Create: `tests/fixtures/ResourceLoadingTest.java`
- Create: `tests/fixtures/ResourceLoadingTestData.txt`
- Create: `tests/fixtures/resource-loading-jar-src/ResourceLoadingJarTest.java`
- Create: `tests/fixtures/resource-loading-jar-src/META-INF/messages.txt`
- Create or refresh: `tests/fixtures/resource-loading.jar`
- Modify: `crates/duke-interpreter/src/tests.rs`

**Step 1: Write the failing tests**

Add fixture harness tests that execute:
- absolute class resource lookup
- relative class resource lookup
- miss returns `null`
- closed stream throws `IOException`
- traversal returns `null`
- `URL.openStream()` round-trip
- JAR-backed class resource lookup for `META-INF/messages.txt`

**Step 2: Run tests to verify they fail**

Run: `cargo test -p duke-interpreter resource_loading_fixture -- --nocapture`
Expected: FAIL because the Java-visible APIs are still incomplete.

**Step 3: Write minimal implementation**

Add the Java fixture and vendored resource files, then wire targeted Rust tests to run those fixture methods through Duke.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p duke-interpreter resource_loading_fixture -- --nocapture`
Expected: PASS.

### Task 5: ServiceLoader dogfooding and regression coverage

**Files:**
- Modify: `crates/duke-interpreter/src/native.rs`
- Modify: `crates/duke-interpreter/src/tests.rs`
- Test: existing `tests/fixtures/ServiceLoader*.java`

**Step 1: Write the failing tests**

Add a focused regression test that proves `ServiceLoader.load(..., loader)` can still discover providers after routing through `ClassLoader.getResources` instead of the Rust-only service file helper. Add a slf4j-probe regression that verifies missing `simplelogger.properties` returns `null` cleanly instead of surfacing `VmError::Unsupported`.

**Step 2: Run tests to verify they fail**

Run: `cargo test -p duke-interpreter service_loader -- --nocapture`
Expected: FAIL or continue failing until the refactor is complete.

**Step 3: Write minimal implementation**

Refactor ServiceLoader natives to call the Java-level `ClassLoader.getResources`, iterate the resulting `Enumeration<URL>`, and load provider config bytes via `URL.openStream`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p duke-interpreter service_loader -- --nocapture`
Expected: PASS, including existing ServiceLoader fixture tests.

### Task 6: Final verification and hygiene

**Files:**
- Modify: `docs/plans/2026-05-01-classpath-resource-api.md`

**Step 1: Run targeted completion checks**

Run:
- `cargo test -p duke-loader`
- `cargo test -p duke-interpreter resource_ -- --nocapture`
- `cargo test -p duke-interpreter service_loader -- --nocapture`

Expected: PASS.

**Step 2: Scan for half-done work**

Run: `git grep -n "TODO\\|FIXME\\|Stub:" -- crates/duke-loader crates/duke-interpreter tests/fixtures`
Expected: no new issue-663 scaffolding left behind.

**Step 3: Review the diff**

Run: `git diff -- crates/duke-loader crates/duke-interpreter tests/fixtures docs/plans/2026-05-01-classpath-resource-api.md`
Expected: one coherent unit of work with tests and fixtures included.
