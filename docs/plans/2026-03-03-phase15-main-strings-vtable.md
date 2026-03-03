# Phase 15: main(String[]) Entry Point, String Natives, Virtual Dispatch Fix

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable running real Java programs with `duke run HelloWorld.class` by supporting `main(String[] args)`, fixing invokevirtual/invokespecial to walk the superclass hierarchy, and adding practical String native methods.

**Architecture:** Add a `duke run` CLI subcommand that constructs a `String[]` array on the heap and calls `main([Ljava/lang/String;)V`. Fix the invokevirtual/invokespecial handler to walk superclass chains when a method isn't found on the declared class. Add String natives for the most commonly used methods (substring, indexOf, isEmpty, contains, compareTo, startsWith, endsWith, trim, toCharArray).

**Tech Stack:** Rust, duke-interpreter crate, duke-gc (Heap), duke-runtime (Slot/VmError), duke binary crate

---

## Background

### Current Gaps
1. **No `main(String[] args)` entry point** — `duke exec` only accepts int args and calls arbitrary static methods. Real Java programs start with `public static void main(String[] args)`.
2. **invokevirtual doesn't walk the superclass chain** — if the Methodref points to class `Dog` for method `move()` but `move()` is defined in superclass `Animal`, invokevirtual fails to find it. The JVM spec (§5.4.3.3) requires walking the hierarchy.
3. **String native methods are thin** — only `length()`, `equals()`, `charAt()`, `valueOf(int)`. Missing: `substring`, `indexOf`, `isEmpty`, `contains`, `compareTo`, `startsWith`, `endsWith`, `trim`, `toCharArray`.

### Key Design Decisions
- `duke run <class> [args...]` is a new subcommand distinct from `duke exec` — `exec` stays for testing individual methods with int args
- `main` entry: build `String[]` on heap (each arg → `heap.allocate_string()`, then allocate array of refs)
- invokevirtual hierarchy walk: extract a helper `resolve_method_in_hierarchy(registry, loader, class, name, desc)` that walks `super_class` chain
- String natives are all simple `string_value` manipulations — no charset or locale complexity

---

## Task 1: Test Fixtures

**Files:**
- Create: `tests/fixtures/MainHello.java` + `.class`
- Create: `tests/fixtures/InheritedMethod.java` + `.class`
- Create: `tests/fixtures/StringMethods.java` + `.class`

### Step 1: Write MainHello.java

```java
public class MainHello {
    public static void main(String[] args) {
        if (args.length == 0) {
            System.out.println("no args");
        } else {
            for (int i = 0; i < args.length; i++) {
                System.out.println(args[i]);
            }
        }
    }

    /** Returns number of args. For testing without main(). */
    public static int countArgs(String[] args) {
        return args.length;
    }
}
```

### Step 2: Write InheritedMethod.java

```java
public class InheritedMethod {
    static class Animal {
        int sound() { return 1; }
        int move() { return 10; }
    }

    static class Dog extends Animal {
        // Overrides sound, inherits move()
        int sound() { return 2; }
    }

    static class Puppy extends Dog {
        // Inherits both sound() from Dog and move() from Animal
    }

    /** Calls inherited method (move defined in Animal, called on Dog). */
    public static int callInherited() {
        Dog d = new Dog();
        return d.move(); // 10 — inherited from Animal
    }

    /** Calls overridden method. */
    public static int callOverridden() {
        Dog d = new Dog();
        return d.sound(); // 2 — overridden in Dog
    }

    /** Two levels of inheritance. */
    public static int callDeepInherited() {
        Puppy p = new Puppy();
        return p.move(); // 10 — inherited from Animal via Dog
    }

    /** Override at middle level. */
    public static int callDeepOverridden() {
        Puppy p = new Puppy();
        return p.sound(); // 2 — inherited from Dog
    }
}
```

### Step 3: Write StringMethods.java

```java
public class StringMethods {
    public static int testSubstring() {
        String s = "HelloWorld";
        String sub = s.substring(5);
        return sub.length(); // 5 ("World")
    }

    public static int testSubstringRange() {
        String s = "HelloWorld";
        String sub = s.substring(0, 5);
        return sub.length(); // 5 ("Hello")
    }

    public static int testIndexOf() {
        String s = "HelloWorld";
        return s.indexOf("World"); // 5
    }

    public static int testIndexOfNotFound() {
        String s = "HelloWorld";
        return s.indexOf("xyz"); // -1
    }

    public static int testContains() {
        String s = "HelloWorld";
        return s.contains("World") ? 1 : 0; // 1
    }

    public static int testIsEmpty() {
        String s = "";
        String t = "hi";
        return (s.isEmpty() && !t.isEmpty()) ? 1 : 0; // 1
    }

    public static int testCompareTo() {
        String a = "apple";
        String b = "banana";
        return (a.compareTo(b) < 0) ? 1 : 0; // 1 (a < b)
    }

    public static int testStartsWith() {
        String s = "HelloWorld";
        return s.startsWith("Hello") ? 1 : 0; // 1
    }

    public static int testEndsWith() {
        String s = "HelloWorld";
        return s.endsWith("World") ? 1 : 0; // 1
    }

    public static int testTrim() {
        String s = "  hi  ";
        return s.trim().length(); // 2
    }

    public static int testToCharArray() {
        String s = "AB";
        char[] chars = s.toCharArray();
        return chars[0] + chars[1]; // 65 + 66 = 131
    }
}
```

### Step 4: Compile

```bash
cd tests/fixtures && javac --release 21 MainHello.java InheritedMethod.java StringMethods.java
```

NOTE: InheritedMethod.java will produce inner class files (InheritedMethod$Animal.class, InheritedMethod$Dog.class, InheritedMethod$Puppy.class). These must all be committed.

### Step 5: Commit

```bash
git add tests/fixtures/MainHello*.java tests/fixtures/MainHello*.class \
        tests/fixtures/InheritedMethod*.java tests/fixtures/InheritedMethod*.class \
        tests/fixtures/StringMethods.java tests/fixtures/StringMethods.class
git commit -m "test(phase15): add MainHello, InheritedMethod, and StringMethods fixtures"
```

---

## Task 2: Fix invokevirtual/invokespecial Superclass Hierarchy Walk

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Add resolve_method_in_hierarchy helper

Add this helper function near the other resolution helpers (near `resolve_methodref`, around line 3600):

```rust
/// Walk the class hierarchy to find a method by name and descriptor.
/// Returns (class_name_where_found, method_index) or None.
fn resolve_method_in_hierarchy(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    start_class: &str,
    method_name: &str,
    method_desc: &str,
) -> Option<(String, usize)> {
    let mut current = start_class.to_string();
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return None; // circular — bail
        }
        let _ = registry.ensure_loaded(&current, loader);
        match registry.get(&current) {
            Ok(ctx) => {
                if let Some(idx) = ctx.methods.iter().position(|m| {
                    m.name == method_name && m.descriptor == method_desc
                }) {
                    return Some((current, idx));
                }
                // Walk up to superclass.
                match &ctx.super_class {
                    Some(s) => current = s.clone(),
                    None => return None, // reached Object with no match
                }
            }
            Err(_) => return None,
        }
    }
}
```

### Step 2: Update invokevirtual/invokespecial handler

In `execute_class()`, find the `Instruction::Invokespecial(cp_idx) | Instruction::Invokevirtual(cp_idx)` arm (line ~2731). Replace the simple single-class lookup with the hierarchy walk.

**Current code (lines 2743-2750):**
```rust
let callee_idx = if loaded {
    let ctx = registry.get(&callee_class)?;
    ctx.methods
        .iter()
        .position(|m| m.name == callee_name && m.descriptor == callee_desc)
} else {
    None
};
```

**Replace with:**
```rust
let resolved = if loaded {
    resolve_method_in_hierarchy(
        registry, loader, &callee_class, &callee_name, &callee_desc,
    )
} else {
    None
};
```

Then update the code below to use `resolved`:

**Current code (line 2751):**
```rust
let callee_idx = match callee_idx {
    Some(i) => i,
    None => {
```

**Replace with:**
```rust
let (dispatch_class, callee_idx) = match resolved {
    Some((cls, i)) => (cls, i),
    None => {
```

And further down where the callee frame is built, change the class used for frame construction and dispatch:

**Current code (line 2794):**
```rust
let (callee_pc_to_idx, callee_frame) = {
    let ctx = registry.get(&callee_class)?;
```

**Replace with:**
```rust
let (callee_pc_to_idx, callee_frame) = {
    let ctx = registry.get(&dispatch_class)?;
```

**Current code (line 2818):**
```rust
current_class = callee_class;
```

**Replace with:**
```rust
current_class = dispatch_class;
```

### Step 3: Also update invokeinterface handler

Find `Instruction::Invokeinterface` (line ~3308). It already does actual-class dispatch, but it also has a single-class lookup that should use the hierarchy walk. Find the method lookup on the actual class (around line 3345-3350) and replace with `resolve_method_in_hierarchy`.

Look for the pattern:
```rust
let callee_idx = {
    match registry.get(&actual_class) {
        Ok(ctx) => ctx.methods.iter().position(|m| m.name == callee_name && m.descriptor == callee_desc),
        Err(_) => None,
    }
};
```

Replace with:
```rust
let resolved = resolve_method_in_hierarchy(
    registry, loader, &actual_class, &callee_name, &callee_desc,
);
```

And adjust the destructuring below to unpack `(dispatch_class, callee_idx)` from `resolved`.

### Step 4: Run tests + commit

```bash
cargo test --workspace
cargo clippy --workspace -- -W clippy::pedantic -W clippy::nursery
git add crates/duke-interpreter/src/lib.rs
git commit -m "fix(interpreter): walk superclass hierarchy in invokevirtual/invokespecial/invokeinterface"
```

---

## Task 3: String Native Methods

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Implement native handler functions

Add these near the other native handler functions (after `native_print_int`, around line 600):

```rust
fn native_string_substring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let begin = match args.get(1) {
        Some(Slot::Int(v)) => *v as usize,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    if begin > s.len() {
        return Err(VmError::ArrayIndexOutOfBounds { index: begin as i32, length: s.len() });
    }
    let sub: String = s.chars().skip(begin).collect();
    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_string_substring_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let begin = match args.get(1) {
        Some(Slot::Int(v)) => *v as usize,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    let end = match args.get(2) {
        Some(Slot::Int(v)) => *v as usize,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    if begin > end || end > s.len() {
        return Err(VmError::ArrayIndexOutOfBounds { index: end as i32, length: s.len() });
    }
    let sub: String = s.chars().skip(begin).take(end - begin).collect();
    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_string_indexof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let target_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let target = heap.get(target_ref)?.string_value.clone().unwrap_or_default();
    let result = s.find(&target).map_or(-1, |i| i as i32);
    Ok(Some(Slot::Int(result)))
}

fn native_string_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let target_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let target = heap.get(target_ref)?.string_value.clone().unwrap_or_default();
    Ok(Some(Slot::Int(if s.contains(&target) { 1 } else { 0 })))
}

fn native_string_isempty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    Ok(Some(Slot::Int(if s.is_empty() { 1 } else { 0 })))
}

fn native_string_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let other_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let other = heap.get(other_ref)?.string_value.clone().unwrap_or_default();
    Ok(Some(Slot::Int(s.cmp(&other) as i32)))
}

fn native_string_startswith(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let prefix_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let prefix = heap.get(prefix_ref)?.string_value.clone().unwrap_or_default();
    Ok(Some(Slot::Int(if s.starts_with(&prefix) { 1 } else { 0 })))
}

fn native_string_endswith(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let suffix_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let suffix = heap.get(suffix_ref)?.string_value.clone().unwrap_or_default();
    Ok(Some(Slot::Int(if s.ends_with(&suffix) { 1 } else { 0 })))
}

fn native_string_trim(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let trimmed = s.trim().to_string();
    let r = heap.allocate_string(trimmed);
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_string_tochararray(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let chars: Vec<char> = s.chars().collect();
    let arr_ref = heap.allocate("[C".to_string(), chars.len());
    for (i, &c) in chars.iter().enumerate() {
        heap.get_mut(arr_ref).unwrap().fields[i] = Slot::Int(c as i32);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}
```

### Step 2: Register all new String natives in bootstrap_stdlib

Add after the existing String registrations (after `native_string_char_at` registration, around line 302):

```rust
// substring(int) and substring(int, int) — different descriptors, different handlers
registry.natives_mut().register(
    "java/lang/String", "substring", "(I)Ljava/lang/String;", native_string_substring,
);
registry.natives_mut().register(
    "java/lang/String", "substring", "(II)Ljava/lang/String;", native_string_substring_range,
);
registry.natives_mut().register(
    "java/lang/String", "indexOf", "(Ljava/lang/String;)I", native_string_indexof,
);
registry.natives_mut().register(
    "java/lang/String", "contains", "(Ljava/lang/CharSequence;)Z", native_string_contains,
);
registry.natives_mut().register(
    "java/lang/String", "isEmpty", "()Z", native_string_isempty,
);
registry.natives_mut().register(
    "java/lang/String", "compareTo", "(Ljava/lang/String;)I", native_string_compareto,
);
registry.natives_mut().register(
    "java/lang/String", "startsWith", "(Ljava/lang/String;)Z", native_string_startswith,
);
registry.natives_mut().register(
    "java/lang/String", "endsWith", "(Ljava/lang/String;)Z", native_string_endswith,
);
registry.natives_mut().register(
    "java/lang/String", "trim", "()Ljava/lang/String;", native_string_trim,
);
registry.natives_mut().register(
    "java/lang/String", "toCharArray", "()[C", native_string_tochararray,
);
```

**IMPORTANT**: The `contains` descriptor uses `CharSequence` not `String` — check the actual bytecode by running `duke dump tests/fixtures/StringMethods.class` and looking at the Methodref for `contains`. If it says `(Ljava/lang/CharSequence;)Z`, use that. The JVM dispatches by exact descriptor match.

### Step 3: Run tests + commit

```bash
cargo test --workspace
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): add String native methods (substring, indexOf, contains, isEmpty, compareTo, startsWith, endsWith, trim, toCharArray)"
```

---

## Task 4: `duke run` Entry Point + Integration Tests

**Files:**
- Modify: `duke/src/main.rs`
- Modify: `crates/duke-interpreter/src/lib.rs` (integration tests only)

### Step 1: Add `duke run` subcommand to main.rs

Add a new dispatch branch in `main()`, after the `exec` branch (around line 33):

```rust
// Dispatch `run`: run main(String[] args) entry point.
if args.len() >= 3 && args[1] == "run" {
    run_main(&args[2..]);
    return;
}
```

Update usage string to include `run`:
```rust
eprintln!("       duke run <classfile.class> [string-arg...]");
```

### Step 2: Implement run_main function

Add after `exec_method` (around line 161):

```rust
/// `duke run <classfile.class> [string-arg...]`
///
/// Runs the `main(String[])` method with string arguments.
fn run_main(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: duke run <classfile.class> [string-arg...]");
        process::exit(1);
    }
    let path = &args[0];
    let string_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();

    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    let ctx = build_class_context(&cf);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);

    let parent = std::path::Path::new(path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let loader = DirectoryLoader::new(parent);
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // Build String[] args array on the heap.
    let mut arg_refs: Vec<Slot> = Vec::new();
    for arg in &string_args {
        let r = heap.allocate_string(arg.to_string());
        arg_refs.push(Slot::Reference(Some(r)));
    }
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), string_args.len());
    for (i, slot) in arg_refs.into_iter().enumerate() {
        heap.get_mut(arr_ref).unwrap().fields[i] = slot;
    }

    let main_args = vec![Slot::Reference(Some(arr_ref))];

    let mut stdout = std::io::stdout();
    match execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut stdout,
        &entry_class,
        "main",
        "([Ljava/lang/String;)V",
        &main_args,
    ) {
        Ok(_) => {} // main returns void
        Err(e) => {
            eprintln!("duke: runtime error: {e}");
            process::exit(1);
        }
    }
}
```

### Step 3: Add ALL integration tests

Add to the `#[cfg(test)] mod tests` section in `crates/duke-interpreter/src/lib.rs`.

**InheritedMethod tests** (need to load inner class files):
```rust
#[test] fn inherited_call_inherited() { /* callInherited → Int(10) */ }
#[test] fn inherited_call_overridden() { /* callOverridden → Int(2) */ }
#[test] fn inherited_call_deep_inherited() { /* callDeepInherited → Int(10) */ }
#[test] fn inherited_call_deep_overridden() { /* callDeepOverridden → Int(2) */ }
```

**StringMethods tests:**
```rust
#[test] fn string_substring() { /* testSubstring → Int(5) */ }
#[test] fn string_substring_range() { /* testSubstringRange → Int(5) */ }
#[test] fn string_indexof() { /* testIndexOf → Int(5) */ }
#[test] fn string_indexof_not_found() { /* testIndexOfNotFound → Int(-1) */ }
#[test] fn string_contains() { /* testContains → Int(1) */ }
#[test] fn string_isempty() { /* testIsEmpty → Int(1) */ }
#[test] fn string_compareto() { /* testCompareTo → Int(1) */ }
#[test] fn string_startswith() { /* testStartsWith → Int(1) */ }
#[test] fn string_endswith() { /* testEndsWith → Int(1) */ }
#[test] fn string_trim() { /* testTrim → Int(2) */ }
#[test] fn string_tochararray() { /* testToCharArray → Int(131) */ }
```

**MainHello tests** (test the main entry point programmatically):
```rust
#[test]
fn main_hello_no_args() {
    // Build String[] with 0 elements, call main, check stdout = "no args\n"
}
#[test]
fn main_hello_with_args() {
    // Build String[] with ["Alice", "Bob"], call main, check stdout = "Alice\nBob\n"
}
```

### Step 4: Full verification

```bash
cargo test --workspace
cargo clippy --workspace -- -W clippy::pedantic -W clippy::nursery
cargo fmt --check
```

Target: 200+ tests passing (184 existing + 4 inherited + 11 string + 2 main = 201).

### Step 5: Commit

```bash
git add duke/src/main.rs crates/duke-interpreter/src/lib.rs
git commit -m "feat(duke): add duke run subcommand for main(String[] args) + Phase 15 integration tests"
```

---

## Wave Execution Strategy

**Wave 1 (parallel):**
- Agent A: Task 1 (fixtures — MainHello, InheritedMethod, StringMethods)
- Agent B: Task 2 (invokevirtual/invokespecial hierarchy walk fix)

**Wave 2 (sequential):**
- Agent C: Task 3 (String native methods) + Task 4 (duke run + integration tests)

**Wave 3 (leader):**
- Verify, commit plan doc, update memory
