# Phase 5: Multi-Frame Method Invocation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `invokestatic` support so the interpreter executes multi-method programs with a proper call stack, enabling one static method to call another within the same class.

**Architecture:** Introduce `ClassContext` (owns CP + all decoded `MethodEntry`s) and `execute_class()` which maintains a `Vec<CallFrame>` call stack. On `invokestatic`, push a `CallFrame` for the callee; on any return instruction, pop back to the caller. The existing `execute()` function remains unchanged for single-method use.

**Tech Stack:** Rust, `duke-runtime` (Frame, Slot, VmError), `duke-bytecode` (Instruction, decode), `duke-classfile` (CpEntry, parse, AttributeData)

---

## Task 1: Create MathUtils Java fixture

**Files:**
- Create: `tests/fixtures/MathUtils.java`
- Compile: `tests/fixtures/MathUtils.class` (via `javac`)

### Step 1: Write MathUtils.java

```java
// tests/fixtures/MathUtils.java
public class MathUtils {
    public static int square(int n) {
        return n * n;
    }

    public static int sumOfSquares(int a, int b) {
        return square(a) + square(b);
    }

    // iterative power — no recursion yet (Phase 6 can add it)
    public static int power(int base, int exp) {
        int result = 1;
        while (exp > 0) {
            result = result * base;
            exp = exp - 1;
        }
        return result;
    }

    // Euclidean gcd (iterative)
    public static int gcd(int a, int b) {
        while (b != 0) {
            int t = b;
            b = a % b;
            a = t;
        }
        return a;
    }
}
```

### Step 2: Compile the fixture

Run: `javac -source 11 -target 11 tests/fixtures/MathUtils.java`

Expected: `tests/fixtures/MathUtils.class` created, no errors.

Note: Use `-source 11 -target 11` (or just `javac`) to keep it compatible. If javac isn't in PATH, check `JAVA_HOME/bin/javac`.

### Step 3: Verify with duke dump

Run: `cargo run --bin duke -- tests/fixtures/MathUtils.class`

Expected: Shows methods `square`, `sumOfSquares`, `power`, `gcd`, each with `invokestatic` in the disassembly for `sumOfSquares`.

### Step 4: Commit

```bash
git add tests/fixtures/MathUtils.java tests/fixtures/MathUtils.class
git commit -m "test(fixtures): add MathUtils.java with cross-method calls for Phase 5"
```

---

## Task 2: Add VmError variants for method resolution

**Files:**
- Modify: `crates/duke-runtime/src/error.rs`

### Step 1: Write failing tests (in duke-interpreter, anticipate these errors)

We'll write these in Task 3 since they exercise the interpreter. Just add the variants now.

### Step 2: Add the two new error variants to error.rs

Add after the existing `InvalidCpIndex` variant:

```rust
    #[error("method not found: {name}{descriptor}")]
    MethodNotFound { name: String, descriptor: String },

    #[error("constant pool index {index} is not a valid Methodref")]
    InvalidMethodref { index: usize },
```

### Step 3: Run tests to verify nothing broke

Run: `cargo test -p duke-runtime`

Expected: All 5 existing tests still pass.

### Step 4: Commit

```bash
git add crates/duke-runtime/src/error.rs
git commit -m "feat(runtime): add MethodNotFound and InvalidMethodref VmError variants"
```

---

## Task 3: Add ClassContext, MethodEntry types and execute_class() stub

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

This is the RED step: add the types and a stub `execute_class()` that always returns `Err(VmError::Unimplemented)`, then write tests that will fail.

### Step 1: Add types above the existing `execute()` function

Add near the top of `lib.rs`, after the imports:

```rust
/// A decoded method ready for execution.
pub struct MethodEntry {
    pub name: String,
    pub descriptor: String,
    pub instructions: Vec<(usize, Instruction)>,
    pub max_stack: u16,
    pub max_locals: u16,
}

/// A parsed class with all methods decoded — the unit of execution for Phase 5+.
pub struct ClassContext {
    pub constant_pool: Vec<Option<CpEntry>>,
    pub methods: Vec<MethodEntry>,
}
```

### Step 2: Add the stub execute_class() after execute()

```rust
/// Execute a static method by name within a loaded class context.
///
/// Supports `invokestatic` calls between methods in the same class.
///
/// # Errors
/// Returns [`VmError`] on execution faults or if `method_name`/`descriptor`
/// are not found in `ctx`.
pub fn execute_class(
    ctx: &ClassContext,
    method_name: &str,
    descriptor: &str,
    args: Vec<Slot>,
) -> VmResult<Option<Slot>> {
    let _ = (ctx, method_name, descriptor, args);
    Err(VmError::Unimplemented { mnemonic: "execute_class" })
}
```

### Step 3: Write failing integration tests using MathUtils.class

Add a new test module section at the bottom of the test block:

```rust
    // ---- Phase 5: ClassContext + execute_class() tests ----

    fn load_class_context(class_name: &str) -> ClassContext {
        use duke_bytecode::decode;
        use duke_classfile::{parse, types::AttributeData};

        let bytes = std::fs::read(fixture(class_name)).expect("fixture not found");
        let cf = parse(&bytes).expect("parse failed");

        let methods = cf
            .methods
            .iter()
            .filter_map(|m| {
                let name = match cf.constant_pool.get(m.name_index.0 as usize) {
                    Some(Some(CpEntry::Utf8(s))) => s.clone(),
                    _ => return None,
                };
                let descriptor = match cf.constant_pool.get(m.descriptor_index.0 as usize) {
                    Some(Some(CpEntry::Utf8(s))) => s.clone(),
                    _ => return None,
                };
                let code = m.attributes.iter().find_map(|a| {
                    if let AttributeData::Code(c) = &a.data { Some(c) } else { None }
                })?;
                let instructions = decode(&code.code).ok()?;
                Some(MethodEntry {
                    name,
                    descriptor,
                    instructions,
                    max_stack: code.max_stack,
                    max_locals: code.max_locals,
                })
            })
            .collect();

        ClassContext {
            constant_pool: cf.constant_pool,
            methods,
        }
    }

    fn run_class_int(class_name: &str, method_name: &str, args: Vec<i32>) -> i32 {
        let ctx = load_class_context(class_name);
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
        match execute_class(&ctx, method_name, "(II)I", &slots).expect("execute_class failed") {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn class_square() {
        let ctx = load_class_context("MathUtils.class");
        let slots = vec![Slot::Int(7)];
        match execute_class(&ctx, "square", "(I)I", &slots).expect("execute_class failed") {
            Some(Slot::Int(49)) => {}
            other => panic!("expected Int(49), got {other:?}"),
        }
    }

    #[test]
    fn class_sum_of_squares() {
        assert_eq!(run_class_int("MathUtils.class", "sumOfSquares", vec![3, 4]), 25);
    }

    #[test]
    fn class_sum_of_squares_5_12() {
        assert_eq!(run_class_int("MathUtils.class", "sumOfSquares", vec![5, 12]), 169);
    }

    #[test]
    fn class_power_2_10() {
        let ctx = load_class_context("MathUtils.class");
        let slots = vec![Slot::Int(2), Slot::Int(10)];
        match execute_class(&ctx, "power", "(II)I", &slots).expect("execute_class failed") {
            Some(Slot::Int(1024)) => {}
            other => panic!("expected Int(1024), got {other:?}"),
        }
    }

    #[test]
    fn class_gcd_48_18() {
        let ctx = load_class_context("MathUtils.class");
        let slots = vec![Slot::Int(48), Slot::Int(18)];
        match execute_class(&ctx, "gcd", "(II)I", &slots).expect("execute_class failed") {
            Some(Slot::Int(6)) => {}
            other => panic!("expected Int(6), got {other:?}"),
        }
    }

    #[test]
    fn class_method_not_found() {
        let ctx = load_class_context("MathUtils.class");
        let err = execute_class(&ctx, "nonExistent", "(I)I", &[]).unwrap_err();
        assert!(matches!(err, VmError::MethodNotFound { .. }));
    }
```

Note: Change `execute_class` signature to take `args: &[Slot]` to avoid clone issues in tests, or keep `Vec<Slot>` — see Task 4 for final signature. For now write tests to match.

### Step 4: Run tests to verify they FAIL

Run: `cargo test -p duke-interpreter class_`

Expected: All 6 new `class_*` tests fail with `Unimplemented { mnemonic: "execute_class" }`.

### Step 5: Commit the stub + failing tests

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "test(interpreter): add ClassContext/MethodEntry types + failing Phase 5 tests"
```

---

## Task 4: Implement execute_class() with invokestatic call stack

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

This is the GREEN step. Replace the stub with a real multi-frame execute loop.

### Step 1: Add internal CallFrame type (private, above execute_class)

```rust
/// Saved state of a caller frame suspended during an invokestatic call.
struct CallFrame {
    frame: Frame,
    method_idx: usize,
    pc_to_idx: HashMap<usize, usize>,
    resume_idx: usize, // instruction index to continue at after callee returns
}
```

### Step 2: Add helper: resolve_methodref

Resolves a CP `Methodref` to `(name, descriptor)` strings.

```rust
/// Resolve a constant pool Methodref to (method_name, descriptor).
fn resolve_methodref(
    cp: &[Option<CpEntry>],
    idx: usize,
) -> VmResult<(String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Methodref { name_and_type_index, .. }) => {
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType { name_index, descriptor_index }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    };
                    Ok((name, desc))
                }
                _ => Err(VmError::InvalidMethodref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidMethodref { index: idx }),
    }
}
```

### Step 3: Add helper: parse_arg_count

Counts argument slots from a JVM method descriptor like `(ILjava/lang/String;II)I`.

```rust
/// Count the number of argument slots in a JVM method descriptor.
///
/// Each primitive type is 1 slot. `long` and `double` are also 1 slot in our
/// model (we use a single `Slot` enum variant for each). Object references
/// (`Ljava/lang/Class;`) and arrays (`[I`) are 1 slot each.
fn parse_arg_count(descriptor: &str) -> usize {
    let params = descriptor
        .find(')')
        .map(|i| &descriptor[1..i])
        .unwrap_or("");

    let mut count = 0;
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => count += 1,
            '[' => {
                // consume array dimension chars and the element type
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next(); // consume 'L'
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next(); // consume primitive element type
                }
                count += 1;
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                count += 1;
            }
            _ => {}
        }
    }
    count
}
```

### Step 4: Implement execute_class()

Replace the stub with the real implementation:

```rust
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
pub fn execute_class(
    ctx: &ClassContext,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>> {
    // Find the entry method.
    let entry_idx = ctx
        .methods
        .iter()
        .position(|m| m.name == method_name && m.descriptor == descriptor)
        .ok_or_else(|| VmError::MethodNotFound {
            name: method_name.to_string(),
            descriptor: descriptor.to_string(),
        })?;

    let mut call_stack: Vec<CallFrame> = Vec::new();

    // Set up initial frame.
    let mut method_idx = entry_idx;
    let mut pc_to_idx: HashMap<usize, usize> = ctx.methods[method_idx]
        .instructions
        .iter()
        .enumerate()
        .map(|(i, &(pc, _))| (pc, i))
        .collect();
    let mut frame = Frame::new(
        usize::from(ctx.methods[method_idx].max_stack),
        usize::from(ctx.methods[method_idx].max_locals),
        args.to_vec(),
    )?;
    let mut idx: usize = 0;

    loop {
        let method = &ctx.methods[method_idx];
        let Some(&(pc, ref instr)) = method.instructions.get(idx) else {
            return Err(VmError::FellOffEnd);
        };
        // Clone the instruction to release the borrow on `ctx.methods`.
        let instr = instr.clone();
        let pc = pc;

        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64).wrapping_add(i64::from($offset)) as usize;
                idx = *pc_to_idx
                    .get(&target)
                    .ok_or(VmError::InvalidBranchTarget { pc: target })?;
                continue;
            }};
        }

        match &instr {
            // ---- invokestatic ----
            Instruction::Invokestatic(cp_idx) => {
                let (callee_name, callee_desc) =
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?;

                let callee_idx = ctx
                    .methods
                    .iter()
                    .position(|m| m.name == callee_name && m.descriptor == callee_desc)
                    .ok_or_else(|| VmError::MethodNotFound {
                        name: callee_name.clone(),
                        descriptor: callee_desc.clone(),
                    })?;

                // Pop args from caller stack (in reverse order into locals).
                let arg_count = parse_arg_count(&callee_desc);
                let mut callee_args: Vec<Slot> = (0..arg_count)
                    .map(|_| frame.pop())
                    .collect::<VmResult<Vec<_>>>()?;
                callee_args.reverse();

                // Build callee pc_to_idx.
                let callee_pc_to_idx: HashMap<usize, usize> = ctx.methods[callee_idx]
                    .instructions
                    .iter()
                    .enumerate()
                    .map(|(i, &(pc, _))| (pc, i))
                    .collect();
                let callee_frame = Frame::new(
                    usize::from(ctx.methods[callee_idx].max_stack),
                    usize::from(ctx.methods[callee_idx].max_locals),
                    callee_args,
                )?;

                // Save caller.
                call_stack.push(CallFrame {
                    frame,
                    method_idx,
                    pc_to_idx,
                    resume_idx: idx + 1,
                });

                // Switch to callee.
                frame = callee_frame;
                method_idx = callee_idx;
                pc_to_idx = callee_pc_to_idx;
                idx = 0;
                continue;
            }

            // ---- returns ----
            Instruction::Return => {
                match call_stack.pop() {
                    None => return Ok(None),
                    Some(caller) => {
                        frame = caller.frame;
                        method_idx = caller.method_idx;
                        pc_to_idx = caller.pc_to_idx;
                        idx = caller.resume_idx;
                        continue;
                    }
                }
            }
            Instruction::Ireturn => {
                let v = frame.pop_int()?;
                match call_stack.pop() {
                    None => return Ok(Some(Slot::Int(v))),
                    Some(caller) => {
                        frame = caller.frame;
                        method_idx = caller.method_idx;
                        pc_to_idx = caller.pc_to_idx;
                        idx = caller.resume_idx;
                        frame.push(Slot::Int(v))?;
                        continue;
                    }
                }
            }
            Instruction::Lreturn => {
                let v = frame.pop_long()?;
                match call_stack.pop() {
                    None => return Ok(Some(Slot::Long(v))),
                    Some(caller) => {
                        frame = caller.frame;
                        method_idx = caller.method_idx;
                        pc_to_idx = caller.pc_to_idx;
                        idx = caller.resume_idx;
                        frame.push(Slot::Long(v))?;
                        continue;
                    }
                }
            }
            Instruction::Freturn => {
                let v = frame.pop_float()?;
                match call_stack.pop() {
                    None => return Ok(Some(Slot::Float(v))),
                    Some(caller) => {
                        frame = caller.frame;
                        method_idx = caller.method_idx;
                        pc_to_idx = caller.pc_to_idx;
                        idx = caller.resume_idx;
                        frame.push(Slot::Float(v))?;
                        continue;
                    }
                }
            }
            Instruction::Dreturn => {
                let v = frame.pop_double()?;
                match call_stack.pop() {
                    None => return Ok(Some(Slot::Double(v))),
                    Some(caller) => {
                        frame = caller.frame;
                        method_idx = caller.method_idx;
                        pc_to_idx = caller.pc_to_idx;
                        idx = caller.resume_idx;
                        frame.push(Slot::Double(v))?;
                        continue;
                    }
                }
            }

            // ---- delegate all other instructions to the same logic as execute() ----
            // (Copy the full match body from execute() here, replacing `return Ok(...)`
            //  for returns with the call-stack-aware versions above)
            other => {
                // Re-use execute() for single-frame instruction semantics by
                // running just this one instruction. Since execute() is a full
                // loop, we instead handle remaining opcodes inline below.
                // See implementation note in docs/adr/ for why we duplicate
                // rather than extract.
                return Err(VmError::Unimplemented { mnemonic: other.mnemonic() });
            }
        }

        idx += 1;
    }
}
```

**IMPORTANT IMPLEMENTATION NOTE:** The `other =>` arm above must be replaced with the full set of instruction handlers from `execute()`. Copy every match arm from `execute()` except the return variants (already handled above) and add `Invokestatic` (already handled above). This is intentional duplication until Phase 6 when we extract a shared dispatch function.

The full list of instructions to copy from `execute()`:
- All constant push instructions (Nop, AconstNull, IconstM1 through Dconst1, Bipush, Sipush)
- Ldc, LdcW, Ldc2W
- All load instructions (Iload, Lload, Fload, Dload, Aload with indexed and numbered variants)
- All store instructions (same pattern)
- Stack manipulation (Pop, Pop2, Dup, Swap)
- All integer arithmetic (Iadd through IincW)
- All long arithmetic (Ladd through Lcmp)
- All float arithmetic (Fadd through Fcmpg)
- All double arithmetic (Dadd through Dcmpg)
- All type conversions (I2l through I2s)
- All branch instructions (Goto, GotoW, Ifeq through IfAcmpne)
- Final `other =>` arm: `return Err(VmError::Unimplemented { mnemonic: other.mnemonic() })`

### Step 5: Update execute_class signature in tests

The tests from Task 3 use `execute_class(&ctx, name, desc, &slots)` — make sure the function signature matches: `args: &[Slot]`.

### Step 6: Run the tests

Run: `cargo test -p duke-interpreter`

Expected: All 33 existing tests + 6 new `class_*` tests pass. Zero clippy warnings.

Run: `cargo clippy -p duke-interpreter -- -W clippy::pedantic -W clippy::nursery`

Fix any warnings before committing.

### Step 7: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): implement execute_class() with invokestatic call stack"
```

---

## Task 5: Unit tests for parse_arg_count and resolve_methodref

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

These helpers are pure functions — they deserve their own unit tests independent of fixture loading.

### Step 1: Write unit tests for parse_arg_count

Add to the tests module:

```rust
    // ---- Unit tests for parse_arg_count ----

    #[test]
    fn arg_count_empty() {
        assert_eq!(parse_arg_count("()V"), 0);
    }

    #[test]
    fn arg_count_single_int() {
        assert_eq!(parse_arg_count("(I)I"), 1);
    }

    #[test]
    fn arg_count_two_ints() {
        assert_eq!(parse_arg_count("(II)I"), 2);
    }

    #[test]
    fn arg_count_long_double() {
        // J = long, D = double — each is 1 slot in our model
        assert_eq!(parse_arg_count("(JD)V"), 2);
    }

    #[test]
    fn arg_count_object_ref() {
        // Ljava/lang/String; = 1 slot
        assert_eq!(parse_arg_count("(Ljava/lang/String;I)V"), 2);
    }

    #[test]
    fn arg_count_array() {
        // [I = int array = 1 slot
        assert_eq!(parse_arg_count("([II)I"), 2);
    }

    #[test]
    fn arg_count_mixed() {
        assert_eq!(parse_arg_count("(ILjava/lang/Object;Z)V"), 3);
    }
```

### Step 2: Write unit tests for resolve_methodref

```rust
    // ---- Unit tests for resolve_methodref ----

    fn make_cp(entries: Vec<Option<CpEntry>>) -> Vec<Option<CpEntry>> {
        let mut cp = vec![None]; // slot 0 reserved
        cp.extend(entries);
        cp
    }

    #[test]
    fn resolve_methodref_valid() {
        use duke_classfile::types::CpIndex;
        // Build a minimal CP:
        // [1] = Methodref { class_index=2, name_and_type_index=3 }
        // [2] = Class { name_index=4 }  (not needed by resolve_methodref)
        // [3] = NameAndType { name_index=4, descriptor_index=5 }
        // [4] = Utf8("square")
        // [5] = Utf8("(I)I")
        let cp = make_cp(vec![
            Some(CpEntry::Methodref {
                class_index: CpIndex(2),
                name_and_type_index: CpIndex(3),
            }),
            Some(CpEntry::Class { name_index: CpIndex(4) }),
            Some(CpEntry::NameAndType {
                name_index: CpIndex(4),
                descriptor_index: CpIndex(5),
            }),
            Some(CpEntry::Utf8("square".to_string())),
            Some(CpEntry::Utf8("(I)I".to_string())),
        ]);
        let (name, desc) = resolve_methodref(&cp, 1).unwrap();
        assert_eq!(name, "square");
        assert_eq!(desc, "(I)I");
    }

    #[test]
    fn resolve_methodref_invalid_index() {
        let cp = make_cp(vec![]);
        let err = resolve_methodref(&cp, 99).unwrap_err();
        assert!(matches!(err, VmError::InvalidMethodref { index: 99 }));
    }

    #[test]
    fn resolve_methodref_not_a_methodref() {
        let cp = make_cp(vec![
            Some(CpEntry::Utf8("not a methodref".to_string())),
        ]);
        let err = resolve_methodref(&cp, 1).unwrap_err();
        assert!(matches!(err, VmError::InvalidMethodref { .. }));
    }
```

### Step 3: Run tests

Run: `cargo test -p duke-interpreter`

Expected: All tests pass.

### Step 4: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "test(interpreter): add unit tests for parse_arg_count and resolve_methodref"
```

---

## Task 6: Update duke binary to use execute_class

**Files:**
- Modify: `duke/src/main.rs`

The `exec` subcommand currently uses `execute()`. Update it to build a `ClassContext` and use `execute_class()` instead, so it handles multi-method programs automatically.

### Step 1: Write a failing test

Manually test (can't easily unit test main.rs):

Run after implementation:
```
cargo run --bin duke -- exec tests/fixtures/MathUtils.class sumOfSquares 3 4
```
Expected output: `Int(25)` (or just `25` if we pretty-print)

### Step 2: Refactor exec_method in duke/src/main.rs

The existing `exec_method` function:
1. Parses the class file
2. Finds the method
3. Decodes bytecode
4. Calls `execute()`

Replace steps 2-4 with:
1. Build a `ClassContext` from the class file (decode all methods)
2. Call `execute_class(&ctx, method_name, descriptor_str, &int_args)`

Key change: `execute_class` requires a descriptor. The binary currently only takes a method name. Two options:
- **Option A**: Find the method's descriptor from CP when looking up by name (use `cp_str(cf, method.descriptor_index)`)
- **Option B**: Require descriptor as a CLI argument

Use Option A (auto-detect): find the method by name, extract its descriptor, then build the context.

New `exec_method` logic:

```rust
fn exec_method(args: &[String]) {
    use duke_bytecode::decode;
    use duke_classfile::types::AttributeData;
    use duke_interpreter::{ClassContext, MethodEntry, execute_class};

    if args.len() < 2 {
        eprintln!("Usage: duke exec <classfile.class> <method> [int-arg...]");
        process::exit(1);
    }
    let path = &args[0];
    let method_name = &args[1];
    let int_args: Vec<Slot> = args[2..]
        .iter()
        .map(|s| {
            Slot::Int(s.parse::<i32>().unwrap_or_else(|_| {
                eprintln!("duke: argument '{s}' is not an integer");
                process::exit(1);
            }))
        })
        .collect();

    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    // Find the target method's descriptor so execute_class can dispatch.
    let target = cf
        .methods
        .iter()
        .find(|m| {
            let Some(Some(CpEntry::Utf8(s))) = cf.constant_pool.get(m.name_index.0 as usize)
            else { return false; };
            s.as_str() == method_name.as_str()
        })
        .unwrap_or_else(|| {
            eprintln!("duke: method '{method_name}' not found");
            process::exit(1);
        });
    let descriptor = cp_str(&cf, target.descriptor_index).unwrap_or("").to_string();

    // Build ClassContext — decode all methods.
    let methods: Vec<MethodEntry> = cf
        .methods
        .iter()
        .filter_map(|m| {
            let name = match cf.constant_pool.get(m.name_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let desc = match cf.constant_pool.get(m.descriptor_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let code = m.attributes.iter().find_map(|a| {
                if let AttributeData::Code(c) = &a.data { Some(c) } else { None }
            })?;
            let instructions = decode(&code.code).ok()?;
            Some(MethodEntry {
                name,
                descriptor: desc,
                instructions,
                max_stack: code.max_stack,
                max_locals: code.max_locals,
            })
        })
        .collect();

    let ctx = ClassContext { constant_pool: cf.constant_pool, methods };

    match execute_class(&ctx, method_name, &descriptor, &int_args) {
        Ok(Some(result)) => println!("{result:?}"),
        Ok(None) => println!("(void)"),
        Err(e) => {
            eprintln!("duke: runtime error: {e}");
            process::exit(1);
        }
    }
}
```

Note: `ClassContext` and `MethodEntry` need to be `pub` in `duke-interpreter` (they already are from Task 3).

### Step 3: Build and manual test

Run: `cargo build --bin duke`

Run: `./target/debug/duke exec tests/fixtures/MathUtils.class sumOfSquares 3 4`
Expected: `Int(25)`

Run: `./target/debug/duke exec tests/fixtures/MathUtils.class gcd 48 18`
Expected: `Int(6)`

Run: `./target/debug/duke exec tests/fixtures/Arithmetic.class add 3 4`
Expected: `Int(7)` (backward-compat with Phase 4 fixture)

### Step 4: Run full test suite

Run: `cargo test`

Expected: All tests pass (including the 33 Arithmetic tests and 6 MathUtils tests).

### Step 5: Clippy clean

Run: `cargo clippy -- -W clippy::pedantic -W clippy::nursery`

Fix any warnings.

### Step 6: Commit

```bash
git add duke/src/main.rs
git commit -m "feat(duke): update exec subcommand to use execute_class for multi-method support"
```

---

## Task 7: Final verification and Phase 5 wrap-up

### Step 1: Full test run

Run: `cargo test`

Expected output: All tests pass. Count should be 73 existing + 6 + unit tests for parse_arg_count (7) + unit tests for resolve_methodref (3) = **~90 tests total**.

### Step 2: Clippy clean across workspace

Run: `cargo clippy --workspace -- -W clippy::pedantic -W clippy::nursery`

Expected: Zero warnings.

### Step 3: Format check

Run: `cargo fmt --check`

Expected: No formatting changes needed.

### Step 4: Smoke test the binary

```bash
cargo build --bin duke
./target/debug/duke exec tests/fixtures/MathUtils.class square 9
# → Int(81)

./target/debug/duke exec tests/fixtures/MathUtils.class sumOfSquares 3 4
# → Int(25)

./target/debug/duke exec tests/fixtures/MathUtils.class power 2 10
# → Int(1024)

./target/debug/duke exec tests/fixtures/MathUtils.class gcd 48 18
# → Int(6)
```

### Step 5: Final commit

```bash
git add -A
git commit -m "feat(phase5): multi-frame invokestatic call stack — execute_class() with ClassContext"
```

---

## Architecture Decision: Why Duplicate Dispatch Logic

The `execute()` and `execute_class()` functions share ~600 lines of match arms. Why not extract a shared function?

**For now:** Duplication is correct. Extracting a shared dispatch would require threading `&mut Frame + &mut idx + &mut pc_to_idx` through a function, which fights Rust's borrow checker significantly (you'd need `*mut Frame` raw pointers or `RefCell`). The functions ARE functionally identical for the 200+ non-call non-return instructions.

**Phase 6 plan:** Extract a `dispatch_one(instr: &Instruction, frame: &mut Frame) -> DispatchResult` where `DispatchResult` is an enum that covers `Continue`, `Jump(usize)`, `Return(Option<Slot>)`, `Invoke(...)`. Both loops then match on `DispatchResult`. This is the clean architecture — defer it to Phase 6 to keep Phase 5 focused.

Document this as `docs/adr/005-dispatch-deduplication.md` in Phase 6.
