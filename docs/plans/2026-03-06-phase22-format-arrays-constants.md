# Phase 22: String.format + Arrays Utilities + Numeric Constants — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `String.format()` with basic format specifiers, `java.util.Arrays` fill/copyOf/sort utilities, and `Integer/Long/Double/Float` class constants (`MAX_VALUE`, `MIN_VALUE`, etc.).

**Architecture:** All additions are native handlers registered in `bootstrap_stdlib()`. Arrays is a pure-static synthetic class. Numeric constants are added as static fields to existing ClassContexts. `String.format` is a static native that parses the format string and formats each boxed arg from the Object[] varargs array. Arrays are stored as `HeapObject.fields` (already established — `arraylength` reads `fields.len()`).

**Tech Stack:** Rust, duke-interpreter, duke-gc, javac 21

---

## Background

### How `String.format` is compiled

`String.format("%d + %d = %d", a, b, c)` compiles to:

```
ldc "%d + %d = %d"
iconst_3
anewarray java/lang/Object       ← varargs array
dup / iconst_0 / iload a / invokestatic Integer.valueOf(I) / aastore
dup / iconst_1 / iload b / invokestatic Integer.valueOf(I) / aastore
dup / iconst_2 / iload c / invokestatic Integer.valueOf(I) / aastore
invokestatic java/lang/String.format(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;
```

The native receives:
- `args[0]` = format string reference
- `args[1]` = Object[] array reference (elements at `heap.get(r)?.fields[0..n]`)

Each element is a boxed value. To unbox:
- **Integer**: `class_name == "java/lang/Integer"`, value at `fields[0]` = `Slot::Int`
- **Long**: `class_name == "java/lang/Long"`, value at `fields[0]` = `Slot::Long`
- **Double**: `class_name == "java/lang/Double"`, value at `fields[0]` = `Slot::Double`
- **Float**: `class_name == "java/lang/Float"`, value at `fields[0]` = `Slot::Float`
- **String**: `string_value` is `Some(...)`
- **Null ref**: print as `"null"`

### How `static_field_idx` works

`getstatic` calls `static_field_idx(ctx, name)` which counts only `is_static=true` fields in order. So to add `Integer.MAX_VALUE` and `MIN_VALUE`:

```rust
fields: vec![
    FieldEntry { name: "value".to_string(), descriptor: "I".to_string(), is_static: false },      // instance field 0
    FieldEntry { name: "MAX_VALUE".to_string(), descriptor: "I".to_string(), is_static: true },   // static idx 0
    FieldEntry { name: "MIN_VALUE".to_string(), descriptor: "I".to_string(), is_static: true },   // static idx 1
],
static_fields: vec![Slot::Int(i32::MAX), Slot::Int(i32::MIN)],
```

---

## Task 1: String.format() Native

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` — `bootstrap_stdlib()` String section, native handlers
- Create: `tests/fixtures/StringFormatTest.java`

### Step 1: Create Java fixture

Create `tests/fixtures/StringFormatTest.java`:

```java
public class StringFormatTest {
    // %s with String arg
    static int testFormatString() {
        String s = String.format("hello %s", "world");
        return s.length();  // "hello world" = 11
    }

    // %d with int arg
    static int testFormatInt() {
        String s = String.format("%d", 42);
        return s.length();  // "42" = 2
    }

    // Multiple args
    static int testFormatMultiple() {
        String s = String.format("%s=%d", "x", 7);
        return s.length();  // "x=7" = 3
    }

    // %f with double — default precision gives 6 decimal places
    static int testFormatDouble() {
        String s = String.format("%.2f", 3.14159);
        return s.length();  // "3.14" = 4
    }

    // %x hex format
    static int testFormatHex() {
        String s = String.format("%x", 255);
        return s.length();  // "ff" = 2
    }

    // %% literal percent
    static int testFormatPercent() {
        String s = String.format("100%%");
        return s.length();  // "100%" = 4
    }

    // Null arg with %s
    static int testFormatNull() {
        String s = String.format("%s", (Object) null);
        return s.length();  // "null" = 4
    }

    // Integer arithmetic then format
    static int testFormatSum() {
        int a = 10, b = 32;
        String s = String.format("%d+%d=%d", a, b, a + b);
        return s.length();  // "10+32=42" = 8
    }
}
```

Compile: `javac --release 21 tests/fixtures/StringFormatTest.java`

### Step 2: Add `String.format` native to `bootstrap_stdlib()`

Find the String natives registration block in `bootstrap_stdlib()`. After the last String registration (currently `native_string_concat`, ~line 922), add inside `bootstrap_stdlib()` before its closing `}`:

```rust
registry.natives_mut().register(
    "java/lang/String",
    "format",
    "(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;",
    native_string_format,
);
```

### Step 3: Implement `format_arg` helper

Add this helper function near the other String natives (after `native_string_concat`, ~line 2005):

```rust
/// Formats a single boxed slot value using the given format specifier.
///
/// `spec` is the format character (s, d, f, x, X).
/// `precision` is the optional precision from `%.Nf`.
fn format_arg(
    spec: char,
    precision: Option<usize>,
    slot: &Slot,
    heap: &duke_gc::Heap,
) -> VmResult<String> {
    match slot {
        Slot::Reference(None) => Ok("null".to_string()),
        Slot::Reference(Some(r)) => {
            let obj = heap.get(*r)?;
            match spec {
                's' => {
                    // String or fallback to numeric string
                    if let Some(ref s) = obj.string_value {
                        return Ok(s.clone());
                    }
                    match obj.fields.first() {
                        Some(Slot::Int(v)) => Ok(v.to_string()),
                        Some(Slot::Long(v)) => Ok(v.to_string()),
                        Some(Slot::Double(v)) => Ok(v.to_string()),
                        Some(Slot::Float(v)) => Ok(v.to_string()),
                        _ => Ok(format!("{}@{:x}", obj.class_name, r)),
                    }
                }
                'd' => match obj.fields.first() {
                    Some(Slot::Int(v)) => Ok(v.to_string()),
                    Some(Slot::Long(v)) => Ok(v.to_string()),
                    _ => Ok("0".to_string()),
                },
                'f' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Double(v)) => *v,
                        Some(Slot::Float(v)) => f64::from(*v),
                        _ => 0.0,
                    };
                    match precision {
                        Some(p) => Ok(format!("{:.prec$}", v, prec = p)),
                        None => Ok(format!("{v:.6}")),
                    }
                }
                'x' => match obj.fields.first() {
                    Some(Slot::Int(v)) => Ok(format!("{:x}", v)),
                    Some(Slot::Long(v)) => Ok(format!("{:x}", v)),
                    _ => Ok("0".to_string()),
                },
                'X' => match obj.fields.first() {
                    Some(Slot::Int(v)) => Ok(format!("{:X}", v)),
                    Some(Slot::Long(v)) => Ok(format!("{:X}", v)),
                    _ => Ok("0".to_string()),
                },
                _ => Ok(String::new()),
            }
        }
        _ => Ok(String::new()),
    }
}
```

### Step 4: Implement `native_string_format`

Add after `format_arg`:

```rust
/// Native: `String.format(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;`
///
/// Parses the format string and formats each boxed arg from the Object[] varargs array.
/// Supports: %s, %d, %f, %.Nf, %x, %X, %%, %n
fn native_string_format(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let fmt_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let fmt = heap.get(fmt_ref)?.string_value.clone().unwrap_or_default();

    // If no args array (or null), return format string as-is
    let arr_len = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.fields.len(),
        Some(Slot::Reference(None)) | None => 0,
        _ => 0,
    };

    let mut result = String::new();
    let mut arg_idx = 0usize;
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] != '%' {
            result.push(chars[i]);
            i += 1;
            continue;
        }
        i += 1; // skip %
        if i >= chars.len() {
            break;
        }

        // Parse optional precision: %.2f
        let mut precision: Option<usize> = None;
        if chars[i] == '.' {
            i += 1;
            let mut prec_str = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                prec_str.push(chars[i]);
                i += 1;
            }
            precision = prec_str.parse().ok();
        }

        // Skip optional width digits (e.g. %10d — honour position but ignore width)
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }

        if i >= chars.len() {
            break;
        }
        let spec = chars[i];
        i += 1;

        match spec {
            '%' => result.push('%'),
            'n' => result.push('\n'),
            's' | 'd' | 'f' | 'x' | 'X' => {
                let slot = if arg_idx < arr_len {
                    match args.get(1) {
                        Some(Slot::Reference(Some(r))) => {
                            heap.get(*r)?.fields.get(arg_idx).cloned()
                                .unwrap_or(Slot::Reference(None))
                        }
                        _ => Slot::Reference(None),
                    }
                } else {
                    Slot::Reference(None)
                };
                arg_idx += 1;
                let formatted = format_arg(spec, precision, &slot, heap)?;
                result.push_str(&formatted);
            }
            _ => {
                result.push('%');
                result.push(spec);
            }
        }
    }

    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}
```

### Step 5: Write 8 integration tests

```rust
#[test]
fn format_string() {
    assert_eq!(run_class_int("StringFormatTest.class", "testFormatString", "()I", vec![]), 11);
}

#[test]
fn format_int() {
    assert_eq!(run_class_int("StringFormatTest.class", "testFormatInt", "()I", vec![]), 2);
}

#[test]
fn format_multiple() {
    assert_eq!(run_class_int("StringFormatTest.class", "testFormatMultiple", "()I", vec![]), 3);
}

#[test]
fn format_double() {
    assert_eq!(run_class_int("StringFormatTest.class", "testFormatDouble", "()I", vec![]), 4);
}

#[test]
fn format_hex() {
    assert_eq!(run_class_int("StringFormatTest.class", "testFormatHex", "()I", vec![]), 2);
}

#[test]
fn format_percent() {
    assert_eq!(run_class_int("StringFormatTest.class", "testFormatPercent", "()I", vec![]), 4);
}

#[test]
fn format_null() {
    assert_eq!(run_class_int("StringFormatTest.class", "testFormatNull", "()I", vec![]), 4);
}

#[test]
fn format_sum() {
    assert_eq!(run_class_int("StringFormatTest.class", "testFormatSum", "()I", vec![]), 8);
}
```

### Step 6: Run tests

```bash
cargo test -p duke-interpreter -- format_
cargo test --workspace
```

Expected: 8 new tests pass, no regressions.

### Step 7: Commit

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/StringFormatTest.java tests/fixtures/StringFormatTest.class
git commit -m "feat(interpreter): implement String.format() with %s %d %f %x %% %n specifiers"
```

---

## Task 2: java.util.Arrays + Numeric Constants

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` — `bootstrap_stdlib()` Integer/Long/Double/Float definitions, Arrays registration, native handlers
- Create: `tests/fixtures/ArraysTest.java`

### Step 1: Create Java fixture

Create `tests/fixtures/ArraysTest.java`:

```java
import java.util.Arrays;

public class ArraysTest {
    // Arrays.fill(int[], int)
    static int testFillInt() {
        int[] arr = new int[4];
        Arrays.fill(arr, 7);
        return arr[0] + arr[3];  // 7 + 7 = 14
    }

    // Arrays.copyOf(int[], int) — truncate
    static int testCopyOfTruncate() {
        int[] arr = {10, 20, 30, 40, 50};
        int[] copy = Arrays.copyOf(arr, 3);
        return copy.length;  // 3
    }

    // Arrays.copyOf(int[], int) — extend with zeros
    static int testCopyOfExtend() {
        int[] arr = {1, 2};
        int[] copy = Arrays.copyOf(arr, 5);
        return copy[4];  // 0 (zero-padded)
    }

    // Arrays.sort(int[])
    static int testSortInt() {
        int[] arr = {5, 2, 8, 1, 9, 3};
        Arrays.sort(arr);
        return arr[0] * 10 + arr[5];  // smallest=1, largest=9 → 19
    }

    // Integer.MAX_VALUE
    static int testIntegerMaxValue() {
        if (Integer.MAX_VALUE == 2147483647) return 1;
        return 0;
    }

    // Integer.MIN_VALUE
    static int testIntegerMinValue() {
        if (Integer.MIN_VALUE == -2147483648) return 1;
        return 0;
    }

    // Long.MAX_VALUE (return as int — just check it's positive)
    static int testLongMaxValue() {
        if (Long.MAX_VALUE > 0) return 1;
        return 0;
    }

    // Double.MAX_VALUE > 0
    static int testDoubleMaxValue() {
        if (Double.MAX_VALUE > 0.0) return 1;
        return 0;
    }

    // Double.NaN
    static int testDoubleNaN() {
        double n = Double.NaN;
        if (Double.isNaN(n)) return 1;
        return 0;
    }
}
```

Compile: `javac --release 21 tests/fixtures/ArraysTest.java`

### Step 2: Add numeric constants to existing ClassContexts in `bootstrap_stdlib()`

**For `java/lang/Integer`** (find the `integer_ctx` definition, ~line 655):

Replace the existing definition with one that includes static constant fields:

```rust
let integer_ctx = ClassContext {
    class_name: "java/lang/Integer".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: vec![
        FieldEntry { name: "value".to_string(), descriptor: "I".to_string(), is_static: false },
        FieldEntry { name: "MAX_VALUE".to_string(), descriptor: "I".to_string(), is_static: true },
        FieldEntry { name: "MIN_VALUE".to_string(), descriptor: "I".to_string(), is_static: true },
        FieldEntry { name: "SIZE".to_string(), descriptor: "I".to_string(), is_static: true },
    ],
    static_fields: vec![Slot::Int(i32::MAX), Slot::Int(i32::MIN), Slot::Int(32)],
    instance_field_count: 1,
    bootstrap_methods: Vec::new(),
};
```

**For `java/lang/Long`** (~line 698):

```rust
let long_ctx = ClassContext {
    class_name: "java/lang/Long".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: vec![
        FieldEntry { name: "value".to_string(), descriptor: "J".to_string(), is_static: false },
        FieldEntry { name: "MAX_VALUE".to_string(), descriptor: "J".to_string(), is_static: true },
        FieldEntry { name: "MIN_VALUE".to_string(), descriptor: "J".to_string(), is_static: true },
    ],
    static_fields: vec![Slot::Long(i64::MAX), Slot::Long(i64::MIN)],
    instance_field_count: 1,
    bootstrap_methods: Vec::new(),
};
```

**For `java/lang/Double`** (~line 736):

```rust
let double_ctx = ClassContext {
    class_name: "java/lang/Double".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: vec![
        FieldEntry { name: "value".to_string(), descriptor: "D".to_string(), is_static: false },
        FieldEntry { name: "MAX_VALUE".to_string(), descriptor: "D".to_string(), is_static: true },
        FieldEntry { name: "MIN_VALUE".to_string(), descriptor: "D".to_string(), is_static: true },
        FieldEntry { name: "NaN".to_string(), descriptor: "D".to_string(), is_static: true },
        FieldEntry { name: "POSITIVE_INFINITY".to_string(), descriptor: "D".to_string(), is_static: true },
        FieldEntry { name: "NEGATIVE_INFINITY".to_string(), descriptor: "D".to_string(), is_static: true },
    ],
    static_fields: vec![
        Slot::Double(f64::MAX),
        Slot::Double(f64::MIN_POSITIVE),
        Slot::Double(f64::NAN),
        Slot::Double(f64::INFINITY),
        Slot::Double(f64::NEG_INFINITY),
    ],
    instance_field_count: 1,
    bootstrap_methods: Vec::new(),
};
```

Also add `Double.isNaN(D)Z` native registration after Double:
```rust
registry.natives_mut().register(
    "java/lang/Double",
    "isNaN",
    "(D)Z",
    native_double_isnan,
);
```

**For `java/lang/Float`** (~line 771) — add MIN_VALUE/MAX_VALUE similarly (optional, skip if it adds too many lines).

### Step 3: Register synthetic `java/util/Arrays` class

In `bootstrap_stdlib()`, before closing `}`:

```rust
// java/util/Arrays — static array utilities, no instance methods
let arrays_ctx = ClassContext {
    class_name: "java/util/Arrays".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: Vec::new(),
    static_fields: Vec::new(),
    instance_field_count: 0,
    bootstrap_methods: Vec::new(),
};
registry.register(arrays_ctx);
registry.natives_mut().register(
    "java/util/Arrays", "fill", "([II)V", native_arrays_fill_int,
);
registry.natives_mut().register(
    "java/util/Arrays", "fill", "([Ljava/lang/Object;Ljava/lang/Object;)V", native_arrays_fill_object,
);
registry.natives_mut().register(
    "java/util/Arrays", "copyOf", "([II)[I", native_arrays_copyof_int,
);
registry.natives_mut().register(
    "java/util/Arrays", "copyOf", "([Ljava/lang/Object;I)[Ljava/lang/Object;", native_arrays_copyof_object,
);
registry.natives_mut().register(
    "java/util/Arrays", "sort", "([I)V", native_arrays_sort_int,
);
```

### Step 4: Implement native handlers

Add after the ArrayList natives:

```rust
// ---- Double.isNaN ----

/// Native: `Double.isNaN(D)Z` — returns 1 if value is NaN.
fn native_double_isnan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Double(v)) => Ok(Some(Slot::Int(i32::from(v.is_nan())))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

// ---- Arrays natives ----

/// Native: `Arrays.fill(int[], int)` — fills all elements with val.
fn native_arrays_fill_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let arr_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Int(v)) => Slot::Int(*v),
        _ => Slot::Int(0),
    };
    let obj = heap.get_mut(arr_ref)?;
    for slot in &mut obj.fields {
        *slot = val.clone();
    }
    Ok(None)
}

/// Native: `Arrays.fill(Object[], Object)` — fills all elements with val.
fn native_arrays_fill_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let arr_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let obj = heap.get_mut(arr_ref)?;
    for slot in &mut obj.fields {
        *slot = val.clone();
    }
    Ok(None)
}

/// Native: `Arrays.copyOf(int[], int)` — copies src to new int[] of given length.
#[allow(clippy::cast_sign_loss)]
fn native_arrays_copyof_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let src_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let new_len = match args.get(1) {
        Some(Slot::Int(n)) => *n as usize,
        _ => 0,
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[I".to_string(), new_len);
    let dst = heap.get_mut(dst_ref)?;
    for i in 0..new_len {
        dst.fields[i] = src_fields.get(i).cloned().unwrap_or(Slot::Int(0));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.copyOf(Object[], int)` — copies src to new Object[] of given length.
#[allow(clippy::cast_sign_loss)]
fn native_arrays_copyof_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let src_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let new_len = match args.get(1) {
        Some(Slot::Int(n)) => *n as usize,
        _ => 0,
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[Ljava/lang/Object;".to_string(), new_len);
    let dst = heap.get_mut(dst_ref)?;
    for i in 0..new_len {
        dst.fields[i] = src_fields.get(i).cloned().unwrap_or(Slot::Reference(None));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.sort(int[])` — sorts in-place using Rust's stable sort.
fn native_arrays_sort_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let arr_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get_mut(arr_ref)?;
    obj.fields.sort_by(|a, b| match (a, b) {
        (Slot::Int(x), Slot::Int(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    });
    Ok(None)
}
```

### Step 5: Write 9 integration tests

```rust
#[test]
fn arrays_fill_int() {
    assert_eq!(run_class_int("ArraysTest.class", "testFillInt", "()I", vec![]), 14);
}

#[test]
fn arrays_copyof_truncate() {
    assert_eq!(run_class_int("ArraysTest.class", "testCopyOfTruncate", "()I", vec![]), 3);
}

#[test]
fn arrays_copyof_extend() {
    assert_eq!(run_class_int("ArraysTest.class", "testCopyOfExtend", "()I", vec![]), 0);
}

#[test]
fn arrays_sort_int() {
    assert_eq!(run_class_int("ArraysTest.class", "testSortInt", "()I", vec![]), 19);
}

#[test]
fn integer_max_value() {
    assert_eq!(run_class_int("ArraysTest.class", "testIntegerMaxValue", "()I", vec![]), 1);
}

#[test]
fn integer_min_value() {
    assert_eq!(run_class_int("ArraysTest.class", "testIntegerMinValue", "()I", vec![]), 1);
}

#[test]
fn long_max_value() {
    assert_eq!(run_class_int("ArraysTest.class", "testLongMaxValue", "()I", vec![]), 1);
}

#[test]
fn double_max_value() {
    assert_eq!(run_class_int("ArraysTest.class", "testDoubleMaxValue", "()I", vec![]), 1);
}

#[test]
fn double_nan() {
    assert_eq!(run_class_int("ArraysTest.class", "testDoubleNaN", "()I", vec![]), 1);
}
```

### Step 6: Run tests

```bash
cargo test -p duke-interpreter -- "arrays_|integer_|long_|double_"
cargo test --workspace
```

Expected: 9 new tests pass, no regressions (~313 total).

### Step 7: Commit

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/ArraysTest.java tests/fixtures/ArraysTest.class
git commit -m "feat(interpreter): add Arrays utilities, numeric constants, and Double.isNaN"
```

---

## Task 3: Smoke Tests + Plan Doc Commit

### Step 1: Full suite

```bash
cargo test --workspace
```

Expected: ~313+ tests, 0 failures.

### Step 2: CLI smoke tests

```bash
cargo run -- exec tests/fixtures/StringFormatTest.class testFormatSum
# Expected: Int(8)

cargo run -- exec tests/fixtures/ArraysTest.class testSortInt
# Expected: Int(19)

cargo run -- exec tests/fixtures/ArraysTest.class testDoubleNaN
# Expected: Int(1)
```

### Step 3: Commit plan doc

```bash
git add docs/plans/2026-03-06-phase22-format-arrays-constants.md
git commit -m "docs: add phase 22 plan (String.format + Arrays + numeric constants)"
```

---

## Summary

| Task | What | New Natives | New Tests |
|------|------|-------------|-----------|
| 1 | `String.format()` with %s %d %f %x %% %n | 1 (+ helper fn) | 8 |
| 2 | `java.util.Arrays` (fill×2, copyOf×2, sort) + Integer/Long/Double constants + Double.isNaN | 6 | 9 |
| 3 | Smoke + plan doc | 0 | 0 |

**Total new tests**: ~17
**Expected final count**: ~313
**New synthetic classes**: 1 (`java/util/Arrays`)
**New natives**: 7
