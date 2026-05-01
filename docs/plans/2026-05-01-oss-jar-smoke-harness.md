# OSS JAR Smoke Harness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a hermetic real-world OSS JAR smoke harness for `slf4j-simple` so Duke can execute third-party bytecode end-to-end and report the first discovered runtime gap clearly.

**Architecture:** Reuse Duke's existing JAR/classpath loading path from `duke-interpreter` tests, vendor the two `slf4j` jars plus a tiny Java entrypoint fixture, and add one focused integration test that runs `main(String[])` with a composite classpath. Keep the harness generic enough to extend to later JARs without inventing a second loader stack.

**Tech Stack:** Rust workspace tests, `duke-interpreter`, vendored Maven JARs, Java 21 fixture compilation, SHA-256 metadata in Markdown.

---

### Task 1: Vendor OSS fixture assets

**Files:**
- Create: `tests/fixtures/oss-jars/LICENSES.md`
- Create: `tests/fixtures/oss-jars/slf4j-api-2.0.13.jar`
- Create: `tests/fixtures/oss-jars/slf4j-simple-2.0.13.jar`
- Create: `tests/fixtures/Slf4jSimpleSmoke.java`
- Create: `tests/fixtures/Slf4jSimpleSmoke.class`

**Step 1: Write the fixture source**

Create `Slf4jSimpleSmoke.java` that calls `LoggerFactory.getLogger("duke-smoke").info("hello {}", "world");`.

**Step 2: Compile the fixture**

Run: `javac -cp tests/fixtures/oss-jars/slf4j-api-2.0.13.jar -d tests/fixtures tests/fixtures/Slf4jSimpleSmoke.java`

Expected: `Slf4jSimpleSmoke.class` appears under `tests/fixtures/`.

**Step 3: Record fixture metadata**

Add `LICENSES.md` with each JAR name, version, upstream URL, SPDX-compatible license, and SHA-256 digest.

### Task 2: Add the failing OSS smoke test

**Files:**
- Create: `crates/duke-interpreter/tests/oss_jar_smoke.rs`

**Step 1: Write the failing test**

Add a test that:
- builds a loader spanning `tests/fixtures/`, `tests/fixtures/oss-jars/slf4j-api-2.0.13.jar`, and `tests/fixtures/oss-jars/slf4j-simple-2.0.13.jar`
- loads `Slf4jSimpleSmoke`
- runs `main([Ljava/lang/String;)V`
- asserts the combined captured output contains `INFO duke-smoke - hello world`
- asserts loaded class provenance includes `org/slf4j/simple/SimpleLogger` from the vendored JAR, not a synthetic class

**Step 2: Run the focused test to verify red**

Run: `cargo test -p duke-interpreter --test oss_jar_smoke -- --nocapture`

Expected: the test fails with a concrete Duke runtime gap or missing output, not a compilation/setup error.

### Task 3: Green the harness or make the gap explicit

**Files:**
- Modify: `crates/duke-interpreter/tests/oss_jar_smoke.rs`
- Modify: supporting interpreter files only if the harness itself needs a small internal helper

**Step 1: Fix harness issues only**

Implement the minimal Rust-side support needed for the smoke test to execute Duke's existing classpath/runtime path correctly.

**Step 2: Decide pass vs ignore based on real runtime**

If Duke executes the log call successfully, keep the test enabled and assert the log output.

If Duke hits a real unsupported native/opcode, change the test to `#[ignore]` with a comment naming the first missing capability in execution order.

**Step 3: Preserve diagnostic quality**

Ensure the failure path exposes the missing native/opcode from Duke's existing error text rather than an opaque panic.

### Task 4: Document compatibility and verify

**Files:**
- Modify: `README.md`

**Step 1: Add compatibility table**

Add a `Real-world JAR compatibility` section with the first `slf4j` entry, version pins, status, and the Duke phase/issue reference.

**Step 2: Run focused verification**

Run:
- `cargo test -p duke-interpreter --test oss_jar_smoke -- --nocapture`
- `cargo test --workspace`
- `cargo fmt --all`

Expected: commands reflect the real state of the harness and repo after the change.
