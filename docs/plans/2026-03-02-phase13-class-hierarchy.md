# Phase 13: Class Hierarchy, Subtype Checks & String Natives Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `super_class` tracking to `ClassContext`, implement hierarchy-aware `instanceof`/`checkcast`/exception matching, fix `anewarray` element type bug, and add core String native methods.

**Architecture:** `ClassContext` gains `super_class: Option<String>`. A new `is_assignable_from(registry, loader, from, to)` function walks the class hierarchy from `from` up through superclasses to check if `to` is an ancestor. `java/lang/Object` is the implicit root (no super). Exception handler matching, `checkcast`, and `instanceof` all switch from exact-match to hierarchy-aware. String native methods (length, equals, valueOf, charAt) are registered alongside existing println stubs.

**Tech Stack:** Rust, duke-interpreter crate, duke-gc (Heap), duke-runtime (Slot/VmError)

---

## Background

### Current Gaps
1. `ClassContext` has no `super_class` field — `ClassFile.super_class` is parsed but discarded in `build_class_context()`
2. `checkcast`/`instanceof` use `o.class_name == check_type_name` (exact match)
3. `find_exception_handler()` uses `ct == class_name` (exact match) — `catch(Exception)` can't catch `RuntimeException`
4. `anewarray` hardcodes `"[Ljava/lang/Object;"` regardless of actual element type
5. No String instance methods (length, equals, etc.)

### Key Design Decisions
- `java/lang/Object` has `super_class: None` — it's the hierarchy root
- `is_assignable_from` walks up from the actual class toward Object, checking each step
- For unloadable superclasses, we soft-fail (return false) — same pattern as `ensure_loaded`
- String natives use the existing `NativeRegistry` + `string_value` on `HeapObject`

---

## Task 1: Test Fixtures

**Files:**
- Create: `tests/fixtures/Hierarchy.java` + `.class`
- Create: `tests/fixtures/ExceptionHierarchy.java` + `.class`

### Step 1: Write Hierarchy.java

```java
/** Tests class hierarchy for instanceof and checkcast. */
public class Hierarchy {
    /** Returns 1 if a Hierarchy instance passes instanceof Object. */
    public static int instanceOfObject() {
        Object obj = new Hierarchy();
        return (obj instanceof Object) ? 1 : 0; // should be 1
    }

    /** Returns 1 if casting Hierarchy to Object works (should always work). */
    public static int castToObject() {
        Hierarchy h = new Hierarchy();
        Object obj = (Object) h; // checkcast to Object — must not throw
        return (obj != null) ? 1 : 0;
    }

    /** Returns 1 if null instanceof anything is false. */
    public static int nullInstanceOf() {
        Object obj = null;
        return (obj instanceof Hierarchy) ? 1 : 0; // should be 0
    }
}
```

### Step 2: Write ExceptionHierarchy.java

```java
/** Tests exception hierarchy matching in catch clauses. */
public class ExceptionHierarchy {
    /** Throws RuntimeException, catches with Exception — should succeed. */
    public static int catchParent() {
        try {
            throw new RuntimeException("test");
        } catch (Exception e) {
            return 1; // caught by parent type
        }
    }

    /** Throws RuntimeException, catches with RuntimeException — exact match. */
    public static int catchExact() {
        try {
            throw new RuntimeException("test");
        } catch (RuntimeException e) {
            return 2;
        }
    }

    /** Throws Exception, tries to catch RuntimeException — should NOT match,
     *  then catches with Exception. */
    public static int catchWrongThenRight() {
        try {
            throw new Exception("test");
        } catch (RuntimeException e) {
            return 0; // should NOT get here
        } catch (Exception e) {
            return 3; // should get here
        }
    }

    /** Catches with Throwable (grandparent of RuntimeException). */
    public static int catchGrandparent() {
        try {
            throw new RuntimeException("test");
        } catch (Throwable t) {
            return 4;
        }
    }
}
```

### Step 3: Compile

```bash
cd tests/fixtures && javac --release 21 Hierarchy.java ExceptionHierarchy.java
```

### Step 4: Commit

```bash
git add tests/fixtures/Hierarchy.java tests/fixtures/Hierarchy.class tests/fixtures/ExceptionHierarchy.java tests/fixtures/ExceptionHierarchy.class
git commit -m "test(phase13): add Hierarchy and ExceptionHierarchy fixtures"
```

---

## Task 2: Add super_class to ClassContext + is_assignable_from + anewarray fix

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Add super_class field to ClassContext

```rust
pub struct ClassContext {
    pub class_name: String,
    /// Superclass name (None for java/lang/Object).
    pub super_class: Option<String>,
    pub constant_pool: Vec<Option<CpEntry>>,
    pub methods: Vec<MethodEntry>,
    pub fields: Vec<FieldEntry>,
    pub static_fields: Vec<Slot>,
    pub instance_field_count: usize,
}
```

### Step 2: Extract super_class in build_class_context

In `build_class_context()`, find where ClassContext is constructed and add super_class extraction. The ClassFile has `super_class: CpIndex` — resolve it to a class name:

```rust
let super_class = if cf.super_class.0 != 0 {
    resolve_class_name_from_cp(&cf.constant_pool, cf.super_class.0 as usize).ok()
} else {
    None
};
```

You'll need a helper to resolve class name from CP (similar to the existing resolve_class_name in the binary crate but working on Vec<Option<CpEntry>>):

```rust
fn resolve_class_name_from_cp(cp: &[Option<CpEntry>], idx: usize) -> VmResult<String> {
    match cp.get(idx) {
        Some(Some(CpEntry::Class { name_index })) => {
            match cp.get(name_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => Ok(s.clone()),
                _ => Err(VmError::InvalidCpIndex { index: name_index.0 as usize }),
            }
        }
        _ => Err(VmError::InvalidCpIndex { index: idx }),
    }
}
```

### Step 3: Update all synthetic ClassContext creation

In `bootstrap_stdlib`, the synthetic System and PrintStream ClassContext objects need `super_class`:
```rust
super_class: Some("java/lang/Object".to_string()),
```

### Step 4: Implement is_assignable_from

```rust
/// Check if `from` is a subtype of `to` (i.e., `from` can be assigned where `to` is expected).
///
/// Walks the class hierarchy from `from` upward through superclasses.
/// Returns `true` if `to` is found in the chain, or if `to` is "java/lang/Object".
fn is_assignable_from(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    from: &str,
    to: &str,
) -> bool {
    // Same type → always assignable.
    if from == to {
        return true;
    }
    // Everything is assignable to Object.
    if to == "java/lang/Object" {
        return true;
    }
    // Walk the hierarchy from `from` upward.
    let mut current = from.to_string();
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return false; // circular hierarchy — bail
        }
        // Try to load and get the class.
        let _ = registry.ensure_loaded(&current, loader);
        let super_name = match registry.get(&current) {
            Ok(ctx) => ctx.super_class.clone(),
            Err(_) => return false, // can't load — soft fail
        };
        match super_name {
            Some(s) if s == to => return true,
            Some(s) => current = s,
            None => return false, // reached Object without finding target
        }
    }
}
```

### Step 5: Update instanceof to use hierarchy

Find the instanceof match arm and replace exact match with:
```rust
Instruction::Instanceof(cp_idx) => {
    let obj_ref = frame.pop()?;
    match obj_ref {
        Slot::Reference(None) => frame.push(Slot::Int(0))?,
        Slot::Reference(Some(r)) => {
            let check_type = { /* resolve class name from cp_idx */ };
            let obj_class = heap.get(r)?.class_name.clone();
            let result = if is_assignable_from(registry, loader, &obj_class, &check_type) { 1 } else { 0 };
            frame.push(Slot::Int(result))?;
        }
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    }
}
```

### Step 6: Update checkcast to use hierarchy

Same pattern — replace exact match with `is_assignable_from`.

### Step 7: Update find_exception_handler to use hierarchy

Change the catch_type comparison from:
```rust
ct == class_name
```
to:
```rust
is_assignable_from(registry, loader, &class_name, ct)
```

Note: `find_exception_handler` currently takes `&ClassContext` and returns a result. It will need `registry` and `loader` parameters added, or the hierarchy check needs to happen at the call site.

IMPORTANT: The borrow checker will be tricky here. `find_exception_handler` is called while iterating, and `is_assignable_from` takes `&mut ClassRegistry`. Consider restructuring: extract the exception table entries first (clone them), release the borrow on registry, then iterate with `is_assignable_from`.

### Step 8: Fix anewarray element type

Find anewarray in both `execute()` and `execute_class()`. Replace the hardcoded `"[Ljava/lang/Object;"` with:
```rust
Instruction::Anewarray(cp_idx) => {
    let element_type = {
        let ctx = registry.get(&current_class)?;
        resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
    };
    let array_type = format!("[L{element_type};");
    // ... rest of allocation using array_type instead of hardcoded string
}
```

For `execute()` (single-method), do the same but resolve from the `cp` parameter.

### Step 9: Write unit tests

```rust
#[test]
fn is_assignable_same_type() {
    // Same type is always assignable
    let mut registry = ClassRegistry::new();
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    assert!(is_assignable_from(&mut registry, &loader, "Foo", "Foo"));
}

#[test]
fn is_assignable_to_object() {
    // Everything is assignable to Object
    let mut registry = ClassRegistry::new();
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    assert!(is_assignable_from(&mut registry, &loader, "Anything", "java/lang/Object"));
}
```

### Step 10: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): class hierarchy support for instanceof, checkcast, exception matching + anewarray fix"
```

---

## Task 3: String Native Methods + Integration Tests

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`
- Create: `tests/fixtures/StringOps.java` + `.class`

### Step 1: Write StringOps.java fixture

```java
public class StringOps {
    /** Returns the length of "Hello". */
    public static int stringLength() {
        String s = "Hello";
        return s.length(); // 5
    }

    /** Returns 1 if two equal strings match. */
    public static int stringEquals() {
        String a = "Duke";
        String b = "Duke";
        return a.equals(b) ? 1 : 0; // 1
    }

    /** Returns 0 if two different strings don't match. */
    public static int stringNotEquals() {
        String a = "Duke";
        String b = "Java";
        return a.equals(b) ? 1 : 0; // 0
    }

    /** Returns the char at index 1 of "Hello" as int. */
    public static int charAtOne() {
        String s = "Hello";
        return (int) s.charAt(1); // 'e' = 101
    }
}
```

### Step 2: Compile

```bash
cd tests/fixtures && javac --release 21 StringOps.java
```

### Step 3: Register String native methods in bootstrap_stdlib

Add after the PrintStream registrations:

```rust
// String instance methods
registry.natives_mut().register(
    "java/lang/String", "length", "()I",
    native_string_length,
);
registry.natives_mut().register(
    "java/lang/String", "equals", "(Ljava/lang/Object;)Z",
    native_string_equals,
);
registry.natives_mut().register(
    "java/lang/String", "charAt", "(I)C",
    native_string_char_at,
);
```

### Step 4: Implement native handlers

```rust
fn native_string_length(args: &[Slot], heap: &mut Heap, _out: &mut dyn Write) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get(this_ref)?;
    let len = obj.string_value.as_ref().map_or(0, |s| s.len());
    Ok(Some(Slot::Int(len as i32)))
}

fn native_string_equals(args: &[Slot], heap: &mut Heap, _out: &mut dyn Write) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let other_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Ok(Some(Slot::Int(0))), // null != anything
        _ => return Ok(Some(Slot::Int(0))),
    };
    let this_str = heap.get(this_ref)?.string_value.clone();
    let other_str = heap.get(other_ref)?.string_value.clone();
    let equal = this_str == other_str;
    Ok(Some(Slot::Int(if equal { 1 } else { 0 })))
}

fn native_string_char_at(args: &[Slot], heap: &mut Heap, _out: &mut dyn Write) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let index = match args.get(1) {
        Some(Slot::Int(i)) => *i,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    let obj = heap.get(this_ref)?;
    let s = obj.string_value.as_deref().unwrap_or("");
    let ch = s.chars().nth(index as usize).ok_or(VmError::ArrayIndexOutOfBounds {
        index,
        length: s.len(),
    })?;
    Ok(Some(Slot::Int(ch as i32)))
}
```

### Step 5: Also register java/lang/String ClassContext in bootstrap_stdlib

```rust
let string_ctx = ClassContext {
    class_name: "java/lang/String".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: Vec::new(),
    static_fields: Vec::new(),
    instance_field_count: 0,
};
registry.register(string_ctx);
```

### Step 6: Add integration tests

```rust
fn load_string_ops_class() -> ClassContext {
    let bytes = std::fs::read("tests/fixtures/StringOps.class").expect("StringOps.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn string_ops_length() {
    let ctx = load_string_ops_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = duke_loader::DirectoryLoader::new("tests/fixtures");
    let mut sink = Vec::new();
    let result = execute_class(&mut registry, &loader, &mut heap, &mut sink, "StringOps", "stringLength", "()I", &[]).unwrap();
    assert_eq!(result, Some(Slot::Int(5)));
}

#[test]
fn string_ops_equals() {
    // ... similar setup ...
    let result = execute_class(&mut registry, &loader, &mut heap, &mut sink, "StringOps", "stringEquals", "()I", &[]).unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_ops_not_equals() {
    let result = execute_class(..., "StringOps", "stringNotEquals", "()I", &[]).unwrap();
    assert_eq!(result, Some(Slot::Int(0)));
}

#[test]
fn string_ops_char_at() {
    let result = execute_class(..., "StringOps", "charAtOne", "()I", &[]).unwrap();
    assert_eq!(result, Some(Slot::Int(101))); // 'e'
}
```

### Step 7: Add Hierarchy + ExceptionHierarchy integration tests

```rust
#[test]
fn hierarchy_instanceof_object() {
    // ... load Hierarchy.class, run instanceOfObject → Int(1)
}

#[test]
fn hierarchy_cast_to_object() {
    // ... load Hierarchy.class, run castToObject → Int(1)
}

#[test]
fn hierarchy_null_instanceof() {
    // ... load Hierarchy.class, run nullInstanceOf → Int(0)
}

#[test]
fn exception_hierarchy_catch_parent() {
    // ... load ExceptionHierarchy.class, run catchParent → Int(1)
}

#[test]
fn exception_hierarchy_catch_exact() {
    // ... run catchExact → Int(2)
}

#[test]
fn exception_hierarchy_catch_wrong_then_right() {
    // ... run catchWrongThenRight → Int(3)
}

#[test]
fn exception_hierarchy_catch_grandparent() {
    // ... run catchGrandparent → Int(4)
}
```

### Step 8: Full verification

```bash
cargo test && cargo clippy --workspace -- -W clippy::pedantic -W clippy::nursery && cargo fmt --check
```

Target: ~180+ tests passing.

### Step 9: Commit

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/StringOps.java tests/fixtures/StringOps.class
git commit -m "feat(interpreter): String native methods (length, equals, charAt) + integration tests"
```

---

## Wave Execution Strategy

**Wave 1 (parallel):**
- Agent A: Task 1 (Hierarchy + ExceptionHierarchy + StringOps fixtures)
- Agent B: Task 2 (super_class field + is_assignable_from + hierarchy-aware type checks + anewarray fix)

**Wave 2 (sequential):**
- Agent C: Task 3 (String natives + ALL integration tests for hierarchy, exceptions, and strings)

**Wave 3 (leader):**
- Verify all tests, run demos, commit, update memory
