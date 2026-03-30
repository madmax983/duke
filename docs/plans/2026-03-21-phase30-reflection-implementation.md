# Phase 30 Reflection Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add minimal but real `java.lang.Class` / `java.lang.reflect.Method` / `java.lang.reflect.Field` support so Duke can load classes by name, inspect declared members, and invoke methods reflectively with standard Java exceptions.

**Architecture:** Keep the interpreter core stable and implement reflection as synthetic runtime objects plus native bridges in `crates/duke-interpreter/src/lib.rs`. Add one small extension to the callback-native boundary so reflection handlers can ask the interpreter to ensure a class is loaded and inspect parsed classfile metadata without giving every native full VM internals. Materialize reflection metadata from parsed classfiles on demand when `Class.getDeclaredMethods()` / `Class.getDeclaredFields()` is called, store the needed flags on wrapper objects, and route `Method.invoke()` through the existing callback path so reflective calls reuse normal bytecode/native dispatch. Refactor class literals and `Class.forName()` to share one class-object interning helper, so `String.class` and `Class.forName("java.lang.String")` stop living separate fake lives.

**Tech Stack:** Rust 2024, `duke-interpreter`, `duke-classfile`, `duke-loader`, `duke-gc`, checked-in Java fixtures under `tests/fixtures`, `javac --release 21`, `cargo test`.

---

### Task 1: Red Tests For Reflection Happy Paths And Failure Paths

**Files:**
- Create: `tests/fixtures/ReflectionTest.java`
- Create: `tests/fixtures/ReflectionTest.class`
- Create: `tests/fixtures/ReflectionTarget.class`
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Write the fixture source**

Create `tests/fixtures/ReflectionTest.java`:

```java
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;

public final class ReflectionTest {
    public static int forNameAndGetName() throws Exception {
        Class<?> cls = Class.forName("ReflectionTarget");
        return cls.getName().equals("ReflectionTarget") ? 1 : 0;
    }

    public static int stringClassLiteralUsesBinaryName() {
        return String.class.getName().equals("java.lang.String") ? 1 : 0;
    }

    public static int declaredMethodsIncludePublicAndPrivate() {
        int sawAdd = 0;
        int sawTimes = 0;
        int sawHidden = 0;
        int sawExplode = 0;
        int sawAddLong = 0;

        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            String name = method.getName();
            if (name.equals("add")) sawAdd = 1;
            if (name.equals("times")) sawTimes = 1;
            if (name.equals("hidden")) sawHidden = 1;
            if (name.equals("explode")) sawExplode = 1;
            if (name.equals("addLong")) sawAddLong = 1;
        }

        return sawAdd + sawTimes + sawHidden + sawExplode + sawAddLong;
    }

    public static int declaredFieldsIncludePublicAndPrivate() {
        int sawBase = 0;
        int sawSecret = 0;

        for (Field field : ReflectionTarget.class.getDeclaredFields()) {
            String name = field.getName();
            if (name.equals("base")) sawBase = 1;
            if (name.equals("secret")) sawSecret = 1;
        }

        return sawBase + sawSecret;
    }

    public static int invokeStaticAdd() throws Exception {
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("add")) {
                Object result = method.invoke(null, Integer.valueOf(2), Integer.valueOf(5));
                return ((Integer) result).intValue();
            }
        }
        return -1;
    }

    public static int invokeInstanceTimes() throws Exception {
        ReflectionTarget target = new ReflectionTarget(7, 11);
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("times")) {
                Object result = method.invoke(target, Integer.valueOf(3));
                return ((Integer) result).intValue();
            }
        }
        return -1;
    }

    public static int invokeStaticAddLong() throws Exception {
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("addLong")) {
                Object result = method.invoke(null, Long.valueOf(2L), Long.valueOf(5L));
                return ((Long) result).longValue() == 7L ? 1 : 0;
            }
        }
        return -1;
    }

    public static int missingClassRaisesClassNotFound() {
        try {
            Class.forName("duke.missing.ReflectionGhost");
            return 0;
        } catch (ClassNotFoundException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int privateMethodRaisesIllegalAccess() throws Exception {
        ReflectionTarget target = new ReflectionTarget(3, 13);
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("hidden")) {
                try {
                    method.invoke(target);
                    return 0;
                } catch (IllegalAccessException e) {
                    return 1;
                }
            }
        }
        return -1;
    }

    public static int targetExceptionIsWrapped() throws Exception {
        ReflectionTarget target = new ReflectionTarget(1, 2);
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("explode")) {
                try {
                    method.invoke(target);
                    return 0;
                } catch (InvocationTargetException e) {
                    return 1;
                }
            }
        }
        return -1;
    }
}

final class ReflectionTarget {
    public int base;
    private int secret;

    ReflectionTarget(int base, int secret) {
        this.base = base;
        this.secret = secret;
    }

    public static int add(int a, int b) {
        return a + b;
    }

    public static long addLong(long a, long b) {
        return a + b;
    }

    public int times(int factor) {
        return base * factor;
    }

    private int hidden() {
        return secret;
    }

    public void explode() {
        throw new RuntimeException("boom");
    }
}
```

**Step 2: Compile the fixture**

Run: `javac --release 21 tests/fixtures/ReflectionTest.java -d tests/fixtures/`

Expected: `tests/fixtures/ReflectionTest.class` and `tests/fixtures/ReflectionTarget.class` are created with no errors.

**Step 3: Add failing interpreter tests**

In `crates/duke-interpreter/src/lib.rs`, add Phase 30 tests that reuse `run_bootstrap_int(...)`:

```rust
#[test]
fn reflection_for_name_and_get_name() {
    assert_eq!(run_bootstrap_int("ReflectionTest.class", "forNameAndGetName", "()I"), 1);
}

#[test]
fn reflection_string_class_literal_uses_binary_name() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "stringClassLiteralUsesBinaryName", "()I"),
        1
    );
}

#[test]
fn reflection_declared_methods_include_public_and_private() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "declaredMethodsIncludePublicAndPrivate",
            "()I",
        ),
        5
    );
}

#[test]
fn reflection_declared_fields_include_public_and_private() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "declaredFieldsIncludePublicAndPrivate",
            "()I",
        ),
        2
    );
}

#[test]
fn reflection_invoke_static_and_instance_methods() {
    assert_eq!(run_bootstrap_int("ReflectionTest.class", "invokeStaticAdd", "()I"), 7);
    assert_eq!(run_bootstrap_int("ReflectionTest.class", "invokeInstanceTimes", "()I"), 21);
}

#[test]
fn reflection_invoke_long_primitive_round_trips() {
    assert_eq!(run_bootstrap_int("ReflectionTest.class", "invokeStaticAddLong", "()I"), 1);
}

#[test]
fn reflection_missing_class_raises_class_not_found() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "missingClassRaisesClassNotFound", "()I"),
        1
    );
}

#[test]
fn reflection_private_method_invoke_raises_illegal_access() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "privateMethodRaisesIllegalAccess", "()I"),
        1
    );
}

#[test]
fn reflection_target_exception_is_wrapped() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "targetExceptionIsWrapped", "()I"),
        1
    );
}
```

**Step 4: Run the tests to verify RED**

Run: `cargo test -p duke-interpreter reflection_ -- --nocapture`

Expected: FAIL because `java/lang/Class` and `java/lang/reflect/*` natives do not exist yet.

**Step 5: Commit the red test baseline**

```bash
git add tests/fixtures/ReflectionTest.java tests/fixtures/ReflectionTest.class tests/fixtures/ReflectionTarget.class crates/duke-interpreter/src/lib.rs
git commit -m "test: add red reflection fixture coverage"
```

### Task 2: Refactor The Callback Native Boundary For Reflection

**Files:**
- Modify: `crates/duke-interpreter/src/registry.rs`
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Add a tiny reflection-aware callback surface**

Extend the callback-native plumbing so selected natives can ask the interpreter to:
- ensure a class is loaded by internal name
- inspect declared members from parsed classfile bytes

Preferred shape:

```rust
pub struct ReflectedMethodInfo {
    pub name: String,
    pub descriptor: String,
    pub is_public: bool,
    pub is_static: bool,
}

pub struct ReflectedFieldInfo {
    pub name: String,
    pub descriptor: String,
    pub is_public: bool,
    pub is_static: bool,
}

pub struct ReflectedClassInfo {
    pub internal_name: String,
    pub binary_name: String,
    pub methods: Vec<ReflectedMethodInfo>,
    pub fields: Vec<ReflectedFieldInfo>,
}

pub type InspectClassFn<'a> = dyn FnMut(&str) -> VmResult<ReflectedClassInfo> + 'a;
pub type EnsureLoadedFn<'a> = dyn FnMut(&str) -> VmResult<()> + 'a;
```

Then extend `CallbackNativeHandler` so it receives the existing `invoke` closure plus the new `ensure_loaded` and `inspect_class` helpers.

**Step 2: Thread the new helpers through the interpreter**

Update the callback-native call sites inside `execute_class(...)` / `run_execution(...)` so the new closures:
- call `registry.ensure_loaded(...)`
- parse raw class bytes with `loader.find_class(...)` + `duke_classfile::parse(...)`
- translate parsed access flags into the small reflection info structs

Keep this helper surface reflection-sized. Do not turn it into a generic god object just because it can.

**Step 3: Re-run callback regression coverage**

Run:

```bash
cargo test -p duke-interpreter callback_fires_via_invokevirtual_bytecode -- --nocapture
cargo test -p duke-interpreter callback_fires_via_invokeinterface_bytecode -- --nocapture
cargo test -p duke-interpreter collections_sort_fixture_executes -- --nocapture
```

Expected: PASS. Reflection tests still FAIL because the reflection natives do not exist yet.

**Step 4: Commit**

```bash
git add crates/duke-interpreter/src/registry.rs crates/duke-interpreter/src/lib.rs
git commit -m "refactor(interpreter): expose reflection metadata to callback natives"
```

### Task 3: Green Phase For Class Objects And Member Enumeration

**Files:**
- Modify: `crates/duke-interpreter/src/registry.rs`
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Add synthetic reflection classes and exception hierarchy**

Extend `bootstrap_stdlib(...)` with synthetic contexts and native registrations for:
- `java/lang/Class`
- `java/lang/reflect/Method`
- `java/lang/reflect/Field`
- `java/lang/ReflectiveOperationException`
- `java/lang/ClassNotFoundException`
- `java/lang/IllegalAccessException`
- `java/lang/reflect/InvocationTargetException`

Recommended field layouts:

```rust
// java/lang/Class
// fields[0] = binary name String ref
let class_ctx = ClassContext { /* instance_field_count: 1 */ };

// java/lang/reflect/Method
// fields[0] = declaring Class ref
// fields[1] = method name String ref
// fields[2] = descriptor String ref
// fields[3] = flags Int bitset (public/static)
let method_ctx = ClassContext { /* instance_field_count: 4 */ };

// java/lang/reflect/Field
// fields[0] = declaring Class ref
// fields[1] = field name String ref
// fields[2] = descriptor String ref
// fields[3] = flags Int bitset (public/static)
let field_ctx = ClassContext { /* instance_field_count: 4 */ };
```

Register callback natives for the APIs that need loader/registry access:
- `java/lang/Class.forName(Ljava/lang/String;)Ljava/lang/Class;`
- `java/lang/Class.getDeclaredMethods()[Ljava/lang/reflect/Method;`
- `java/lang/Class.getDeclaredFields()[Ljava/lang/reflect/Field;`

Register simple natives for the APIs that only read wrapper state:
- `java/lang/Class.getName()Ljava/lang/String;`
- `java/lang/reflect/Method.getName()Ljava/lang/String;`
- `java/lang/reflect/Field.getName()Ljava/lang/String;`

**Step 2: Refactor class-object interning**

Add helpers near the other reflection utilities:

```rust
fn binary_name_to_internal_name(name: &str) -> String {
    name.replace('.', "/")
}

fn internal_name_to_binary_name(name: &str) -> String {
    name.replace('/', ".")
}

fn intern_class_object(
    heap: &mut duke_gc::Heap,
    string_intern: &mut HashMap<usize, u64>,
    cache_key: usize,
    internal_name: &str,
) -> u64 { /* allocate java/lang/Class with binary-name String field */ }
```

Then update both `Ldc` / `LdcW` class-constant branches to call that helper instead of writing the internal name into `string_value`.

**Step 3: Materialize declared methods and fields from parsed classfiles**

Inside the new callback-backed `Class.getDeclaredMethods()` / `Class.getDeclaredFields()` natives:
1. read the binary class name from the `Class` object
2. convert to internal name
3. use the new `inspect_class(...)` helper
4. walk the returned `methods` / `fields`
5. allocate wrapper objects that store name, descriptor, declaring class, and minimal flag bits
6. return an object array of wrapper refs

Keep it deliberately small:
- include private members in enumeration
- skip `<clinit>`
- include `<init>` only if you explicitly want constructor reflection later; for this phase, skip constructors and stay focused on `Method`

**Step 4: Verify the metadata slice turns green**

Run:

```bash
cargo test -p duke-interpreter reflection_for_name_and_get_name -- --nocapture
cargo test -p duke-interpreter reflection_string_class_literal_uses_binary_name -- --nocapture
cargo test -p duke-interpreter reflection_declared_methods_include_public_and_private -- --nocapture
cargo test -p duke-interpreter reflection_declared_fields_include_public_and_private -- --nocapture
```

Expected: PASS. The invoke-related tests still FAIL.

**Step 5: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(reflection): add Class and member enumeration natives"
```

### Task 4: Green Phase For `Method.invoke()` Success Paths

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Implement reflection argument unboxing and return boxing helpers**

Add helpers close to `parse_arg_types(...)`:

```rust
fn unbox_reflection_arg(
    heap: &duke_gc::Heap,
    expected: char,
    slot: Slot,
) -> VmResult<Slot> { /* handle L/[ directly, unbox I/J/F/D/C/Z from wrappers */ }

fn box_reflection_return(
    heap: &mut duke_gc::Heap,
    return_desc: &str,
    slot: Option<Slot>,
) -> VmResult<Option<Slot>> { /* box primitives, keep refs, map void -> null */ }
```

Support at least:
- `I` via `java/lang/Integer`
- `J` via `java/lang/Long`
- `F` via `java/lang/Float`
- `D` via `java/lang/Double`
- `C` via `java/lang/Character`
- `Z` via `java/lang/Boolean`
- `L` / `[` as raw references

If `Boolean.valueOf/booleanValue` or `Float.valueOf/floatValue` are missing, add those natives in this task before trying to fake your way around them.

**Step 2: Implement `Method.invoke(...)` as a callback native**

Register:

```rust
registry.natives_mut().register_callback(
    "java/lang/reflect/Method",
    "invoke",
    "(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;",
    native_reflect_method_invoke,
);
```

Implementation outline:

```rust
fn native_reflect_method_invoke(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    invoke: &mut InvokeFn<'_>,
) -> VmResult<Option<Slot>> {
    // 1. Read wrapper metadata (declaring class, name, descriptor, flags)
    // 2. Build invoke arg list:
    //    - prepend receiver for instance methods
    //    - unbox Object[] elements according to descriptor parameter types
    // 3. Call back into the interpreter/native path
    // 4. Box primitive return values back to Object
}
```

Behavior rules:
- static method: receiver may be null and must not be prepended
- instance method: receiver must be non-null and becomes arg 0
- `Object[]` may be null only when the target descriptor has zero parameters

**Step 3: Verify the success-path invoke tests**

Run:

```bash
cargo test -p duke-interpreter reflection_invoke_static_and_instance_methods -- --nocapture
cargo test -p duke-interpreter reflection_invoke_long_primitive_round_trips -- --nocapture
```

Expected: PASS. The exception-path reflection tests still FAIL.

**Step 4: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(reflection): add Method.invoke success paths"
```

### Task 5: Green Phase For Reflection Failure Paths

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Step 1: Map `Class.forName` failures to `ClassNotFoundException`**

Inside the `Class.forName` native:
- translate binary name -> internal name
- call `registry.ensure_loaded(...)`
- if loading fails softly, return:

```rust
Err(VmError::JavaException {
    class_name: "java/lang/ClassNotFoundException".to_string(),
})
```

Do not leak `VmError::ClassNotFound` to Java-space here.

**Step 2: Enforce access checks in `Method.invoke`**

Before invoking:
- if wrapper flags say non-public, return `IllegalAccessException`
- do not implement `setAccessible(true)` in this phase

Minimal rule for this phase:

```rust
if !is_public {
    return Err(VmError::JavaException {
        class_name: "java/lang/IllegalAccessException".to_string(),
    });
}
```

**Step 3: Wrap target exceptions in `InvocationTargetException`**

When the callback `invoke(...)` returns a Java exception from the target method:

```rust
match invoke(...) {
    Err(VmError::JavaException { .. }) => {
        return Err(VmError::JavaException {
            class_name: "java/lang/reflect/InvocationTargetException".to_string(),
        });
    }
    other => { /* existing success/error handling */ }
}
```

For this phase, you do not need to preserve the original throwable object as a cause field. Catching the wrapper type correctly is the acceptance target.

**Step 4: Verify the failure-path tests**

Run:

```bash
cargo test -p duke-interpreter reflection_missing_class_raises_class_not_found -- --nocapture
cargo test -p duke-interpreter reflection_private_method_invoke_raises_illegal_access -- --nocapture
cargo test -p duke-interpreter reflection_target_exception_is_wrapped -- --nocapture
cargo test -p duke-interpreter reflection_ -- --nocapture
```

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(reflection): add standard reflection failure mapping"
```

### Task 6: Refactor, Regression Coverage, And Final Verification

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`
- Modify: `docs/plans/vantage-spec-phase30-reflection.md`

**Step 1: Refactor helper duplication**

Extract or tighten small helpers so the reflection code stops breeding inside `lib.rs`:
- `class_binary_name_from_ref`
- `method_wrapper_metadata`
- `field_wrapper_metadata`
- `parse_reflection_member_flags`
- `read_object_array_slots`

Do not add new behavior here. This is pure refactor after green.

**Step 2: Add a short completion note to the phase spec**

Append a concise implementation note to `docs/plans/vantage-spec-phase30-reflection.md` describing:
- what landed in Phase 30
- what remains out of scope (`Constructor`, `setAccessible`, preserving exception causes, deep field access)

**Step 3: Format**

Run: `cargo fmt --all`

**Step 4: Run focused and broader verification**

Run:

```bash
cargo test -p duke-interpreter reflection_ -- --nocapture
cargo test -p duke-interpreter collections_sort_fixture_executes -- --nocapture
cargo test -p duke-interpreter parseargs_ -- --nocapture
cargo test -p duke-interpreter threading_ -- --nocapture
cargo test -p duke-interpreter --lib
```

Expected: all commands PASS.

**Step 5: Review diff for completeness**

Run:

```bash
git status --short
git diff --stat
```

Confirm the slice includes:
- fixture source
- compiled fixture classes
- red/green reflection tests
- runtime changes
- spec completion note

**Step 6: Commit**

```bash
git add docs/plans/vantage-spec-phase30-reflection.md crates/duke-interpreter/src/lib.rs
git commit -m "feat(duke): add core reflection natives"
```

---

## Execution Notes

- Keep this phase on `Class`, `Method`, and `Field`. Do not let constructors, deep field access, or `setAccessible` sneak in wearing a fake mustache.
- Prefer wrapper-object metadata over widening `MethodEntry` unless you hit a hard blocker. The whole point is to keep reflection churn local.
- If `Method.invoke` starts accreting special cases, stop and extract the reflection helpers into a `reflect.rs` module instead of making `lib.rs` even more cursed.
