# AccessController DoPrivileged Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Duke's narrow no-security-manager `java.security.AccessController.doPrivileged` compatibility path for SLF4J smoke execution.

**Architecture:** Register synthetic `java/security/AccessController`, `PrivilegedAction`, `PrivilegedExceptionAction`, and `PrivilegedActionException` in `duke-interpreter`. Implement the two in-scope `doPrivileged` overloads as callback natives that invoke the guest action `run()` method, propagate unchecked exceptions, and wrap checked exceptions in `PrivilegedActionException`.

**Tech Stack:** Rust workspace, `duke-interpreter`, Java 21 fixture bytecode, Cargo tests.

---

### Task 1: Red Fixture

**Files:**
- Create: `tests/fixtures/AccessControllerDoPrivilegedTest.java`
- Modify: `crates/duke-interpreter/src/tests.rs`

**Steps:**
1. Add a Java fixture covering `PrivilegedAction` return value, null return, unchecked propagation, `PrivilegedExceptionAction` return value, and checked `IOException` wrapping.
2. Add Rust fixture tests named `access_controller_*`.
3. Run `javac --release 21 tests/fixtures/AccessControllerDoPrivilegedTest.java`.
4. Run `cargo test -p duke-interpreter access_controller_ -- --nocapture` and verify the failure is the missing AccessController capability.

### Task 2: Green Implementation

**Files:**
- Modify: `crates/duke-interpreter/src/stdlib.rs`
- Modify: `crates/duke-interpreter/src/native.rs`

**Steps:**
1. Register the synthetic security classes and `PrivilegedActionException.getException()`.
2. Register callback natives for the two supported `doPrivileged` overloads.
3. Invoke `run()` on the guest action without permission checks.
4. For checked action failures, wrap the original thrown exception object as the wrapper cause.
5. Re-run `cargo test -p duke-interpreter access_controller_ -- --nocapture`.

### Task 3: OSS Smoke Advancement

**Files:**
- Modify: `crates/duke-interpreter/tests/oss_jar_smoke.rs`
- Modify if needed: `README.md`, `MEMORY.md`

**Steps:**
1. Update the explicit-missing-capability test so it no longer permits `Missing class: java/security/AccessController`.
2. Run `cargo test -p duke-interpreter --test oss_jar_smoke -- --nocapture`.
3. Record the next blocker if the full SLF4J canary still cannot be promoted.

### Task 4: Refactor And Verify

**Files:**
- All touched files.

**Steps:**
1. Run `cargo fmt`.
2. Re-run the two required targeted test commands.
3. Scan affected files for TODO/FIXME/stubs.
4. Review the diff for a complete unit of work.
