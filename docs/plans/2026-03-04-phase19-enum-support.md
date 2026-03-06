# Phase 19: Enum Support & LDC Class Literals — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable Java enum types to compile and run on Duke, including `ordinal()`, `name()`, `values()`, `valueOf(String)`, and switch-on-enum via `tableswitch`.

**Architecture:** Add a synthetic `java/lang/Enum` class with `ordinal()`/`name()` instance methods backed by native handlers, implement `Object.clone()` native for `values()` array cloning, and extend `ldc_push` to handle `CpEntry::Class` literals (needed for `Enum.valueOf(Class, String)`). A lightweight `java/lang/Class` synthetic object represents class literals on the heap.

**Tech Stack:** Rust, duke-interpreter, duke-gc, duke-runtime, javac 21

---

## Background: How Javac 21 Compiles Enums

Given `enum Color { RED, GREEN, BLUE }`, javac produces a **final class** extending `java/lang/Enum`:

```
// Compiled from javac 21
final class Color extends java.lang.Enum<Color> {
  public static final Color RED;
  public static final Color GREEN;
  public static final Color BLUE;
  private static final Color[] $VALUES;

  static {  // <clinit>
    RED   = new Color("RED", 0);
    GREEN = new Color("GREEN", 1);
    BLUE  = new Color("BLUE", 2);
    $VALUES = new Color[] { RED, GREEN, BLUE };
  }

  private Color(String name, int ordinal) {
    super(name, ordinal);  // invokespecial Enum.<init>(String,I)V
  }

  public static Color[] values() {
    return $VALUES.clone();  // invokevirtual Object.clone()
  }

  public static Color valueOf(String name) {
    return Enum.valueOf(Color.class, name);  // ldc CpEntry::Class
  }
}
```

Key bytecode operations Duke must handle:
- **`invokespecial java/lang/Enum.<init>(Ljava/lang/String;I)V`** — store name + ordinal as fields
- **`invokevirtual ordinal()I`** / **`invokevirtual name()Ljava/lang/String;`** — dispatch via hierarchy walk to Enum natives
- **`invokevirtual clone()Ljava/lang/Object;`** — shallow-copy an array for `values()`
- **`ldc CpEntry::Class`** — push a Class literal reference for `valueOf(Class, String)`
- **`invokestatic java/lang/Enum.valueOf(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;`** — search by name

Switch on enum uses `ordinal()` → `tableswitch`, which already works.

---

## Task 1: LDC Class Literals + Synthetic `java/lang/Class`

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (lines 6017-6025: `ldc_push`, lines 264-550: `bootstrap_stdlib`, lines 4050-4111: execute_class LDC handlers)
- Test: `crates/duke-interpreter/src/lib.rs` (test module at bottom)
- Create: `tests/fixtures/ClassLiteral.java`

### What Duke needs

When bytecode does `ldc` with a `CpEntry::Class`, the JVM pushes a reference to a `java.lang.Class` object onto the stack. We represent this as a `HeapObject` with `class_name = "java/lang/Class"` and a `string_value` holding the represented class name (reusing the existing String content field).

### Step 1: Write the Java fixture

Create `tests/fixtures/ClassLiteral.java`:

```java
public class ClassLiteral {
    // Returns 1 if String.class name contains "String"
    static int testStringClass() {
        Class<?> c = String.class;
        // We can't call getName() yet, but the object should be non-null
        if (c != null) return 1;
        return 0;
    }

    // Returns 1 if int.class is non-null
    static int testPrimitiveClass() {
        // Note: ldc for primitive .class uses "I" descriptor, not a Class CP entry
        // This test just verifies basic Class literal loading works
        Class<?> c = ClassLiteral.class;
        if (c != null) return 1;
        return 0;
    }

    // Two ldc of same class should give same reference (interned)
    static int testClassInterning() {
        Class<?> a = String.class;
        Class<?> b = String.class;
        if (a == b) return 1;
        return 0;
    }
}
```

Compile: `javac --release 21 tests/fixtures/ClassLiteral.java`

### Step 2: Register synthetic `java/lang/Class` in `bootstrap_stdlib`

In `bootstrap_stdlib()`, after the Object registration (~line 549), add:

```rust
// java/lang/Class — lightweight stub for class literals
let class_ctx = ClassContext {
    class_name: "java/lang/Class".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: Vec::new(),
    static_fields: Vec::new(),
    instance_field_count: 0,
    bootstrap_methods: Vec::new(),
};
registry.register(class_ctx);
```

### Step 3: Extend `ldc_push` to handle `CpEntry::Class`

Modify `ldc_push` (line 6017-6025) to accept `heap` and `class_intern` parameters. But actually, since `ldc_push` doesn't have access to heap, the simpler approach is to handle `CpEntry::Class` in the LDC arms of `execute_class` **before** falling through to `ldc_push`, the same way `CpEntry::String` is handled.

In the `Instruction::Ldc(raw_idx)` arm (line 4050-4079), add a Class check alongside the String check:

```rust
Instruction::Ldc(raw_idx) => {
    let cp_idx = usize::from(*raw_idx);
    // Check for String constant
    let string_info = { /* existing String logic */ };
    if let Some(s) = string_info {
        /* existing String interning logic */
    } else {
        // Check for Class constant
        let class_info = {
            let ctx = registry.get(&current_class)?;
            if let Some(CpEntry::Class { name_index }) =
                ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
            {
                match ctx.constant_pool.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                    Some(CpEntry::Utf8(s)) => Some(s.clone()),
                    _ => None,
                }
            } else {
                None
            }
        };
        if let Some(class_name) = class_info {
            // Intern class literals: same CP index → same heap ref
            let r = if let Some(&cached) = string_intern.get(&(cp_idx + 100_000)) {
                cached
            } else {
                let r = heap.allocate("java/lang/Class".to_string(), 0);
                heap.get_mut(r).unwrap().string_value = Some(class_name);
                string_intern.insert(cp_idx + 100_000, r);
                r
            };
            frame.push(Slot::Reference(Some(r)))?;
        } else {
            let ctx = registry.get(&current_class)?;
            ldc_push(&mut frame, &ctx.constant_pool, cp_idx)?;
        }
    }
}
```

**Important**: We offset the intern key by `100_000` to avoid collision with String intern keys that use the same `HashMap<usize, u64>`. Both String and Class use the `string_intern` map.

Apply the **same pattern** to the `Instruction::LdcW | Instruction::Ldc2W` arm (lines 4081-4111).

### Step 4: Write integration tests

Add tests to the interpreter test module:

```rust
#[test]
fn class_literal_non_null() {
    assert_eq!(
        run_class_int("ClassLiteral.class", "testStringClass", "()I", vec![]),
        1
    );
}

#[test]
fn class_literal_self_ref() {
    assert_eq!(
        run_class_int("ClassLiteral.class", "testPrimitiveClass", "()I", vec![]),
        1
    );
}

#[test]
fn class_literal_interning() {
    assert_eq!(
        run_class_int("ClassLiteral.class", "testClassInterning", "()I", vec![]),
        1
    );
}
```

### Step 5: Run tests

```bash
cargo test -p duke-interpreter -- class_literal
```

Expected: 3 new tests pass.

### Step 6: Commit

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/ClassLiteral.java tests/fixtures/ClassLiteral.class
git commit -m "feat(interpreter): implement ldc Class literals with synthetic java/lang/Class"
```

---

## Task 2: java/lang/Enum Synthetic Class + Object.clone() + Enum Natives

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (`bootstrap_stdlib`, native handlers section)
- Create: `tests/fixtures/SimpleEnum.java`

### What Duke needs

1. **`java/lang/Enum` synthetic class** with 2 instance fields (`name:Ljava/lang/String;`, `ordinal:I`) and super_class = `java/lang/Object`
2. **`Enum.<init>(Ljava/lang/String;I)V`** — native that stores name + ordinal into fields
3. **`Enum.ordinal()I`** — native that reads the ordinal field
4. **`Enum.name()Ljava/lang/String;`** — native that reads the name field
5. **`Enum.valueOf(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;`** — static native that searches a class's `values()` for a matching name
6. **`Object.clone()Ljava/lang/Object;`** — native that shallow-copies a heap object (needed for `Color.values()`)

### Step 1: Write the Java fixture

Create `tests/fixtures/SimpleEnum.java`:

```java
public class SimpleEnum {
    enum Color { RED, GREEN, BLUE }

    // ordinal(): RED=0, GREEN=1, BLUE=2
    static int testOrdinal() {
        return Color.GREEN.ordinal();  // expect 1
    }

    // name(): returns string, check length as proxy
    static int testName() {
        String n = Color.RED.name();
        return n.length();  // "RED" = 3
    }

    // values(): returns all enum constants
    static int testValues() {
        Color[] all = Color.values();
        return all.length;  // expect 3
    }

    // valueOf(String): look up by name
    static int testValueOf() {
        Color c = Color.valueOf("BLUE");
        return c.ordinal();  // expect 2
    }

    // switch on enum via ordinal -> tableswitch
    static int testSwitch() {
        Color c = Color.GREEN;
        switch (c) {
            case RED: return 10;
            case GREEN: return 20;
            case BLUE: return 30;
            default: return -1;
        }
    }

    // equality: same enum constant should be same reference
    static int testEquality() {
        Color a = Color.RED;
        Color b = Color.RED;
        if (a == b) return 1;
        return 0;
    }
}
```

Compile: `javac --release 21 tests/fixtures/SimpleEnum.java`

This produces `SimpleEnum.class` and `SimpleEnum$Color.class`.

### Step 2: Implement `Object.clone()` native

Add after the existing `native_object_tostring` function (~line 1010):

```rust
/// Native: `Object.clone()` — shallow-copies a heap object.
/// For arrays, copies elements. For regular objects, copies fields.
fn native_object_clone(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get(this_ref)?;
    let cloned = HeapObject {
        class_name: obj.class_name.clone(),
        fields: obj.fields.clone(),
        string_value: obj.string_value.clone(),
    };
    let new_ref = heap.allocate(cloned.class_name.clone(), 0);
    // Overwrite the freshly allocated empty object with the cloned data
    let dest = heap.get_mut(new_ref)?;
    dest.fields = cloned.fields;
    dest.string_value = cloned.string_value;
    Ok(Some(Slot::Reference(Some(new_ref))))
}
```

Register in `bootstrap_stdlib` after the Object `toString` registration (~line 549):

```rust
registry.natives_mut().register(
    "java/lang/Object",
    "clone",
    "()Ljava/lang/Object;",
    native_object_clone,
);
```

**Note**: You'll need to add `use duke_gc::HeapObject;` at the top of the file if not already imported.

### Step 3: Register synthetic `java/lang/Enum` class

Add in `bootstrap_stdlib` after the RuntimeException hierarchy (~line 590):

```rust
// java/lang/Enum — abstract superclass for all enums
// Fields: name (String) at index 0, ordinal (int) at index 1
let enum_ctx = ClassContext {
    class_name: "java/lang/Enum".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: vec![
        FieldEntry {
            name: "name".to_string(),
            descriptor: "Ljava/lang/String;".to_string(),
            is_static: false,
        },
        FieldEntry {
            name: "ordinal".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        },
    ],
    static_fields: Vec::new(),
    instance_field_count: 2,
    bootstrap_methods: Vec::new(),
};
registry.register(enum_ctx);
```

### Step 4: Implement Enum native handlers

Add native functions after `native_object_clone`:

```rust
/// Native: `Enum.<init>(Ljava/lang/String;I)V` — stores name + ordinal.
/// args: [this_ref, name_ref, ordinal_int]
fn native_enum_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let name_slot = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let ordinal = match args.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let obj = heap.get_mut(this_ref)?;
    // field[0] = name (String reference), field[1] = ordinal (Int)
    if obj.fields.len() >= 2 {
        obj.fields[0] = name_slot;
        obj.fields[1] = Slot::Int(ordinal);
    }
    Ok(None)
}

/// Native: `Enum.ordinal()I` — returns the ordinal field.
/// args: [this_ref]
fn native_enum_ordinal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get(this_ref)?;
    // ordinal is at field index 1
    match obj.fields.get(1) {
        Some(Slot::Int(v)) => Ok(Some(Slot::Int(*v))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `Enum.name()Ljava/lang/String;` — returns the name field.
/// args: [this_ref]
fn native_enum_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get(this_ref)?;
    // name is at field index 0
    match obj.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(slot.clone())),
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Enum.valueOf(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;`
/// Searches the enum class's static fields for a constant matching the given name.
/// args: [class_ref, name_ref]
fn native_enum_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    // arg 0 = Class literal reference, arg 1 = name String reference
    let class_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let name_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };

    // Get the target name string
    let target_name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .unwrap_or_default();

    // Get the class name from the Class literal
    let enum_class_name = heap
        .get(class_ref)?
        .string_value
        .clone()
        .unwrap_or_default();

    // Search all heap objects for enum constants of this class with matching name
    // This is a linear scan — acceptable for enum valueOf which is rare
    let obj_count = heap.len();
    for i in 0..obj_count {
        let obj = heap.get(i as u64)?;
        if obj.class_name == enum_class_name && obj.fields.len() >= 2 {
            // field[0] = name ref, field[1] = ordinal
            if let Some(Slot::Reference(Some(name_r))) = obj.fields.first() {
                if let Ok(name_obj) = heap.get(*name_r) {
                    if name_obj.string_value.as_deref() == Some(target_name.as_str()) {
                        return Ok(Some(Slot::Reference(Some(i as u64))));
                    }
                }
            }
        }
    }

    Err(VmError::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    })
}
```

### Step 5: Register Enum natives in `bootstrap_stdlib`

After the Enum ClassContext registration:

```rust
registry.natives_mut().register(
    "java/lang/Enum",
    "<init>",
    "(Ljava/lang/String;I)V",
    native_enum_init,
);
registry.natives_mut().register(
    "java/lang/Enum",
    "ordinal",
    "()I",
    native_enum_ordinal,
);
registry.natives_mut().register(
    "java/lang/Enum",
    "name",
    "()Ljava/lang/String;",
    native_enum_name,
);
registry.natives_mut().register(
    "java/lang/Enum",
    "valueOf",
    "(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;",
    native_enum_valueof,
);
```

### Step 6: Handle Enum `<init>` dispatch

When javac compiles an enum constructor, it generates `invokespecial java/lang/Enum.<init>(Ljava/lang/String;I)V`. Duke's `invokespecial` handler already checks `native_registry` for native methods, so the `native_enum_init` will be found and called.

**However**, there's a subtlety: enum subclasses (e.g., `SimpleEnum$Color`) have their own `<init>` that calls `super.<init>` (i.e., `Enum.<init>`). The subclass `<init>` is loaded from the `.class` file and will use `invokespecial` to call Enum's `<init>`. This should work via `resolve_method_in_hierarchy` which walks up to `java/lang/Enum` where it finds the native.

**Important**: When Duke allocates a `new SimpleEnum$Color` object, it must have **at least 2 fields** for `name` and `ordinal` (inherited from Enum). The `instance_field_count` is determined by `build_class_context()` which counts only the fields declared in the `.class` file itself. Enum subclasses don't redeclare `name`/`ordinal` — they inherit them.

To handle this, we need to ensure that when allocating an object whose superclass is `java/lang/Enum`, we include the Enum's 2 fields. The `new` opcode in `execute_class` already uses the class's own `instance_field_count`:

Check the `new` opcode handler — it uses `instance_field_count` from the target class's `ClassContext`. For enum subclasses loaded from `.class` files, their `instance_field_count` will be 0 (they have no declared fields). We need to add the parent's fields.

**Fix**: In the `new` opcode handler in `execute_class`, after getting `instance_field_count`, walk the superclass chain to accumulate inherited fields. OR, simpler: adjust `build_class_context` or `ensure_loaded` to account for inherited field counts.

**Simplest approach**: In the `new` opcode handler, after resolving the target class, check if its `instance_field_count` is 0 but its super has fields, and add the super's fields. Actually, the cleanest fix is in the `new` opcode:

Find the `new` handler in execute_class (search for `Instruction::New`). It does:
```rust
let field_count = ctx.instance_field_count;
let r = heap.allocate(class_name.clone(), field_count);
```

Change this to walk the class hierarchy and sum up `instance_field_count` from all ancestors:

```rust
// Sum instance fields from entire class hierarchy
let mut total_fields = ctx.instance_field_count;
let mut super_name = ctx.super_class.clone();
while let Some(ref sn) = super_name {
    if let Ok(super_ctx) = registry.get(sn) {
        total_fields += super_ctx.instance_field_count;
        super_name = super_ctx.super_class.clone();
    } else {
        break;
    }
}
let r = heap.allocate(class_name.clone(), total_fields);
```

**Also fix `getfield`/`putfield`**: These use field index relative to the current class. With inherited fields, the parent's fields occupy the first N slots, and the child's fields follow. We need to offset field indices.

Actually — this is getting complex. **Simpler approach**: Since enum subclasses only use inherited fields (name + ordinal), and `Enum.<init>` writes to fields[0] and fields[1] via `putfield` (which goes through the native handler), we should make sure the native handler stores correctly.

**Simplest approach**: In `native_enum_init`, the handler already writes to `obj.fields[0]` and `obj.fields[1]`. The object needs at least 2 fields. To guarantee this, modify the `new` opcode to compute inherited field count:

```rust
Instruction::New(cp_idx) => {
    let class_name = {
        let ctx = registry.get(&current_class)?;
        resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
    };
    registry.ensure_initialized(&class_name, &loader, heap, stdout)?;
    let field_count = {
        let ctx = registry.get(&class_name)?;
        let mut count = ctx.instance_field_count;
        let mut sc = ctx.super_class.clone();
        while let Some(ref s) = sc {
            match registry.get(s) {
                Ok(sctx) => {
                    count += sctx.instance_field_count;
                    sc = sctx.super_class.clone();
                }
                Err(_) => break,
            }
        }
        count
    };
    let r = heap.allocate(class_name, field_count);
    frame.push(Slot::Reference(Some(r)))?;
}
```

**And fix `getfield`/`putfield`** to compute field offset by walking the hierarchy. The field index from `build_class_context` is relative to the declaring class, but in the heap object, inherited fields come first. When `putfield` targets field "ordinal" on an Enum subclass, it resolves to field index 1 in the `java/lang/Enum` ClassContext. In the heap object, Enum fields are at indices 0..1 and the subclass fields (if any) start at index 2.

This is actually already how it works IF we allocate enough fields. The `getfield`/`putfield` handlers resolve the field name, find its index in the **declaring class**, and access `obj.fields[index]`. If the declaring class is `java/lang/Enum` and it finds "ordinal" at index 1, it accesses `obj.fields[1]`. The object was allocated with inherited + own fields, so field[0] = name, field[1] = ordinal — exactly right.

**BUT** — we need to verify that `getfield`/`putfield` can resolve fields declared on a superclass. Check the current getfield handler: it resolves the Fieldref, gets class_name + field_name, looks up the field index in that class's ClassContext. If the Fieldref says `java/lang/Enum.ordinal`, it looks up Enum's ClassContext and finds ordinal at index 1. The heap object has Enum's 2 fields at [0,1]. This works.

If the Fieldref says `SimpleEnum$Color.ordinal`, we'd need to search up the hierarchy. But javac generates `Enum.ordinal` in the Fieldref, not `Color.ordinal`. So this should be fine.

### Step 7: Write integration tests

```rust
#[test]
fn enum_ordinal() {
    assert_eq!(
        run_class_int("SimpleEnum.class", "testOrdinal", "()I", vec![]),
        1  // GREEN.ordinal() = 1
    );
}

#[test]
fn enum_name_length() {
    assert_eq!(
        run_class_int("SimpleEnum.class", "testName", "()I", vec![]),
        3  // "RED".length() = 3
    );
}

#[test]
fn enum_values_length() {
    assert_eq!(
        run_class_int("SimpleEnum.class", "testValues", "()I", vec![]),
        3  // 3 enum constants
    );
}

#[test]
fn enum_valueof() {
    assert_eq!(
        run_class_int("SimpleEnum.class", "testValueOf", "()I", vec![]),
        2  // BLUE.ordinal() = 2
    );
}

#[test]
fn enum_switch() {
    assert_eq!(
        run_class_int("SimpleEnum.class", "testSwitch", "()I", vec![]),
        20  // GREEN case
    );
}

#[test]
fn enum_equality() {
    assert_eq!(
        run_class_int("SimpleEnum.class", "testEquality", "()I", vec![]),
        1  // same static field reference
    );
}
```

### Step 8: Run tests

```bash
cargo test -p duke-interpreter -- enum_
cargo test -p duke-interpreter  # Full suite — expect ~259+ tests
```

### Step 9: Commit

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/SimpleEnum.java tests/fixtures/SimpleEnum.class "tests/fixtures/SimpleEnum$Color.class"
git commit -m "feat(interpreter): implement java.lang.Enum natives, Object.clone, and enum support"
```

---

## Task 3: Smoke Tests + Plan Doc Commit

**Files:**
- Modify: `docs/plans/2026-03-04-phase19-enum-support.md` (mark complete)
- Run: full test suite + CLI smoke tests

### Step 1: Run full test suite

```bash
cargo test --workspace
```

Expected: ~265+ tests passing, zero failures.

### Step 2: CLI smoke tests

```bash
cargo run -- exec tests/fixtures/ClassLiteral.class testStringClass
# Expected output: Int(1)

cargo run -- exec tests/fixtures/SimpleEnum.class testOrdinal
# Expected output: Int(1)

cargo run -- exec tests/fixtures/SimpleEnum.class testValues
# Expected output: Int(3)

cargo run -- exec tests/fixtures/SimpleEnum.class testSwitch
# Expected output: Int(20)
```

### Step 3: Commit plan doc

```bash
git add docs/plans/2026-03-04-phase19-enum-support.md
git commit -m "docs: add phase 19 plan (enum support & ldc class literals)"
```

---

## Summary

| Task | What | New Tests |
|------|------|-----------|
| 1 | LDC Class literals + synthetic `java/lang/Class` | 3 |
| 2 | `java/lang/Enum` + `Object.clone()` + inherited field alloc + 4 Enum natives | 6 |
| 3 | Smoke tests + plan doc | 0 |

**Total new tests**: ~9
**Expected final test count**: ~262+
**New opcodes**: 0 (reuses existing LDC, invokevirtual, invokestatic, etc.)
**New natives**: 5 (`Object.clone`, `Enum.<init>`, `Enum.ordinal`, `Enum.name`, `Enum.valueOf`)
**New synthetic classes**: 2 (`java/lang/Class`, `java/lang/Enum`)
