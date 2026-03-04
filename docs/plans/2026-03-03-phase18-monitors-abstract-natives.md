# Phase 18: Monitor Stubs, Abstract Classes & Extended Natives

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add monitor no-op stubs (unblocking synchronized code), abstract class enforcement, and a large batch of missing native methods (Math, Long, Double, Float, Boolean, extended String ops) to make Duke capable of running realistic Java programs.

**Architecture:** All changes are in `crates/duke-interpreter/src/lib.rs` (opcode handlers + native registrations in `bootstrap_stdlib`) and `crates/duke-runtime/src/error.rs` (new error variant). Three new test fixture Java files exercise the additions. Monitor stubs are no-ops since Duke is single-threaded. Abstract enforcement prevents instantiation of abstract classes and calls to unimplemented abstract methods.

**Tech Stack:** Rust, duke-interpreter, duke-runtime, javac --release 21

---

## Background

### What's Missing
1. **monitorenter/monitorexit** — Already decoded in `duke-bytecode` as `Instruction::Monitorenter` / `Instruction::Monitorexit`, but the `execute_class` match block in `duke-interpreter` has no arms for them → `VmError::Unimplemented`. Many JDK classes and user code use `synchronized`, so these crash immediately.
2. **Abstract enforcement** — `ACC_ABSTRACT` on classes/methods is parsed by `duke-classfile` in access flags, but the interpreter never checks it. You can `new AbstractClass()` without error.
3. **Extended Math** — Only `max(int,int)`, `min(int,int)`, `abs(int)` are registered. Missing: `sqrt`, `pow`, `floor`, `ceil`, `round`, `abs(long)`, `abs(double)`, `max/min(long)`, `max/min(double)`, `PI`, `E`.
4. **Long/Double/Float/Boolean classes** — `Long.parseLong`, `Double.parseDouble`, `Float.parseFloat`, `Boolean.parseBoolean`, `Long.valueOf`, `Long.toString`, etc. are all missing.
5. **Extended String** — `toUpperCase`, `toLowerCase`, `replace`, `split`, `hashCode`, `toString` (returns self) are missing.

### Key Patterns
- Native handlers are `fn(&[Slot], &mut Heap, &mut dyn Write) -> VmResult<Option<Slot>>`
- Instance methods get `this` as `args[0]`, then params from `args[1..]`
- Static methods get params from `args[0..]`
- String objects: `heap.get(r)?.string_value.clone().unwrap_or_default()` to read content
- Allocate string result: `heap.allocate_string(val.to_string())` → `Ok(Some(Slot::Reference(Some(r))))`
- Bootstrap registration: `registry.natives_mut().register("class", "method", "descriptor", fn_name)`
- Synthetic ClassContext: set `super_class: Some("java/lang/Object".to_string())`, empty methods/fields, `bootstrap_methods: Vec::new()`

---

### Task 1: Monitor Stubs & Abstract Class Enforcement

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (add monitorenter/monitorexit arms + abstract checks)
- Modify: `crates/duke-runtime/src/error.rs` (add `AbstractMethodError` and `InstantiationError` variants)
- Create: `tests/fixtures/MonitorAndAbstract.java`

**Step 1: Write the test fixture**

Create `tests/fixtures/MonitorAndAbstract.java`:

```java
public class MonitorAndAbstract {
    // Test: synchronized block doesn't crash (monitor no-op)
    public static int syncBlock(int x) {
        Object lock = new Object();
        int result;
        synchronized (lock) {
            result = x * 2;
        }
        return result;
    }

    // Test: synchronized method doesn't crash
    public static synchronized int syncMethod(int x) {
        return x + 10;
    }

    // Test: nested synchronized
    public static int nestedSync(int x) {
        Object a = new Object();
        Object b = new Object();
        int r;
        synchronized (a) {
            synchronized (b) {
                r = x + 5;
            }
        }
        return r;
    }
}
```

Compile: `javac --release 21 tests/fixtures/MonitorAndAbstract.java`

**Step 2: Add VmError variants**

In `crates/duke-runtime/src/error.rs`, add two new variants to the `VmError` enum:

```rust
    #[error("InstantiationError: cannot instantiate abstract class {class_name}")]
    InstantiationError { class_name: String },

    #[error("AbstractMethodError: {class_name}.{method_name}")]
    AbstractMethodError {
        class_name: String,
        method_name: String,
    },
```

**Step 3: Add monitorenter/monitorexit no-op arms**

In `crates/duke-interpreter/src/lib.rs`, in the `execute_class` main match block (before the `other => Unimplemented` catch-all near line 5003), add:

```rust
            // Monitor stubs — Duke is single-threaded, so these are no-ops.
            // monitorenter pops objectref, monitorexit pops objectref.
            Instruction::Monitorenter => {
                let _obj = frame.pop()?;
            }
            Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }
```

Also add identical arms in the `execute()` (standalone) match block (before its `other => Unimplemented` catch-all near line 2959):

```rust
            Instruction::Monitorenter => {
                let _obj = frame.pop()?;
            }
            Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }
```

**Step 4: Add integration tests**

Add tests in the `#[cfg(test)]` module at the bottom of `crates/duke-interpreter/src/lib.rs`:

```rust
    // ---- Phase 18: MonitorAndAbstract ----

    fn load_monitor_class() -> ClassContext {
        let bytes = std::fs::read("tests/fixtures/MonitorAndAbstract.class").unwrap();
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn monitor_sync_block() {
        let ctx = load_monitor_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "MonitorAndAbstract", "syncBlock", "(I)I", &[Slot::Int(7)],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(14)));
    }

    #[test]
    fn monitor_sync_method() {
        let ctx = load_monitor_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "MonitorAndAbstract", "syncMethod", "(I)I", &[Slot::Int(5)],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(15)));
    }

    #[test]
    fn monitor_nested_sync() {
        let ctx = load_monitor_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "MonitorAndAbstract", "nestedSync", "(I)I", &[Slot::Int(10)],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(15)));
    }
```

**Step 5: Run tests**

```bash
cargo test -p duke-interpreter -- monitor
```

Expected: 3 tests pass.

**Step 6: Commit**

```bash
git add crates/duke-runtime/src/error.rs crates/duke-interpreter/src/lib.rs tests/fixtures/MonitorAndAbstract.java tests/fixtures/MonitorAndAbstract.class
git commit -m "feat(interpreter): add monitorenter/monitorexit no-op stubs and abstract error variants"
```

---

### Task 2: Extended Math & Numeric Class Natives

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (new native handler fns + registrations in `bootstrap_stdlib`)
- Create: `tests/fixtures/ExtendedMath.java`

**Step 1: Write the test fixture**

Create `tests/fixtures/ExtendedMath.java`:

```java
public class ExtendedMath {
    // Math.sqrt
    public static int testSqrt() {
        double r = Math.sqrt(16.0);
        return r == 4.0 ? 1 : 0;
    }

    // Math.pow
    public static int testPow() {
        double r = Math.pow(2.0, 10.0);
        return r == 1024.0 ? 1 : 0;
    }

    // Math.floor / Math.ceil
    public static int testFloorCeil() {
        double f = Math.floor(3.7);
        double c = Math.ceil(3.2);
        return (f == 3.0 && c == 4.0) ? 1 : 0;
    }

    // Math.round(double) → long
    public static long testRound() {
        return Math.round(3.6);
    }

    // Math.abs for long and double
    public static int testAbsLong() {
        long r = Math.abs(-42L);
        return r == 42L ? 1 : 0;
    }

    public static int testAbsDouble() {
        double r = Math.abs(-3.14);
        return r == 3.14 ? 1 : 0;
    }

    // Math.max/min for long
    public static long testMaxLong() {
        return Math.max(100L, 200L);
    }

    public static long testMinLong() {
        return Math.min(100L, 200L);
    }

    // Math.max/min for double
    public static int testMaxDouble() {
        double r = Math.max(1.5, 2.5);
        return r == 2.5 ? 1 : 0;
    }

    // Long.parseLong
    public static long testParseLong() {
        return Long.parseLong("9876543210");
    }

    // Double.parseDouble
    public static int testParseDouble() {
        double d = Double.parseDouble("3.14");
        // Compare with epsilon
        return (d > 3.13 && d < 3.15) ? 1 : 0;
    }

    // Float.parseFloat
    public static int testParseFloat() {
        float f = Float.parseFloat("2.5");
        return f == 2.5f ? 1 : 0;
    }

    // Boolean.parseBoolean
    public static int testParseBoolean() {
        boolean t = Boolean.parseBoolean("true");
        boolean f = Boolean.parseBoolean("false");
        boolean x = Boolean.parseBoolean("yes");
        return (t && !f && !x) ? 1 : 0;
    }

    // Long.valueOf / Long.toString
    public static int testLongValueOf() {
        Long boxed = Long.valueOf(42L);
        long val = boxed.longValue();
        return val == 42L ? 1 : 0;
    }

    // Math.PI and Math.E (static fields)
    public static int testMathConstants() {
        double pi = Math.PI;
        double e = Math.E;
        return (pi > 3.14 && pi < 3.15 && e > 2.71 && e < 2.72) ? 1 : 0;
    }
}
```

Compile: `javac --release 21 tests/fixtures/ExtendedMath.java`

**Step 2: Add native handler functions**

In `crates/duke-interpreter/src/lib.rs`, add these native handler functions after the existing `native_math_abs_int` (around line 1690):

```rust
// ---- Extended Math natives ----

fn native_math_sqrt(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    Ok(Some(Slot::Double(a.sqrt())))
}

fn native_math_pow(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    Ok(Some(Slot::Double(a.powf(b))))
}

fn native_math_floor(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    Ok(Some(Slot::Double(a.floor())))
}

fn native_math_ceil(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    Ok(Some(Slot::Double(a.ceil())))
}

#[allow(clippy::cast_possible_truncation)]
fn native_math_round_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    Ok(Some(Slot::Long(a.round() as i64)))
}

fn native_math_abs_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Long", got: "other" }),
    };
    Ok(Some(Slot::Long(a.wrapping_abs())))
}

fn native_math_abs_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    Ok(Some(Slot::Double(a.abs())))
}

fn native_math_max_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Long", got: "other" }),
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Long", got: "other" }),
    };
    Ok(Some(Slot::Long(a.max(b))))
}

fn native_math_min_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Long", got: "other" }),
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Long", got: "other" }),
    };
    Ok(Some(Slot::Long(a.min(b))))
}

fn native_math_max_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    Ok(Some(Slot::Double(a.max(b))))
}

fn native_math_min_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    Ok(Some(Slot::Double(a.min(b))))
}

// ---- Long class natives ----

fn native_long_parselong(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: i64 = s.trim().parse().map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Long(val)))
}

fn native_long_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Long", got: "other" }),
    };
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_long_longvalue(
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

fn native_long_tostring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Long", got: "other" }),
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

// ---- Double class natives ----

fn native_double_parsedouble(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f64 = s.trim().parse().map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Double(val)))
}

fn native_double_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Double", got: "other" }),
    };
    let r = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Double(val);
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_double_doublevalue(
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

// ---- Float class natives ----

fn native_float_parsefloat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f32 = s.trim().parse().map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Float(val)))
}

// ---- Boolean class natives ----

fn native_boolean_parseboolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Ok(Some(Slot::Int(0))),
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val = s.eq_ignore_ascii_case("true");
    Ok(Some(Slot::Int(i32::from(val))))
}
```

**Step 3: Register in `bootstrap_stdlib`**

In `bootstrap_stdlib()`, after the existing `Math.abs(I)I` registration (around line 609), add:

```rust
    // Extended Math natives
    registry.natives_mut().register("java/lang/Math", "sqrt", "(D)D", native_math_sqrt);
    registry.natives_mut().register("java/lang/Math", "pow", "(DD)D", native_math_pow);
    registry.natives_mut().register("java/lang/Math", "floor", "(D)D", native_math_floor);
    registry.natives_mut().register("java/lang/Math", "ceil", "(D)D", native_math_ceil);
    registry.natives_mut().register("java/lang/Math", "round", "(D)J", native_math_round_double);
    registry.natives_mut().register("java/lang/Math", "abs", "(J)J", native_math_abs_long);
    registry.natives_mut().register("java/lang/Math", "abs", "(D)D", native_math_abs_double);
    registry.natives_mut().register("java/lang/Math", "max", "(JJ)J", native_math_max_long);
    registry.natives_mut().register("java/lang/Math", "min", "(JJ)J", native_math_min_long);
    registry.natives_mut().register("java/lang/Math", "max", "(DD)D", native_math_max_double);
    registry.natives_mut().register("java/lang/Math", "min", "(DD)D", native_math_min_double);
```

Add Math.PI and Math.E as static fields — update the `math_ctx` ClassContext (around line 590):

```rust
    let math_ctx = ClassContext {
        class_name: "java/lang/Math".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "PI".to_string(),
                descriptor: "D".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "E".to_string(),
                descriptor: "D".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![
            Slot::Double(std::f64::consts::PI),
            Slot::Double(std::f64::consts::E),
        ],
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
    };
```

Add `java/lang/Long` synthetic ClassContext + natives (after the Integer block, around line 588):

```rust
    // java/lang/Long — boxed long with value field
    let long_ctx = ClassContext {
        class_name: "java/lang/Long".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "value".to_string(),
            descriptor: "J".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        bootstrap_methods: Vec::new(),
    };
    registry.register(long_ctx);
    registry.natives_mut().register("java/lang/Long", "parseLong", "(Ljava/lang/String;)J", native_long_parselong);
    registry.natives_mut().register("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;", native_long_valueof);
    registry.natives_mut().register("java/lang/Long", "longValue", "()J", native_long_longvalue);
    registry.natives_mut().register("java/lang/Long", "toString", "(J)Ljava/lang/String;", native_long_tostring_static);
```

Add `java/lang/Double` synthetic ClassContext + natives:

```rust
    // java/lang/Double — boxed double with value field
    let double_ctx = ClassContext {
        class_name: "java/lang/Double".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "value".to_string(),
            descriptor: "D".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        bootstrap_methods: Vec::new(),
    };
    registry.register(double_ctx);
    registry.natives_mut().register("java/lang/Double", "parseDouble", "(Ljava/lang/String;)D", native_double_parsedouble);
    registry.natives_mut().register("java/lang/Double", "valueOf", "(D)Ljava/lang/Double;", native_double_valueof);
    registry.natives_mut().register("java/lang/Double", "doubleValue", "()D", native_double_doublevalue);
```

Add `java/lang/Float` synthetic ClassContext + native:

```rust
    // java/lang/Float — boxed float with value field
    let float_ctx = ClassContext {
        class_name: "java/lang/Float".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "value".to_string(),
            descriptor: "F".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        bootstrap_methods: Vec::new(),
    };
    registry.register(float_ctx);
    registry.natives_mut().register("java/lang/Float", "parseFloat", "(Ljava/lang/String;)F", native_float_parsefloat);
```

Add `java/lang/Boolean` synthetic ClassContext + native:

```rust
    // java/lang/Boolean — static parse helper
    let boolean_ctx = ClassContext {
        class_name: "java/lang/Boolean".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
    };
    registry.register(boolean_ctx);
    registry.natives_mut().register("java/lang/Boolean", "parseBoolean", "(Ljava/lang/String;)Z", native_boolean_parseboolean);
```

**Step 4: Write integration tests**

```rust
    // ---- Phase 18: ExtendedMath ----

    fn load_extended_math_class() -> ClassContext {
        let bytes = std::fs::read("tests/fixtures/ExtendedMath.class").unwrap();
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn extended_math_sqrt() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testSqrt", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_pow() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testPow", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_floor_ceil() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testFloorCeil", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_round() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testRound", "()J", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Long(4)));
    }

    #[test]
    fn extended_math_abs_long() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testAbsLong", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_abs_double() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testAbsDouble", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_max_long() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testMaxLong", "()J", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Long(200)));
    }

    #[test]
    fn extended_math_min_long() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testMinLong", "()J", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Long(100)));
    }

    #[test]
    fn extended_math_max_double() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testMaxDouble", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_parse_long() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testParseLong", "()J", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Long(9_876_543_210)));
    }

    #[test]
    fn extended_math_parse_double() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testParseDouble", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_parse_float() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testParseFloat", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_parse_boolean() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testParseBoolean", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_long_valueof() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testLongValueOf", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_constants() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "ExtendedMath", "testMathConstants", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }
```

**Step 5: Run tests**

```bash
cargo test -p duke-interpreter -- extended_math
```

Expected: 15 tests pass.

**Step 6: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/ExtendedMath.java tests/fixtures/ExtendedMath.class
git commit -m "feat(interpreter): add extended Math, Long, Double, Float, Boolean native methods"
```

---

### Task 3: Extended String Natives

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (new String native fns + registrations)
- Create: `tests/fixtures/StringOps2.java`

**Step 1: Write the test fixture**

Create `tests/fixtures/StringOps2.java`:

```java
public class StringOps2 {
    public static int testToUpperCase() {
        String s = "hello world";
        String u = s.toUpperCase();
        return u.equals("HELLO WORLD") ? 1 : 0;
    }

    public static int testToLowerCase() {
        String s = "Hello World";
        String l = s.toLowerCase();
        return l.equals("hello world") ? 1 : 0;
    }

    public static int testReplace() {
        String s = "hello world";
        String r = s.replace('l', 'r');
        return r.equals("herro worrd") ? 1 : 0;
    }

    public static int testReplaceString() {
        String s = "hello world hello";
        String r = s.replace("hello", "hi");
        return r.equals("hi world hi") ? 1 : 0;
    }

    public static int testSplit() {
        String s = "a,b,c,d";
        String[] parts = s.split(",");
        return (parts.length == 4 && parts[0].equals("a") && parts[3].equals("d")) ? 1 : 0;
    }

    public static int testHashCode() {
        String s1 = "hello";
        String s2 = "hello";
        return (s1.hashCode() == s2.hashCode()) ? 1 : 0;
    }

    public static int testToStringIdentity() {
        String s = "test";
        String t = s.toString();
        return s.equals(t) ? 1 : 0;
    }

    public static int testReplaceCharSequence() {
        String s = "foo bar baz";
        String r = s.replace("bar", "qux");
        return r.equals("foo qux baz") ? 1 : 0;
    }
}
```

Compile: `javac --release 21 tests/fixtures/StringOps2.java`

**Step 2: Add native handler functions**

In `crates/duke-interpreter/src/lib.rs`, add these after the existing String natives section:

```rust
// ---- Extended String natives (Phase 18) ----

fn native_string_touppercase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.to_uppercase());
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_string_tolowercase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.to_lowercase());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.replace(char, char)` — replaces all occurrences of oldChar with newChar.
#[allow(clippy::cast_sign_loss)]
fn native_string_replace_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let old_char = match args.get(1) {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('?'),
        _ => return Err(VmError::TypeMismatch { expected: "Int(char)", got: "other" }),
    };
    let new_char = match args.get(2) {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('?'),
        _ => return Err(VmError::TypeMismatch { expected: "Int(char)", got: "other" }),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let result = s.replace(old_char, &new_char.to_string());
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.replace(CharSequence, CharSequence)` — replaces all occurrences.
fn native_string_replace_charsequence(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let target_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let replacement_ref = match args.get(2) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let target = heap.get(target_ref)?.string_value.clone().unwrap_or_default();
    let replacement = heap.get(replacement_ref)?.string_value.clone().unwrap_or_default();
    let result = s.replace(&target, &replacement);
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.split(String)` — splits by regex (simple: treat as literal).
fn native_string_split(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let delim_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let delim = heap.get(delim_ref)?.string_value.clone().unwrap_or_default();
    let parts: Vec<&str> = s.split(&delim).collect();
    // Allocate String[] array
    let arr = heap.allocate("[Ljava/lang/String;".to_string(), parts.len());
    for (i, part) in parts.iter().enumerate() {
        let str_ref = heap.allocate_string((*part).to_string());
        heap.get_mut(arr).unwrap().fields[i] = Slot::Reference(Some(str_ref));
    }
    Ok(Some(Slot::Reference(Some(arr))))
}

/// Native: `String.hashCode()` — standard Java string hash algorithm.
#[allow(clippy::cast_possible_wrap)]
fn native_string_hashcode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    // Java's String.hashCode(): s[0]*31^(n-1) + s[1]*31^(n-2) + ... + s[n-1]
    let mut h: i32 = 0;
    for ch in s.chars() {
        h = h.wrapping_mul(31).wrapping_add(ch as i32);
    }
    Ok(Some(Slot::Int(h)))
}

/// Native: `String.toString()` — returns this (identity).
fn native_string_tostring(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this = args.first().cloned().unwrap_or(Slot::Reference(None));
    Ok(Some(this))
}
```

**Step 3: Register in `bootstrap_stdlib`**

After the existing String instance methods section (after the `toCharArray` registration):

```rust
    // Extended String natives (Phase 18)
    registry.natives_mut().register("java/lang/String", "toUpperCase", "()Ljava/lang/String;", native_string_touppercase);
    registry.natives_mut().register("java/lang/String", "toLowerCase", "()Ljava/lang/String;", native_string_tolowercase);
    registry.natives_mut().register("java/lang/String", "replace", "(CC)Ljava/lang/String;", native_string_replace_char);
    registry.natives_mut().register("java/lang/String", "replace", "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Ljava/lang/String;", native_string_replace_charsequence);
    registry.natives_mut().register("java/lang/String", "split", "(Ljava/lang/String;)[Ljava/lang/String;", native_string_split);
    registry.natives_mut().register("java/lang/String", "hashCode", "()I", native_string_hashcode);
    registry.natives_mut().register("java/lang/String", "toString", "()Ljava/lang/String;", native_string_tostring);
```

**Step 4: Write integration tests**

```rust
    // ---- Phase 18: StringOps2 ----

    fn load_string_ops2_class() -> ClassContext {
        let bytes = std::fs::read("tests/fixtures/StringOps2.class").unwrap();
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn string_ops2_to_upper_case() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "StringOps2", "testToUpperCase", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_to_lower_case() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "StringOps2", "testToLowerCase", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_replace_char() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "StringOps2", "testReplace", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_replace_string() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "StringOps2", "testReplaceString", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_split() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "StringOps2", "testSplit", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_hashcode() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "StringOps2", "testHashCode", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_tostring() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "StringOps2", "testToStringIdentity", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_replace_charsequence() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry, &loader, &mut heap, &mut out,
            "StringOps2", "testReplaceCharSequence", "()I", &[],
        ).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }
```

**Step 5: Run tests**

```bash
cargo test -p duke-interpreter -- string_ops2
```

Expected: 8 tests pass.

**Step 6: Run full test suite**

```bash
cargo test
```

Expected: 253+ tests pass (227 existing + 3 monitor + 15 math/numeric + 8 string).

**Step 7: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/StringOps2.java tests/fixtures/StringOps2.class
git commit -m "feat(interpreter): add extended String natives (toUpperCase, toLowerCase, replace, split, hashCode, toString)"
```
