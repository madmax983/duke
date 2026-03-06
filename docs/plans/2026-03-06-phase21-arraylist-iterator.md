# Phase 21: ArrayList + Iterator (For-Each Over Collections) — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `java/util/ArrayList` and a companion iterator so that for-each loops over collections (`for (T x : list)`) execute correctly.

**Architecture:** Two synthetic classes registered in `bootstrap_stdlib()`. ArrayList stores elements directly in `HeapObject.fields` — fields[0] is a size counter (`Slot::Int`), fields[1..] are the element slots pushed dynamically via `Vec::push`. The iterator is a private synthetic class `duke/util/ArrayListIterator` holding a reference to the list and a cursor index. Both `invokevirtual` (for `add`/`get`/`size`/`iterator`) and `invokeinterface` (for `hasNext`/`next`) already dispatch to native handlers — no opcode changes needed.

**Tech Stack:** Rust, duke-interpreter (`crates/duke-interpreter/src/lib.rs`), duke-gc, javac 21

---

## Background: For-Each Bytecode Pattern

Javac compiles `for (String s : list)` to:

```
aload list_ref
invokevirtual  ArrayList.iterator()Ljava/util/Iterator;   ← returns our iterator
astore iter_ref

[loop top]
aload iter_ref
invokeinterface Iterator.hasNext()Z                        ← native on actual class
ifeq [loop end]
aload iter_ref
invokeinterface Iterator.next()Ljava/lang/Object;          ← native on actual class
checkcast java/lang/String
astore s
[loop body]
goto [loop top]
[loop end]
```

Both `invokeinterface` calls dispatch through Duke's native registry using the **actual runtime class** of the object — so as long as our iterator object's class is registered with `hasNext`/`next` natives, the calls resolve correctly.

---

## Task 1: ArrayList + ArrayListIterator Synthetic Classes + 8 Natives

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` — `bootstrap_stdlib()` ending at ~line 1098, native handlers section (~line 7201+)
- Create: `tests/fixtures/ArrayListTest.java`

### Step 1: Create the Java fixture

Create `tests/fixtures/ArrayListTest.java`:

```java
import java.util.ArrayList;

public class ArrayListTest {
    // Basic construction and size
    static int testSize() {
        ArrayList<String> list = new ArrayList<>();
        list.add("a");
        list.add("bb");
        list.add("ccc");
        return list.size();  // expect 3
    }

    // get() by index
    static int testGet() {
        ArrayList<String> list = new ArrayList<>();
        list.add("hello");
        list.add("world");
        String s = list.get(1);
        return s.length();  // "world" = 5
    }

    // For-each loop — count elements
    static int testForEachCount() {
        ArrayList<String> list = new ArrayList<>();
        list.add("a");
        list.add("b");
        list.add("c");
        int count = 0;
        for (String s : list) {
            count++;
        }
        return count;  // expect 3
    }

    // For-each loop — sum string lengths
    static int testForEachSum() {
        ArrayList<String> list = new ArrayList<>();
        list.add("hi");
        list.add("there");
        list.add("x");
        int total = 0;
        for (String s : list) {
            total += s.length();
        }
        return total;  // 2 + 5 + 1 = 8
    }

    // Empty list — for-each body never executes
    static int testEmptyForEach() {
        ArrayList<String> list = new ArrayList<>();
        int count = 0;
        for (String s : list) {
            count++;
        }
        return count;  // expect 0
    }

    // Single element
    static int testSingleElement() {
        ArrayList<String> list = new ArrayList<>();
        list.add("only");
        for (String s : list) {
            return s.length();  // "only" = 4
        }
        return -1;
    }

    // add() returns true (boolean)
    static int testAddReturnsTrue() {
        ArrayList<String> list = new ArrayList<>();
        boolean result = list.add("x");
        if (result) return 1;
        return 0;
    }
}
```

Compile:
```bash
javac --release 21 tests/fixtures/ArrayListTest.java
```

### Step 2: Register synthetic `java/util/ArrayList` in `bootstrap_stdlib()`

Insert before the closing `}` of `bootstrap_stdlib()` (currently ~line 1098):

```rust
// java/util/ArrayList — dynamic list backed by growable fields
// fields[0] = size (Int), fields[1..] = elements (pushed dynamically)
let arraylist_ctx = ClassContext {
    class_name: "java/util/ArrayList".to_string(),
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
registry.register(arraylist_ctx);
registry.natives_mut().register(
    "java/util/ArrayList", "<init>", "()V", native_arraylist_init,
);
registry.natives_mut().register(
    "java/util/ArrayList", "add", "(Ljava/lang/Object;)Z", native_arraylist_add,
);
registry.natives_mut().register(
    "java/util/ArrayList", "get", "(I)Ljava/lang/Object;", native_arraylist_get,
);
registry.natives_mut().register(
    "java/util/ArrayList", "size", "()I", native_arraylist_size,
);
registry.natives_mut().register(
    "java/util/ArrayList", "iterator", "()Ljava/util/Iterator;", native_arraylist_iterator,
);
```

### Step 3: Register synthetic `duke/util/ArrayListIterator` in `bootstrap_stdlib()`

This is an internal synthetic class, not a real Java class. The name just needs to be unique.

```rust
// duke/util/ArrayListIterator — internal iterator for ArrayList
// fields[0] = ArrayList reference, fields[1] = current index (Int)
let iter_ctx = ClassContext {
    class_name: "duke/util/ArrayListIterator".to_string(),
    super_class: Some("java/lang/Object".to_string()),
    constant_pool: Vec::new(),
    methods: Vec::new(),
    fields: vec![
        FieldEntry {
            name: "list".to_string(),
            descriptor: "Ljava/util/ArrayList;".to_string(),
            is_static: false,
        },
        FieldEntry {
            name: "index".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        },
    ],
    static_fields: Vec::new(),
    instance_field_count: 2,
    bootstrap_methods: Vec::new(),
};
registry.register(iter_ctx);
registry.natives_mut().register(
    "duke/util/ArrayListIterator", "hasNext", "()Z", native_arraylist_iter_hasnext,
);
registry.natives_mut().register(
    "duke/util/ArrayListIterator", "next", "()Ljava/lang/Object;", native_arraylist_iter_next,
);
```

### Step 4: Implement ArrayList native handlers

Add after the Character natives (~line 7201). All follow `NativeHandler` signature: `fn(&[Slot], &mut duke_gc::Heap, &mut dyn Write) -> VmResult<Option<Slot>>`.

```rust
// ---- ArrayList natives ----

/// Native: `ArrayList.<init>()V` — initializes with size=0.
/// fields[0] = Int(0)  (size counter)
fn native_arraylist_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    // fields[0] was already set to Int(0) by allocate(), but set explicitly for clarity
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ArrayList.add(Object)Z` — appends element, returns true.
/// Increments fields[0] (size), pushes element onto fields.
fn native_arraylist_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let element = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let obj = heap.get_mut(this_ref)?;
    // Increment size
    if let Some(Slot::Int(ref mut sz)) = obj.fields.first_mut() {
        *sz += 1;
    }
    // Append element after size field
    obj.fields.push(element);
    Ok(Some(Slot::Int(1))) // boolean true
}

/// Native: `ArrayList.get(I)Object` — returns element at index.
/// Element is at fields[index + 1] (fields[0] is size).
#[allow(clippy::cast_sign_loss)]
fn native_arraylist_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let idx = match args.get(1) {
        Some(Slot::Int(i)) => *i as usize,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    let obj = heap.get(this_ref)?;
    // fields[0] = size, fields[1..] = elements
    match obj.fields.get(idx + 1) {
        Some(slot) => Ok(Some(slot.clone())),
        None => Err(VmError::JavaException {
            class_name: "java/lang/ArrayIndexOutOfBoundsException".to_string(),
        }),
    }
}

/// Native: `ArrayList.size()I` — returns the size counter at fields[0].
fn native_arraylist_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(Slot::Int(sz)) => Ok(Some(Slot::Int(*sz))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `ArrayList.iterator()Iterator` — creates an ArrayListIterator.
/// Allocates duke/util/ArrayListIterator with fields[0]=list_ref, fields[1]=0.
fn native_arraylist_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let iter_ref = heap.allocate("duke/util/ArrayListIterator".to_string(), 2);
    {
        let iter_obj = heap.get_mut(iter_ref)?;
        iter_obj.fields[0] = Slot::Reference(Some(this_ref)); // list ref
        iter_obj.fields[1] = Slot::Int(0);                    // cursor index
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
}

// ---- ArrayListIterator natives ----

/// Native: `ArrayListIterator.hasNext()Z`
/// Returns true if cursor index < list size.
fn native_arraylist_iter_hasnext(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let iter_obj = heap.get(this_ref)?;
    let list_ref = match iter_obj.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Int(0))), // no list → false
    };
    let cursor = match iter_obj.fields.get(1) {
        Some(Slot::Int(i)) => *i,
        _ => 0,
    };
    let list_size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(cursor < list_size))))
}

/// Native: `ArrayListIterator.next()Object`
/// Returns element at cursor, then increments cursor.
#[allow(clippy::cast_sign_loss)]
fn native_arraylist_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    // Read list ref and cursor
    let (list_ref, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let lr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let c = match iter_obj.fields.get(1) {
            Some(Slot::Int(i)) => *i,
            _ => 0,
        };
        (lr, c)
    };
    // Get element from list (fields[cursor + 1])
    let element = {
        let list_obj = heap.get(list_ref)?;
        match list_obj.fields.get(cursor as usize + 1) {
            Some(slot) => slot.clone(),
            None => return Err(VmError::JavaException {
                class_name: "java/util/NoSuchElementException".to_string(),
            }),
        }
    };
    // Advance cursor
    heap.get_mut(this_ref)?.fields[1] = Slot::Int(cursor + 1);
    Ok(Some(element))
}
```

### Step 5: Write 7 integration tests

Add to the test module at the bottom of `crates/duke-interpreter/src/lib.rs`:

```rust
#[test]
fn arraylist_size() {
    assert_eq!(
        run_class_int("ArrayListTest.class", "testSize", "()I", vec![]),
        3
    );
}

#[test]
fn arraylist_get() {
    assert_eq!(
        run_class_int("ArrayListTest.class", "testGet", "()I", vec![]),
        5
    );
}

#[test]
fn arraylist_foreach_count() {
    assert_eq!(
        run_class_int("ArrayListTest.class", "testForEachCount", "()I", vec![]),
        3
    );
}

#[test]
fn arraylist_foreach_sum() {
    assert_eq!(
        run_class_int("ArrayListTest.class", "testForEachSum", "()I", vec![]),
        8
    );
}

#[test]
fn arraylist_empty_foreach() {
    assert_eq!(
        run_class_int("ArrayListTest.class", "testEmptyForEach", "()I", vec![]),
        0
    );
}

#[test]
fn arraylist_single_element() {
    assert_eq!(
        run_class_int("ArrayListTest.class", "testSingleElement", "()I", vec![]),
        4
    );
}

#[test]
fn arraylist_add_returns_true() {
    assert_eq!(
        run_class_int("ArrayListTest.class", "testAddReturnsTrue", "()I", vec![]),
        1
    );
}
```

### Step 6: Run tests

```bash
cargo test -p duke-interpreter -- arraylist
```

Expected: 7 new tests pass.

### Step 7: Run full suite

```bash
cargo test --workspace
```

Expected: ~296+ tests, 0 failures.

### Step 8: Commit

```bash
git add crates/duke-interpreter/src/lib.rs tests/fixtures/ArrayListTest.java tests/fixtures/ArrayListTest.class
git commit -m "feat(interpreter): implement java.util.ArrayList with Iterator and for-each support"
```

---

## Task 2: Smoke Tests + Plan Doc Commit

**Files:**
- Modify: `docs/plans/2026-03-06-phase21-arraylist-iterator.md` (mark complete)

### Step 1: CLI smoke tests

```bash
cargo run -- exec tests/fixtures/ArrayListTest.class testForEachSum
# Expected: Int(8)

cargo run -- exec tests/fixtures/ArrayListTest.class testEmptyForEach
# Expected: Int(0)

cargo run -- exec tests/fixtures/ArrayListTest.class testForEachCount
# Expected: Int(3)
```

### Step 2: Commit plan doc

```bash
git add docs/plans/2026-03-06-phase21-arraylist-iterator.md
git commit -m "docs: add phase 21 plan (ArrayList + Iterator for-each support)"
```

---

## Summary

| Task | What | New Natives | New Tests |
|------|------|-------------|-----------|
| 1 | `java/util/ArrayList` + `duke/util/ArrayListIterator` | 7 (init, add, get, size, iterator, hasNext, next) | 7 |
| 2 | Smoke tests + plan doc | 0 | 0 |

**Total new tests**: 7
**Expected final count**: ~296+
**New synthetic classes**: 2 (`java/util/ArrayList`, `duke/util/ArrayListIterator`)
**New natives**: 7

### Design Notes

- **ArrayList element storage**: `fields[0]` = size counter, `fields[1..]` = elements pushed with `Vec::push`. No separate backing array needed — HeapObject.fields is already a `Vec<Slot>`.
- **Iterator class name**: `duke/util/ArrayListIterator` (internal name, not real Java). Duke's invokeinterface dispatches by **actual runtime class**, so the name just needs to be registered with `hasNext`/`next` natives.
- **No interface declaration needed**: Duke doesn't enforce `implements Iterable`/`implements Iterator`. The bytecode just calls methods by name+descriptor — if the native is registered, it resolves.
- **checkcast after `next()`**: javac inserts `checkcast java/lang/String` after `Iterator.next()`. Duke's checkcast is permissive (doesn't enforce type at runtime for synthetic classes), so this passes through.
