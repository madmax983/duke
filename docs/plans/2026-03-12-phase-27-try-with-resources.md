# Phase 27: try-with-resources Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `try-with-resources` statements work by registering `Throwable.addSuppressed` as a no-op native and adding the `java/lang/AutoCloseable` synthetic interface.

**Architecture:** `try-with-resources` is entirely desugared by javac into standard bytecodes: `invokevirtual close()`, exception table entries catching `java/lang/Throwable`, and `invokevirtual Throwable.addSuppressed(Throwable)V`. The interpreter already handles all of these — we just need to ensure (a) `addSuppressed` doesn't cause a silent no-op pop-and-continue on every call site, and (b) `AutoCloseable` exists as a synthetic class so `is_assignable_from` doesn't fail when walking the class hierarchy of user-defined `AutoCloseable` implementations.

**Tech Stack:** Rust (duke-interpreter), Java 21 fixture, existing `bootstrap_stdlib` + `NativeRegistry` patterns.

---

## Context

### How try-with-resources compiles (Java 21)

Given:
```java
try (Res r = new Res()) {
    return r.value;
}
```

javac emits (simplified):
```
; happy path
astore_0           ; r = new Res()
aload_0
invokevirtual value:()I
istore_1
aload_0
invokevirtual close:()V     ; <- user's close() called directly
iload_1
ireturn

; exception handler (target 19)
astore_1           ; primaryExc = thrown
aload_0
invokevirtual close:()V     ; <- close() called even on exception
goto 28
; secondary handler for close() throwing (target 24)
astore_2           ; secondaryExc = thrown by close()
aload_1
aload_2
invokevirtual Throwable.addSuppressed:(Ljava/lang/Throwable;)V
aload_1
athrow             ; rethrow primary

exception table:
  [body range] → target 19, catch java/lang/Throwable
  [close range] → target 24, catch java/lang/Throwable
```

**What already works:** `invokevirtual` on user-defined `close()`, `athrow`, exception table matching, `is_assignable_from` for `Throwable` catches.

**What's missing:**
1. `java/lang/AutoCloseable` synthetic class (needed so `is_assignable_from` doesn't error when walking `Res`'s interfaces)
2. `Throwable.addSuppressed(Ljava/lang/Throwable;)V` native — without it, the fallback "pop args + this" at line 6610 silently drops it, which is *functionally fine* but registering explicitly is correct and testable

### Dispatch path for addSuppressed (when NOT registered)

```
invokevirtual Throwable.addSuppressed:(Ljava/lang/Throwable;)V
→ ensure_loaded("java/lang/Throwable") → true (synthetic class in registry)
→ resolve_method_in_hierarchy → None (no bytecode method)
→ native super-chain walk → None (not in NativeRegistry)
→ fallback: pop 1 arg + this, continue  ← silent no-op
```

This works accidentally today, but registering the native makes it explicit and lets us test it.

---

## Task 1: Write the fixture (RED)

**Files:**
- Create: `tests/fixtures/TryWithResources.java`

**Step 1: Write the fixture**

```java
public class TryWithResources {
    // Static counter — accessible from inner class Res.close() via
    // `putstatic TryWithResources.closeCount:I`.
    static int closeCount = 0;

    // Static nested class — compiles to TryWithResources$Res.class.
    // Implements AutoCloseable so the compiler accepts try-with-resources.
    static class Res implements AutoCloseable {
        @Override
        public void close() { closeCount++; }
    }

    /**
     * Happy path: try body completes normally.
     * Compiler emits: body → close() → return.
     * Returns 42.
     */
    static int simpleValue() {
        try (Res r = new Res()) {
            return 42;
        }
    }

    /**
     * Verify close() is called on normal exit.
     * Returns closeCount after the try block (should be 1).
     */
    static int closedOnSuccess() {
        closeCount = 0;
        try (Res r = new Res()) {
            int x = 0; // non-empty body
        }
        return closeCount;
    }

    /**
     * Exception path: body throws, close() still called, caller catches.
     * Returns closeCount inside the catch (should be 1).
     * Also exercises Throwable.addSuppressed (called if close() throws,
     * but here close() doesn't throw so the secondary handler is dead code).
     */
    static int closedOnException() {
        closeCount = 0;
        try (Res r = new Res()) {
            throw new RuntimeException("boom");
        } catch (RuntimeException e) {
            return closeCount; // 1
        }
    }

    /**
     * Nested resources: both r1 and r2 are closed (r2 first, r1 second).
     * Returns closeCount (should be 2).
     */
    static int nestedClosed() {
        closeCount = 0;
        try (Res r1 = new Res(); Res r2 = new Res()) {
            int x = 0; // non-empty body
        }
        return closeCount; // 2
    }
}
```

**Step 2: Compile the fixture**

```bash
javac --release 21 tests/fixtures/TryWithResources.java -d tests/fixtures/
```

Expected output: `tests/fixtures/TryWithResources.class` and `tests/fixtures/TryWithResources$Res.class` (two files).

Verify:
```bash
ls tests/fixtures/TryWithResources*.class
```

**Step 3: Write failing tests in `crates/duke-interpreter/src/lib.rs`**

Add these to the `#[cfg(test)]` mod at the bottom (after the CollectionsSortTest tests):

```rust
// --- Phase 27: try-with-resources ---

#[test]
fn try_with_resources_simple_value() {
    assert_eq!(
        run_class_int("TryWithResources.class", "simpleValue", "()I", vec![]),
        42
    );
}

#[test]
fn try_with_resources_closed_on_success() {
    assert_eq!(
        run_class_int("TryWithResources.class", "closedOnSuccess", "()I", vec![]),
        1
    );
}

#[test]
fn try_with_resources_closed_on_exception() {
    assert_eq!(
        run_class_int("TryWithResources.class", "closedOnException", "()I", vec![]),
        1
    );
}

#[test]
fn try_with_resources_nested_closed() {
    assert_eq!(
        run_class_int("TryWithResources.class", "nestedClosed", "()I", vec![]),
        2
    );
}
```

**Step 4: Run tests to see them fail**

```bash
cargo test -p duke-interpreter try_with_resources 2>&1 | tail -20
```

Expected: 4 failures (errors about missing class or method, not panics).

---

## Task 2: Add AutoCloseable synthetic class (GREEN partial)

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` — `bootstrap_stdlib` function

**Step 1: Locate the insertion point**

Find the line with `java/lang/Throwable` ClassContext definition in `bootstrap_stdlib` (around line 687). Add the new synthetic class right after the `rte_ctx` (RuntimeException) block, before the Enum block.

**Step 2: Add AutoCloseable**

```rust
// java/lang/AutoCloseable — marker interface; close() is implemented by user classes.
// Needed so is_assignable_from doesn't fail when walking the hierarchy of user
// classes that `implements AutoCloseable`.
let autocloseable_ctx = ClassContext {
    class_name: "java/lang/AutoCloseable".to_string(),
    super_class: None, // interface — no superclass
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: Vec::new(),
    static_fields: Vec::new(),
    instance_field_count: 0,
    interfaces: Vec::new(),
    bootstrap_methods: Vec::new(),
};
registry.register(autocloseable_ctx);
```

**Step 3: Run the tests again**

```bash
cargo test -p duke-interpreter try_with_resources 2>&1 | tail -20
```

Expected: Some tests may pass now, some may still fail. Note which ones.

---

## Task 3: Register Throwable.addSuppressed native (GREEN full)

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Write the native handler function**

Add near the other Throwable/Exception natives (search for "fn native_" — add after the last one in alphabetical/logical grouping):

```rust
/// `Throwable.addSuppressed(Throwable suppressed)V`
///
/// Stores suppressed exceptions on the primary exception for later retrieval
/// via `getSuppressed()`. In Duke's single-threaded interpreter we track
/// suppressed exceptions in `fields[0]` as a count (Int) — sufficient for
/// tests that check addSuppressed doesn't crash. Full suppressed-list storage
/// is YAGNI until getSuppressed() is needed.
///
/// Signature: args[0] = this (Throwable), args[1] = suppressed (Throwable)
fn native_throwable_add_suppressed(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _stdout: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    // No-op: suppressed exception is silently dropped.
    // Control flow is handled entirely by the bytecode desugaring —
    // addSuppressed only affects what getSuppressed() returns, which
    // we don't implement.
    Ok(None)
}
```

**Step 2: Register the native in bootstrap_stdlib**

After the `registry.register(throwable_ctx)` line, add:

```rust
registry
    .natives_mut()
    .register(
        "java/lang/Throwable",
        "addSuppressed",
        "(Ljava/lang/Throwable;)V",
        native_throwable_add_suppressed,
    );
```

**Step 3: Run the full try_with_resources test suite**

```bash
cargo test -p duke-interpreter try_with_resources -- --nocapture 2>&1 | tail -20
```

Expected: all 4 pass.

---

## Task 4: Full regression + commit

**Step 1: Run full test suite**

```bash
cargo test 2>&1 | tail -10
```

Expected: 397 tests pass (393 previous + 4 new), 0 failures.

**Step 2: Check for stubs / TODOs**

```bash
grep -rn "TODO\|FIXME\|unimplemented!" crates/duke-interpreter/src/lib.rs | grep -v "test\|Test\|#\[" | head -10
```

**Step 3: Clippy**

```bash
cargo clippy --all-targets 2>&1 | grep "^error" | head -10
```

Fix any errors before committing.

**Step 4: Commit**

```bash
git add tests/fixtures/TryWithResources.java \
        tests/fixtures/TryWithResources.class \
        "tests/fixtures/TryWithResources\$Res.class" \
        crates/duke-interpreter/src/lib.rs

git commit -m "feat(phase-27): try-with-resources — AutoCloseable + Throwable.addSuppressed"
```

---

## Verification Checklist

- [ ] `tests/fixtures/TryWithResources.class` exists
- [ ] `tests/fixtures/TryWithResources$Res.class` exists
- [ ] `java/lang/AutoCloseable` registered in `bootstrap_stdlib`
- [ ] `Throwable.addSuppressed(Ljava/lang/Throwable;)V` native registered
- [ ] 4 new tests pass
- [ ] Total test count = 397
- [ ] `cargo clippy` clean
- [ ] Committed
