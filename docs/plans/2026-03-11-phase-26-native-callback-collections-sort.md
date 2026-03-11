# Phase 26 — Native Callback Mechanism + `Collections.sort`

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `CallbackNativeHandler` type that lets native methods call back into the interpreter to invoke Java bytecode, then use it to implement `ArrayList.sort(Comparator)` (the Java 8+ target of `Collections.sort`) with natural ordering via `compareTo` dispatch.

**Architecture:** A new `CallbackNativeHandler` function-pointer type sits alongside the existing `NativeHandler`. The `NativeRegistry` stores a `HandlerKind` enum (`Simple` | `Callback`). When the interpreter dispatches a `Callback` handler it creates an `invoke_cb` closure that captures `registry` and `loader` and delegates to `execute_class` — letting natives invoke interpreter bytecode mid-execution using sequential reborrows (no unsafe). Java 8+ compiles `Collections.sort(list)` to `invokeinterface List.sort:(Ljava/util/Comparator;)V`; Duke dispatches that to the receiver's runtime type (`java/util/ArrayList`), so we register the callback handler there.

**Tech Stack:** Rust, `crates/duke-interpreter/src/lib.rs` (primary), `crates/duke-gc` (write_field), `tests/fixtures/CollectionsSortTest.java`.

---

## Task 1: `CallbackNativeHandler` + `HandlerKind` + `NativeRegistry` refactor

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Background:** `NativeHandler` is `fn(&[Slot], &mut Heap, &mut dyn Write) -> VmResult<Option<Slot>>`. It lives in `duke-interpreter` (depends on `duke-gc::Heap`). We add a parallel type whose last parameter is an `invoke` closure that uses the "loan" pattern — the handler passes its `heap` and `output` borrows *into* the callback, getting them back after each call returns. All 85+ existing `register` calls stay unchanged; they're automatically wrapped in `HandlerKind::Simple`.

### Step 1a: Write the failing tests

Add to `#[cfg(test)]` in `crates/duke-interpreter/src/lib.rs`:

```rust
#[test]
fn native_registry_register_callback_can_be_looked_up() {
    fn dummy_cb(
        _args: &[Slot],
        _heap: &mut duke_gc::Heap,
        _out: &mut dyn std::io::Write,
        _invoke: &mut dyn FnMut(
            &mut duke_gc::Heap,
            &mut dyn std::io::Write,
            &str,
            &str,
            &str,
            Vec<Slot>,
        ) -> VmResult<Option<Slot>>,
    ) -> VmResult<Option<Slot>> {
        Ok(None)
    }
    let mut reg = NativeRegistry::new();
    reg.register_callback("Test", "method", "()V", dummy_cb);
    assert!(matches!(
        reg.get_kind("Test", "method", "()V"),
        Some(HandlerKind::Callback(_))
    ));
}

#[test]
fn native_registry_register_simple_stays_simple() {
    fn dummy(
        _args: &[Slot],
        _heap: &mut duke_gc::Heap,
        _out: &mut dyn std::io::Write,
    ) -> VmResult<Option<Slot>> {
        Ok(None)
    }
    let mut reg = NativeRegistry::new();
    reg.register("Test", "method", "()V", dummy);
    assert!(matches!(
        reg.get_kind("Test", "method", "()V"),
        Some(HandlerKind::Simple(_))
    ));
}
```

### Step 1b: Run to verify failure

```
cargo test -p duke-interpreter native_registry_register_callback
```

Expected: compile error — `CallbackNativeHandler`, `HandlerKind`, `register_callback`, `get_kind` don't exist yet.

### Step 1c: Implement

Find `NativeHandler` in `crates/duke-interpreter/src/lib.rs`. Directly below it, add:

```rust
/// A native handler that can call back into the interpreter to invoke Java methods.
///
/// The `invoke` closure takes `heap` and `output` as *parameters* (not captured),
/// using the "loan" pattern: the handler passes its borrows through each call and
/// gets them back when the call returns. Sequential reborrows — no unsafe required.
pub type CallbackNativeHandler = fn(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn std::io::Write,
    invoke: &mut dyn FnMut(
        &mut duke_gc::Heap,
        &mut dyn std::io::Write,
        &str,   // class name
        &str,   // method name
        &str,   // descriptor
        Vec<Slot>,
    ) -> VmResult<Option<Slot>>,
) -> VmResult<Option<Slot>>;

/// Stored in `NativeRegistry` — all existing handlers stay `Simple`.
#[derive(Copy, Clone)]
pub enum HandlerKind {
    Simple(NativeHandler),
    Callback(CallbackNativeHandler),
}
```

Update `NativeRegistry`:

1. Change the internal `HashMap` value type from `NativeHandler` to `HandlerKind`.
2. Update `register` to wrap: `HandlerKind::Simple(handler)` — callers unchanged.
3. Add `register_callback`:

```rust
pub fn register_callback(
    &mut self,
    class: &str,
    method: &str,
    descriptor: &str,
    handler: CallbackNativeHandler,
) {
    self.handlers.insert(
        (class.to_string(), method.to_string(), descriptor.to_string()),
        HandlerKind::Callback(handler),
    );
}
```

4. Add `get_kind` returning a *copied* `HandlerKind` (both variants are `Copy` since they're `fn` pointers):

```rust
pub fn get_kind(
    &self,
    class: &str,
    method: &str,
    descriptor: &str,
) -> Option<HandlerKind> {
    self.handlers
        .get(&(class.to_string(), method.to_string(), descriptor.to_string()))
        .copied()
}
```

If an existing `get` method returned `NativeHandler` directly, check whether anything calls it — if so, keep it as a convenience wrapper that panics on `Callback` variants, OR update callers in Task 2.

### Step 1d: Run tests

```
cargo test -p duke-interpreter native_registry
cargo test --workspace
```

Expected: both new tests pass; all 371 existing tests pass.

### Step 1e: Commit

```
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(native): add CallbackNativeHandler + HandlerKind enum to NativeRegistry"
```

---

## Task 2: Interpreter dispatch for `Callback` handlers

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Background:** The interpreter dispatches native handlers at several invoke sites. Currently it does something like:

```rust
if let Some(handler) = registry.natives.get(&class, &method, &desc) {
    let result = handler(&callee_args, heap, output)?;
    ...
}
```

We change this to match on `HandlerKind`. The key: `.get_kind(...)` returns a *copied* `HandlerKind` — no live borrow into `registry` — so `registry` is free to be captured in the `invoke_cb` closure inside the `Callback` arm.

### Step 2a: Write the failing test

```rust
#[test]
fn callback_handler_is_dispatched_with_invoke_fn() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static CALLED: AtomicBool = AtomicBool::new(false);

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // Simple helper native: returns 42.
    registry.natives.register(
        "duke/test/Helper",
        "answer",
        "()I",
        |_args, _heap, _out| Ok(Some(Slot::Int(42))),
    );

    // Callback native: invokes the helper and returns its result.
    registry.natives.register_callback(
        "duke/test/Caller",
        "call",
        "()I",
        |_args, heap, output, invoke| {
            CALLED.store(true, std::sync::atomic::Ordering::SeqCst);
            invoke(heap, output, "duke/test/Helper", "answer", "()I", vec![])
        },
    );

    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "duke/test/Caller",
        "call",
        "()I",
        vec![],
    );
    assert!(result.is_ok(), "callback dispatch failed: {result:?}");
    assert_eq!(result.unwrap(), Some(Slot::Int(42)));
    assert!(
        CALLED.load(std::sync::atomic::Ordering::SeqCst),
        "callback handler was never invoked"
    );
}
```

### Step 2b: Run to verify failure

```
cargo test -p duke-interpreter callback_handler_is_dispatched
```

Expected: fails or panics because the `Callback` variant isn't handled.

### Step 2c: Implement

Find every `registry.natives.get(...)` (or equivalent) call in the native dispatch path inside `execute_class`. Replace each with a `match` on `get_kind`:

```rust
match registry.natives.get_kind(&class_name, &method_name, &desc) {
    Some(HandlerKind::Simple(h)) => {
        let result = h(&callee_args, heap, output)?;
        // ... existing result-handling (push to frame, etc.) ...
    }
    Some(HandlerKind::Callback(h)) => {
        // invoke_cb captures registry + loader by reborrow.
        // heap and output are NOT captured — the handler passes them through.
        let mut invoke_cb = |heap: &mut duke_gc::Heap,
                              output: &mut dyn std::io::Write,
                              class: &str,
                              method: &str,
                              desc: &str,
                              args: Vec<Slot>|
         -> VmResult<Option<Slot>> {
            execute_class(registry, loader, heap, output, class, method, desc, args)
        };
        let result = h(&callee_args, heap, output, &mut invoke_cb)?;
        // ... same result-handling as Simple arm ...
    }
    None => {
        // ... existing "method not found" handling (unchanged) ...
    }
}
```

There may be 1–4 such dispatch sites (one per invoke instruction type). Update all of them with the same pattern.

### Step 2d: Run tests

```
cargo test -p duke-interpreter callback_handler_is_dispatched
cargo test --workspace
```

Expected: new test passes; all 371 existing tests pass.

### Step 2e: Commit

```
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(native): dispatch CallbackNativeHandler with invoke_cb closure in interpreter"
```

---

## Task 3: `compareTo` natives for Integer, String, Long, Double

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs` (in `bootstrap_stdlib`)

**Background:** `ArrayList.sort(null)` calls `compareTo(Ljava/lang/Object;)I` on each element pair. We need this registered for the four commonly-sorted types. These are plain `Simple` handlers — no callback needed.

### Step 3a: Write the failing tests

```rust
#[test]
fn integer_compare_to_less_returns_negative() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let a = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(a).unwrap().fields[0] = Slot::Int(3);
    let b = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(b).unwrap().fields[0] = Slot::Int(5);
    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut out,
        "java/lang/Integer", "compareTo", "(Ljava/lang/Object;)I",
        vec![Slot::Reference(Some(a)), Slot::Reference(Some(b))],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(-1)));
}

#[test]
fn integer_compare_to_equal_returns_zero() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let a = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(a).unwrap().fields[0] = Slot::Int(7);
    let b = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(b).unwrap().fields[0] = Slot::Int(7);
    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut out,
        "java/lang/Integer", "compareTo", "(Ljava/lang/Object;)I",
        vec![Slot::Reference(Some(a)), Slot::Reference(Some(b))],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(0)));
}

#[test]
fn string_compare_to_apple_less_than_banana() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let a = heap.allocate_string("apple".to_string());
    let b = heap.allocate_string("banana".to_string());
    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry, &loader, &mut heap, &mut out,
        "java/lang/String", "compareTo", "(Ljava/lang/Object;)I",
        vec![Slot::Reference(Some(a)), Slot::Reference(Some(b))],
    )
    .unwrap();
    match result {
        Some(Slot::Int(n)) => assert!(n < 0, "apple < banana: expected negative, got {n}"),
        other => panic!("expected Int, got {other:?}"),
    }
}
```

### Step 3b: Run to verify failure

```
cargo test -p duke-interpreter integer_compare_to
cargo test -p duke-interpreter string_compare_to
```

Expected: fail — `compareTo` not registered yet.

### Step 3c: Implement

Add to the Integer section of `bootstrap_stdlib`:

```rust
registry.natives.register(
    "java/lang/Integer",
    "compareTo",
    "(Ljava/lang/Object;)I",
    |args, heap, _out| {
        let int_val = |s: &Slot| -> VmResult<i32> {
            match s {
                Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                    Some(Slot::Int(n)) => Ok(*n),
                    _ => Err(VmError::InvalidRef { address: *r }),
                },
                _ => Err(VmError::NullPointerException),
            }
        };
        let a = int_val(args.first().unwrap_or(&Slot::Reference(None)))?;
        let b = int_val(args.get(1).unwrap_or(&Slot::Reference(None)))?;
        Ok(Some(Slot::Int(match a.cmp(&b) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        })))
    },
);
```

Add to the String section:

```rust
registry.natives.register(
    "java/lang/String",
    "compareTo",
    "(Ljava/lang/Object;)I",
    |args, heap, _out| {
        let str_val = |s: &Slot| -> VmResult<String> {
            match s {
                Slot::Reference(Some(r)) => {
                    Ok(heap.get(*r)?.string_value.clone().unwrap_or_default())
                }
                _ => Err(VmError::NullPointerException),
            }
        };
        let a = str_val(args.first().unwrap_or(&Slot::Reference(None)))?;
        let b = str_val(args.get(1).unwrap_or(&Slot::Reference(None)))?;
        Ok(Some(Slot::Int(match a.as_str().cmp(b.as_str()) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        })))
    },
);
```

Add to the Long section (fields[0] is `Slot::Long`):

```rust
registry.natives.register(
    "java/lang/Long",
    "compareTo",
    "(Ljava/lang/Object;)I",
    |args, heap, _out| {
        let long_val = |s: &Slot| -> VmResult<i64> {
            match s {
                Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                    Some(Slot::Long(n)) => Ok(*n),
                    _ => Err(VmError::InvalidRef { address: *r }),
                },
                _ => Err(VmError::NullPointerException),
            }
        };
        let a = long_val(args.first().unwrap_or(&Slot::Reference(None)))?;
        let b = long_val(args.get(1).unwrap_or(&Slot::Reference(None)))?;
        Ok(Some(Slot::Int(match a.cmp(&b) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        })))
    },
);
```

Add to the Double section (fields[0] is `Slot::Double`):

```rust
registry.natives.register(
    "java/lang/Double",
    "compareTo",
    "(Ljava/lang/Object;)I",
    |args, heap, _out| {
        let double_val = |s: &Slot| -> VmResult<f64> {
            match s {
                Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                    Some(Slot::Double(n)) => Ok(*n),
                    _ => Err(VmError::InvalidRef { address: *r }),
                },
                _ => Err(VmError::NullPointerException),
            }
        };
        let a = double_val(args.first().unwrap_or(&Slot::Reference(None)))?;
        let b = double_val(args.get(1).unwrap_or(&Slot::Reference(None)))?;
        Ok(Some(Slot::Int(
            a.partial_cmp(&b)
                .map(|o| match o {
                    std::cmp::Ordering::Less => -1i32,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                })
                .unwrap_or(0), // NaN case → 0
        )))
    },
);
```

### Step 3d: Run tests

```
cargo test -p duke-interpreter integer_compare_to
cargo test -p duke-interpreter string_compare_to
cargo test --workspace
```

Expected: all new tests pass; all 371 existing tests pass.

### Step 3e: Commit

```
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(native): compareTo for Integer, String, Long, Double"
```

---

## Task 4: `ArrayList.sort(Comparator)` as a `CallbackNativeHandler`

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

**Background:** Java 8+ compiles `Collections.sort(list)` to `invokeinterface java/util/List.sort:(Ljava/util/Comparator;)V` with a `null` Comparator argument. Duke dispatches `invokeinterface` by looking up the method on the receiver's runtime class (`java/util/ArrayList`). We register `sort(Ljava/util/Comparator;)V` on `java/util/ArrayList` as a `CallbackNativeHandler`. Only `null` Comparator (natural ordering) is supported; non-null throws.

### Step 4a: Write the failing test

```rust
#[test]
fn array_list_sort_integers_via_callback() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // Build ArrayList [Integer(3), Integer(1), Integer(4)]
    let list = heap.allocate("java/util/ArrayList".to_string(), 4);
    heap.get_mut(list).unwrap().fields[0] = Slot::Int(3); // size
    let make_int = |heap: &mut duke_gc::Heap, n: i32| -> u64 {
        let r = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int(n);
        r
    };
    let i3 = make_int(&mut heap, 3);
    let i1 = make_int(&mut heap, 1);
    let i4 = make_int(&mut heap, 4);
    heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(i3));
    heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(i1));
    heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(i4));

    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/util/ArrayList",
        "sort",
        "(Ljava/util/Comparator;)V",
        vec![Slot::Reference(Some(list)), Slot::Reference(None)], // null Comparator
    )
    .unwrap();

    // After sort: fields[1..=3] = Integer(1), Integer(3), Integer(4)
    let val = |heap: &duke_gc::Heap, s: &Slot| -> i32 {
        match s {
            Slot::Reference(Some(r)) => {
                match heap.get(*r).unwrap().fields.first() {
                    Some(Slot::Int(n)) => *n,
                    _ => -1,
                }
            }
            _ => -1,
        }
    };
    let f = |i: usize| heap.get(list).unwrap().fields[i].clone();
    assert_eq!(val(&heap, &f(1)), 1);
    assert_eq!(val(&heap, &f(2)), 3);
    assert_eq!(val(&heap, &f(3)), 4);
}

#[test]
fn array_list_sort_strings_via_callback() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let list = heap.allocate("java/util/ArrayList".to_string(), 4);
    heap.get_mut(list).unwrap().fields[0] = Slot::Int(3);
    let sb = heap.allocate_string("banana".to_string());
    let sa = heap.allocate_string("apple".to_string());
    let sc = heap.allocate_string("cherry".to_string());
    heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(sb));
    heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(sa));
    heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(sc));

    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/util/ArrayList",
        "sort",
        "(Ljava/util/Comparator;)V",
        vec![Slot::Reference(Some(list)), Slot::Reference(None)],
    )
    .unwrap();

    let str_val = |heap: &duke_gc::Heap, s: &Slot| -> String {
        match s {
            Slot::Reference(Some(r)) => heap.get(*r).unwrap().string_value.clone().unwrap_or_default(),
            _ => String::new(),
        }
    };
    let f = |i: usize| heap.get(list).unwrap().fields[i].clone();
    assert_eq!(str_val(&heap, &f(1)), "apple");
    assert_eq!(str_val(&heap, &f(2)), "banana");
    assert_eq!(str_val(&heap, &f(3)), "cherry");
}
```

### Step 4b: Run to verify failure

```
cargo test -p duke-interpreter array_list_sort
```

Expected: fail — `sort` not registered on ArrayList yet.

### Step 4c: Implement

Add a top-level (non-closure) function for the sort handler in `crates/duke-interpreter/src/lib.rs`:

```rust
fn array_list_sort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn std::io::Write,
    invoke: &mut dyn FnMut(
        &mut duke_gc::Heap,
        &mut dyn std::io::Write,
        &str,
        &str,
        &str,
        Vec<Slot>,
    ) -> VmResult<Option<Slot>>,
) -> VmResult<Option<Slot>> {
    // args[0] = ArrayList ref, args[1] = Comparator (null = natural ordering)
    let list_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };

    // Only null Comparator (natural ordering) supported.
    if !matches!(args.get(1), Some(Slot::Reference(None)) | None) {
        return Err(VmError::ClassNotFound {
            name: "ArrayList.sort with non-null Comparator is not yet supported".into(),
        });
    }

    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n as usize,
        _ => return Ok(None),
    };

    if size <= 1 {
        return Ok(None);
    }

    // Collect element refs (fields[1..=size]).
    let mut elems: Vec<u64> = (1..=size)
        .filter_map(|i| match heap.get(list_ref).ok()?.fields.get(i) {
            Some(Slot::Reference(Some(r))) => Some(*r),
            _ => None,
        })
        .collect();

    if elems.len() != size {
        return Ok(None); // malformed ArrayList — bail safely
    }

    // Insertion sort — O(n²), correct, easy to verify.
    for i in 1..elems.len() {
        let key = elems[i];
        let mut j = i;
        while j > 0 {
            let receiver = elems[j - 1];
            let class_name = heap.get(receiver)?.class_name.clone();
            let cmp = invoke(
                heap,
                output,
                &class_name,
                "compareTo",
                "(Ljava/lang/Object;)I",
                vec![Slot::Reference(Some(receiver)), Slot::Reference(Some(key))],
            )?;
            match cmp {
                Some(Slot::Int(n)) if n <= 0 => break,
                _ => {}
            }
            elems[j] = elems[j - 1];
            j -= 1;
        }
        elems[j] = key;
    }

    // Write sorted elements back using the write barrier.
    for (i, &r) in elems.iter().enumerate() {
        heap.write_field(list_ref, i + 1, Slot::Reference(Some(r)))?;
    }

    Ok(None)
}
```

Register it in `bootstrap_stdlib` in the ArrayList section:

```rust
registry.natives.register_callback(
    "java/util/ArrayList",
    "sort",
    "(Ljava/util/Comparator;)V",
    array_list_sort,
);
```

### Step 4d: Run tests

```
cargo test -p duke-interpreter array_list_sort
cargo test --workspace
```

Expected: both new tests pass; all 371 existing tests pass.

### Step 4e: Commit

```
git add crates/duke-interpreter/src/lib.rs
git commit -m "feat(native): ArrayList.sort(Comparator) via callback — insertion sort with compareTo"
```

---

## Task 5: `CollectionsSortTest` fixture + integration test

**Files:**
- Create: `tests/fixtures/CollectionsSortTest.java`
- Compile: `tests/fixtures/CollectionsSortTest.class`
- Modify: `crates/duke-interpreter/src/lib.rs`

**Background:** Java 8+ compiles `Collections.sort(list)` to `list.sort(null)`. The integration test runs the real bytecode through Duke end-to-end, exercising the full path: `invokeinterface List.sort` → `ArrayList.sort` `CallbackNativeHandler` → `invoke_cb` → `execute_class(Integer.compareTo)` → result → sort → output.

### Step 5a: Create the fixture

Create `tests/fixtures/CollectionsSortTest.java`:

```java
import java.util.*;

public class CollectionsSortTest {
    public static void main(String[] args) {
        // Integer sort (exercises callback → Integer.compareTo native)
        List<Integer> ints = new ArrayList<>();
        ints.add(3); ints.add(1); ints.add(4); ints.add(1); ints.add(5);
        Collections.sort(ints);
        for (int x : ints) System.out.println(x);

        // String sort (exercises callback → String.compareTo native)
        List<String> strs = new ArrayList<>();
        strs.add("banana");
        strs.add("apple");
        strs.add("cherry");
        Collections.sort(strs);
        for (String s : strs) System.out.println(s);
    }
}
```

### Step 5b: Compile and verify with HotSpot

```
javac --release 21 tests/fixtures/CollectionsSortTest.java -d tests/fixtures/
java -cp tests/fixtures CollectionsSortTest
```

Expected output (verify this matches before writing the assertion):

```
1
1
3
4
5
apple
banana
cherry
```

Also inspect the bytecode to confirm the sort call site:

```
javap -c tests/fixtures/CollectionsSortTest.class | grep -A2 "sort"
```

Expected to see `invokeinterface java/util/List.sort:(Ljava/util/Comparator;)V` (not `invokestatic Collections.sort`). This confirms Duke must dispatch via `ArrayList.sort`.

### Step 5c: Write the failing integration test

Add to `#[cfg(test)]` in `crates/duke-interpreter/src/lib.rs`:

```rust
#[test]
fn collections_sort_end_to_end() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "CollectionsSortTest",
        "main",
        "([Ljava/lang/String;)V",
        vec![Slot::Reference(Some(arr_ref))],
    );
    assert!(result.is_ok(), "CollectionsSortTest failed: {result:?}");
    let output = String::from_utf8(out).unwrap();
    // Integer sort: 1 1 3 4 5 (one per line)
    assert!(
        output.contains("1\n1\n3\n4\n5"),
        "integer sort wrong:\n{output}"
    );
    // String sort: apple banana cherry (one per line)
    assert!(
        output.contains("apple\nbanana\ncherry"),
        "string sort wrong:\n{output}"
    );
}
```

### Step 5d: Run the integration test

```
cargo test -p duke-interpreter collections_sort_end_to_end -- --nocapture
```

If the test fails with a bytecode error, inspect the generated `.class` with `javap -c` to understand what's happening, then fix the dispatch path.

### Step 5e: Run full workspace

```
cargo test --workspace
```

Expected: all tests pass.

### Step 5f: Commit

```
git add tests/fixtures/CollectionsSortTest.java tests/fixtures/CollectionsSortTest.class \
        crates/duke-interpreter/src/lib.rs
git commit -m "test(native): CollectionsSortTest — integer and string sort via Collections.sort callback"
```

---

## Task 6: Regression pass + MEMORY.md update

**Files:**
- Modify: `C:\Users\markm\.claude\projects\C--Users-markm-duke\memory\MEMORY.md`

### Step 6a: Full regression

```
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
```

If `fmt --check` fails, run `cargo fmt --all` and re-verify tests.

### Step 6b: Update MEMORY.md

Update `C:\Users\markm\.claude\projects\C--Users-markm-duke\memory\MEMORY.md`:

- Status line: `Phase 26 Complete (N tests passing)` with actual count
- Update interpreter test count
- Add **Phase 26 Additions** section with:
  - `CallbackNativeHandler` type + `HandlerKind` enum in duke-interpreter
  - `NativeRegistry::register_callback` + `get_kind`
  - `compareTo` natives for Integer, String, Long, Double
  - `ArrayList.sort(Ljava/util/Comparator;)V` callback handler (insertion sort)
  - `CollectionsSortTest.java` fixture
  - Note: `Collections.sort(list)` compiles to `list.sort(null)` in Java 8+

### Step 6c: Final commit

```
git commit -m "docs: update MEMORY.md for Phase 26 native callback + Collections.sort"
```

---

## Summary

| Task | Scope | Key Artifact |
|------|-------|--------------|
| 1 | `CallbackNativeHandler` + `HandlerKind` + `NativeRegistry` | `register_callback`, `get_kind` |
| 2 | Interpreter dispatch for `Callback` handlers | `invoke_cb` closure, recursive `execute_class` |
| 3 | `compareTo` natives — Integer, String, Long, Double | 4 `Simple` handlers |
| 4 | `ArrayList.sort(Comparator)` callback handler | Insertion sort via `compareTo` |
| 5 | Fixture + integration test | `CollectionsSortTest.java` |
| 6 | Regression + MEMORY.md | All tests green, docs updated |

**Invariants throughout:**
- All 85+ existing `NativeHandler` registrations stay unchanged — wrapped in `HandlerKind::Simple`
- `get_kind` returns a *copied* `HandlerKind` — no live borrow into registry when the closure is created
- `invoke_cb` captures `registry` and `loader` by reborrow; `heap`/`output` are loaned through as parameters
- `ArrayList.sort` uses `write_field` for GC write-barrier correctness
- `Collections.sort(list)` in Java 8+ bytecode = `invokeinterface List.sort:(Ljava/util/Comparator;)V` → dispatches to `ArrayList.sort`
