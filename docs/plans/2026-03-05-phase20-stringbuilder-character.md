# Phase 20: StringBuilder + Character Natives — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `java/lang/StringBuilder` and `java/lang/Character` synthetic classes with native handlers, enabling string building in loops and character classification/conversion.

**Architecture:** Both classes are registered as synthetic `ClassContext` entries in `bootstrap_stdlib()` with all methods backed by native handlers. StringBuilder uses the existing `string_value` field on `HeapObject` to store its internal buffer as a Rust `String`. Character is a pure-static utility class (all methods are static).

**Tech Stack:** Rust, duke-interpreter, duke-gc, duke-runtime, javac 21

---

## Background

### StringBuilder

Nearly every realistic Java program uses `StringBuilder` for efficient string construction. Javac compiles `new StringBuilder().append("hello").append(42).toString()` into:

```
new java/lang/StringBuilder
dup
invokespecial StringBuilder.<init>()V
ldc "hello"
invokevirtual StringBuilder.append(Ljava/lang/String;)Ljava/lang/StringBuilder;
bipush 42
invokevirtual StringBuilder.append(I)Ljava/lang/StringBuilder;
invokevirtual StringBuilder.toString()Ljava/lang/String;
```

Key: each `append` returns `this` (for chaining), and `toString` produces the final `String`.

Duke's invokedynamic StringConcatFactory handles `"a" + b + "c"` patterns, but explicit `new StringBuilder()` usage (common in loops, conditional building) requires this class.

### Character

`java.lang.Character` provides static utility methods for character classification and conversion. Used heavily in parsing, validation, and text processing. All methods are static — no instance fields needed.

---

## Task 1: StringBuilder Synthetic Class + Natives

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (bootstrap_stdlib ~line 925, native handlers)
- Create: `tests/fixtures/StringBuilderTest.java`
- Test: `crates/duke-interpreter/src/lib.rs` (test module)

### Step 1: Create the Java fixture

Create `tests/fixtures/StringBuilderTest.java`:

```java
public class StringBuilderTest {
    // Basic: new StringBuilder().append("hello").toString()
    static int testBasicAppend() {
        StringBuilder sb = new StringBuilder();
        sb.append("hello");
        String s = sb.toString();
        return s.length();  // expect 5
    }

    // Chaining: append returns this
    static int testChaining() {
        String s = new StringBuilder().append("a").append("bc").append("def").toString();
        return s.length();  // expect 6
    }

    // Append int
    static int testAppendInt() {
        String s = new StringBuilder().append("val=").append(42).toString();
        return s.length();  // "val=42" = 6
    }

    // Append long
    static int testAppendLong() {
        String s = new StringBuilder().append(100L).toString();
        return s.length();  // "100" = 3
    }

    // Append boolean
    static int testAppendBoolean() {
        String s = new StringBuilder().append(true).toString();
        return s.length();  // "true" = 4
    }

    // Append char
    static int testAppendChar() {
        String s = new StringBuilder().append('X').toString();
        return s.length();  // "X" = 1
    }

    // Append double
    static int testAppendDouble() {
        String s = new StringBuilder().append(3.14).toString();
        // Double.toString(3.14) = "3.14" (4 chars)
        return s.length();  // expect 4
    }

    // Append float
    static int testAppendFloat() {
        String s = new StringBuilder().append(1.5f).toString();
        // Float.toString(1.5f) = "1.5" (3 chars)
        return s.length();  // expect 3
    }

    // Init with String arg
    static int testInitWithString() {
        StringBuilder sb = new StringBuilder("start");
        sb.append("end");
        return sb.toString().length();  // "startend" = 8
    }

    // length() method
    static int testLength() {
        StringBuilder sb = new StringBuilder();
        sb.append("abc");
        return sb.length();  // expect 3
    }

    // Loop building
    static int testLoopBuild() {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 5; i++) {
            sb.append(i);
        }
        return sb.toString().length();  // "01234" = 5
    }

    // Append Object (calls toString on the object)
    static int testAppendString() {
        StringBuilder sb = new StringBuilder();
        String s = "hello";
        sb.append(s);
        return sb.toString().length();  // 5
    }
}
```

Compile: `javac --release 21 tests/fixtures/StringBuilderTest.java`

### Step 2: Register synthetic `java/lang/StringBuilder` in `bootstrap_stdlib`

Insert before the closing `}` of `bootstrap_stdlib()` (~line 925):

```rust
// java/lang/StringBuilder — mutable string buffer
// Uses string_value on HeapObject as the internal buffer
let sb_ctx = ClassContext {
    class_name: "java/lang/StringBuilder".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: Vec::new(),
    static_fields: Vec::new(),
    instance_field_count: 0,
    bootstrap_methods: Vec::new(),
};
registry.register(sb_ctx);
```

### Step 3: Implement StringBuilder native handlers

Add after the existing native handlers (after `native_enum_valueof`, ~line 1195):

```rust
// ---- StringBuilder natives ----

/// Native: `StringBuilder.<init>()V` — initializes with empty buffer.
fn native_sb_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    heap.get_mut(this_ref)?.string_value = Some(String::new());
    Ok(None)
}

/// Native: `StringBuilder.<init>(Ljava/lang/String;)V` — init with string content.
fn native_sb_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let initial = match args.get(1) {
        Some(Slot::Reference(Some(r))) => {
            heap.get(*r)?.string_value.clone().unwrap_or_default()
        }
        _ => String::new(),
    };
    heap.get_mut(this_ref)?.string_value = Some(initial);
    Ok(None)
}

/// Native: `StringBuilder.append(Ljava/lang/String;)Ljava/lang/StringBuilder;`
fn native_sb_append_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let to_append = match args.get(1) {
        Some(Slot::Reference(Some(r))) => {
            heap.get(*r)?.string_value.clone().unwrap_or_default()
        }
        Some(Slot::Reference(None)) => "null".to_string(),
        _ => String::new(),
    };
    if let Some(ref mut buf) = heap.get_mut(this_ref)?.string_value {
        buf.push_str(&to_append);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(I)Ljava/lang/StringBuilder;`
fn native_sb_append_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    if let Some(ref mut buf) = heap.get_mut(this_ref)?.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(J)Ljava/lang/StringBuilder;`
fn native_sb_append_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    if let Some(ref mut buf) = heap.get_mut(this_ref)?.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(D)Ljava/lang/StringBuilder;`
fn native_sb_append_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        _ => 0.0,
    };
    if let Some(ref mut buf) = heap.get_mut(this_ref)?.string_value {
        buf.push_str(&format!("{val}"));
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(F)Ljava/lang/StringBuilder;`
fn native_sb_append_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Float(v)) => *v,
        _ => 0.0,
    };
    if let Some(ref mut buf) = heap.get_mut(this_ref)?.string_value {
        buf.push_str(&format!("{val}"));
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(Z)Ljava/lang/StringBuilder;`
fn native_sb_append_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v != 0,
        _ => false,
    };
    if let Some(ref mut buf) = heap.get_mut(this_ref)?.string_value {
        buf.push_str(if val { "true" } else { "false" });
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(C)Ljava/lang/StringBuilder;`
fn native_sb_append_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let ch = match args.get(1) {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('\0'),
        _ => '\0',
    };
    if let Some(ref mut buf) = heap.get_mut(this_ref)?.string_value {
        buf.push(ch);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.toString()Ljava/lang/String;`
fn native_sb_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap
        .get(this_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `StringBuilder.length()I`
fn native_sb_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let len = heap
        .get(this_ref)?
        .string_value
        .as_ref()
        .map_or(0, String::len);
    Ok(Some(Slot::Int(len as i32)))
}
```

### Step 4: Register all StringBuilder natives in `bootstrap_stdlib`

After the `sb_ctx` registration:

```rust
registry.natives_mut().register(
    "java/lang/StringBuilder", "<init>", "()V", native_sb_init,
);
registry.natives_mut().register(
    "java/lang/StringBuilder", "<init>", "(Ljava/lang/String;)V", native_sb_init_string,
);
registry.natives_mut().register(
    "java/lang/StringBuilder", "append", "(Ljava/lang/String;)Ljava/lang/StringBuilder;", native_sb_append_string,
);
registry.natives_mut().register(
    "java/lang/StringBuilder", "append", "(I)Ljava/lang/StringBuilder;", native_sb_append_int,
);
registry.natives_mut().register(
    "java/lang/StringBuilder", "append", "(J)Ljava/lang/StringBuilder;", native_sb_append_long,
);
registry.natives_mut().register(
    "java/lang/StringBuilder", "append", "(D)Ljava/lang/StringBuilder;", native_sb_append_double,
);
registry.natives_mut().register(
    "java/lang/StringBuilder", "append", "(F)Ljava/lang/StringBuilder;", native_sb_append_float,
);
registry.natives_mut().register(
    "java/lang/StringBuilder", "append", "(Z)Ljava/lang/StringBuilder;", native_sb_append_boolean,
);
registry.natives_mut().register(
    "java/lang/StringBuilder", "append", "(C)Ljava/lang/StringBuilder;", native_sb_append_char,
);
registry.natives_mut().register(
    "java/lang/StringBuilder", "toString", "()Ljava/lang/String;", native_sb_tostring,
);
registry.natives_mut().register(
    "java/lang/StringBuilder", "length", "()I", native_sb_length,
);
```

### Step 5: Write 12 integration tests

```rust
#[test]
fn sb_basic_append() {
    assert_eq!(run_class_int("StringBuilderTest.class", "testBasicAppend", "()I", vec![]), 5);
}

#[test]
fn sb_chaining() {
    assert_eq!(run_class_int("StringBuilderTest.class", "testChaining", "()I", vec![]), 6);
}

#[test]
fn sb_append_int() {
    assert_eq!(run_class_int("StringBuilderTest.class", "testAppendInt", "()I", vec![]), 6);
}

#[test]
fn sb_append_long() {
    assert_eq!(run_class_int("StringBuilderTest.class", "testAppendLong", "()I", vec![]), 3);
}

#[test]
fn sb_append_boolean() {
    assert_eq!(run_class_int("StringBuilderTest.class", "testAppendBoolean", "()I", vec![]), 4);
}

#[test]
fn sb_append_char() {
    assert_eq!(run_class_int("StringBuilderTest.class", "testAppendChar", "()I", vec![]), 1);
}

#[test]
fn sb_append_double() {
    assert_eq!(run_class_int("StringBuilderTest.class", "testAppendDouble", "()I", vec![]), 4);
}

#[test]
fn sb_append_float() {
    assert_eq!(run_class_int("StringBuilderTest.class", "testAppendFloat", "()I", vec![]), 3);
}

#[test]
fn sb_init_with_string() {
    assert_eq!(run_class_int("StringBuilderTest.class", "testInitWithString", "()I", vec![]), 8);
}

#[test]
fn sb_length() {
    assert_eq!(run_class_int("StringBuilderTest.class", "testLength", "()I", vec![]), 3);
}

#[test]
fn sb_loop_build() {
    assert_eq!(run_class_int("StringBuilderTest.class", "testLoopBuild", "()I", vec![]), 5);
}

#[test]
fn sb_append_string_object() {
    assert_eq!(run_class_int("StringBuilderTest.class", "testAppendString", "()I", vec![]), 5);
}
```

### Step 6: Run tests

```bash
cargo test -p duke-interpreter -- sb_
cargo test --workspace
```

### Step 7: Commit

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/StringBuilderTest.java tests/fixtures/StringBuilderTest.class
git commit -m "feat(interpreter): implement java.lang.StringBuilder with 11 native methods"
```

---

## Task 2: Character Synthetic Class + Natives

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (bootstrap_stdlib, native handlers)
- Create: `tests/fixtures/CharacterTest.java`
- Test: `crates/duke-interpreter/src/lib.rs` (test module)

### Step 1: Create the Java fixture

Create `tests/fixtures/CharacterTest.java`:

```java
public class CharacterTest {
    static int testIsDigit() {
        if (Character.isDigit('5')) return 1;
        return 0;
    }

    static int testIsDigitFalse() {
        if (Character.isDigit('A')) return 0;
        return 1;
    }

    static int testIsLetter() {
        if (Character.isLetter('Z')) return 1;
        return 0;
    }

    static int testIsLetterFalse() {
        if (Character.isLetter('3')) return 0;
        return 1;
    }

    static int testIsWhitespace() {
        if (Character.isWhitespace(' ')) return 1;
        return 0;
    }

    static int testIsUpperCase() {
        if (Character.isUpperCase('A') && !Character.isUpperCase('a')) return 1;
        return 0;
    }

    static int testIsLowerCase() {
        if (Character.isLowerCase('z') && !Character.isLowerCase('Z')) return 1;
        return 0;
    }

    static int testToUpperCase() {
        char c = Character.toUpperCase('a');
        return (int) c;  // 'A' = 65
    }

    static int testToLowerCase() {
        char c = Character.toLowerCase('A');
        return (int) c;  // 'a' = 97
    }

    static int testIsLetterOrDigit() {
        if (Character.isLetterOrDigit('a') && Character.isLetterOrDigit('5')
            && !Character.isLetterOrDigit(' ')) return 1;
        return 0;
    }

    // Test valueOf returns a Character object
    static int testValueOf() {
        Character c = Character.valueOf('X');
        char v = c.charValue();
        return (int) v;  // 'X' = 88
    }
}
```

Compile: `javac --release 21 tests/fixtures/CharacterTest.java`

### Step 2: Register synthetic `java/lang/Character` in `bootstrap_stdlib`

After the StringBuilder registration:

```rust
// java/lang/Character — static character utilities + boxed char
let character_ctx = ClassContext {
    class_name: "java/lang/Character".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: vec![FieldEntry {
        name: "value".to_string(),
        descriptor: "C".to_string(),
        is_static: false,
    }],
    static_fields: Vec::new(),
    instance_field_count: 1,
    bootstrap_methods: Vec::new(),
};
registry.register(character_ctx);
```

### Step 3: Implement Character native handlers

```rust
// ---- Character natives ----

/// Native: `Character.isDigit(C)Z` — static
fn native_char_is_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = match args.first() {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('\0'),
        _ => '\0',
    };
    Ok(Some(Slot::Int(i32::from(ch.is_ascii_digit()))))
}

/// Native: `Character.isLetter(C)Z` — static
fn native_char_is_letter(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = match args.first() {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('\0'),
        _ => '\0',
    };
    Ok(Some(Slot::Int(i32::from(ch.is_alphabetic()))))
}

/// Native: `Character.isWhitespace(C)Z` — static
fn native_char_is_whitespace(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = match args.first() {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('\0'),
        _ => '\0',
    };
    Ok(Some(Slot::Int(i32::from(ch.is_whitespace()))))
}

/// Native: `Character.isUpperCase(C)Z` — static
fn native_char_is_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = match args.first() {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('\0'),
        _ => '\0',
    };
    Ok(Some(Slot::Int(i32::from(ch.is_uppercase()))))
}

/// Native: `Character.isLowerCase(C)Z` — static
fn native_char_is_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = match args.first() {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('\0'),
        _ => '\0',
    };
    Ok(Some(Slot::Int(i32::from(ch.is_lowercase()))))
}

/// Native: `Character.toUpperCase(C)C` — static
fn native_char_to_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = match args.first() {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('\0'),
        _ => '\0',
    };
    let upper = ch.to_uppercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(upper as i32)))
}

/// Native: `Character.toLowerCase(C)C` — static
fn native_char_to_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = match args.first() {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('\0'),
        _ => '\0',
    };
    let lower = ch.to_lowercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(lower as i32)))
}

/// Native: `Character.isLetterOrDigit(C)Z` — static
fn native_char_is_letter_or_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = match args.first() {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('\0'),
        _ => '\0',
    };
    Ok(Some(Slot::Int(i32::from(ch.is_alphanumeric()))))
}

/// Native: `Character.valueOf(C)Ljava/lang/Character;` — static, boxes a char
#[allow(clippy::unnecessary_wraps)]
fn native_char_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let r = heap.allocate("java/lang/Character".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Character.charValue()C` — instance, unboxes Character to char
fn native_char_charvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = heap.get(this_ref)?.fields[0].clone();
    Ok(Some(val))
}
```

### Step 4: Register all Character natives in `bootstrap_stdlib`

After the Character ClassContext registration:

```rust
registry.natives_mut().register(
    "java/lang/Character", "isDigit", "(C)Z", native_char_is_digit,
);
registry.natives_mut().register(
    "java/lang/Character", "isLetter", "(C)Z", native_char_is_letter,
);
registry.natives_mut().register(
    "java/lang/Character", "isWhitespace", "(C)Z", native_char_is_whitespace,
);
registry.natives_mut().register(
    "java/lang/Character", "isUpperCase", "(C)Z", native_char_is_uppercase,
);
registry.natives_mut().register(
    "java/lang/Character", "isLowerCase", "(C)Z", native_char_is_lowercase,
);
registry.natives_mut().register(
    "java/lang/Character", "toUpperCase", "(C)C", native_char_to_uppercase,
);
registry.natives_mut().register(
    "java/lang/Character", "toLowerCase", "(C)C", native_char_to_lowercase,
);
registry.natives_mut().register(
    "java/lang/Character", "isLetterOrDigit", "(C)Z", native_char_is_letter_or_digit,
);
registry.natives_mut().register(
    "java/lang/Character", "valueOf", "(C)Ljava/lang/Character;", native_char_valueof,
);
registry.natives_mut().register(
    "java/lang/Character", "charValue", "()C", native_char_charvalue,
);
```

### Step 5: Write 11 integration tests

```rust
#[test]
fn char_is_digit() {
    assert_eq!(run_class_int("CharacterTest.class", "testIsDigit", "()I", vec![]), 1);
}

#[test]
fn char_is_digit_false() {
    assert_eq!(run_class_int("CharacterTest.class", "testIsDigitFalse", "()I", vec![]), 1);
}

#[test]
fn char_is_letter() {
    assert_eq!(run_class_int("CharacterTest.class", "testIsLetter", "()I", vec![]), 1);
}

#[test]
fn char_is_letter_false() {
    assert_eq!(run_class_int("CharacterTest.class", "testIsLetterFalse", "()I", vec![]), 1);
}

#[test]
fn char_is_whitespace() {
    assert_eq!(run_class_int("CharacterTest.class", "testIsWhitespace", "()I", vec![]), 1);
}

#[test]
fn char_is_uppercase() {
    assert_eq!(run_class_int("CharacterTest.class", "testIsUpperCase", "()I", vec![]), 1);
}

#[test]
fn char_is_lowercase() {
    assert_eq!(run_class_int("CharacterTest.class", "testIsLowerCase", "()I", vec![]), 1);
}

#[test]
fn char_to_uppercase() {
    assert_eq!(run_class_int("CharacterTest.class", "testToUpperCase", "()I", vec![]), 65);
}

#[test]
fn char_to_lowercase() {
    assert_eq!(run_class_int("CharacterTest.class", "testToLowerCase", "()I", vec![]), 97);
}

#[test]
fn char_is_letter_or_digit() {
    assert_eq!(run_class_int("CharacterTest.class", "testIsLetterOrDigit", "()I", vec![]), 1);
}

#[test]
fn char_valueof_and_charvalue() {
    assert_eq!(run_class_int("CharacterTest.class", "testValueOf", "()I", vec![]), 88);
}
```

### Step 6: Run tests

```bash
cargo test -p duke-interpreter -- char_
cargo test --workspace
```

### Step 7: Commit

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/CharacterTest.java tests/fixtures/CharacterTest.class
git commit -m "feat(interpreter): implement java.lang.Character with 10 native methods"
```

---

## Task 3: Smoke Tests + Plan Doc Commit

**Files:**
- Modify: `docs/plans/2026-03-05-phase20-stringbuilder-character.md` (mark complete)
- Run: full test suite + CLI smoke tests

### Step 1: Run full test suite

```bash
cargo test --workspace
```

Expected: ~289+ tests passing, zero failures.

### Step 2: CLI smoke tests

```bash
cargo run -- exec tests/fixtures/StringBuilderTest.class testChaining
# Expected: Int(6)

cargo run -- exec tests/fixtures/StringBuilderTest.class testLoopBuild
# Expected: Int(5)

cargo run -- exec tests/fixtures/CharacterTest.class testToUpperCase
# Expected: Int(65)

cargo run -- exec tests/fixtures/CharacterTest.class testIsDigit
# Expected: Int(1)
```

### Step 3: Commit plan doc

```bash
git add docs/plans/2026-03-05-phase20-stringbuilder-character.md
git commit -m "docs: add phase 20 plan (StringBuilder + Character natives)"
```

---

## Summary

| Task | What | New Natives | New Tests |
|------|------|-------------|-----------|
| 1 | StringBuilder synthetic class | 11 (init x2, append x7, toString, length) | 12 |
| 2 | Character synthetic class | 10 (isDigit, isLetter, isWhitespace, isUpperCase, isLowerCase, toUpperCase, toLowerCase, isLetterOrDigit, valueOf, charValue) | 11 |
| 3 | Smoke tests + plan doc | 0 | 0 |

**Total new tests**: ~23
**Expected final test count**: ~289+
**New natives**: 21
**New synthetic classes**: 2 (`java/lang/StringBuilder`, `java/lang/Character`)
