# Phase 11: Native Method Stubs & Hello World Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a native method registry so that `System.out.println` works, enabling `duke exec HelloWorld.class main` to print "Hello, World!" to stdout.

**Architecture:** Add a `NativeRegistry` to `ClassRegistry` mapping `(class_name, method_name, descriptor)` → native handler functions. Bootstrap a minimal `java/lang/System` + `java/io/PrintStream` so that `getstatic java/lang/System.out` returns a PrintStream ref and `invokevirtual PrintStream.println(*)` calls a Rust function that writes to stdout. Integration points: invokestatic, invokespecial, and invokevirtual check native registry before the no-op/error fallback.

**Tech Stack:** Rust, duke-interpreter crate, duke-gc (Heap), duke-runtime (Slot/VmError)

---

## Background

### Current State (Phase 10)
- `ClassRegistry` holds `HashMap<String, ClassContext>` for loaded classes
- `invokestatic` hard-fails with `VmError::MethodNotFound` if method not found (line ~1376)
- `invokevirtual`/`invokespecial` soft-fail: pop args + `this`, continue (lines ~2119-2128)
- `getstatic` cross-class: loads target class via registry, reads static field
- No native method support exists

### What Needs to Change
1. **New struct**: `NativeRegistry` — stores native handler function pointers
2. **New function**: `bootstrap_stdlib()` — pre-registers System, PrintStream, native println stubs
3. **Interpreter integration**: Check native registry at invokevirtual/invokespecial/invokestatic dispatch before fallback
4. **Captured output**: Tests need `Vec<u8>` output sink, not real stdout

### Key Types

```rust
/// Native method handler: receives args (including `this` for instance methods)
/// and a mutable heap reference, returns an optional Slot.
type NativeHandler = fn(&[Slot], &mut Heap) -> VmResult<Option<Slot>>;

/// Maps (class, method, descriptor) → Rust function.
pub struct NativeRegistry {
    methods: HashMap<(String, String, String), NativeHandler>,
}
```

### Captured Output Strategy

Production code writes to real stdout. Tests need to capture output. We'll use a `Vec<u8>` output sink passed through `execute_class`:

```rust
pub fn execute_class(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut Heap,
    stdout: &mut dyn std::io::Write,  // NEW parameter
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>>
```

Native handlers also need the sink, so the handler signature becomes:
```rust
type NativeHandler = fn(&[Slot], &mut Heap, &mut dyn std::io::Write) -> VmResult<Option<Slot>>;
```

---

## Task 1: Test Fixture — Hello.java

Create a simple Java test class that avoids `String[] args` complexity.

**Files:**
- Create: `tests/fixtures/Hello.java`
- Create: `tests/fixtures/Hello.class` (compiled)

**Step 1: Write Hello.java**

```java
public class Hello {
    /** Prints "Hello, Duke!" to stdout and returns void. */
    public static void greet() {
        System.out.println("Hello, Duke!");
    }

    /** Prints an integer to stdout. */
    public static void printNum(int n) {
        System.out.println(n);
    }

    /** Prints a string and returns 42. */
    public static int greetAndReturn() {
        System.out.println("Greetings!");
        return 42;
    }

    /** Prints an empty line. */
    public static void blankLine() {
        System.out.println();
    }
}
```

**Step 2: Compile**

```bash
cd tests/fixtures && javac --release 21 Hello.java
```

**Step 3: Verify bytecode**

```bash
cargo run -- dump tests/fixtures/Hello.class
```

Expect to see: `getstatic` (System.out), `ldc` (string), `invokevirtual` (PrintStream.println).

**Step 4: Commit**

```bash
git add tests/fixtures/Hello.java tests/fixtures/Hello.class
git commit -m "test(phase11): add Hello fixture for native println stubs"
```

---

## Task 2: NativeRegistry Struct + NativeHandler Type

Add the native method registry to the interpreter crate.

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (add NativeRegistry near ClassRegistry)

**Step 1: Write failing test**

Add test at the bottom of the test module in `lib.rs`:

```rust
#[test]
fn native_registry_stores_and_retrieves() {
    fn dummy_handler(_args: &[Slot], _heap: &mut Heap, _out: &mut dyn std::io::Write) -> VmResult<Option<Slot>> {
        Ok(Some(Slot::Int(99)))
    }
    let mut natives = NativeRegistry::new();
    natives.register("Foo", "bar", "(I)I", dummy_handler);
    let handler = natives.get("Foo", "bar", "(I)I");
    assert!(handler.is_some());
    let mut heap = Heap::new();
    let mut out = Vec::new();
    let result = handler.unwrap()(&[Slot::Int(1)], &mut heap, &mut out).unwrap();
    assert_eq!(result, Some(Slot::Int(99)));
}

#[test]
fn native_registry_returns_none_for_missing() {
    let natives = NativeRegistry::new();
    assert!(natives.get("Foo", "bar", "(I)I").is_none());
}
```

**Step 2: Run tests — expect compile failure**

```bash
cargo test -p duke-interpreter -- native_registry 2>&1
```

Expected: doesn't compile — `NativeRegistry` doesn't exist.

**Step 3: Implement NativeRegistry**

Add near line 62 (after `ClassRegistry`):

```rust
use std::io::Write;

/// Signature for native method implementations.
///
/// Arguments:
/// - `&[Slot]`: method arguments (including `this` in slot 0 for instance methods)
/// - `&mut Heap`: the object heap for reading/writing objects
/// - `&mut dyn Write`: output sink (stdout in production, Vec<u8> in tests)
pub type NativeHandler = fn(&[Slot], &mut Heap, &mut dyn Write) -> VmResult<Option<Slot>>;

/// Registry of native method implementations.
///
/// Maps `(class_name, method_name, descriptor)` to a Rust function pointer.
pub struct NativeRegistry {
    methods: HashMap<(String, String, String), NativeHandler>,
}

impl NativeRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }

    /// Register a native method handler.
    pub fn register(
        &mut self,
        class: &str,
        method: &str,
        descriptor: &str,
        handler: NativeHandler,
    ) {
        self.methods.insert(
            (class.to_string(), method.to_string(), descriptor.to_string()),
            handler,
        );
    }

    /// Look up a native handler for the given class/method/descriptor.
    #[must_use]
    pub fn get(&self, class: &str, method: &str, descriptor: &str) -> Option<&NativeHandler> {
        self.methods.get(&(
            class.to_string(),
            method.to_string(),
            descriptor.to_string(),
        ))
    }
}

impl Default for NativeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Run tests — expect pass**

```bash
cargo test -p duke-interpreter -- native_registry
```

Expected: 2 tests pass.

**Step 5: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): add NativeRegistry and NativeHandler type"
```

---

## Task 3: Wire NativeRegistry + stdout Into execute_class

This is the big integration task. Changes:
1. Add `&mut dyn Write` parameter to `execute_class`
2. Store `NativeRegistry` on `ClassRegistry`
3. Add `bootstrap_stdlib()` to pre-register System/PrintStream
4. Check native registry in invokevirtual/invokespecial/invokestatic dispatch
5. Update all call sites (tests + binary)
6. Add integration tests for Hello.class

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`
- Modify: `duke/src/main.rs`

### Step 1: Write failing integration tests

Add at the end of the test module:

```rust
// ---- Phase 11: native println tests ----

fn load_hello_class() -> ClassContext {
    let bytes = std::fs::read("tests/fixtures/Hello.class").expect("Hello.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn hello_greet_prints_to_output() {
    let ctx = load_hello_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    bootstrap_stdlib(&mut registry);
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    let mut heap = Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut out,
        "Hello", "greet", "()V", &[],
    ).unwrap();
    assert_eq!(result, None);
    assert_eq!(String::from_utf8_lossy(&out), "Hello, Duke!\n");
}

#[test]
fn hello_print_num() {
    let ctx = load_hello_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    bootstrap_stdlib(&mut registry);
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    let mut heap = Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut out,
        "Hello", "printNum", "(I)V", &[Slot::Int(42)],
    ).unwrap();
    assert_eq!(result, None);
    assert_eq!(String::from_utf8_lossy(&out), "42\n");
}

#[test]
fn hello_greet_and_return() {
    let ctx = load_hello_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    bootstrap_stdlib(&mut registry);
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    let mut heap = Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut out,
        "Hello", "greetAndReturn", "()I", &[],
    ).unwrap();
    assert_eq!(result, Some(Slot::Int(42)));
    assert_eq!(String::from_utf8_lossy(&out), "Greetings!\n");
}

#[test]
fn hello_blank_line() {
    let ctx = load_hello_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    bootstrap_stdlib(&mut registry);
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    let mut heap = Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut out,
        "Hello", "blankLine", "()V", &[],
    ).unwrap();
    assert_eq!(result, None);
    assert_eq!(String::from_utf8_lossy(&out), "\n");
}
```

### Step 2: Add `natives` field to ClassRegistry

```rust
pub struct ClassRegistry {
    classes: HashMap<String, ClassContext>,
    natives: NativeRegistry,
}

impl ClassRegistry {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            natives: NativeRegistry::new(),
        }
    }

    /// Get a reference to the native registry.
    pub fn natives(&self) -> &NativeRegistry {
        &self.natives
    }

    /// Get a mutable reference to the native registry.
    pub fn natives_mut(&mut self) -> &mut NativeRegistry {
        &mut self.natives
    }
}
```

### Step 3: Implement `bootstrap_stdlib()`

Add as a public function:

```rust
/// Bootstrap minimal JDK standard library classes for native method support.
///
/// Registers:
/// - `java/lang/System` with static field `out` (Reference to a PrintStream)
/// - `java/io/PrintStream` as a heap object
/// - Native handlers for `PrintStream.println(Ljava/lang/String;)V`,
///   `PrintStream.println(I)V`, and `PrintStream.println()V`
pub fn bootstrap_stdlib(registry: &mut ClassRegistry, heap: &mut Heap) {
    // Allocate a PrintStream object on the heap.
    let ps_ref = heap.allocate("java/io/PrintStream".to_string(), 0);

    // Create java/lang/System ClassContext with a single static field `out`.
    let system_ctx = ClassContext {
        class_name: "java/lang/System".to_string(),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "out".to_string(),
            descriptor: "Ljava/io/PrintStream;".to_string(),
            is_static: true,
        }],
        static_fields: vec![Slot::Reference(Some(ps_ref))],
        instance_field_count: 0,
    };
    registry.register(system_ctx);

    // Create java/io/PrintStream ClassContext (empty — all methods are native).
    let ps_ctx = ClassContext {
        class_name: "java/io/PrintStream".to_string(),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
    };
    registry.register(ps_ctx);

    // Register native println handlers.
    registry.natives_mut().register(
        "java/io/PrintStream", "println", "(Ljava/lang/String;)V",
        native_println_string,
    );
    registry.natives_mut().register(
        "java/io/PrintStream", "println", "(I)V",
        native_println_int,
    );
    registry.natives_mut().register(
        "java/io/PrintStream", "println", "()V",
        native_println_void,
    );
}

/// Native: `PrintStream.println(String)` — prints string content + newline.
fn native_println_string(args: &[Slot], heap: &mut Heap, out: &mut dyn Write) -> VmResult<Option<Slot>> {
    // args[0] = this (PrintStream ref), args[1] = String ref
    let string_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => {
            writeln!(out, "null").ok();
            return Ok(None);
        }
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let obj = heap.get(string_ref)?;
    let text = obj.string_value.as_deref().unwrap_or("null");
    writeln!(out, "{text}").ok();
    Ok(None)
}

/// Native: `PrintStream.println(int)` — prints integer + newline.
fn native_println_int(args: &[Slot], _heap: &mut Heap, out: &mut dyn Write) -> VmResult<Option<Slot>> {
    // args[0] = this (PrintStream ref), args[1] = int value
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    writeln!(out, "{val}").ok();
    Ok(None)
}

/// Native: `PrintStream.println()` — prints empty newline.
fn native_println_void(_args: &[Slot], _heap: &mut Heap, out: &mut dyn Write) -> VmResult<Option<Slot>> {
    writeln!(out).ok();
    Ok(None)
}
```

### Step 4: Add `stdout` parameter to `execute_class` and wire native dispatch

Change `execute_class` signature:

```rust
pub fn execute_class(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>> {
```

At the `invokevirtual`/`invokespecial` fallback (where `callee_idx` is `None`, lines ~2119-2128), insert a native registry check **before** the no-op pop:

```rust
None => {
    // Check native registry before no-op fallback.
    if let Some(handler) = registry.natives().get(&callee_class, &callee_name, &callee_desc) {
        let handler = *handler;
        let arg_count = parse_arg_count(&callee_desc);
        let mut native_args: Vec<Slot> = (0..arg_count)
            .map(|_| frame.pop())
            .collect::<VmResult<Vec<_>>>()?;
        native_args.reverse();
        let this_slot = frame.pop()?;  // pop `this`
        native_args.insert(0, this_slot);
        let result = handler(&native_args, heap, stdout)?;
        if let Some(val) = result {
            frame.push(val)?;
        }
        idx += 1;
        continue;
    }
    // Unloadable or missing — pop args + this and continue.
    let arg_count = parse_arg_count(&callee_desc);
    for _ in 0..arg_count {
        frame.pop()?;
    }
    frame.pop()?; // pop `this`
    idx += 1;
    continue;
}
```

At `invokestatic` (line ~1372), insert native check **before** the `MethodNotFound` error:

```rust
// After ensure_loaded, when looking up method:
let callee_idx = {
    let ctx = registry.get(&callee_class)?;
    ctx.methods
        .iter()
        .position(|m| m.name == callee_name && m.descriptor == callee_desc)
};
match callee_idx {
    Some(i) => { /* existing dispatch code */ }
    None => {
        // Check native registry before erroring.
        if let Some(handler) = registry.natives().get(&callee_class, &callee_name, &callee_desc) {
            let handler = *handler;
            let arg_count = parse_arg_count(&callee_desc);
            let mut native_args: Vec<Slot> = (0..arg_count)
                .map(|_| frame.pop())
                .collect::<VmResult<Vec<_>>>()?;
            native_args.reverse();
            let result = handler(&native_args, heap, stdout)?;
            if let Some(val) = result {
                frame.push(val)?;
            }
            idx += 1;
            continue;
        }
        return Err(VmError::MethodNotFound {
            name: callee_name,
            descriptor: callee_desc,
        });
    }
}
```

### Step 5: Update all existing tests

All existing `execute_class` calls need the new `stdout` parameter. Add `&mut std::io::Write`:

Pattern: find every `execute_class(` in test code, add `&mut Vec::new()` or `&mut sink` after `&mut heap`.

For tests that don't check output:
```rust
let mut sink = Vec::new();
execute_class(&mut registry, &loader, &mut heap, &mut sink, ...)
```

### Step 6: Update binary crate

In `duke/src/main.rs`, update `exec_method`:
```rust
let mut stdout = std::io::stdout();
match execute_class(
    &mut registry,
    &loader,
    &mut heap,
    &mut stdout,
    &entry_class,
    method_name,
    &descriptor,
    &int_args,
)
```

Also add `bootstrap_stdlib` call after `registry.register(ctx)`:
```rust
registry.register(ctx);
bootstrap_stdlib(&mut registry, &mut heap);
```

Update imports:
```rust
use duke_interpreter::{ClassRegistry, bootstrap_stdlib, build_class_context, execute_class};
```

### Step 7: Run all tests — expect pass

```bash
cargo test 2>&1
```

Expected: all 147 existing tests + 4 new Hello tests pass (151+).

### Step 8: Run clippy

```bash
cargo clippy --workspace -- -W clippy::pedantic -W clippy::nursery 2>&1
```

Expected: zero errors.

### Step 9: Demo runs

```bash
cargo run -- exec tests/fixtures/Hello.class greet
# Expected stdout: Hello, Duke!

cargo run -- exec tests/fixtures/Hello.class printNum 42
# Expected stdout: 42

cargo run -- exec tests/fixtures/Hello.class greetAndReturn
# Expected stdout: Greetings!
# Expected return: Int(42)

cargo run -- exec tests/fixtures/HelloWorld.class main
# Expected stdout: Hello, World!
# (Note: main takes String[] — will need to pass empty ref or handle differently)
```

### Step 10: Commit

```bash
git add crates/duke-interpreter/src/lib.rs duke/src/main.rs
git commit -m "feat(interpreter): native method registry with println stubs — Hello World works"
```

---

## Task 4: Update Plan Doc + Memory

**Step 1: Mark plan as complete** — add `COMPLETE` to header
**Step 2: Update MEMORY.md** — add Phase 11 section

---

## Wave Execution Strategy

**Wave 1 (parallel):**
- Agent A: Task 1 (Hello.java fixture)
- Agent B: Task 2 (NativeRegistry struct + tests)

**Wave 2 (sequential):**
- Agent C: Task 3 (full integration — wiring, dispatch, existing test updates, binary update)

**Wave 3 (leader):**
- Verify all tests pass, run demos, commit, update memory
