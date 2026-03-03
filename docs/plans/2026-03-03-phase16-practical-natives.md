# Phase 16: Practical Native Methods & Type Coverage

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Unlock practical CLI and string-manipulation Java programs by adding println overloads for all types, Integer.parseInt/valueOf, String.valueOf overloads, String.concat, System.exit, and Math basics.

**Architecture:** All additions are native method handlers registered in `bootstrap_stdlib`. No new opcodes needed — Phase 15 completed all instruction-level work. This phase is purely about expanding the standard library surface via `NativeHandler` functions.

**Tech Stack:** Rust, duke-interpreter crate, duke-gc (Heap), duke-runtime (Slot/VmError)

---

## Background

### Current Native Coverage
**PrintStream:** `println(String)`, `println(int)`, `println()`, `print(String)`, `print(int)`
**String:** `length`, `equals`, `charAt`, `valueOf(int)`, `substring(I)`, `substring(II)`, `indexOf`, `contains`, `isEmpty`, `compareTo`, `startsWith`, `endsWith`, `trim`, `toCharArray`
**Object:** `hashCode`, `toString`

### Missing (Blocks Real Programs)
1. `println(long)`, `println(double)`, `println(float)`, `println(boolean)`, `println(Object)` — can't print non-int/non-string types
2. `print(long)`, `print(double)`, `print(float)`, `print(boolean)`, `print(Object)` — same for no-newline print
3. `Integer.parseInt(String)` — can't parse CLI args to int
4. `Integer.valueOf(int)`, `Integer.toString(int)` — boxing/conversion
5. `String.valueOf(long/double/boolean/char/Object)` — type conversion to strings
6. `String.concat(String)` — string concatenation without invokedynamic
7. `System.exit(int)` — clean program termination
8. `Math.max/min/abs` — basic math utilities

---

## Task 1: Test Fixtures

**Files:**
- Create: `tests/fixtures/PrintAll.java` + `.class`
- Create: `tests/fixtures/ParseArgs.java` + `.class`
- Create: `tests/fixtures/StringConcat.java` + `.class`

### Step 1: Write PrintAll.java

```java
public class PrintAll {
    /** Prints a long value via println. */
    public static int printLong() {
        long x = 9876543210L;
        System.out.println(x);
        return 1;
    }

    /** Prints a double value via println. */
    public static int printDouble() {
        double d = 3.14;
        System.out.println(d);
        return 1;
    }

    /** Prints a float value via println. */
    public static int printFloat() {
        float f = 2.5f;
        System.out.println(f);
        return 1;
    }

    /** Prints a boolean value via println. */
    public static int printBoolean() {
        System.out.println(true);
        System.out.println(false);
        return 1;
    }

    /** Prints a char value via println. */
    public static int printChar() {
        char c = 'Z';
        System.out.println(c);
        return 1;
    }

    /** Uses print (no newline) for long, then println. */
    public static int printMixed() {
        System.out.print("val=");
        System.out.print(42);
        System.out.println();
        return 1;
    }
}
```

### Step 2: Write ParseArgs.java

```java
public class ParseArgs {
    /** Parses first string arg as int, returns it. */
    public static int parseInt(String[] args) {
        return Integer.parseInt(args[0]);
    }

    /** Parses two args and returns their sum. */
    public static int addParsed(String[] args) {
        int a = Integer.parseInt(args[0]);
        int b = Integer.parseInt(args[1]);
        return a + b;
    }

    /** Tests Integer.valueOf returns non-null. */
    public static int valueOf() {
        Integer i = Integer.valueOf(42);
        return i.intValue(); // 42
    }

    /** Tests Math.max. */
    public static int mathMax() {
        return Math.max(3, 7); // 7
    }

    /** Tests Math.min. */
    public static int mathMin() {
        return Math.min(3, 7); // 3
    }

    /** Tests Math.abs. */
    public static int mathAbs() {
        return Math.abs(-5); // 5
    }
}
```

### Step 3: Write StringConcat.java

```java
public class StringConcat {
    /** Concat two strings and return length. */
    public static int concatLength() {
        String a = "Hello";
        String b = "World";
        String c = a.concat(b);
        return c.length(); // 10
    }

    /** valueOf for boolean. */
    public static int boolToString() {
        String s = String.valueOf(true);
        return s.length(); // 4 ("true")
    }

    /** valueOf for long. */
    public static int longToString() {
        String s = String.valueOf(100L);
        return s.length(); // 3 ("100")
    }

    /** valueOf for char. */
    public static int charToString() {
        String s = String.valueOf('A');
        return s.length(); // 1
    }

    /** valueOf for double. */
    public static int doubleToString() {
        String s = String.valueOf(3.14);
        return (s.length() > 0) ? 1 : 0; // 1
    }
}
```

### Step 4: Compile

```bash
cd tests/fixtures && javac --release 21 PrintAll.java ParseArgs.java StringConcat.java
```

### Step 5: Commit

```bash
git add tests/fixtures/PrintAll.java tests/fixtures/PrintAll.class \
        tests/fixtures/ParseArgs.java tests/fixtures/ParseArgs.class \
        tests/fixtures/StringConcat.java tests/fixtures/StringConcat.class
git commit -m "test(phase16): add PrintAll, ParseArgs, and StringConcat fixtures"
```

---

## Task 2: println/print Overloads + System.exit

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Add native handler functions

Add near the existing `native_println_int` / `native_print_int` handlers:

```rust
fn native_println_long(
    args: &[Slot], _heap: &mut duke_gc::Heap, out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Long", got: "other" }),
    };
    writeln!(out, "{val}").ok();
    Ok(None)
}

fn native_println_float(
    args: &[Slot], _heap: &mut duke_gc::Heap, out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Float(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Float", got: "other" }),
    };
    writeln!(out, "{val}").ok();
    Ok(None)
}

fn native_println_double(
    args: &[Slot], _heap: &mut duke_gc::Heap, out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    writeln!(out, "{val}").ok();
    Ok(None)
}

fn native_println_boolean(
    args: &[Slot], _heap: &mut duke_gc::Heap, out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v != 0,
        _ => return Err(VmError::TypeMismatch { expected: "Int(boolean)", got: "other" }),
    };
    writeln!(out, "{val}").ok();
    Ok(None)
}

fn native_println_char(
    args: &[Slot], _heap: &mut duke_gc::Heap, out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('?'),
        _ => return Err(VmError::TypeMismatch { expected: "Int(char)", got: "other" }),
    };
    writeln!(out, "{val}").ok();
    Ok(None)
}

fn native_println_object(
    args: &[Slot], heap: &mut duke_gc::Heap, out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    match args.get(1) {
        Some(Slot::Reference(Some(r))) => {
            let obj = heap.get(*r)?;
            if let Some(s) = &obj.string_value {
                writeln!(out, "{s}").ok();
            } else {
                let hash = *r as i32;
                writeln!(out, "{}@{:x}", obj.class_name, hash).ok();
            }
        }
        Some(Slot::Reference(None)) => { writeln!(out, "null").ok(); }
        _ => { writeln!(out, "<unknown>").ok(); }
    }
    Ok(None)
}

fn native_print_long(
    args: &[Slot], _heap: &mut duke_gc::Heap, out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Long", got: "other" }),
    };
    write!(out, "{val}").ok();
    Ok(None)
}

fn native_print_float(
    args: &[Slot], _heap: &mut duke_gc::Heap, out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Float(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Float", got: "other" }),
    };
    write!(out, "{val}").ok();
    Ok(None)
}

fn native_print_double(
    args: &[Slot], _heap: &mut duke_gc::Heap, out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    write!(out, "{val}").ok();
    Ok(None)
}

fn native_print_boolean(
    args: &[Slot], _heap: &mut duke_gc::Heap, out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v != 0,
        _ => return Err(VmError::TypeMismatch { expected: "Int(boolean)", got: "other" }),
    };
    write!(out, "{val}").ok();
    Ok(None)
}

fn native_print_char(
    args: &[Slot], _heap: &mut duke_gc::Heap, out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('?'),
        _ => return Err(VmError::TypeMismatch { expected: "Int(char)", got: "other" }),
    };
    write!(out, "{val}").ok();
    Ok(None)
}

fn native_print_object(
    args: &[Slot], heap: &mut duke_gc::Heap, out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    match args.get(1) {
        Some(Slot::Reference(Some(r))) => {
            let obj = heap.get(*r)?;
            if let Some(s) = &obj.string_value {
                write!(out, "{s}").ok();
            } else {
                let hash = *r as i32;
                write!(out, "{}@{:x}", obj.class_name, hash).ok();
            }
        }
        Some(Slot::Reference(None)) => { write!(out, "null").ok(); }
        _ => { write!(out, "<unknown>").ok(); }
    }
    Ok(None)
}

fn native_system_exit(
    args: &[Slot], _heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let code = match args.get(0) {
        Some(Slot::Int(v)) => *v,
        _ => 1,
    };
    Err(VmError::SystemExit { code })
}
```

NOTE: `System.exit` is a static method so `args[0]` is the int, no `this`. We return a `VmError::SystemExit` variant instead of calling `std::process::exit()` so tests can catch it. You'll need to add `SystemExit { code: i32 }` to `VmError` in `crates/duke-runtime/src/error.rs`.

### Step 2: Add SystemExit to VmError

In `crates/duke-runtime/src/error.rs`, add:
```rust
#[error("System.exit({code})")]
SystemExit { code: i32 },
```

### Step 3: Handle SystemExit in main.rs

In `duke/src/main.rs`, in both `exec_method` and `run_main`, change the error handling:
```rust
Err(VmError::SystemExit { code }) => {
    process::exit(code);
}
Err(e) => {
    eprintln!("duke: runtime error: {e}");
    process::exit(1);
}
```

You'll need to import `VmError` — add `use duke_runtime::VmError;` at the top.

### Step 4: Register all in bootstrap_stdlib

```rust
// println overloads
registry.natives_mut().register("java/io/PrintStream", "println", "(J)V", native_println_long);
registry.natives_mut().register("java/io/PrintStream", "println", "(F)V", native_println_float);
registry.natives_mut().register("java/io/PrintStream", "println", "(D)V", native_println_double);
registry.natives_mut().register("java/io/PrintStream", "println", "(Z)V", native_println_boolean);
registry.natives_mut().register("java/io/PrintStream", "println", "(C)V", native_println_char);
registry.natives_mut().register("java/io/PrintStream", "println", "(Ljava/lang/Object;)V", native_println_object);

// print overloads
registry.natives_mut().register("java/io/PrintStream", "print", "(J)V", native_print_long);
registry.natives_mut().register("java/io/PrintStream", "print", "(F)V", native_print_float);
registry.natives_mut().register("java/io/PrintStream", "print", "(D)V", native_print_double);
registry.natives_mut().register("java/io/PrintStream", "print", "(Z)V", native_print_boolean);
registry.natives_mut().register("java/io/PrintStream", "print", "(C)V", native_print_char);
registry.natives_mut().register("java/io/PrintStream", "print", "(Ljava/lang/Object;)V", native_print_object);

// System.exit (static method — registered under java/lang/System)
registry.natives_mut().register("java/lang/System", "exit", "(I)V", native_system_exit);
```

### Step 5: Run tests + commit

```bash
cargo test --workspace
git add crates/duke-interpreter/src/lib.rs crates/duke-runtime/src/error.rs duke/src/main.rs
git commit -m "feat(interpreter): add println/print overloads for all types + System.exit"
```

---

## Task 3: Integer, String.valueOf overloads, String.concat, Math natives + Integration Tests

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Add Integer native handlers

```rust
fn native_integer_parseint(
    args: &[Slot], heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    // Static method — args[0] is the String ref
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: i32 = s.trim().parse().map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Int(val)))
}

fn native_integer_valueof(
    args: &[Slot], heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    // Static method — args[0] is the int
    let val = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    // Allocate an Integer object with one field (the int value)
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_integer_intvalue(
    args: &[Slot], heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    // Instance method — args[0] is `this`
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = heap.get(this_ref)?.fields[0].clone();
    Ok(Some(val))
}

fn native_integer_tostring(
    args: &[Slot], heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    // Static method — args[0] is the int
    let val = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}
```

### Step 2: Add String.valueOf overloads

```rust
fn native_string_value_of_long(
    args: &[Slot], heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Long", got: "other" }),
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_string_value_of_double(
    args: &[Slot], heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_string_value_of_boolean(
    args: &[Slot], heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v != 0,
        _ => return Err(VmError::TypeMismatch { expected: "Int(boolean)", got: "other" }),
    };
    let r = heap.allocate_string(if val { "true" } else { "false" }.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_string_value_of_char(
    args: &[Slot], heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('?'),
        _ => return Err(VmError::TypeMismatch { expected: "Int(char)", got: "other" }),
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_string_value_of_object(
    args: &[Slot], heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let obj = heap.get(*r)?;
            let s = if let Some(sv) = &obj.string_value {
                sv.clone()
            } else {
                let hash = *r as i32;
                format!("{}@{:x}", obj.class_name, hash)
            };
            let r = heap.allocate_string(s);
            Ok(Some(Slot::Reference(Some(r))))
        }
        Some(Slot::Reference(None)) => {
            let r = heap.allocate_string("null".to_string());
            Ok(Some(Slot::Reference(Some(r))))
        }
        _ => Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    }
}
```

### Step 3: Add String.concat

```rust
fn native_string_concat(
    args: &[Slot], heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s1 = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let other_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let s2 = heap.get(other_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(format!("{s1}{s2}"));
    Ok(Some(Slot::Reference(Some(r))))
}
```

### Step 4: Add Math natives

```rust
fn native_math_max_int(
    args: &[Slot], _heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    Ok(Some(Slot::Int(a.max(b))))
}

fn native_math_min_int(
    args: &[Slot], _heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    Ok(Some(Slot::Int(a.min(b))))
}

fn native_math_abs_int(
    args: &[Slot], _heap: &mut duke_gc::Heap, _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    Ok(Some(Slot::Int(a.wrapping_abs())))
}
```

### Step 5: Register all in bootstrap_stdlib

Create synthetic ClassContexts for `java/lang/Integer` and `java/lang/Math`:

```rust
// java/lang/Integer
let integer_ctx = ClassContext {
    class_name: "java/lang/Integer".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: vec![FieldEntry { name: "value".to_string(), descriptor: "I".to_string(), is_static: false }],
    static_fields: Vec::new(),
    instance_field_count: 1,
};
registry.register(integer_ctx);
registry.natives_mut().register("java/lang/Integer", "parseInt", "(Ljava/lang/String;)I", native_integer_parseint);
registry.natives_mut().register("java/lang/Integer", "valueOf", "(I)Ljava/lang/Integer;", native_integer_valueof);
registry.natives_mut().register("java/lang/Integer", "intValue", "()I", native_integer_intvalue);
registry.natives_mut().register("java/lang/Integer", "toString", "(I)Ljava/lang/String;", native_integer_tostring);

// java/lang/Math (all static, no instance fields)
let math_ctx = ClassContext {
    class_name: "java/lang/Math".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: Vec::new(),
    static_fields: Vec::new(),
    instance_field_count: 0,
};
registry.register(math_ctx);
registry.natives_mut().register("java/lang/Math", "max", "(II)I", native_math_max_int);
registry.natives_mut().register("java/lang/Math", "min", "(II)I", native_math_min_int);
registry.natives_mut().register("java/lang/Math", "abs", "(I)I", native_math_abs_int);

// String.valueOf overloads
registry.natives_mut().register("java/lang/String", "valueOf", "(J)Ljava/lang/String;", native_string_value_of_long);
registry.natives_mut().register("java/lang/String", "valueOf", "(D)Ljava/lang/String;", native_string_value_of_double);
registry.natives_mut().register("java/lang/String", "valueOf", "(Z)Ljava/lang/String;", native_string_value_of_boolean);
registry.natives_mut().register("java/lang/String", "valueOf", "(C)Ljava/lang/String;", native_string_value_of_char);
registry.natives_mut().register("java/lang/String", "valueOf", "(Ljava/lang/Object;)Ljava/lang/String;", native_string_value_of_object);

// String.concat
registry.natives_mut().register("java/lang/String", "concat", "(Ljava/lang/String;)Ljava/lang/String;", native_string_concat);
```

### Step 6: Add ALL integration tests

**PrintAll tests** (check stdout output):
```rust
#[test] fn print_long() { /* printLong → Int(1), stdout contains "9876543210" */ }
#[test] fn print_double() { /* printDouble → Int(1), stdout contains "3.14" */ }
#[test] fn print_float() { /* printFloat → Int(1), stdout contains "2.5" */ }
#[test] fn print_boolean() { /* printBoolean → Int(1), stdout contains "true\nfalse" */ }
#[test] fn print_char() { /* printChar → Int(1), stdout contains "Z" */ }
#[test] fn print_mixed() { /* printMixed → Int(1), stdout = "val=42\n" */ }
```

**ParseArgs tests** (build String[] on heap):
```rust
#[test] fn parse_int_arg() { /* parseInt(["123"]) → Int(123) */ }
#[test] fn add_parsed_args() { /* addParsed(["10", "20"]) → Int(30) */ }
#[test] fn integer_valueof() { /* valueOf → Int(42) */ }
#[test] fn math_max() { /* mathMax → Int(7) */ }
#[test] fn math_min() { /* mathMin → Int(3) */ }
#[test] fn math_abs() { /* mathAbs → Int(5) */ }
```

**StringConcat tests:**
```rust
#[test] fn concat_length() { /* concatLength → Int(10) */ }
#[test] fn bool_to_string() { /* boolToString → Int(4) */ }
#[test] fn long_to_string() { /* longToString → Int(3) */ }
#[test] fn char_to_string() { /* charToString → Int(1) */ }
#[test] fn double_to_string() { /* doubleToString → Int(1) */ }
```

### Step 7: Full verification

```bash
cargo test --workspace
cargo clippy --workspace -- -W clippy::pedantic -W clippy::nursery
cargo fmt --check
```

Target: 218+ tests passing (201 existing + 6 print + 6 parse + 5 concat = 218).

### Step 8: Commit

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(interpreter): Integer/Math/String natives + Phase 16 integration tests"
```

---

## Wave Execution Strategy

**Wave 1 (parallel):**
- Agent A: Task 1 (fixtures — PrintAll, ParseArgs, StringConcat)
- Agent B: Task 2 (println/print overloads + System.exit + VmError::SystemExit)

**Wave 2 (sequential):**
- Agent C: Task 3 (Integer, Math, String.valueOf/concat + ALL integration tests)

**Wave 3 (leader):**
- Verify, commit plan doc, update memory
