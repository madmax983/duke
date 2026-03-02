# Phase 8: Exception Table Dispatch + Switch Statements

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add try/catch exception handling via exception table dispatch, plus `tableswitch`
and `lookupswitch` control flow to the Duke interpreter.

**Architecture:** Add `ExceptionEntry` struct (resolved catch_type names) to `MethodEntry`,
populated from the already-parsed `CodeAttribute.exception_table` in `build_class_context()`.
Rework `athrow` in `execute_class` to search exception table → handler jump, or unwind call
stack. Switch opcodes are pure control-flow dispatch with no heap interaction.

**Tech Stack:** Rust, `duke-interpreter` (execute_class, execute, MethodEntry, build_class_context),
`duke-classfile` (ExceptionTableEntry — already parsed), `duke-bytecode` (Tableswitch, Lookupswitch).

---

## Wave 1: Independent Preparatory Work (run both tasks in parallel)

### Task 1: Create ExceptionTest.java Fixture + Failing Tests

**Files:**
- Create: `tests/fixtures/ExceptionTest.java`
- Create: `tests/fixtures/ExceptionTest.class` (compiled)
- Modify: `crates/duke-interpreter/src/lib.rs` (add failing integration tests)

**Step 1: Write ExceptionTest.java**

Create `tests/fixtures/ExceptionTest.java`:

```java
public class ExceptionTest {

    /**
     * Basic try/catch: throw RuntimeException, catch it, return 42.
     *
     * Bytecode will contain an exception table entry mapping the
     * throw range to the catch handler. athrow dispatches to handler,
     * which pushes the exception ref, then returns 42.
     */
    public static int catchSimple() {
        try {
            throw new RuntimeException();
        } catch (RuntimeException e) {
            return 42;
        }
    }

    /**
     * Uncaught throw: no handler in the table for this method.
     * Should propagate as VmError::JavaException.
     */
    public static int uncaught() {
        throw new RuntimeException();
    }

    /**
     * Catch-all (finally): try { return 1; } finally { }
     * javac compiles finally with catch_type=0 (catch-all) entries.
     * After the finally block, the value is returned.
     */
    public static int finallyBlock() {
        int result = 0;
        try {
            result = 10;
        } finally {
            result += 1;
        }
        return result;
    }

    /**
     * Dense switch → tableswitch instruction.
     * Cases 0,1,2 with default.
     */
    public static int switchDense(int x) {
        switch (x) {
            case 0: return 10;
            case 1: return 20;
            case 2: return 30;
            default: return -1;
        }
    }

    /**
     * Sparse switch → lookupswitch instruction.
     * Cases 100,200,300 with default.
     */
    public static int switchSparse(int x) {
        switch (x) {
            case 100: return 1;
            case 200: return 2;
            case 300: return 3;
            default: return 0;
        }
    }

    /**
     * Caller catches exception thrown by callee.
     * Tests exception unwinding across call stack frames.
     */
    public static int catchFromCallee() {
        try {
            uncaught(); // throws RuntimeException
            return 0;   // never reached
        } catch (RuntimeException e) {
            return 99;
        }
    }
}
```

**Step 2: Compile the fixture**

```bash
cd tests/fixtures && javac --release 21 ExceptionTest.java
```

**Step 3: Verify the exception table exists with `javap`**

```bash
javap -v -c tests/fixtures/ExceptionTest.class 2>&1 | grep -A 5 "Exception table"
```

Expected: exception table entries for `catchSimple`, `finallyBlock`, and `catchFromCallee`.

**Step 4: Add failing integration tests to lib.rs**

Read `crates/duke-interpreter/src/lib.rs` first to understand the test helpers
(`run_class_int`, `run_class_long`, etc.) and fixture loading pattern (`load_class`).

Add these tests to the `#[cfg(test)]` block:

```rust
// ---- Phase 8: Exceptions ----

#[test]
fn exception_catch_simple() {
    let cf = load_class("ExceptionTest");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    let result = execute_class(&mut ctx, &mut heap, "catchSimple", "()I", &[]);
    assert_eq!(result.unwrap(), Some(Slot::Int(42)));
}

#[test]
fn exception_uncaught_propagates() {
    let cf = load_class("ExceptionTest");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    let result = execute_class(&mut ctx, &mut heap, "uncaught", "()I", &[]);
    let err = result.unwrap_err();
    assert!(matches!(err, VmError::JavaException { .. }));
}

#[test]
fn exception_finally_block() {
    let cf = load_class("ExceptionTest");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    let result = execute_class(&mut ctx, &mut heap, "finallyBlock", "()I", &[]);
    assert_eq!(result.unwrap(), Some(Slot::Int(11))); // 10 + 1
}

#[test]
fn exception_catch_from_callee() {
    let cf = load_class("ExceptionTest");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    let result = execute_class(&mut ctx, &mut heap, "catchFromCallee", "()I", &[]);
    assert_eq!(result.unwrap(), Some(Slot::Int(99)));
}

// ---- Phase 8: Switch statements ----

#[test]
fn switch_dense_case0() {
    let cf = load_class("ExceptionTest");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    let result = execute_class(&mut ctx, &mut heap, "switchDense", "(I)I", &[Slot::Int(0)]);
    assert_eq!(result.unwrap(), Some(Slot::Int(10)));
}

#[test]
fn switch_dense_case2() {
    let cf = load_class("ExceptionTest");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    let result = execute_class(&mut ctx, &mut heap, "switchDense", "(I)I", &[Slot::Int(2)]);
    assert_eq!(result.unwrap(), Some(Slot::Int(30)));
}

#[test]
fn switch_dense_default() {
    let cf = load_class("ExceptionTest");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    let result = execute_class(&mut ctx, &mut heap, "switchDense", "(I)I", &[Slot::Int(99)]);
    assert_eq!(result.unwrap(), Some(Slot::Int(-1)));
}

#[test]
fn switch_sparse_case200() {
    let cf = load_class("ExceptionTest");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    let result = execute_class(&mut ctx, &mut heap, "switchSparse", "(I)I", &[Slot::Int(200)]);
    assert_eq!(result.unwrap(), Some(Slot::Int(2)));
}

#[test]
fn switch_sparse_default() {
    let cf = load_class("ExceptionTest");
    let mut ctx = build_class_context(&cf);
    let mut heap = Heap::new();
    let result = execute_class(&mut ctx, &mut heap, "switchSparse", "(I)I", &[Slot::Int(999)]);
    assert_eq!(result.unwrap(), Some(Slot::Int(0)));
}
```

**Step 5: Run tests to verify failure**

```bash
cargo test -p duke-interpreter -- exception_ switch_ 2>&1 | head -40
```

Expected: failures — exception tests hit `JavaException` (no handler dispatch) or `Unimplemented`.
Switch tests fail with `Unimplemented { mnemonic: "tableswitch" }` or `Unimplemented { mnemonic: "lookupswitch" }`.

**Step 6: Commit**

```bash
git add tests/fixtures/ExceptionTest.java tests/fixtures/ExceptionTest.class crates/duke-interpreter/src/lib.rs
git commit -m "test(phase8): add ExceptionTest fixture with try/catch and switch tests"
```

---

### Task 2: Add ExceptionEntry to MethodEntry + Update build_class_context

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (MethodEntry struct + build_class_context fn)

**Step 1: Add ExceptionEntry struct and update MethodEntry**

In `crates/duke-interpreter/src/lib.rs`, after the existing `FieldEntry` struct, add:

```rust
/// A resolved exception table entry for handler dispatch.
///
/// Built from `duke_classfile::types::ExceptionTableEntry` with catch_type
/// resolved from a CP index to a class name string.
pub struct ExceptionEntry {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    /// `None` for catch-all (finally). `Some(class_name)` for typed catches.
    pub catch_type: Option<String>,
}
```

Then add `exception_table` field to `MethodEntry`:

```rust
pub struct MethodEntry {
    pub name: String,
    pub descriptor: String,
    pub instructions: Vec<(usize, Instruction)>,
    pub max_stack: u16,
    pub max_locals: u16,
    pub exception_table: Vec<ExceptionEntry>,  // NEW
}
```

**Step 2: Update build_class_context to populate exception_table**

In `build_class_context()`, where `MethodEntry` is constructed (around line 2213), add
exception table extraction. The `code` variable (a `&CodeAttribute`) already has
`exception_table: Vec<ExceptionTableEntry>`. Resolve each entry's `catch_type` from
the constant pool:

```rust
let exception_table: Vec<ExceptionEntry> = code
    .exception_table
    .iter()
    .map(|e| {
        let catch_type = if e.catch_type.0 == 0 {
            None // catch-all (finally)
        } else {
            // Resolve CpIndex → Class → name_index → Utf8
            let class_name = match cf
                .constant_pool
                .get(e.catch_type.0 as usize)
                .and_then(|x| x.as_ref())
            {
                Some(CpEntry::Class { name_index }) => {
                    match cf
                        .constant_pool
                        .get(name_index.0 as usize)
                        .and_then(|x| x.as_ref())
                    {
                        Some(CpEntry::Utf8(s)) => Some(s.clone()),
                        _ => None,
                    }
                }
                _ => None,
            };
            class_name
        };
        ExceptionEntry {
            start_pc: e.start_pc,
            end_pc: e.end_pc,
            handler_pc: e.handler_pc,
            catch_type,
        }
    })
    .collect();

Some(MethodEntry {
    name,
    descriptor,
    instructions,
    max_stack: code.max_stack,
    max_locals: code.max_locals,
    exception_table,  // NEW
})
```

**Step 3: Run tests to verify no regressions**

```bash
cargo test -p duke-interpreter 2>&1 | tail -5
```

Expected: all existing tests still pass (the new field doesn't change behavior yet).

**Step 4: fmt + clippy**

```bash
cargo fmt && cargo clippy -p duke-interpreter -- -W clippy::pedantic 2>&1 | grep "^error" | head -5
```

**Step 5: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): add ExceptionEntry to MethodEntry, populate from CodeAttribute"
```

---

## Wave 2: Core Implementation (sequential, depends on Wave 1)

### Task 3: Implement Exception Table Dispatch, Switch, and Type Check Opcodes

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Background: Exception dispatch algorithm**

When `athrow` executes in `execute_class`:

1. Pop objectref from stack → get `class_name` from heap.
2. Save the objectref for re-push into handler's stack.
3. Search the CURRENT method's `exception_table` for a matching handler:
   - `start_pc <= current_pc < end_pc`
   - `catch_type` is `None` (catch-all) OR `catch_type == class_name` (exact match)
   - Take the FIRST match (JVM spec says order matters).
4. If handler found in current method:
   - Clear the operand stack.
   - Push the exception objectref onto the stack.
   - Jump to `handler_pc`.
   - Continue execution.
5. If NO handler found in current method:
   - Pop the call stack (like a return with no value).
   - Search the CALLER method's exception table at the caller's `pc`.
   - Repeat until handler found or call stack exhausted.
6. If call stack exhausted with no handler:
   - Return `Err(VmError::JavaException { class_name })`.

**Limitation:** We use exact class name matching, not subtype checks. `catch (Exception e)`
won't catch `RuntimeException`. This is acceptable for Phase 8; subtype checking requires
class hierarchy loading (Phase 9+).

**Step 1: Write unit tests for switch instructions (in execute())**

Add to the `#[cfg(test)]` block:

```rust
// ---- Phase 8: Switch ----

#[test]
fn tableswitch_match() {
    use duke_bytecode::Instruction::*;
    // switch (1) { case 0: return 10; case 1: return 20; case 2: return 30; default: return -1; }
    let instrs = vec![
        (0, Iconst1),                                              // push 1
        (1, Tableswitch { default: 100, low: 0, high: 2,
             offsets: vec![10, 20, 30] }),                          // pc=1
        (11, Bipush(10)),  (13, Ireturn),                           // case 0: offset 10 → pc 11
        (21, Bipush(20)),  (23, Ireturn),                           // case 1: offset 20 → pc 21
        (31, Bipush(30)),  (33, Ireturn),                           // case 2: offset 30 → pc 31
        (101, Bipush(-1)), (103, Ireturn),                          // default: offset 100 → pc 101
    ];
    let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(20))); // case 1 → 20
}

#[test]
fn tableswitch_default() {
    use duke_bytecode::Instruction::*;
    let instrs = vec![
        (0, Bipush(99)),                                           // push 99
        (2, Tableswitch { default: 100, low: 0, high: 2,
             offsets: vec![10, 20, 30] }),                          // pc=2
        (12, Bipush(10)),  (14, Ireturn),
        (22, Bipush(20)),  (24, Ireturn),
        (32, Bipush(30)),  (34, Ireturn),
        (102, Bipush(-1)), (104, Ireturn),                         // default → pc 102
    ];
    let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(-1))); // default
}

#[test]
fn lookupswitch_match() {
    use duke_bytecode::Instruction::*;
    let instrs = vec![
        (0, Sipush(200)),                                          // push 200
        (3, Lookupswitch { default: 100,
             pairs: vec![(100, 10), (200, 20), (300, 30)] }),       // pc=3
        (13, Bipush(1)), (15, Ireturn),                             // key 100: offset 10 → pc 13
        (23, Bipush(2)), (25, Ireturn),                             // key 200: offset 20 → pc 23
        (33, Bipush(3)), (35, Ireturn),                             // key 300: offset 30 → pc 33
        (103, Bipush(0)), (105, Ireturn),                           // default: offset 100 → pc 103
    ];
    let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(2))); // key 200 → 2
}

#[test]
fn lookupswitch_default() {
    use duke_bytecode::Instruction::*;
    let instrs = vec![
        (0, Sipush(999)),
        (3, Lookupswitch { default: 100,
             pairs: vec![(100, 10), (200, 20), (300, 30)] }),
        (13, Bipush(1)), (15, Ireturn),
        (23, Bipush(2)), (25, Ireturn),
        (33, Bipush(3)), (35, Ireturn),
        (103, Bipush(0)), (105, Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(0))); // default → 0
}
```

**Step 2: Run unit tests to confirm failure (RED)**

```bash
cargo test -p duke-interpreter -- tableswitch lookupswitch 2>&1 | head -20
```

Expected: `Unimplemented` errors.

**Step 3: Implement tableswitch and lookupswitch in `execute()`**

Add to the `execute()` match block (add near the branch/goto section):

```rust
Instruction::Tableswitch { default, low, high, offsets } => {
    let key = frame.pop_int()?;
    let offset = if key >= *low && key <= *high {
        offsets[(key - low) as usize]
    } else {
        *default
    };
    jump!(offset);
}
Instruction::Lookupswitch { default, pairs } => {
    let key = frame.pop_int()?;
    let offset = pairs
        .iter()
        .find(|(k, _)| *k == key)
        .map_or(*default, |(_, off)| *off);
    jump!(offset);
}
```

**Step 4: Copy the same match arms to `execute_class()`**

Same code, different match block.

**Step 5: Run switch unit tests (GREEN)**

```bash
cargo test -p duke-interpreter -- tableswitch lookupswitch 2>&1
```

Expected: all 4 pass.

**Step 6: Run switch integration tests**

```bash
cargo test -p duke-interpreter -- switch_ 2>&1
```

Expected: `switch_dense_*` and `switch_sparse_*` all pass.

**Step 7: Implement exception table dispatch in `execute_class()`**

This is the big one. The `athrow` arm in `execute_class` needs to be reworked.

First, add a helper function (outside execute_class, as a free fn):

```rust
/// Search a method's exception table for a handler matching the given pc and class name.
fn find_exception_handler(
    exception_table: &[ExceptionEntry],
    pc: usize,
    class_name: &str,
) -> Option<u16> {
    exception_table.iter().find_map(|entry| {
        let in_range = pc >= entry.start_pc as usize && pc < entry.end_pc as usize;
        let type_matches = match &entry.catch_type {
            None => true, // catch-all (finally)
            Some(ct) => ct == class_name,
        };
        if in_range && type_matches {
            Some(entry.handler_pc)
        } else {
            None
        }
    })
}
```

Then rework the `Instruction::Athrow` arm in `execute_class`:

```rust
Instruction::Athrow => {
    let exception_ref = frame.pop_ref()?;
    let class_name = heap.get(exception_ref)?.class_name.clone();

    // Search current method's exception table.
    if let Some(handler_pc) = find_exception_handler(
        &ctx.methods[method_idx].exception_table, pc, &class_name
    ) {
        // Handler found: clear stack, push exception ref, jump to handler.
        frame.clear_stack();
        frame.push(Slot::Reference(Some(exception_ref)))?;
        idx = *pc_to_idx
            .get(&(handler_pc as usize))
            .ok_or(VmError::InvalidBranchTarget { pc: handler_pc as usize })?;
        continue;
    }

    // No handler in current method — unwind call stack.
    loop {
        match call_stack.pop() {
            None => {
                // No more frames — propagate to Rust caller.
                return Err(VmError::JavaException { class_name });
            }
            Some(caller) => {
                // Restore caller frame and check ITS exception table.
                // The caller's PC is the invoke instruction that led to the callee.
                // We need the PC of the invoke instruction to search the caller's table.
                frame = caller.frame;
                method_idx = caller.method_idx;
                pc_to_idx = caller.pc_to_idx;

                // The PC for the caller is at caller.resume_idx - 1 (the invoke instruction).
                // But we need the bytecode PC, not the instruction index.
                // Get the bytecode PC from the instruction list.
                let caller_pc = if caller.resume_idx > 0 {
                    ctx.methods[method_idx].instructions[caller.resume_idx - 1].0
                } else {
                    0
                };

                if let Some(handler_pc) = find_exception_handler(
                    &ctx.methods[method_idx].exception_table, caller_pc, &class_name
                ) {
                    frame.clear_stack();
                    frame.push(Slot::Reference(Some(exception_ref)))?;
                    idx = *pc_to_idx
                        .get(&(handler_pc as usize))
                        .ok_or(VmError::InvalidBranchTarget { pc: handler_pc as usize })?;
                    break; // Break inner loop, continue outer execution loop.
                }
                // No handler here either — keep unwinding.
            }
        }
    }
    continue; // After break from inner loop, continue execution.
}
```

**Step 8: Add `clear_stack()` method to Frame**

In `crates/duke-runtime/src/frame.rs`, add:

```rust
/// Clear the operand stack (used by exception handler dispatch).
pub fn clear_stack(&mut self) {
    self.stack.clear();
}
```

**Step 9: Run exception integration tests**

```bash
cargo test -p duke-interpreter -- exception_ 2>&1
```

Expected: `exception_catch_simple`, `exception_uncaught_propagates`, `exception_finally_block`,
`exception_catch_from_callee` all pass.

**Debugging notes if tests fail:**

- `exception_catch_simple`: If this fails, the issue is likely in handler dispatch. Check that
  the bytecode PC matches the exception table ranges. Use `javap -v ExceptionTest.class` to
  see exact PCs and exception table entries.
- `exception_finally_block`: Finally blocks use catch_type=0 (catch-all). javac generates
  complex bytecode for finally (astore + rethrow). If this is too complex, it's acceptable
  to skip this test and mark it `#[ignore]` with a comment.
- `exception_catch_from_callee`: Tests cross-frame unwinding. The caller's invoke instruction
  PC must fall within the try block's range in the caller's exception table.

**Step 10: Run the full test suite**

```bash
cargo test 2>&1 | tail -15
```

Expected: all tests pass.

**Step 11: fmt + clippy**

```bash
cargo fmt && cargo clippy --all -- -W clippy::pedantic 2>&1 | grep "^error" | head -5
```

**Step 12: Commit**

```bash
git add crates/duke-runtime/src/frame.rs crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): implement exception table dispatch, tableswitch, lookupswitch"
```

---

## Final Verification

After all tasks are done:

**Step 1: Full test suite**

```bash
cargo test 2>&1 | grep "test result"
```

Expected: all pass. Count should be ~125+ (114 previous + ~13 new).

**Step 2: fmt + clippy clean**

```bash
cargo fmt --check && cargo clippy --all -- -W clippy::pedantic -W clippy::nursery 2>&1 | grep "^error"
```

Expected: no errors.

**Step 3: Demo runs**

```bash
cargo build -p duke 2>/dev/null
./target/debug/duke exec tests/fixtures/ExceptionTest.class catchSimple
./target/debug/duke exec tests/fixtures/ExceptionTest.class switchDense 1
./target/debug/duke exec tests/fixtures/ExceptionTest.class switchSparse 200
./target/debug/duke exec tests/fixtures/ExceptionTest.class catchFromCallee
```

Expected:
```
Int(42)
Int(20)
Int(2)
Int(99)
```

**Step 4: Update MEMORY.md**

Update status to Phase 8 Complete. Add new test count, opcodes, ExceptionEntry.

---

## Implementation Notes for Agents

### Exception table — how Java bytecode structures it

When javac compiles `try { ... } catch (RuntimeException e) { ... }`:
- The `try` block bytecode spans from `start_pc` to `end_pc`
- `handler_pc` points to the first instruction of the catch block
- `catch_type` is the CP index of the exception class (or 0 for catch-all/finally)
- The handler starts with the exception objectref on the stack (like aload)
- The handler typically does `astore N` to store the exception in a local var

### `new RuntimeException()` in our interpreter

`Instruction::New` resolves the class name from the CP ("java/lang/RuntimeException")
and allocates a HeapObject with `ctx.instance_field_count` fields (which is the CURRENT
class's count, not RuntimeException's). Since ExceptionTest has 0 instance fields,
RuntimeException gets allocated with 0 fields. This is fine — we only use the class_name.

`invokespecial RuntimeException.<init>()` is handled as a no-op pop (cross-class init,
already implemented in Phase 6).

### Wave 1 Conflict Avoidance

Both tasks touch `crates/duke-interpreter/src/lib.rs`:
- Task 1 appends tests to the `#[cfg(test)]` module at the END of the file
- Task 2 modifies structs at the TOP of the file and `build_class_context` in the MIDDLE

These regions don't overlap, so both can run in parallel. If merge conflicts arise,
the Wave 2 agent can resolve them (test additions are append-only).

### `frame.clear_stack()` is critical

Exception handlers expect a clean operand stack with only the exception ref on it.
Without `clear_stack()`, leftover operands from the try block will corrupt the handler.

### `finallyBlock()` may be complex

javac generates complex bytecode for `finally` blocks. If `exception_finally_block` is
too hard to get passing, it's acceptable to `#[ignore]` it with a comment and move on.
The core exception dispatch (catchSimple, uncaught, catchFromCallee) is the priority.
