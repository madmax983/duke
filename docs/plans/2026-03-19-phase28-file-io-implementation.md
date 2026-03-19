# Phase 28 File I/O Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add synthetic `java.io.File`, `FileInputStream`, `FileOutputStream`, and `FileDescriptor` support to Duke so fixture programs can query file metadata, read bytes, write bytes, and close streams correctly with try-with-resources.

**Architecture:** Keep this slice inside `crates/duke-interpreter/src/lib.rs` by extending `bootstrap_stdlib` with synthetic `java/io/*` classes plus native methods, and add a small interpreter-owned host file table keyed by heap object refs rather than storing OS handles directly in heap fields. Reuse existing `VmError::JavaException { class_name }` for `java/io/IOException` and `java/io/FileNotFoundException`, and drive the work with fixture-backed integration tests first.

**Tech Stack:** Rust 2024, `duke-interpreter`, `duke-gc`, `duke-runtime`, checked-in Java fixtures under `tests/fixtures`, `javac --release 21`, cargo test.

---

### Task 1: Red Phase For File Metadata

**Owner:** Implementer agent 1

**Files:**
- Create: `tests/fixtures/FileIoTest.java`
- Create: `tests/fixtures/FileIoTest.class`
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Write the failing fixture**

Create `tests/fixtures/FileIoTest.java` with small static methods that only cover metadata first:

```java
import java.io.File;

public class FileIoTest {
    public static int fileExists(String path) {
        return new File(path).exists() ? 1 : 0;
    }

    public static int fileKind(String path) {
        File f = new File(path);
        if (f.isFile()) return 1;
        if (f.isDirectory()) return 2;
        return 0;
    }
}
```

**Step 2: Compile the fixture**

Run: `javac --release 21 tests/fixtures/FileIoTest.java -d tests/fixtures/`

Expected: `tests/fixtures/FileIoTest.class` created with no errors.

**Step 3: Write failing Rust integration tests**

Add tests near the Phase 27 block in `crates/duke-interpreter/src/lib.rs` that:
- create a temp file
- create a temp directory
- execute `FileIoTest.fileExists`
- execute `FileIoTest.fileKind`
- assert current Duke fails because `java/io/File` is missing or unsupported

Prefer helper shape:

```rust
fn run_bootstrap_int_with_args(
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: Vec<Slot>,
) -> i32
```

Use heap-allocated Java strings for path arguments.

**Step 4: Run the metadata tests to verify RED**

Run: `cargo test -p duke-interpreter file_io_metadata -- --nocapture`

Expected: FAIL because `java/io/File` is not yet bootstrapped or its natives are missing.

**Step 5: Commit the red test changes**

```bash
git add tests/fixtures/FileIoTest.java tests/fixtures/FileIoTest.class crates/duke-interpreter/src/lib.rs
git commit -m "test: add red tests for file metadata support"
```

### Task 2: Green Phase For File Metadata

**Owner:** Implementer agent 2

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Add synthetic stdlib class definitions**

Extend `bootstrap_stdlib` with:
- `java/io/File`
- `java/io/FileDescriptor`
- `java/io/IOException`
- `java/io/FileNotFoundException`

Recommended field layout:
- `java/io/File`: fields[0] = path `String`
- `java/io/FileDescriptor`: fields[0] = opaque id `int`, fields[1] = closed flag `int`

**Step 2: Register minimal natives for metadata**

Add natives:
- `java/io/File.<init>(Ljava/lang/String;)V`
- `java/io/File.exists()Z`
- `java/io/File.isFile()Z`
- `java/io/File.isDirectory()Z`

Implementation rules:
- extract the path string from `this.fields[0]`
- call host `std::fs::metadata`
- return `false` on not found
- return `VmError::JavaException { class_name: "java/io/IOException".to_string() }` on other host failures

**Step 3: Run metadata tests to verify GREEN**

Run: `cargo test -p duke-interpreter file_io_metadata -- --nocapture`

Expected: PASS.

**Step 4: Refactor metadata helpers**

Extract helpers for:
- reading Java path strings from heap refs
- building Java exception class names
- checking the `this` reference shape

Do not add stream behavior yet.

**Step 5: Run the same tests again**

Run: `cargo test -p duke-interpreter file_io_metadata -- --nocapture`

Expected: still PASS.

**Step 6: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat: add java.io.File metadata natives"
```

### Task 3: Red Phase For Stream Read/Write With Try-With-Resources

**Owner:** Implementer agent 3

**Files:**
- Modify: `tests/fixtures/FileIoTest.java`
- Create: `tests/fixtures/FileIoTest.class`
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Extend the fixture with read/write scenarios**

Append methods to `tests/fixtures/FileIoTest.java`:

```java
import java.io.FileInputStream;
import java.io.FileOutputStream;

public static int readAllAndSum(String path) throws Exception {
    int sum = 0;
    try (FileInputStream in = new FileInputStream(path)) {
        int b;
        while ((b = in.read()) != -1) {
            sum += b;
        }
    }
    return sum;
}

public static int copyAndCount(String from, String to) throws Exception {
    byte[] buf = new byte[64];
    int written = 0;
    try (FileInputStream in = new FileInputStream(from);
         FileOutputStream out = new FileOutputStream(to)) {
        int n = in.read(buf);
        out.write(buf, 0, n);
        written = n;
    }
    return written;
}
```

If `write(byte[], int, int)` compiles to unsupported overloads, inspect bytecode and adapt the fixture to the smallest supported path. The first target is single-byte read/write and whole-array read/write, not the full JDK surface.

**Step 2: Recompile fixture**

Run: `javac --release 21 tests/fixtures/FileIoTest.java -d tests/fixtures/`

Expected: updated class file produced.

**Step 3: Add failing Rust tests**

Add tests that:
- create a temp input file with known bytes
- run `readAllAndSum`
- run a write/copy method
- verify output file contents on the host
- verify try-with-resources closes both streams by attempting host reopen after Duke returns

Use clear test names:
- `file_io_read_single_bytes_sums_contents`
- `file_io_copy_via_try_with_resources_writes_expected_bytes`

**Step 4: Run targeted tests to verify RED**

Run: `cargo test -p duke-interpreter file_io_ -- --nocapture`

Expected: FAIL because `FileInputStream` and `FileOutputStream` natives are missing.

**Step 5: Commit**

```bash
git add tests/fixtures/FileIoTest.java tests/fixtures/FileIoTest.class crates/duke-interpreter/src/lib.rs
git commit -m "test: add red tests for file streams"
```

### Task 4: Green Phase For Stream Read/Write

**Owner:** Implementer agent 4

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Add interpreter-owned host file table**

Inside `crates/duke-interpreter/src/lib.rs`, introduce a small state carrier for host handles, for example:

```rust
#[derive(Debug, Default)]
struct HostFileTable {
    next_id: i32,
    readers: HashMap<i32, std::fs::File>,
    writers: HashMap<i32, std::fs::File>,
}
```

Thread this state through native callbacks by attaching it to the interpreter execution scope in the least invasive way possible. Preferred approach:
- keep `execute_class` signature stable if feasible
- add a local state object in `execute_class`
- allow native callback closures to capture or access it through a narrow helper layer

If that is too invasive, add a `native_state` field to `ClassRegistry` or another interpreter-owned struct, but keep the state clearly interpreter-owned, not heap-owned.

**Step 2: Add synthetic stream classes and natives**

Bootstrap:
- `java/io/FileInputStream`
- `java/io/FileOutputStream`

Recommended field layout:
- stream `fields[0]` = `FileDescriptor` ref
- stream `fields[1]` = path `String` ref or reserved slot if needed

Implement natives:
- `FileInputStream.<init>(Ljava/lang/String;)V`
- `FileInputStream.read()I`
- `FileInputStream.read([B)I`
- `FileInputStream.close()V`
- `FileOutputStream.<init>(Ljava/lang/String;)V`
- `FileOutputStream.write(I)V`
- `FileOutputStream.write([B)V`
- `FileOutputStream.close()V`

Behavior:
- open input in read mode
- open output in create/truncate/write mode
- `read()` returns `-1` at EOF
- `read([B)` returns count or `-1` at EOF
- `write([B)` writes full array contents
- closed stream operations throw `java/io/IOException`
- missing input path throws `java/io/FileNotFoundException`

**Step 3: Run targeted stream tests**

Run: `cargo test -p duke-interpreter file_io_ -- --nocapture`

Expected: PASS for metadata and stream happy-path tests.

**Step 4: Add unit tests for native helpers**

Add small native-level tests near existing native unit tests covering:
- closing twice is harmless
- read after close raises `IOException`
- nonexistent file open raises `FileNotFoundException`

**Step 5: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat: add file input and output stream natives"
```

### Task 5: Refactor And Error Coverage

**Owner:** Implementer agent 5

**Files:**
- Modify: `tests/fixtures/FileIoTest.java`
- Create: `tests/fixtures/FileIoTest.class`
- Modify: `crates/duke-interpreter/src/lib.rs`
- Modify: `docs/plans/vantage-spec-phase28-file-io.md`

**Step 1: Add explicit error-path fixture methods**

Add fixture methods for:
- opening a missing input file
- reading after explicit close
- writing after explicit close

Return sentinel ints when exceptions are caught so bytecode stays simple.

**Step 2: Write failing tests for those methods**

Add focused tests:
- `file_io_missing_input_raises_file_not_found`
- `file_io_read_after_close_raises_io_exception`
- `file_io_write_after_close_raises_io_exception`

Run them first and confirm they fail for the expected reason.

**Step 3: Implement minimal fixes**

Refactor common helpers:
- `open_input_file`
- `open_output_file`
- `close_file_descriptor`
- `file_descriptor_id`
- `throw_io_exception`

Keep exception class names centralized as constants.

**Step 4: Re-run the targeted and broader suites**

Run:
- `cargo test -p duke-interpreter file_io_ -- --nocapture`
- `cargo test -p duke-interpreter try_with_resources -- --nocapture`
- `cargo test -p duke-interpreter native_ -- --nocapture`

Expected: PASS.

**Step 5: Update plan/status doc**

Add a short completion note to `docs/plans/vantage-spec-phase28-file-io.md` documenting what landed and what remains out of scope.

**Step 6: Commit**

```bash
git add tests/fixtures/FileIoTest.java tests/fixtures/FileIoTest.class crates/duke-interpreter/src/lib.rs docs/plans/vantage-spec-phase28-file-io.md
git commit -m "refactor: harden file io error handling"
```

### Task 6: Final Verification

**Owner:** Controller

**Files:**
- None unless verification finds a bug

**Step 1: Format**

Run: `cargo fmt --all`

**Step 2: Run focused tests**

Run: `cargo test -p duke-interpreter file_io_ -- --nocapture`

**Step 3: Run regression slices**

Run:
- `cargo test -p duke-interpreter try_with_resources -- --nocapture`
- `cargo test -p duke-interpreter collections_sort_fixture_executes -- --nocapture`
- `cargo test -p duke-interpreter parseargs_ -- --nocapture`

**Step 4: Review diff for completeness**

Run:

```bash
git status --short
git diff --stat
```

Confirm the slice includes fixture source, compiled fixture class, runtime code, and test coverage together.

**Step 5: No commit here unless verification required a fix**

If verification forces a fix, repeat RED/GREEN/REFACTOR on the smallest failing test before adding more code.
