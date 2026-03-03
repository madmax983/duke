# Phase 17: invokedynamic — StringConcatFactory + LambdaMetafactory

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the `invokedynamic` opcode with native short-circuits for StringConcatFactory (Java 9+ string `+`) and LambdaMetafactory (lambdas/method references), unlocking modern Java, Kotlin, and Scala bytecode.

**Architecture:** Parse the `BootstrapMethods` class attribute in duke-classfile. In the interpreter, when `invokedynamic` is hit, resolve the bootstrap method handle from the constant pool, identify the factory by class name, and short-circuit: StringConcatFactory parses a recipe template and substitutes operand-stack args inline; LambdaMetafactory creates a synthetic proxy HeapObject that stores captured variables, with SAM dispatch handled specially in invokevirtual/invokeinterface.

**Tech Stack:** Rust, duke-classfile (attribute parsing), duke-interpreter (opcode + dispatch), duke-gc (heap allocation)

---

## Background

### How invokedynamic works (JVM Spec §6.5)

The `invokedynamic` instruction references a `CpEntry::InvokeDynamic` in the constant pool:
```
InvokeDynamic {
    bootstrap_method_attr_index: u16,  // index into BootstrapMethods attribute
    name_and_type_index: CpIndex,      // call site name + descriptor
}
```

The `BootstrapMethods` attribute (§4.7.23) is a **class-level** attribute:
```
BootstrapMethods {
    num_bootstrap_methods: u16,
    bootstrap_methods[]: {
        bootstrap_method_ref: u16,       // CP index → MethodHandle
        num_bootstrap_arguments: u16,
        bootstrap_arguments[]: u16,      // CP indices → constants
    }
}
```

A real JVM calls the bootstrap method once per call site, gets back a `CallSite` wrapping a `MethodHandle`, and caches it. We skip all that machinery and **short-circuit known factories directly**.

### StringConcatFactory (Java 9+ string `+`)

`javac` compiles `"Hello " + name + "!"` to:
```
aload_1                     // push name
invokedynamic #N, 0         // makeConcatWithConstants:(Ljava/lang/String;)Ljava/lang/String;
```

Bootstrap args: `[String recipe]` where recipe uses `\u0001` for dynamic arg placeholders.
Recipe `"Hello \u0001!"` + 1 stack arg → concatenated String.

### LambdaMetafactory (lambdas)

`javac` compiles `x -> x * 2` to a static method `lambda$method$0` and:
```
invokedynamic #N, 0         // apply:()LIntOp;
```

Bootstrap args: `[MethodType erased, MethodHandle impl, MethodType specialized]`.
Creates a proxy object implementing the functional interface, delegating to the impl method.

---

## Task 1: BootstrapMethods Parsing + Fixtures

**Files:**
- Modify: `crates/duke-classfile/src/types.rs`
- Modify: `crates/duke-classfile/src/parser.rs`
- Modify: `crates/duke-interpreter/src/lib.rs` (ClassContext + build_class_context)
- Create: `tests/fixtures/StringConcatTest.java`
- Create: `tests/fixtures/LambdaTest.java`

### Step 1: Add BootstrapMethodEntry to types.rs

In `crates/duke-classfile/src/types.rs`, add the struct and AttributeData variant:

```rust
/// Single entry in the BootstrapMethods attribute (§4.7.23).
#[derive(Debug, Clone)]
pub struct BootstrapMethodEntry {
    /// CP index pointing to a CONSTANT_MethodHandle.
    pub method_ref: CpIndex,
    /// CP indices pointing to static arguments (String, MethodType, MethodHandle, etc.).
    pub arguments: Vec<CpIndex>,
}
```

Add to the `AttributeData` enum (before the `Raw` variant):

```rust
    /// BootstrapMethods attribute (§4.7.23) — required for invokedynamic.
    BootstrapMethods(Vec<BootstrapMethodEntry>),
```

### Step 2: Parse BootstrapMethods in parser.rs

In `crates/duke-classfile/src/parser.rs`, add a new arm in `decode_known_attribute()` (inside the `match name` block, before the `_ => Raw` fallback):

```rust
        "BootstrapMethods" => {
            let num = c.read_u16()?;
            let mut entries = Vec::with_capacity(num as usize);
            for _ in 0..num {
                let method_ref = c.read_cp_index()?;
                let num_args = c.read_u16()?;
                let mut arguments = Vec::with_capacity(num_args as usize);
                for _ in 0..num_args {
                    arguments.push(c.read_cp_index()?);
                }
                entries.push(BootstrapMethodEntry {
                    method_ref,
                    arguments,
                });
            }
            AttributeData::BootstrapMethods(entries)
        }
```

Add the import for `BootstrapMethodEntry` at the top of parser.rs alongside existing type imports.

### Step 3: Add bootstrap_methods to ClassContext

In `crates/duke-interpreter/src/lib.rs`, add a new field to `ClassContext` (line ~59):

```rust
pub struct ClassContext {
    pub class_name: String,
    pub super_class: Option<String>,
    pub constant_pool: Vec<Option<CpEntry>>,
    pub methods: Vec<MethodEntry>,
    pub fields: Vec<FieldEntry>,
    pub static_fields: Vec<Slot>,
    pub instance_field_count: usize,
    /// BootstrapMethods entries from the class attribute (needed for invokedynamic).
    pub bootstrap_methods: Vec<duke_classfile::types::BootstrapMethodEntry>,
}
```

Update **every** place that constructs a `ClassContext`:
- All synthetic contexts in `bootstrap_stdlib()` — add `bootstrap_methods: Vec::new()`
- The `ClassContext` in `build_class_context()` — populated from class attributes (see step 4)
- The `ClassContext` in `ensure_loaded()` (if it constructs one)

### Step 4: Extract bootstrap methods in build_class_context()

In `build_class_context()` (line ~4540), before the final `ClassContext { ... }` construction, add:

```rust
    // Extract BootstrapMethods from class-level attributes.
    let bootstrap_methods = cf
        .attributes
        .iter()
        .find_map(|a| {
            if let AttributeData::BootstrapMethods(entries) = &a.data {
                Some(entries.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();
```

Then add `bootstrap_methods,` to the `ClassContext { ... }` struct literal.

### Step 5: Create test fixtures

**`tests/fixtures/StringConcatTest.java`** — compile with `javac` (JDK 21 default, uses invokedynamic):

```java
public class StringConcatTest {
    public static int testSimple() {
        String name = "World";
        String result = "Hello, " + name + "!";
        return result.equals("Hello, World!") ? 1 : 0;
    }

    public static int testInt() {
        int x = 42;
        String result = "Value: " + x;
        return result.equals("Value: 42") ? 1 : 0;
    }

    public static int testChain() {
        int a = 3, b = 5;
        String result = a + " + " + b + " = " + (a + b);
        return result.equals("3 + 5 = 8") ? 1 : 0;
    }

    public static int testBoolean() {
        boolean flag = true;
        String result = "active=" + flag;
        return result.equals("active=true") ? 1 : 0;
    }

    public static int testEmpty() {
        String s = "";
        String result = s + "ok";
        return result.equals("ok") ? 1 : 0;
    }
}
```

**`tests/fixtures/LambdaTest.java`** — uses invokedynamic for lambda creation:

```java
public class LambdaTest {
    interface IntOp {
        int apply(int x);
    }

    public static int applyOp(IntOp op, int val) {
        return op.apply(val);
    }

    // Simple lambda, no capture
    public static int testDouble() {
        return applyOp(x -> x * 2, 5);
    }

    // Lambda with variable capture
    public static int testCapture() {
        int base = 100;
        return applyOp(x -> x + base, 7);
    }

    // Static method reference
    public static int negate(int x) {
        return -x;
    }

    public static int testMethodRef() {
        return applyOp(LambdaTest::negate, 42);
    }

    // Multiple captures
    public static int testMultiCapture() {
        int a = 10, b = 20;
        return applyOp(x -> x + a + b, 3);
    }
}
```

Compile both:
```bash
cd tests/fixtures
javac StringConcatTest.java
javac LambdaTest.java
```

This produces `StringConcatTest.class`, `LambdaTest.class`, and `LambdaTest$IntOp.class`.

### Step 6: Run tests to verify nothing broke

```bash
cargo test
```

Expected: all 218 existing tests pass, no regressions from the new field.

### Step 7: Commit

```bash
git add crates/duke-classfile/src/types.rs crates/duke-classfile/src/parser.rs \
        crates/duke-interpreter/src/lib.rs \
        tests/fixtures/StringConcatTest.java tests/fixtures/StringConcatTest.class \
        tests/fixtures/LambdaTest.java tests/fixtures/LambdaTest.class \
        tests/fixtures/LambdaTest\$IntOp.class
git commit -m "feat(classfile): parse BootstrapMethods attribute, add invokedynamic fixtures"
```

---

## Task 2: invokedynamic Opcode + StringConcatFactory

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Depends on:** Task 1

### Step 1: Add CP resolution helpers

Add these helper functions near the existing `resolve_methodref()` and `resolve_class_name()` functions (around line ~4796):

```rust
/// Resolve a MethodHandle CP entry to (reference_kind, class_name, method_name, descriptor).
fn resolve_method_handle(
    cp: &[Option<CpEntry>],
    cp_idx: usize,
) -> VmResult<(u8, String, String, String)> {
    let (kind, ref_idx) = match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::MethodHandle {
            reference_kind,
            reference_index,
        }) => (*reference_kind, reference_index.0 as usize),
        _ => return Err(VmError::InvalidCpIndex { index: cp_idx }),
    };
    let (class_name, method_name, descriptor) = resolve_methodref(cp, ref_idx)?;
    Ok((kind, class_name, method_name, descriptor))
}

/// Resolve a NameAndType CP entry to (name, descriptor).
fn resolve_name_and_type(
    cp: &[Option<CpEntry>],
    cp_idx: usize,
) -> VmResult<(String, String)> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::NameAndType {
            name_index,
            descriptor_index,
        }) => {
            let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => return Err(VmError::InvalidCpIndex { index: name_index.0 as usize }),
            };
            let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(VmError::InvalidCpIndex {
                        index: descriptor_index.0 as usize,
                    })
                }
            };
            Ok((name, desc))
        }
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Resolve a CP String entry to its UTF-8 content. Also handles bare Utf8 entries.
fn resolve_cp_string(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::String { string_index }) => {
            match cp.get(string_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => Err(VmError::InvalidCpIndex {
                    index: string_index.0 as usize,
                }),
            }
        }
        Some(CpEntry::Utf8(s)) => Ok(s.clone()),
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}
```

### Step 2: Add StringConcatFactory recipe handler

Add this function near the other native handlers:

```rust
/// Execute a StringConcatFactory recipe: walk the recipe string, replacing
/// `\u{1}` placeholders with stringified dynamic args from the operand stack.
fn execute_string_concat_recipe(
    recipe: &str,
    dynamic_args: &[Slot],
    constants: &[String],
    heap: &mut duke_gc::Heap,
) -> VmResult<Slot> {
    let mut result = String::new();
    let mut dyn_idx = 0;
    let mut const_idx = 0;

    for ch in recipe.chars() {
        match ch {
            '\u{1}' => {
                // Dynamic arg from operand stack
                if dyn_idx < dynamic_args.len() {
                    stringify_slot(&dynamic_args[dyn_idx], heap, &mut result)?;
                    dyn_idx += 1;
                }
            }
            '\u{2}' => {
                // Constant from bootstrap args
                if const_idx < constants.len() {
                    result.push_str(&constants[const_idx]);
                    const_idx += 1;
                }
            }
            other => result.push(other),
        }
    }

    let r = heap.allocate_string(result);
    Ok(Slot::Reference(Some(r)))
}

/// Convert a Slot to its string representation (like Java's String.valueOf).
fn stringify_slot(slot: &Slot, heap: &duke_gc::Heap, out: &mut String) -> VmResult<()> {
    match slot {
        Slot::Int(v) => out.push_str(&v.to_string()),
        Slot::Long(v) => out.push_str(&v.to_string()),
        Slot::Float(v) => out.push_str(&format_java_float(*v)),
        Slot::Double(v) => out.push_str(&format_java_double(*v)),
        Slot::Reference(None) => out.push_str("null"),
        Slot::Reference(Some(r)) => {
            let obj = heap.get(*r)?;
            if let Some(s) = &obj.string_value {
                out.push_str(s);
            } else {
                // Fallback: ClassName@hashcode
                out.push_str(&obj.class_name);
                out.push('@');
                out.push_str(&format!("{:x}", r));
            }
        }
        Slot::ReturnAddress(v) => out.push_str(&v.to_string()),
    }
    Ok(())
}

/// Format a float like Java's Float.toString (no trailing zeros except "X.0" form).
fn format_java_float(v: f32) -> String {
    if v.is_nan() { return "NaN".to_string(); }
    if v.is_infinite() { return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string(); }
    let s = format!("{v}");
    if s.contains('.') { s } else { format!("{v}.0") }
}

/// Format a double like Java's Double.toString.
fn format_java_double(v: f64) -> String {
    if v.is_nan() { return "NaN".to_string(); }
    if v.is_infinite() { return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string(); }
    let s = format!("{v}");
    if s.contains('.') { s } else { format!("{v}.0") }
}
```

### Step 3: Add the invokedynamic opcode handler

In the `execute_class` function's main opcode match (around line ~3070 where other instructions are handled), add the `Invokedynamic` arm. Place it near the other invoke instructions:

```rust
            Instruction::Invokedynamic(cp_idx) => {
                let cp_idx_val = usize::from(cp_idx.0);

                // 1. Resolve InvokeDynamic CP entry.
                let (bsm_idx, call_name, call_desc) = {
                    let ctx = registry.get(&current_class)?;
                    let cp = &ctx.constant_pool;
                    match cp.get(cp_idx_val).and_then(|e| e.as_ref()) {
                        Some(CpEntry::InvokeDynamic {
                            bootstrap_method_attr_index,
                            name_and_type_index,
                        }) => {
                            let (name, desc) =
                                resolve_name_and_type(cp, name_and_type_index.0 as usize)?;
                            (*bootstrap_method_attr_index as usize, name, desc)
                        }
                        _ => return Err(VmError::InvalidCpIndex { index: cp_idx_val }),
                    }
                };

                // 2. Look up the bootstrap method entry.
                let (bsm_kind, bsm_class, _bsm_name, _bsm_desc, bsm_args) = {
                    let ctx = registry.get(&current_class)?;
                    let bsm_entry = ctx
                        .bootstrap_methods
                        .get(bsm_idx)
                        .ok_or(VmError::InvalidCpIndex { index: bsm_idx })?;
                    let (kind, class, name, desc) =
                        resolve_method_handle(&ctx.constant_pool, bsm_entry.method_ref.0 as usize)?;
                    let args: Vec<CpIndex> = bsm_entry.arguments.clone();
                    (kind, class, name, desc, args)
                };

                // 3. Dispatch based on bootstrap method class.
                if bsm_class == "java/lang/invoke/StringConcatFactory" {
                    // --- StringConcatFactory.makeConcatWithConstants ---
                    // Pop dynamic args from the stack (count from call_desc).
                    let arg_count = parse_arg_count(&call_desc);
                    let mut dynamic_args: Vec<Slot> = (0..arg_count)
                        .map(|_| frame.pop())
                        .collect::<VmResult<Vec<_>>>()?;
                    dynamic_args.reverse();

                    // Resolve recipe (first bootstrap arg) and constants (remaining).
                    let (recipe, constants) = {
                        let ctx = registry.get(&current_class)?;
                        let cp = &ctx.constant_pool;
                        let recipe = if !bsm_args.is_empty() {
                            resolve_cp_string(cp, bsm_args[0].0 as usize)?
                        } else {
                            String::new()
                        };
                        let mut consts = Vec::new();
                        for arg_idx in bsm_args.iter().skip(1) {
                            // Skip MethodType args (they're not string constants).
                            if let Ok(s) = resolve_cp_string(cp, arg_idx.0 as usize) {
                                consts.push(s);
                            }
                        }
                        (recipe, consts)
                    };

                    let result =
                        execute_string_concat_recipe(&recipe, &dynamic_args, &constants, heap)?;
                    frame.push(result)?;
                } else if bsm_class == "java/lang/invoke/LambdaMetafactory" {
                    // --- LambdaMetafactory --- handled in Task 3.
                    // For now, push a null reference as placeholder.
                    let arg_count = parse_arg_count(&call_desc);
                    for _ in 0..arg_count {
                        frame.pop()?;
                    }
                    frame.push(Slot::Reference(None))?;
                } else {
                    // Unknown bootstrap method — pop args and push null.
                    let arg_count = parse_arg_count(&call_desc);
                    for _ in 0..arg_count {
                        frame.pop()?;
                    }
                    // If return type is non-void, push a default.
                    if !call_desc.ends_with(")V") {
                        frame.push(Slot::Reference(None))?;
                    }
                }
            }
```

### Step 4: Write StringConcatFactory integration tests

Add to the `#[cfg(test)]` module in `crates/duke-interpreter/src/lib.rs`:

```rust
    // ---- Phase 17: StringConcatFactory ----

    #[test]
    fn string_concat_simple() {
        let r = run_fixture("StringConcatTest", "testSimple", "()I", &[]);
        assert_eq!(r, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_int() {
        let r = run_fixture("StringConcatTest", "testInt", "()I", &[]);
        assert_eq!(r, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_chain() {
        let r = run_fixture("StringConcatTest", "testChain", "()I", &[]);
        assert_eq!(r, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_boolean() {
        let r = run_fixture("StringConcatTest", "testBoolean", "()I", &[]);
        assert_eq!(r, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_empty() {
        let r = run_fixture("StringConcatTest", "testEmpty", "()I", &[]);
        assert_eq!(r, Some(Slot::Int(1)));
    }
```

### Step 5: Run tests

```bash
cargo test -p duke-interpreter
```

Expected: all existing tests + 5 new StringConcatFactory tests pass.

### Step 6: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): implement invokedynamic + StringConcatFactory recipe engine"
```

---

## Task 3: LambdaMetafactory + Lambda Dispatch

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Depends on:** Task 2

### Step 1: Add LambdaInfo struct and registry

Add near the `ClassRegistry` definition (around line ~65):

```rust
/// Metadata for a lambda proxy object created by LambdaMetafactory.
#[derive(Debug, Clone)]
struct LambdaInfo {
    /// The implementation method's class (e.g., "LambdaTest").
    pub impl_class: String,
    /// The implementation method's name (e.g., "lambda$doubleIt$0").
    pub impl_method: String,
    /// The implementation method's descriptor (e.g., "(I)I").
    pub impl_desc: String,
    /// MethodHandle reference_kind (5=invokeVirtual, 6=invokeStatic, etc.)
    pub impl_kind: u8,
    /// The SAM interface method name (e.g., "apply").
    pub sam_method: String,
    /// The SAM interface method descriptor (e.g., "(I)I").
    pub sam_desc: String,
    /// Number of captured variables stored as fields in the proxy object.
    pub captured_count: usize,
}
```

Add a `lambdas` field to `ClassRegistry`:

```rust
pub struct ClassRegistry {
    classes: HashMap<String, ClassContext>,
    natives: NativeRegistry,
    initialized: HashSet<String>,
    /// Lambda proxy class metadata, keyed by synthetic class name.
    lambdas: HashMap<String, LambdaInfo>,
    /// Counter for generating unique lambda class names.
    lambda_counter: u64,
}
```

Update `ClassRegistry::new()` to initialize the new fields:

```rust
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            natives: NativeRegistry::new(),
            initialized: HashSet::new(),
            lambdas: HashMap::new(),
            lambda_counter: 0,
        }
    }
```

Add accessor methods:

```rust
    /// Register a lambda proxy class and return its synthetic class name.
    fn register_lambda(&mut self, info: LambdaInfo) -> String {
        let name = format!("$$Lambda${}", self.lambda_counter);
        self.lambda_counter += 1;
        self.lambdas.insert(name.clone(), info);
        name
    }

    /// Look up lambda info by class name.
    fn get_lambda(&self, class_name: &str) -> Option<&LambdaInfo> {
        self.lambdas.get(class_name)
    }
```

### Step 2: Implement LambdaMetafactory handler

Replace the placeholder `LambdaMetafactory` branch in the `Invokedynamic` handler (from Task 2) with:

```rust
                } else if bsm_class == "java/lang/invoke/LambdaMetafactory" {
                    // --- LambdaMetafactory.metafactory ---
                    // Bootstrap args: [MethodType erased, MethodHandle impl, MethodType specialized]

                    // Resolve the implementation MethodHandle (bootstrap arg index 1).
                    let (impl_kind, impl_class, impl_method, impl_desc) = {
                        let ctx = registry.get(&current_class)?;
                        let cp = &ctx.constant_pool;
                        if bsm_args.len() < 3 {
                            return Err(VmError::Unimplemented {
                                detail: "LambdaMetafactory requires 3 bootstrap args".to_string(),
                            });
                        }
                        resolve_method_handle(cp, bsm_args[1].0 as usize)?
                    };

                    // The call site name (from NameAndType) is the SAM method name.
                    let sam_method = call_name.clone();

                    // Resolve erased SAM descriptor from bootstrap arg 0.
                    let sam_desc = {
                        let ctx = registry.get(&current_class)?;
                        let cp = &ctx.constant_pool;
                        match cp.get(bsm_args[0].0 as usize).and_then(|e| e.as_ref()) {
                            Some(CpEntry::MethodType { descriptor_index }) => {
                                match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                                    Some(CpEntry::Utf8(s)) => s.clone(),
                                    _ => return Err(VmError::InvalidCpIndex {
                                        index: descriptor_index.0 as usize,
                                    }),
                                }
                            }
                            _ => return Err(VmError::InvalidCpIndex {
                                index: bsm_args[0].0 as usize,
                            }),
                        }
                    };

                    // Pop captured variables from the stack (described by call_desc args).
                    let captured_count = parse_arg_count(&call_desc);
                    let mut captured_args: Vec<Slot> = (0..captured_count)
                        .map(|_| frame.pop())
                        .collect::<VmResult<Vec<_>>>()?;
                    captured_args.reverse();

                    // Register the lambda metadata.
                    let lambda_info = LambdaInfo {
                        impl_class: impl_class.clone(),
                        impl_method: impl_method.clone(),
                        impl_desc: impl_desc.clone(),
                        impl_kind,
                        sam_method,
                        sam_desc,
                        captured_count,
                    };
                    let lambda_class = registry.register_lambda(lambda_info);

                    // Create the proxy HeapObject with captured vars as fields.
                    let r = heap.allocate(lambda_class.clone(), captured_count);
                    for (i, slot) in captured_args.into_iter().enumerate() {
                        heap.get_mut(r)?.fields[i] = slot;
                    }

                    // Ensure the impl class is loaded (the lambda body lives there).
                    let _ = registry.ensure_loaded(&impl_class, loader);

                    frame.push(Slot::Reference(Some(r)))?;
                }
```

### Step 3: Add lambda dispatch to invokeinterface

In the `Instruction::Invokeinterface` handler (around line ~4407), in the `None` branch after the native registry check fails and before the no-op fallback, add lambda dispatch:

```rust
                            // Check lambda registry for SAM dispatch.
                            if let Some(lambda_info) = registry.get_lambda(&actual_class).cloned() {
                                if callee_name == lambda_info.sam_method {
                                    // Build impl args: captured fields + SAM args.
                                    let this_ref = match &callee_args[0] {
                                        Slot::Reference(Some(r)) => *r,
                                        _ => return Err(VmError::NullPointerException),
                                    };
                                    let obj = heap.get(this_ref)?;
                                    let mut impl_args: Vec<Slot> = Vec::new();
                                    for i in 0..lambda_info.captured_count {
                                        impl_args.push(obj.fields[i].clone());
                                    }
                                    // SAM args (skip `this` at index 0).
                                    impl_args.extend(callee_args[1..].iter().cloned());

                                    // Ensure impl class is loaded.
                                    let _ = registry.ensure_loaded(&lambda_info.impl_class, loader);

                                    if lambda_info.impl_kind == 6 {
                                        // invokeStatic: dispatch to static impl method.
                                        let resolved = resolve_method_in_hierarchy(
                                            registry,
                                            loader,
                                            &lambda_info.impl_class,
                                            &lambda_info.impl_method,
                                            &lambda_info.impl_desc,
                                        );
                                        if let Some((dispatch_class, impl_idx)) = resolved {
                                            let (callee_pc_to_idx, callee_frame) = {
                                                let ctx = registry.get(&dispatch_class)?;
                                                let pci: HashMap<usize, usize> = ctx.methods[impl_idx]
                                                    .instructions
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(i, &(pc, _))| (pc, i))
                                                    .collect();
                                                let f = Frame::new(
                                                    usize::from(ctx.methods[impl_idx].max_stack),
                                                    usize::from(ctx.methods[impl_idx].max_locals),
                                                    impl_args,
                                                )?;
                                                (pci, f)
                                            };
                                            call_stack.push(CallFrame {
                                                frame,
                                                method_idx,
                                                pc_to_idx,
                                                resume_idx: idx + 1,
                                                class_name: current_class.clone(),
                                            });
                                            frame = callee_frame;
                                            method_idx = impl_idx;
                                            pc_to_idx = callee_pc_to_idx;
                                            current_class = dispatch_class;
                                            idx = 0;
                                            continue;
                                        }
                                    } else if lambda_info.impl_kind == 5 || lambda_info.impl_kind == 9 {
                                        // invokeVirtual / invokeInterface: first impl_arg is receiver.
                                        let resolved = resolve_method_in_hierarchy(
                                            registry,
                                            loader,
                                            &lambda_info.impl_class,
                                            &lambda_info.impl_method,
                                            &lambda_info.impl_desc,
                                        );
                                        if let Some((dispatch_class, impl_idx)) = resolved {
                                            let (callee_pc_to_idx, callee_frame) = {
                                                let ctx = registry.get(&dispatch_class)?;
                                                let pci: HashMap<usize, usize> = ctx.methods[impl_idx]
                                                    .instructions
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(i, &(pc, _))| (pc, i))
                                                    .collect();
                                                let f = Frame::new(
                                                    usize::from(ctx.methods[impl_idx].max_stack),
                                                    usize::from(ctx.methods[impl_idx].max_locals),
                                                    impl_args,
                                                )?;
                                                (pci, f)
                                            };
                                            call_stack.push(CallFrame {
                                                frame,
                                                method_idx,
                                                pc_to_idx,
                                                resume_idx: idx + 1,
                                                class_name: current_class.clone(),
                                            });
                                            frame = callee_frame;
                                            method_idx = impl_idx;
                                            pc_to_idx = callee_pc_to_idx;
                                            current_class = dispatch_class;
                                            idx = 0;
                                            continue;
                                        }
                                        // If not found as bytecode, try native.
                                        if let Some(handler) = registry
                                            .natives()
                                            .get(&lambda_info.impl_class, &lambda_info.impl_method, &lambda_info.impl_desc)
                                            .copied()
                                        {
                                            let result = handler(&impl_args, heap, stdout)?;
                                            if let Some(val) = result {
                                                frame.push(val)?;
                                            }
                                            idx += 1;
                                            continue;
                                        }
                                    }
                                    // Fallback: pop and continue.
                                    idx += 1;
                                    continue;
                                }
                            }
```

### Step 4: Add same lambda dispatch to invokevirtual

The invokevirtual handler (line ~3780) also needs lambda dispatch. In its `None` branch (line ~3805), after the native registry check, add the same lambda dispatch logic. However, invokevirtual resolves from the CP class (static type), not from the heap object. We need to peek at the actual receiver class.

**Before** the existing `None` arm's native check (line ~3806), add:

```rust
                    None => {
                        // Peek at actual receiver class for lambda dispatch.
                        // The args haven't been popped yet, so we need to peek.
                        let arg_count = parse_arg_count(&callee_desc);
                        let stack_len = frame.stack_len();
                        let this_pos = stack_len - arg_count - 1;
                        let actual_class_opt = if let Ok(Slot::Reference(Some(r))) = frame.peek_at(this_pos) {
                            heap.get(r).ok().map(|o| o.class_name.clone())
                        } else {
                            None
                        };

                        if let Some(ref actual_class) = actual_class_opt {
                            if let Some(lambda_info) = registry.get_lambda(actual_class).cloned() {
                                if callee_name == lambda_info.sam_method {
                                    // Pop args + this.
                                    let mut callee_args: Vec<Slot> = (0..arg_count)
                                        .map(|_| frame.pop())
                                        .collect::<VmResult<Vec<_>>>()?;
                                    callee_args.reverse();
                                    let this_slot = frame.pop()?;
                                    let this_ref = match &this_slot {
                                        Slot::Reference(Some(r)) => *r,
                                        _ => return Err(VmError::NullPointerException),
                                    };

                                    // Build impl args: captured fields + SAM args.
                                    let obj = heap.get(this_ref)?;
                                    let mut impl_args: Vec<Slot> = Vec::new();
                                    for i in 0..lambda_info.captured_count {
                                        impl_args.push(obj.fields[i].clone());
                                    }
                                    impl_args.extend(callee_args.iter().cloned());

                                    let _ = registry.ensure_loaded(&lambda_info.impl_class, loader);

                                    let resolved = resolve_method_in_hierarchy(
                                        registry,
                                        loader,
                                        &lambda_info.impl_class,
                                        &lambda_info.impl_method,
                                        &lambda_info.impl_desc,
                                    );
                                    if let Some((dispatch_class, impl_idx)) = resolved {
                                        let (callee_pc_to_idx, callee_frame) = {
                                            let ctx = registry.get(&dispatch_class)?;
                                            let pci: HashMap<usize, usize> = ctx.methods[impl_idx]
                                                .instructions
                                                .iter()
                                                .enumerate()
                                                .map(|(i, &(pc, _))| (pc, i))
                                                .collect();
                                            let f = Frame::new(
                                                usize::from(ctx.methods[impl_idx].max_stack),
                                                usize::from(ctx.methods[impl_idx].max_locals),
                                                impl_args,
                                            )?;
                                            (pci, f)
                                        };
                                        call_stack.push(CallFrame {
                                            frame,
                                            method_idx,
                                            pc_to_idx,
                                            resume_idx: idx + 1,
                                            class_name: current_class.clone(),
                                        });
                                        frame = callee_frame;
                                        method_idx = impl_idx;
                                        pc_to_idx = callee_pc_to_idx;
                                        current_class = dispatch_class;
                                        idx = 0;
                                        continue;
                                    }
                                }
                            }
                        }

                        // Check native registry before no-op fallback (existing code).
```

**Note:** This requires adding `peek_at(index)` and `stack_len()` methods to `Frame`. Add to `crates/duke-runtime/src/frame.rs`:

```rust
    /// Peek at the slot at a given absolute position in the operand stack (0-indexed from bottom).
    pub fn peek_at(&self, index: usize) -> VmResult<Slot> {
        self.operand_stack
            .get(index)
            .cloned()
            .ok_or(VmError::StackUnderflow)
    }

    /// Current depth of the operand stack.
    pub fn stack_len(&self) -> usize {
        self.operand_stack.len()
    }
```

### Step 5: Write LambdaMetafactory integration tests

Add to the `#[cfg(test)]` module:

```rust
    // ---- Phase 17: LambdaMetafactory ----

    #[test]
    fn lambda_simple_no_capture() {
        // x -> x * 2, applied to 5 → 10
        let r = run_fixture("LambdaTest", "testDouble", "()I", &[]);
        assert_eq!(r, Some(Slot::Int(10)));
    }

    #[test]
    fn lambda_with_capture() {
        // base=100, x -> x + base, applied to 7 → 107
        let r = run_fixture("LambdaTest", "testCapture", "()I", &[]);
        assert_eq!(r, Some(Slot::Int(107)));
    }

    #[test]
    fn lambda_method_reference() {
        // LambdaTest::negate applied to 42 → -42
        let r = run_fixture("LambdaTest", "testMethodRef", "()I", &[]);
        assert_eq!(r, Some(Slot::Int(-42)));
    }

    #[test]
    fn lambda_multi_capture() {
        // a=10, b=20, x -> x + a + b, applied to 3 → 33
        let r = run_fixture("LambdaTest", "testMultiCapture", "()I", &[]);
        assert_eq!(r, Some(Slot::Int(33)));
    }
```

### Step 6: Run all tests

```bash
cargo test
```

Expected: all 218 existing tests + 5 StringConcat + 4 Lambda = 227 tests passing.

### Step 7: Smoke test

```bash
# String concatenation
cargo run -- exec tests/fixtures/StringConcatTest.class testSimple
# Expected: Int(1)

# Lambda
cargo run -- exec tests/fixtures/LambdaTest.class testDouble
# Expected: Int(10)

cargo run -- exec tests/fixtures/LambdaTest.class testCapture
# Expected: Int(107)
```

### Step 8: Commit

```bash
git add crates/duke-interpreter/src/lib.rs crates/duke-runtime/src/frame.rs
git commit -m "feat(interpreter): implement LambdaMetafactory with captured var dispatch"
```

---

## Summary

| Deliverable | What it unlocks |
|---|---|
| BootstrapMethods attribute parsing | Foundation for all invokedynamic usage |
| invokedynamic opcode handler | Dispatch to known bootstrap factories |
| StringConcatFactory recipe engine | Java 9+ string `+` concatenation |
| LambdaMetafactory proxy generation | Lambda expressions, method references |
| Lambda SAM dispatch (invokeinterface + invokevirtual) | Calling lambdas through functional interfaces |
| 9 new integration tests | StringConcat (5) + Lambda (4) |

**After Phase 17:** `duke run` can execute modern Java programs compiled with default `javac` (no `--release 8` needed), including string concatenation with `+` and lambda expressions.
