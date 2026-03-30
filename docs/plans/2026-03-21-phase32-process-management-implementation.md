# Phase 32 Process Management Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add minimal but real `java.lang.ProcessBuilder` / `java.lang.Runtime.exec` support to Duke so fixture programs can spawn host processes, pipe stdin/stdout/stderr, wait for completion, observe exit codes, and destroy child processes safely.

**Architecture:** Reuse Duke's existing host-handle boundary in `duke-gc` instead of smuggling OS process state into heap objects. Extend `HostFileHandle` with child-process and child-pipe variants, add heap helpers for spawning/waiting/killing processes, and bootstrap a synthetic process stdlib slice in `crates/duke-interpreter/src/lib.rs`: `java/lang/Process`, `java/lang/ProcessImpl`, `java/lang/ProcessBuilder`, `java/lang/Runtime`, plus lightweight Duke-private pipe stream wrappers. Keep the Java-side surface narrow and native-backed: `ProcessBuilder` stores command + working directory, `ProcessImpl` stores opaque host ids, and `Runtime.exec(...)` delegates into the same spawn helper path. Drive everything from checked-in Java fixtures first, using `java` itself as the spawned child so tests stay cross-platform on Windows and Linux.

**Tech Stack:** Rust 2024, `std::process::{Child, Command, Stdio}`, `duke-interpreter`, `duke-gc`, `duke-runtime`, checked-in Java fixtures under `tests/fixtures`, `javac --release 21`, `javap`, cargo test, cargo clippy.

---

## Background Notes

- `java/io/File`, `java/io/InputStream`, `java/io/OutputStream`, `FileInputStream`, and `FileOutputStream` already exist as synthetic/native-backed classes. Process pipe wrappers should mirror the socket/zip stream pattern rather than inventing a new I/O path.
- `execute_class_to_completion` already supports long-running top-level work and Java thread coordination. Phase 32 does not need a new scheduler primitive unless process waiting proves impossible through the current native boundary.
- The acceptance criteria call out `ProcessImpl.start` "or equivalent." For Duke's synthetic stdlib, the equivalent bridge will be `ProcessBuilder.start()` plus `Runtime.exec(...)` delegating to the same Rust spawn helper.
- Cross-platform testing matters here. Avoid shell builtins such as `echo`, `cmd /c`, or `sh -c`; use `java -cp ... ProcessChildMain ...` so the fixture behaves the same on CI Linux and local Windows.

### Task 1: Red Fixture For Process Spawn, Pipes, Wait, Destroy, And Exec Failure

**Files:**
- Create: `tests/fixtures/ProcessManagementTest.java`
- Create: `tests/fixtures/ProcessManagementTest.class`
- Create: `tests/fixtures/ProcessChildMain.class`
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Write the fixture**

Create `tests/fixtures/ProcessManagementTest.java` with:

- a public test class exposing only static entry methods Duke will invoke
- a package-private `ProcessChildMain` class in the same source file with a `public static void main(String[] args)` entry point for the spawned host JVM

Recommended fixture coverage:

```java
import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;

public final class ProcessManagementTest {
    public static int spawnAndReadStdout(String javaCmd, String classpath, String workDir) throws Exception { ... }
    public static int runtimeExecReadsStdout(String javaCmd, String classpath, String workDir) throws Exception { ... }
    public static int pipeStdinToChild(String javaCmd, String classpath, String workDir) throws Exception { ... }
    public static int readErrorStream(String javaCmd, String classpath, String workDir) throws Exception { ... }
    public static int destroySleepingChild(String javaCmd, String classpath, String workDir) throws Exception { ... }
    public static int missingExecutableRaisesIoException() { ... }
}

final class ProcessChildMain {
    public static void main(String[] args) throws Exception {
        switch (args[0]) {
            case "stdout" -> System.out.print("duke-out");
            case "stderr" -> System.err.print("duke-err");
            case "stdin-echo" -> { /* read bytes from stdin, print transformed output */ }
            case "sleep" -> { /* sleep long enough for destroy() test */ }
            default -> System.exit(9);
        }
    }
}
```

Fixture rules:

- `spawnAndReadStdout(...)` should use `new ProcessBuilder(new String[] { ... }).directory(new File(workDir)).start()`.
- `runtimeExecReadsStdout(...)` should use `Runtime.getRuntime().exec(String[], null, new File(workDir))`.
- `pipeStdinToChild(...)` must write bytes through `Process.getOutputStream()`, close the stream, read `getInputStream()`, then `waitFor()`.
- `readErrorStream(...)` must read `Process.getErrorStream()` and verify `waitFor() == 0`.
- `destroySleepingChild(...)` must start a long-enough sleeper, call `destroy()`, then `waitFor()`, and accept either a non-zero exit code or an `IllegalThreadStateException`-free terminated child depending on platform.
- `missingExecutableRaisesIoException()` must return `1` only when `IOException` is thrown for a clearly fake executable name.

**Step 2: Compile and inspect the fixture**

Run:

`javac --release 21 tests/fixtures/ProcessManagementTest.java -d tests/fixtures/`

Then inspect:

`javap -classpath tests/fixtures -c ProcessManagementTest`

Expected:

- `tests/fixtures/ProcessManagementTest.class` and `tests/fixtures/ProcessChildMain.class` are created
- bytecode confirms the actual descriptors Duke must support, especially:
  - `java/lang/ProcessBuilder.<init>([Ljava/lang/String;)V`
  - `java/lang/ProcessBuilder.directory(Ljava/io/File;)Ljava/lang/ProcessBuilder;`
  - `java/lang/ProcessBuilder.start()Ljava/lang/Process;`
  - `java/lang/Runtime.getRuntime()Ljava/lang/Runtime;`
  - `java/lang/Runtime.exec([Ljava/lang/String;[Ljava/lang/String;Ljava/io/File;)Ljava/lang/Process;`
  - `java/lang/Process.getInputStream()Ljava/io/InputStream;`
  - `java/lang/Process.getErrorStream()Ljava/io/InputStream;`
  - `java/lang/Process.getOutputStream()Ljava/io/OutputStream;`
  - `java/lang/Process.waitFor()I`
  - `java/lang/Process.destroy()V`

If the compiler emits a different overload than expected, adjust the fixture first rather than overbuilding the stdlib surface.

**Step 3: Add failing interpreter tests**

In `crates/duke-interpreter/src/lib.rs`, add Phase 32 helpers and red tests near the other fixture-backed integration blocks.

Add a helper shaped like:

```rust
fn run_bootstrap_with_string_args(
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[String],
) -> VmResult<Option<Slot>>
```

Reuse the existing helper if it already matches.

Add tests:

- `process_spawn_and_read_stdout_returns_expected_sum`
- `process_runtime_exec_reads_stdout`
- `process_pipe_stdin_to_child_round_trips`
- `process_read_error_stream_returns_expected_sum`
- `process_destroy_terminates_sleeping_child`
- `process_missing_executable_raises_io_exception`

Use real host arguments:

- `java` executable from `PATH`
- fixture classpath pointing at `tests/fixtures`
- working directory pointing at the repository root

For assertions, prefer fixture methods that return small ints rather than depending on captured Duke stdout.

**Step 4: Run the targeted tests to verify RED**

Run:

`cargo test -p duke-interpreter process_ -- --nocapture`

Expected: FAIL because `java/lang/ProcessBuilder`, `java/lang/Process`, `java/lang/Runtime`, and the spawn/wait/destroy natives do not exist yet.

**Step 5: Commit the red fixture/tests**

```bash
git add tests/fixtures/ProcessManagementTest.java tests/fixtures/ProcessManagementTest.class tests/fixtures/ProcessChildMain.class crates/duke-interpreter/src/lib.rs
git commit -m "test: add red process management fixtures"
```

### Task 2: Add Host Process And Pipe Runtime Support

**Files:**
- Modify: `crates/duke-gc/src/lib.rs`

**Step 1: Extend the host handle enum**

Add host-side variants for process ownership and pipes, for example:

```rust
pub enum HostFileHandle {
    ...
    Process(HostProcessHandle),
    ProcessStdout(std::process::ChildStdout),
    ProcessStderr(std::process::ChildStderr),
    ProcessStdin(std::process::ChildStdin),
}

pub struct HostProcessHandle {
    child: std::process::Child,
    exit_code: Option<i32>,
}
```

Store the `Child` itself only once so `waitFor`, `exitValue`, and `destroy` all refer to one host process record.

**Step 2: Add heap helpers**

Add the smallest host API Duke needs:

- `spawn_host_process(command: &[String], cwd: Option<&Path>) -> VmResult<SpawnedProcessIds>`
- `wait_host_process(id: i32) -> VmResult<i32>`
- `host_process_exit_value(id: i32) -> VmResult<i32>`
- `destroy_host_process(id: i32) -> VmResult<()>`

Where `SpawnedProcessIds` includes:

- process id
- stdin handle id
- stdout handle id
- stderr handle id

Implementation rules:

- reject empty command arrays with `java/io/IOException`
- use `Command::new(&command[0])` plus `.args(&command[1..])`
- map working directory if provided
- default all three child stdio streams to `Stdio::piped()`
- map spawn failures to `VmError::JavaException { class_name: "java/io/IOException".into() }`
- cache the exit code after the first successful wait so repeated `waitFor()` / `exitValue()` are stable
- treat `destroy()` as idempotent once the process has already exited

**Step 3: Reuse the existing byte-oriented I/O boundary**

Update:

- `read_host_file_byte` to accept `ProcessStdout` and `ProcessStderr`
- `write_host_file_byte` to accept `ProcessStdin`
- `close_host_file` to drop process pipe handles cleanly without touching the owning child process record

**Step 4: Add small heap-level tests if useful**

If `duke-gc` already has host-handle unit tests nearby, add one focused test for spawning `java` and reading one byte from stdout. Keep it small; the main behavior proof stays in the interpreter fixture tests.

**Step 5: Run the smallest relevant verification**

Run:

`cargo test -p duke-gc host_process -- --nocapture`

If no focused unit test is added, use:

`cargo test -p duke-interpreter process_ -- --nocapture`

Expected: compile should advance, but the interpreter integration tests will still fail until the stdlib/natives exist.

**Step 6: Commit**

```bash
git add crates/duke-gc/src/lib.rs
git commit -m "refactor(gc): add host process handle support"
```

### Task 3: Bootstrap Synthetic Process Stdlib And Spawn Natives

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Add synthetic classes**

Extend `bootstrap_stdlib` with:

- `java/lang/Process`
- `java/lang/ProcessImpl`
- `java/lang/ProcessBuilder`
- `java/lang/Runtime`
- `duke/process/ProcessInputStream`
- `duke/process/ProcessErrorStream`
- `duke/process/ProcessOutputStream`

Recommended field layouts:

- `java/lang/ProcessImpl`
  - `fields[0]` = process id `int`
  - `fields[1]` = stdin id `int`
  - `fields[2]` = stdout id `int`
  - `fields[3]` = stderr id `int`
- `java/lang/ProcessBuilder`
  - `fields[0]` = command array ref
  - `fields[1]` = directory `File` ref or `null`
- `java/lang/Runtime`
  - no instance fields; bootstrap one singleton object into a static field if needed
- pipe stream wrappers
  - `fields[0]` = opaque host handle id `int`

`ProcessImpl` should extend `Process`. The stream wrappers should extend `InputStream` / `OutputStream` so the existing virtual native lookup logic works.

**Step 2: Register minimal natives**

Register:

- `java/lang/ProcessBuilder.<init>([Ljava/lang/String;)V`
- `java/lang/ProcessBuilder.directory(Ljava/io/File;)Ljava/lang/ProcessBuilder;`
- `java/lang/ProcessBuilder.start()Ljava/lang/Process;`
- `java/lang/Runtime.getRuntime()Ljava/lang/Runtime;`
- `java/lang/Runtime.exec([Ljava/lang/String;)Ljava/lang/Process;`
- `java/lang/Runtime.exec([Ljava/lang/String;[Ljava/lang/String;Ljava/io/File;)Ljava/lang/Process;`
- `java/lang/ProcessImpl.getInputStream()Ljava/io/InputStream;`
- `java/lang/ProcessImpl.getErrorStream()Ljava/io/InputStream;`
- `java/lang/ProcessImpl.getOutputStream()Ljava/io/OutputStream;`
- `java/lang/ProcessImpl.waitFor()I`
- `java/lang/ProcessImpl.exitValue()I`
- `java/lang/ProcessImpl.destroy()V`
- `duke/process/ProcessInputStream.read()I`
- `duke/process/ProcessInputStream.read([B)I`
- `duke/process/ProcessInputStream.close()V`
- `duke/process/ProcessErrorStream.read()I`
- `duke/process/ProcessErrorStream.read([B)I`
- `duke/process/ProcessErrorStream.close()V`
- `duke/process/ProcessOutputStream.write(I)V`
- `duke/process/ProcessOutputStream.write([B)V`
- `duke/process/ProcessOutputStream.close()V`

Reuse the existing file stream byte-read / byte-write natives where practical instead of copying logic.

**Step 3: Implement spawn helpers**

Add interpreter-side helpers for:

- extracting `String[]` command arrays into `Vec<String>`
- extracting a `java/io/File` path from a possibly-null directory object
- allocating a `java/lang/ProcessImpl` from returned host ids
- allocating pipe wrapper objects lazily for `getInputStream` / `getErrorStream` / `getOutputStream`

`Runtime.exec(...)` should delegate to the same spawn helper as `ProcessBuilder.start()`.

Do not implement environment mapping in this phase unless the fixture bytecode forces it; accept `null` environment arrays and ignore them.

**Step 4: Run the process tests to verify GREEN for spawn + pipes**

Run:

`cargo test -p duke-interpreter process_ -- --nocapture`

Expected: the Phase 32 tests pass or narrow down to `waitFor`, `exitValue`, or `destroy` semantics only.

**Step 5: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): add process spawn and pipe natives"
```

### Task 4: Green Phase For Wait, Exit Value, Destroy, And Failure Semantics

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`
- Modify: `crates/duke-gc/src/lib.rs`

**Step 1: Finish process lifecycle semantics**

Ensure:

- `waitFor()` blocks until termination and returns the cached exit code
- `exitValue()` returns the exit code if the child is already dead
- `exitValue()` throws `IllegalThreadStateException` if the child is still running
- `destroy()` kills the child if alive, leaves already-dead children alone, and does not panic if called twice

If `IllegalThreadStateException` is not already bootstrapped, add it under `java/lang`.

**Step 2: Confirm I/O failure behavior**

Spawn failures, invalid process ids, or closed pipe misuse should surface as:

- `java/io/IOException` for OS/process boundary failures
- `NullPointerException` for null Java object misuse
- `IllegalThreadStateException` only for `exitValue()` on a live child

Avoid inventing bespoke VM-only errors for user-observable Java API behavior.

**Step 3: Re-run targeted tests**

Run:

- `cargo test -p duke-interpreter process_ -- --nocapture`
- `cargo test -p duke-interpreter file_io_ -- --nocapture`
- `cargo test -p duke-interpreter net_ -- --nocapture`

Expected: Phase 32 passes, and the reused stream machinery does not regress file/socket behavior.

**Step 4: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs crates/duke-gc/src/lib.rs
git commit -m "fix(interpreter): finish process lifecycle semantics"
```

### Task 5: Refactor, Lint, And Close The Loop

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`
- Modify: `crates/duke-gc/src/lib.rs`
- Modify: `docs/plans/vantage-spec-phase32-process-management.md` if completion notes are kept inline elsewhere

**Step 1: Refactor duplicated helpers**

Extract or tighten helpers for:

- reading heap strings and file paths
- reading opaque handle ids from wrapper objects
- allocating stream wrapper objects
- turning `String[]` heap arrays into Rust vectors

Keep the helper surface local to the interpreter unless it clearly belongs in `duke-gc`.

**Step 2: Scan for nearby TODOs / stubs**

Run:

`rg -n "TODO|FIXME|Stub:" crates/duke-interpreter/src crates/duke-gc/src`

If the process-management area introduced or exposed adjacent stubs, finish them now instead of shipping a half-built API.

**Step 3: Run full verification for the touched surface**

Run:

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -W clippy::pedantic -W clippy::nursery`
- `cargo test -p duke-interpreter process_ -- --nocapture`
- `cargo test -p duke-interpreter file_io_ -- --nocapture`
- `cargo test -p duke-interpreter threading_ -- --nocapture`
- `cargo test -p duke-interpreter reflection_ -- --nocapture`
- `cargo test -p duke-interpreter net_ -- --nocapture`

Expected: all commands pass.

**Step 4: Final commit**

```bash
git add docs/plans/2026-03-21-phase32-process-management-implementation.md crates/duke-gc/src/lib.rs crates/duke-interpreter/src/lib.rs tests/fixtures/ProcessManagementTest.java tests/fixtures/ProcessManagementTest.class tests/fixtures/ProcessChildMain.class
git commit -m "feat(interpreter): implement phase 32 process management"
```
