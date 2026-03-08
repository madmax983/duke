# Phase 23: java.util.HashMap + java.util.HashSet — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `java.util.HashMap` (put/get/containsKey/size/remove/isEmpty/getOrDefault) and `java.util.HashSet` (add/contains/remove/size/isEmpty) as synthetic native classes in the Duke JVM interpreter.

**Architecture:** All additions are synthetic classes registered in `bootstrap_stdlib()` in `crates/duke-interpreter/src/lib.rs`. HashMap stores entries as interleaved key-value pairs in `HeapObject.fields`: `fields[0]` = entry count (`Slot::Int`), `fields[1]` = key0, `fields[2]` = val0, `fields[3]` = key1, etc. HashSet stores elements similarly: `fields[0]` = size, `fields[1..]` = elements (deduped on add). A shared `slots_equal` helper compares keys by value (string_value for Strings, fields[0] for boxed primitives, identity for others).

**Tech Stack:** Rust, duke-interpreter, duke-gc, javac 21

---

## Background

### HeapObject layout (from duke-gc)

```rust
struct HeapObject {
    class_name: String,
    fields: Vec<Slot>,
    string_value: Option<String>,
}
```

Arrays and collections reuse `fields` as the backing store. Arrays use `fields[0..n]` directly. ArrayList uses `fields[0]` = size + `fields[1..]` = elements. We follow the same pattern for HashMap and HashSet.

### HashMap entry layout

```
fields[0]  = Slot::Int(n)     // number of key-value pairs
fields[1]  = key0             // Slot (Reference to String or boxed primitive)
fields[2]  = val0
fields[3]  = key1
fields[4]  = val1
...
```

`<init>` sets `fields[0] = Slot::Int(0)` and leaves `fields` with just that one element. `put` appends two elements per new entry. `remove` swap-removes the pair.

### HashSet entry layout

```
fields[0]  = Slot::Int(n)     // number of elements
fields[1]  = elem0
fields[2]  = elem1
...
```

Same as ArrayList but `add` deduplicates using `slots_equal`.

### Key equality: `slots_equal` helper

```rust
fn slots_equal(a: &Slot, b: &Slot, heap: &duke_gc::Heap) -> bool {
    match (a, b) {
        (Slot::Reference(None), Slot::Reference(None)) => true,
        (Slot::Reference(Some(ra)), Slot::Reference(Some(rb))) => {
            if ra == rb { return true; }
            let oa = match heap.get(*ra) { Ok(o) => o, Err(_) => return false };
            let ob = match heap.get(*rb) { Ok(o) => o, Err(_) => return false };
            // String equality by value
            if oa.string_value.is_some() || ob.string_value.is_some() {
                return oa.string_value == ob.string_value;
            }
            // Same class: compare first field (Integer, Long, Double, etc.)
            if oa.class_name == ob.class_name {
                oa.fields.first() == ob.fields.first()
            } else {
                false
            }
        }
        _ => false,
    }
}
```

**Why clone fields before searching:** `native_hashmap_get` takes `heap: &mut Heap`. Calling `heap.get(this_ref)?.fields.clone()` drops the immutable borrow before any mutable borrow. Then `slots_equal(..., heap)` works because `heap: &mut Heap` coerces to `&Heap` for the immutable call. After `slots_equal` returns, any subsequent `heap.get_mut(...)` in the same scope is safe.

### How `checkcast` works (already implemented)

`checkcast` at lines 4402–4431 compares the heap object's `class_name` to the CP-resolved class name. Our synthetic Integer objects have `class_name = "java/lang/Integer"`, so `(Integer) map.get("key")` passes the cast when the value is an Integer object. Null references also pass checkcast (per spec).

### bootstrap_stdlib location

`bootstrap_stdlib` closes at **line 1297** of `crates/duke-interpreter/src/lib.rs`. The last registration before the `}` is:
```rust
registry.natives_mut().register("java/util/Arrays", "sort", "([I)V", native_arrays_sort_int);
}
```
Add new ClassContext registrations and native registrations **before** that `}`.

### Test helper

```rust
fn run_bootstrap_int(class_name: &str, method_name: &str, descriptor: &str) -> i32
```

Located at line ~11733. Calls `bootstrap_stdlib`, loads the fixture class, runs the method, returns the `i32` result. Used for all HashMap/HashSet tests.

---

## Task 1: java.util.HashMap

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` — `bootstrap_stdlib()` + native handlers section
- Create: `tests/fixtures/HashMapTest.java`

### Step 1: Create Java fixture

Create `tests/fixtures/HashMapTest.java`:

```java
import java.util.HashMap;

public class HashMapTest {
    // Basic put + get
    static int testPutAndGet() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(10));
        map.put("b", Integer.valueOf(20));
        Integer va = (Integer) map.get("a");
        Integer vb = (Integer) map.get("b");
        return va.intValue() + vb.intValue();  // 30
    }

    // Size after puts
    static int testSize() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", Integer.valueOf(1));
        map.put("y", Integer.valueOf(2));
        map.put("z", Integer.valueOf(3));
        return map.size();  // 3
    }

    // containsKey present and absent
    static int testContainsKey() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("key", Integer.valueOf(42));
        if (map.containsKey("key") && !map.containsKey("missing")) return 1;
        return 0;
    }

    // get missing key returns null
    static int testGetMissing() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(1));
        if (map.get("nope") == null) return 1;
        return 0;
    }

    // remove decreases size
    static int testRemove() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(5));
        map.put("b", Integer.valueOf(6));
        map.remove("a");
        return map.size();  // 1
    }

    // isEmpty on empty and non-empty
    static int testIsEmpty() {
        HashMap<String, Integer> map = new HashMap<>();
        if (!map.isEmpty()) return 0;
        map.put("k", Integer.valueOf(1));
        if (map.isEmpty()) return 0;
        return 1;
    }

    // overwrite existing key keeps size the same
    static int testOverwrite() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("k", Integer.valueOf(1));
        map.put("k", Integer.valueOf(99));
        return map.size();  // 1 (not 2)
    }

    // getOrDefault: present returns value, absent returns default
    static int testGetOrDefault() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(7));
        Integer v1 = (Integer) map.getOrDefault("a", Integer.valueOf(0));
        Integer v2 = (Integer) map.getOrDefault("missing", Integer.valueOf(99));
        return v1.intValue() + v2.intValue();  // 106
    }
}
```

### Step 2: Compile fixture

```bash
cd /c/Users/markm/duke && javac --release 21 tests/fixtures/HashMapTest.java
```

Expected: `tests/fixtures/HashMapTest.class` created, no errors.

### Step 3: Register HashMap ClassContext in `bootstrap_stdlib()`

In `crates/duke-interpreter/src/lib.rs`, find the closing `}` of `bootstrap_stdlib` (near line 1297). **Before** that `}`, add:

```rust
// java/util/HashMap — linear-scan hash map (fields[0]=size, fields[1..]=interleaved key-val pairs)
let hashmap_ctx = ClassContext {
    class_name: "java/util/HashMap".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: vec![FieldEntry {
        name: "size".to_string(),
        descriptor: "I".to_string(),
        is_static: false,
    }],
    static_fields: Vec::new(),
    instance_field_count: 1,
    bootstrap_methods: Vec::new(),
};
registry.register(hashmap_ctx);
registry.natives_mut().register(
    "java/util/HashMap", "<init>", "()V", native_hashmap_init,
);
registry.natives_mut().register(
    "java/util/HashMap", "put",
    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
    native_hashmap_put,
);
registry.natives_mut().register(
    "java/util/HashMap", "get",
    "(Ljava/lang/Object;)Ljava/lang/Object;",
    native_hashmap_get,
);
registry.natives_mut().register(
    "java/util/HashMap", "containsKey",
    "(Ljava/lang/Object;)Z",
    native_hashmap_contains_key,
);
registry.natives_mut().register(
    "java/util/HashMap", "size", "()I", native_hashmap_size,
);
registry.natives_mut().register(
    "java/util/HashMap", "remove",
    "(Ljava/lang/Object;)Ljava/lang/Object;",
    native_hashmap_remove,
);
registry.natives_mut().register(
    "java/util/HashMap", "isEmpty", "()Z", native_hashmap_is_empty,
);
registry.natives_mut().register(
    "java/util/HashMap", "getOrDefault",
    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
    native_hashmap_get_or_default,
);
```

### Step 4: Add `slots_equal` helper and 8 HashMap natives

Find the end of the ArrayList iterator natives (the last `native_arraylist_iter_*` function). Add after them (before the Arrays natives section):

```rust
// ---- slots_equal helper (used by HashMap and HashSet) ----

/// Semantic equality for HeapObject-backed keys.
/// - String objects: equal if string_value matches
/// - Boxed primitives (Integer, Long, Double, etc.): equal if same class + same fields[0]
/// - Other references: equal by identity (ObjRef)
/// - Null == Null
fn slots_equal(a: &Slot, b: &Slot, heap: &duke_gc::Heap) -> bool {
    match (a, b) {
        (Slot::Reference(None), Slot::Reference(None)) => true,
        (Slot::Reference(Some(ra)), Slot::Reference(Some(rb))) => {
            if ra == rb {
                return true; // identity shortcut
            }
            let oa = match heap.get(*ra) {
                Ok(o) => o,
                Err(_) => return false,
            };
            let ob = match heap.get(*rb) {
                Ok(o) => o,
                Err(_) => return false,
            };
            // String equality by value
            if oa.string_value.is_some() || ob.string_value.is_some() {
                return oa.string_value == ob.string_value;
            }
            // Same class: compare first field (handles Integer, Long, Double, etc.)
            if oa.class_name == ob.class_name {
                oa.fields.first() == ob.fields.first()
            } else {
                false
            }
        }
        _ => false,
    }
}

// ---- HashMap natives ----

/// Native: `HashMap.<init>()V` — initialises size counter.
fn native_hashmap_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.is_empty() {
        obj.fields.push(Slot::Int(0));
    } else {
        obj.fields[0] = Slot::Int(0);
    }
    Ok(None)
}

/// Native: `HashMap.put(Object, Object)Object`
///
/// If key already present: updates value, returns old value.
/// If key absent: appends key-value pair, increments size, returns null.
fn native_hashmap_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let val = args.get(2).cloned().unwrap_or(Slot::Reference(None));

    // Clone fields to allow immutable heap lookups during key search
    let fields = heap.get(this_ref)?.fields.clone();

    // fields[0] = size, pairs start at index 1
    let mut i = 1usize;
    while i + 1 < fields.len() {
        if slots_equal(&fields[i], &key, heap) {
            let old = fields[i + 1].clone();
            heap.get_mut(this_ref)?.fields[i + 1] = val;
            return Ok(Some(old));
        }
        i += 2;
    }

    // New key — append pair and increment size
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException),
    }
    obj.fields.push(key);
    obj.fields.push(val);
    Ok(Some(Slot::Reference(None)))
}

/// Native: `HashMap.get(Object)Object` — returns value for key, or null.
fn native_hashmap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    let mut i = 1usize;
    while i + 1 < fields.len() {
        if slots_equal(&fields[i], &key, heap) {
            return Ok(Some(fields[i + 1].clone()));
        }
        i += 2;
    }
    Ok(Some(Slot::Reference(None)))
}

/// Native: `HashMap.containsKey(Object)Z` — 1 if key present, 0 otherwise.
fn native_hashmap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    let mut i = 1usize;
    while i + 1 < fields.len() {
        if slots_equal(&fields[i], &key, heap) {
            return Ok(Some(Slot::Int(1)));
        }
        i += 2;
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `HashMap.size()I` — returns entry count.
fn native_hashmap_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Ok(Some(Slot::Int(*n))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `HashMap.remove(Object)Object` — removes key-value pair, returns old value or null.
///
/// Uses swap-remove: replaces the removed pair with the last pair for O(1) removal.
fn native_hashmap_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    let mut i = 1usize;
    while i + 1 < fields.len() {
        if slots_equal(&fields[i], &key, heap) {
            let old_val = fields[i + 1].clone();
            let obj = heap.get_mut(this_ref)?;
            // Swap-remove the pair at index i, i+1 with the last pair
            let last_val_idx = obj.fields.len() - 1;
            let last_key_idx = obj.fields.len() - 2;
            obj.fields.swap(i + 1, last_val_idx);
            obj.fields.swap(i, last_key_idx);
            obj.fields.truncate(obj.fields.len() - 2);
            // Decrement size
            match obj.fields.first_mut() {
                Some(Slot::Int(sz)) => *sz -= 1,
                _ => return Err(VmError::NullPointerException),
            }
            return Ok(Some(old_val));
        }
        i += 2;
    }
    Ok(Some(Slot::Reference(None)))
}

/// Native: `HashMap.isEmpty()Z` — 1 if size == 0, else 0.
fn native_hashmap_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) => Ok(Some(Slot::Int(1))),
        Some(Slot::Int(_)) => Ok(Some(Slot::Int(0))),
        _ => Ok(Some(Slot::Int(1))),
    }
}

/// Native: `HashMap.getOrDefault(Object, Object)Object`
///
/// Returns value if key present, else returns the default value.
fn native_hashmap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let default = args.get(2).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    let mut i = 1usize;
    while i + 1 < fields.len() {
        if slots_equal(&fields[i], &key, heap) {
            return Ok(Some(fields[i + 1].clone()));
        }
        i += 2;
    }
    Ok(Some(default))
}
```

### Step 5: Write 8 integration tests

Find the test module at the bottom of `crates/duke-interpreter/src/lib.rs`. Add after the existing `double_nan` test (or near the end of the tests section):

```rust
#[test]
fn hashmap_put_and_get() {
    assert_eq!(run_bootstrap_int("HashMapTest.class", "testPutAndGet", "()I"), 30);
}

#[test]
fn hashmap_size() {
    assert_eq!(run_bootstrap_int("HashMapTest.class", "testSize", "()I"), 3);
}

#[test]
fn hashmap_contains_key() {
    assert_eq!(run_bootstrap_int("HashMapTest.class", "testContainsKey", "()I"), 1);
}

#[test]
fn hashmap_get_missing() {
    assert_eq!(run_bootstrap_int("HashMapTest.class", "testGetMissing", "()I"), 1);
}

#[test]
fn hashmap_remove() {
    assert_eq!(run_bootstrap_int("HashMapTest.class", "testRemove", "()I"), 1);
}

#[test]
fn hashmap_is_empty() {
    assert_eq!(run_bootstrap_int("HashMapTest.class", "testIsEmpty", "()I"), 1);
}

#[test]
fn hashmap_overwrite() {
    assert_eq!(run_bootstrap_int("HashMapTest.class", "testOverwrite", "()I"), 1);
}

#[test]
fn hashmap_get_or_default() {
    assert_eq!(run_bootstrap_int("HashMapTest.class", "testGetOrDefault", "()I"), 106);
}
```

### Step 6: Run tests

```bash
cargo test -p duke-interpreter -- hashmap_
```

Expected: 8 tests pass. If `testRemove` returns the wrong value, debug the swap-remove logic — after remove("a"), only "b" remains so `size()` = 1.

```bash
cargo test --workspace
```

Expected: ~321 total, 0 failures.

### Step 7: Commit

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/HashMapTest.java tests/fixtures/HashMapTest.class
git commit -m "feat(interpreter): implement java.util.HashMap with 8 native methods"
```

---

## Task 2: java.util.HashSet

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` — `bootstrap_stdlib()` + native handlers
- Create: `tests/fixtures/HashSetTest.java`

### Step 1: Create Java fixture

Create `tests/fixtures/HashSetTest.java`:

```java
import java.util.HashSet;

public class HashSetTest {
    // add and contains
    static int testAddAndContains() {
        HashSet<String> set = new HashSet<>();
        set.add("hello");
        set.add("world");
        if (set.contains("hello") && set.contains("world")) return 1;
        return 0;
    }

    // size after adds
    static int testSize() {
        HashSet<Integer> set = new HashSet<>();
        set.add(Integer.valueOf(1));
        set.add(Integer.valueOf(2));
        set.add(Integer.valueOf(3));
        return set.size();  // 3
    }

    // no duplicates
    static int testNoDuplicates() {
        HashSet<String> set = new HashSet<>();
        set.add("x");
        set.add("x");  // duplicate — ignored
        set.add("y");
        return set.size();  // 2
    }

    // remove
    static int testRemove() {
        HashSet<String> set = new HashSet<>();
        set.add("a");
        set.add("b");
        boolean removed = set.remove("a");
        if (removed && set.size() == 1 && !set.contains("a")) return 1;
        return 0;
    }

    // isEmpty
    static int testIsEmpty() {
        HashSet<String> set = new HashSet<>();
        if (!set.isEmpty()) return 0;
        set.add("item");
        if (set.isEmpty()) return 0;
        return 1;
    }

    // add returns false for duplicate
    static int testAddReturnsFalse() {
        HashSet<String> set = new HashSet<>();
        boolean first = set.add("dup");
        boolean second = set.add("dup");
        if (first && !second) return 1;
        return 0;
    }
}
```

### Step 2: Compile fixture

```bash
cd /c/Users/markm/duke && javac --release 21 tests/fixtures/HashSetTest.java
```

Expected: `tests/fixtures/HashSetTest.class` created, no errors.

### Step 3: Register HashSet ClassContext in `bootstrap_stdlib()`

In `crates/duke-interpreter/src/lib.rs`, in `bootstrap_stdlib()`, **after** the HashMap registration block (and still before the closing `}`), add:

```rust
// java/util/HashSet — deduplicated element set (fields[0]=size, fields[1..]=elements)
let hashset_ctx = ClassContext {
    class_name: "java/util/HashSet".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: vec![FieldEntry {
        name: "size".to_string(),
        descriptor: "I".to_string(),
        is_static: false,
    }],
    static_fields: Vec::new(),
    instance_field_count: 1,
    bootstrap_methods: Vec::new(),
};
registry.register(hashset_ctx);
registry.natives_mut().register(
    "java/util/HashSet", "<init>", "()V", native_hashset_init,
);
registry.natives_mut().register(
    "java/util/HashSet", "add", "(Ljava/lang/Object;)Z", native_hashset_add,
);
registry.natives_mut().register(
    "java/util/HashSet", "contains", "(Ljava/lang/Object;)Z", native_hashset_contains,
);
registry.natives_mut().register(
    "java/util/HashSet", "remove", "(Ljava/lang/Object;)Z", native_hashset_remove,
);
registry.natives_mut().register(
    "java/util/HashSet", "size", "()I", native_hashset_size,
);
registry.natives_mut().register(
    "java/util/HashSet", "isEmpty", "()Z", native_hashset_is_empty,
);
```

### Step 4: Implement 6 HashSet native handlers

Add after the HashMap natives (before the Arrays natives section):

```rust
// ---- HashSet natives ----

/// Native: `HashSet.<init>()V` — initialises size counter.
fn native_hashset_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.is_empty() {
        obj.fields.push(Slot::Int(0));
    } else {
        obj.fields[0] = Slot::Int(0);
    }
    Ok(None)
}

/// Native: `HashSet.add(Object)Z` — adds element if not already present.
/// Returns 1 if added, 0 if already present.
fn native_hashset_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let element = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    // fields[0] = size, fields[1..] = elements
    for i in 1..fields.len() {
        if slots_equal(&fields[i], &element, heap) {
            return Ok(Some(Slot::Int(0))); // already present
        }
    }

    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException),
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1)))
}

/// Native: `HashSet.contains(Object)Z` — 1 if element present, 0 otherwise.
fn native_hashset_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let element = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    for i in 1..fields.len() {
        if slots_equal(&fields[i], &element, heap) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `HashSet.remove(Object)Z` — removes element if present, returns 1 if removed.
fn native_hashset_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let element = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    for i in 1..fields.len() {
        if slots_equal(&fields[i], &element, heap) {
            let obj = heap.get_mut(this_ref)?;
            let last_idx = obj.fields.len() - 1;
            obj.fields.swap(i, last_idx);
            obj.fields.truncate(obj.fields.len() - 1);
            match obj.fields.first_mut() {
                Some(Slot::Int(sz)) => *sz -= 1,
                _ => return Err(VmError::NullPointerException),
            }
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `HashSet.size()I` — returns element count.
fn native_hashset_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Ok(Some(Slot::Int(*n))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `HashSet.isEmpty()Z` — 1 if size == 0, else 0.
fn native_hashset_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) => Ok(Some(Slot::Int(1))),
        Some(Slot::Int(_)) => Ok(Some(Slot::Int(0))),
        _ => Ok(Some(Slot::Int(1))),
    }
}
```

### Step 5: Write 6 integration tests

```rust
#[test]
fn hashset_add_and_contains() {
    assert_eq!(run_bootstrap_int("HashSetTest.class", "testAddAndContains", "()I"), 1);
}

#[test]
fn hashset_size() {
    assert_eq!(run_bootstrap_int("HashSetTest.class", "testSize", "()I"), 3);
}

#[test]
fn hashset_no_duplicates() {
    assert_eq!(run_bootstrap_int("HashSetTest.class", "testNoDuplicates", "()I"), 2);
}

#[test]
fn hashset_remove() {
    assert_eq!(run_bootstrap_int("HashSetTest.class", "testRemove", "()I"), 1);
}

#[test]
fn hashset_is_empty() {
    assert_eq!(run_bootstrap_int("HashSetTest.class", "testIsEmpty", "()I"), 1);
}

#[test]
fn hashset_add_returns_false() {
    assert_eq!(run_bootstrap_int("HashSetTest.class", "testAddReturnsFalse", "()I"), 1);
}
```

### Step 6: Run tests

```bash
cargo test -p duke-interpreter -- hashset_
cargo test --workspace
```

Expected: 6 new tests pass, ~327 total, 0 failures.

### Step 7: Commit

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/HashSetTest.java tests/fixtures/HashSetTest.class
git commit -m "feat(interpreter): implement java.util.HashSet with 6 native methods"
```

---

## Task 3: Smoke Tests + Plan Doc Commit

### Step 1: Full workspace suite

```bash
cargo test --workspace 2>&1 | tail -15
```

Expected: 327 tests, 0 failures.

### Step 2: CLI smoke tests

```bash
cargo run -- exec tests/fixtures/HashMapTest.class testPutAndGet
# Expected: Int(30)

cargo run -- exec tests/fixtures/HashMapTest.class testOverwrite
# Expected: Int(1)

cargo run -- exec tests/fixtures/HashSetTest.class testNoDuplicates
# Expected: Int(2)
```

### Step 3: Commit plan doc

```bash
git add docs/plans/2026-03-07-phase23-hashmap-hashset.md
git commit -m "docs: add phase 23 plan (HashMap + HashSet)"
```

---

## Summary

| Task | What | New Natives | New Tests |
|------|------|-------------|-----------|
| 1 | `java.util.HashMap` (put/get/containsKey/size/remove/isEmpty/getOrDefault) + `slots_equal` helper | 8 | 8 |
| 2 | `java.util.HashSet` (add/contains/remove/size/isEmpty) | 6 | 6 |
| 3 | Smoke + plan doc | 0 | 0 |

**Total new tests**: 14
**Expected final count**: ~327
**New synthetic classes**: 2 (`java/util/HashMap`, `java/util/HashSet`)
**New natives**: 14 (+ 1 helper function)
**Key implementation detail**: `slots_equal` clones `fields` before searching so `heap` can be reborrowed for string/boxed-primitive comparisons. HashMap uses swap-remove for O(1) deletion.
