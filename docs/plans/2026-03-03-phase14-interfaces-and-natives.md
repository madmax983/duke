# Phase 14: invokeinterface, multianewarray & More Natives Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement `invokeinterface` opcode for interface method dispatch, `multianewarray` for multi-dimensional arrays, and expand native method coverage (Object.hashCode/toString, String.valueOf, PrintStream.print, System.exit).

**Architecture:** `invokeinterface` works like `invokevirtual` but resolves from `InterfaceMethodref` instead of `Methodref`. We extend `resolve_methodref` to accept both CP entry types. `multianewarray` recursively allocates nested arrays on the heap. New native methods follow the existing `NativeHandler` pattern.

**Tech Stack:** Rust, duke-interpreter crate, duke-gc (Heap), duke-runtime (Slot/VmError)

---

## Background

### Current Gaps
1. `invokeinterface` — not implemented; blocks Collections, Iterator, Comparable, any interface-based code
2. `multianewarray` — not implemented; blocks `int[][]` and multi-dimensional arrays
3. `resolve_methodref` only handles `CpEntry::Methodref`, not `InterfaceMethodref`
4. Missing practical native methods: Object.hashCode/toString, String.valueOf, PrintStream.print, System.exit

### Key Design Decisions
- `invokeinterface` dispatch: identical to `invokevirtual` (pop args + this, lookup method, push frame) — the only difference is the CP entry type and the `count` byte (which we ignore, it's a JVM optimization hint)
- `resolve_methodref` extended to match both `Methodref` and `InterfaceMethodref` — same structure
- `multianewarray`: allocate outermost array, then recursively allocate inner arrays as references
- `Object.hashCode()` returns the heap address as hash (simple but valid)
- `Object.toString()` returns `"ClassName@hexHash"` as a heap String
- `System.exit(int)` calls `std::process::exit()` in production, returns error in tests

---

## Task 1: Test Fixtures

**Files:**
- Create: `tests/fixtures/InterfaceTest.java` + `.class`
- Create: `tests/fixtures/MultiArray.java` + `.class`
- Create: `tests/fixtures/MoreNatives.java` + `.class`

### Step 1: Write InterfaceTest.java

```java
/** Tests invokeinterface dispatch. */
public class InterfaceTest {
    interface Adder {
        int add(int a, int b);
    }

    static class SimpleAdder implements Adder {
        public int add(int a, int b) {
            return a + b;
        }
    }

    static class DoubleAdder implements Adder {
        public int add(int a, int b) {
            return (a + b) * 2;
        }
    }

    /** Calls interface method on SimpleAdder. */
    public static int callSimple() {
        Adder a = new SimpleAdder();
        return a.add(3, 4); // 7
    }

    /** Calls interface method on DoubleAdder. */
    public static int callDouble() {
        Adder a = new DoubleAdder();
        return a.add(3, 4); // 14
    }

    /** Polymorphic dispatch: same interface, different impl. */
    public static int polymorphic(int which) {
        Adder a;
        if (which == 0) {
            a = new SimpleAdder();
        } else {
            a = new DoubleAdder();
        }
        return a.add(5, 3); // 8 or 16
    }
}
```

### Step 2: Write MultiArray.java

```java
public class MultiArray {
    /** Creates a 2x3 int[][] and returns sum of all elements after init. */
    public static int sum2d() {
        int[][] grid = new int[2][3];
        int val = 1;
        for (int i = 0; i < 2; i++) {
            for (int j = 0; j < 3; j++) {
                grid[i][j] = val++;
            }
        }
        // grid = {{1,2,3},{4,5,6}} → sum = 21
        int sum = 0;
        for (int i = 0; i < 2; i++) {
            for (int j = 0; j < 3; j++) {
                sum += grid[i][j];
            }
        }
        return sum; // 21
    }

    /** Returns dimensions: outer length * inner length. */
    public static int dimensions() {
        int[][] grid = new int[3][4];
        return grid.length * grid[0].length; // 12
    }
}
```

### Step 3: Write MoreNatives.java

```java
public class MoreNatives {
    /** Returns hashCode of a new Object (should be non-zero). */
    public static int objectHashCode() {
        Object o = new MoreNatives();
        return (o.hashCode() != 0) ? 1 : 0; // 1
    }

    /** Returns String.valueOf(42). Tests static native method. */
    public static int valueOfInt() {
        String s = String.valueOf(42);
        return s.length(); // 2 ("42" has 2 chars)
    }

    /** Prints without newline then with newline. */
    public static int printNoNewline() {
        System.out.print("AB");
        System.out.println("CD");
        return 1;
    }
}
```

### Step 4: Compile

```bash
cd tests/fixtures && javac --release 21 InterfaceTest.java MultiArray.java MoreNatives.java
```

NOTE: InterfaceTest.java may produce inner class files (InterfaceTest$Adder.class, InterfaceTest$SimpleAdder.class, InterfaceTest$DoubleAdder.class). These must also be committed.

### Step 5: Commit

```bash
git add tests/fixtures/InterfaceTest*.java tests/fixtures/InterfaceTest*.class tests/fixtures/MultiArray.java tests/fixtures/MultiArray.class tests/fixtures/MoreNatives.java tests/fixtures/MoreNatives.class
git commit -m "test(phase14): add InterfaceTest, MultiArray, and MoreNatives fixtures"
```

---

## Task 2: invokeinterface + multianewarray + resolve_methodref extension

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Extend resolve_methodref to handle InterfaceMethodref

Currently `resolve_methodref` only matches `CpEntry::Methodref`. Add `CpEntry::InterfaceMethodref` as an additional match arm:

```rust
fn resolve_methodref(cp: &[Option<CpEntry>], idx: usize) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Methodref {
            class_index,
            name_and_type_index,
        })
        | Some(CpEntry::InterfaceMethodref {
            class_index,
            name_and_type_index,
        }) => {
            // ... existing resolution logic unchanged ...
        }
        _ => Err(VmError::InvalidMethodref { index: idx }),
    }
}
```

### Step 2: Implement invokeinterface in execute_class

Add a new match arm. `invokeinterface` works identically to `invokevirtual` — the `count` byte is just a JVM optimization hint that we ignore:

```rust
Instruction::Invokeinterface { index: cp_idx, count: _ } => {
    // Resolve via InterfaceMethodref — same structure as Methodref.
    let (callee_class, callee_name, callee_desc) = {
        let ctx = registry.get(&current_class)?;
        resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
    };
    if callee_name == "<clinit>" {
        idx += 1;
        continue;
    }
    let loaded = registry.ensure_loaded(&callee_class, loader)?;

    // For interface dispatch, look up the method on the ACTUAL object's class,
    // not the interface class. This is the key difference from invokevirtual.
    // Peek at `this` to get the actual class.
    let arg_count = parse_arg_count(&callee_desc);
    let stack_len = frame.operand_stack_len();
    let this_slot = frame.peek_at(arg_count)?; // `this` is below args
    let actual_class = match this_slot {
        Slot::Reference(Some(r)) => heap.get(r)?.class_name.clone(),
        Slot::Reference(None) => return Err(VmError::NullPointerException),
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };

    // Try to load the actual class and find the method there.
    let _ = registry.ensure_loaded(&actual_class, loader);
    let callee_idx = {
        match registry.get(&actual_class) {
            Ok(ctx) => ctx.methods.iter().position(|m| m.name == callee_name && m.descriptor == callee_desc),
            Err(_) => None,
        }
    };
    let (dispatch_class, callee_idx) = match callee_idx {
        Some(i) => (actual_class.clone(), i),
        None => {
            // Check native registry.
            if let Some(handler) = registry.natives().get(&callee_class, &callee_name, &callee_desc) {
                let handler = *handler;
                let mut native_args: Vec<Slot> = (0..arg_count)
                    .map(|_| frame.pop())
                    .collect::<VmResult<Vec<_>>>()?;
                native_args.reverse();
                let this_slot = frame.pop()?;
                native_args.insert(0, this_slot);
                let result = handler(&native_args, heap, stdout)?;
                if let Some(val) = result {
                    frame.push(val)?;
                }
                idx += 1;
                continue;
            }
            // Also try actual class in native registry.
            if let Some(handler) = registry.natives().get(&actual_class, &callee_name, &callee_desc) {
                let handler = *handler;
                let mut native_args: Vec<Slot> = (0..arg_count)
                    .map(|_| frame.pop())
                    .collect::<VmResult<Vec<_>>>()?;
                native_args.reverse();
                let this_slot = frame.pop()?;
                native_args.insert(0, this_slot);
                let result = handler(&native_args, heap, stdout)?;
                if let Some(val) = result {
                    frame.push(val)?;
                }
                idx += 1;
                continue;
            }
            // No-op fallback: pop args + this.
            for _ in 0..arg_count {
                frame.pop()?;
            }
            frame.pop()?;
            idx += 1;
            continue;
        }
    };
    // Standard frame push (same as invokevirtual).
    let mut callee_args: Vec<Slot> = (0..arg_count)
        .map(|_| frame.pop())
        .collect::<VmResult<Vec<_>>>()?;
    callee_args.reverse();
    let this_slot = frame.pop()?;
    callee_args.insert(0, this_slot);
    let (callee_pc_to_idx, callee_frame) = {
        let ctx = registry.get(&dispatch_class)?;
        let pci: HashMap<usize, usize> = ctx.methods[callee_idx]
            .instructions.iter().enumerate()
            .map(|(i, &(pc, _))| (pc, i)).collect();
        let f = Frame::new(
            usize::from(ctx.methods[callee_idx].max_stack),
            usize::from(ctx.methods[callee_idx].max_locals),
            callee_args,
        )?;
        (pci, f)
    };
    call_stack.push(CallFrame {
        frame, method_idx, pc_to_idx,
        resume_idx: idx + 1,
        class_name: current_class.clone(),
    });
    frame = callee_frame;
    method_idx = callee_idx;
    pc_to_idx = callee_pc_to_idx;
    current_class = dispatch_class;
    idx = 0;
    continue;
}
```

NOTE: `peek_at` may not exist on Frame. You may need to add it, or calculate the this position differently. Alternative: pop all args, peek at this (which is now on top), then re-push args. Or just use the same approach as invokevirtual — pop args, pop this, insert this at front.

Actually, the simpler approach: handle invokeinterface EXACTLY like invokevirtual, but after getting the `callee_class` from the InterfaceMethodref, look up the method on the actual receiver's class instead:

```rust
// After resolving callee_class/name/desc from CP and popping args + this:
// Use the actual object's class for method lookup, not the interface class.
let actual_class = match &this_slot {
    Slot::Reference(Some(r)) => heap.get(*r)?.class_name.clone(),
    _ => callee_class.clone(),
};
```

### Step 3: Implement multianewarray in execute_class

```rust
Instruction::Multianewarray { index: cp_idx, dimensions } => {
    let element_type = {
        let ctx = registry.get(&current_class)?;
        resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
    };
    // Pop dimension sizes from stack (first dimension is deepest on stack).
    let dims: Vec<i32> = (0..*dimensions)
        .map(|_| frame.pop_int())
        .collect::<VmResult<Vec<_>>>()?;
    let dims: Vec<i32> = dims.into_iter().rev().collect();

    // Check for negative sizes.
    for &d in &dims {
        if d < 0 {
            return Err(VmError::NegativeArraySize { size: d });
        }
    }

    // Recursively allocate arrays.
    fn alloc_multi(heap: &mut Heap, dims: &[i32], depth: usize, type_name: &str) -> u64 {
        let size = dims[depth] as usize;
        if depth == dims.len() - 1 {
            // Innermost dimension — allocate leaf array.
            let r = heap.allocate(type_name.to_string(), size);
            // Init fields to correct default based on leaf type.
            r
        } else {
            // Outer dimension — allocate array of references.
            let array_type = &type_name[..type_name.len() - (dims.len() - depth - 1) * 2];
            // Actually, just use the full type for outer, sub-type for inner.
            let r = heap.allocate(type_name.to_string(), size);
            for i in 0..size {
                let inner = alloc_multi(heap, dims, depth + 1, &type_name[1..]);
                heap.get_mut(r).unwrap().fields[i] = Slot::Reference(Some(inner));
            }
            r
        }
    }

    let r = alloc_multi(heap, &dims, 0, &element_type);
    frame.push(Slot::Reference(Some(r)))?;
}
```

NOTE: The type name slicing logic above is simplified. For `[[I` (int[][]), the outer array is `[[I`, inner is `[I`. So stripping one `[` per dimension works. Be careful with reference array types like `[[Ljava/lang/String;`.

### Step 4: Add peek_at to Frame if needed

In `crates/duke-runtime/src/frame.rs`:
```rust
pub fn peek_at(&self, n: usize) -> VmResult<Slot> {
    let idx = self.operand_stack.len().checked_sub(n + 1)
        .ok_or(VmError::StackUnderflow)?;
    Ok(self.operand_stack[idx])
}

pub fn operand_stack_len(&self) -> usize {
    self.operand_stack.len()
}
```

### Step 5: Run tests + commit

```bash
cargo test && cargo clippy --workspace -- -W clippy::pedantic -W clippy::nursery
git add crates/duke-interpreter/src/lib.rs crates/duke-runtime/src/frame.rs
git commit -m "feat(interpreter): implement invokeinterface and multianewarray opcodes"
```

---

## Task 3: More Native Methods + Integration Tests

**Files:**
- Modify: `crates/duke-interpreter/src/lib.rs`

### Step 1: Add native methods to bootstrap_stdlib

Register these additional native handlers:

```rust
// Object instance methods
registry.natives_mut().register("java/lang/Object", "hashCode", "()I", native_object_hashcode);
registry.natives_mut().register("java/lang/Object", "toString", "()Ljava/lang/String;", native_object_tostring);

// String static methods
registry.natives_mut().register("java/lang/String", "valueOf", "(I)Ljava/lang/String;", native_string_value_of_int);

// PrintStream.print (no newline)
registry.natives_mut().register("java/io/PrintStream", "print", "(Ljava/lang/String;)V", native_print_string);
registry.natives_mut().register("java/io/PrintStream", "print", "(I)V", native_print_int);
```

### Step 2: Implement native handlers

```rust
fn native_object_hashcode(args: &[Slot], _heap: &mut Heap, _out: &mut dyn Write) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => Ok(Some(Slot::Int(*r as i32))),
        _ => Err(VmError::NullPointerException),
    }
}

fn native_object_tostring(args: &[Slot], heap: &mut Heap, _out: &mut dyn Write) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let class_name = heap.get(this_ref)?.class_name.clone();
    let hash = this_ref as i32;
    let s = format!("{class_name}@{hash:x}");
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_string_value_of_int(args: &[Slot], heap: &mut Heap, _out: &mut dyn Write) -> VmResult<Option<Slot>> {
    // Static method — no `this`. args[0] is the int.
    let val = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    let s = val.to_string();
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

fn native_print_string(args: &[Slot], heap: &mut Heap, out: &mut dyn Write) -> VmResult<Option<Slot>> {
    let string_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => { write!(out, "null").ok(); return Ok(None); }
        _ => return Err(VmError::TypeMismatch { expected: "Reference", got: "other" }),
    };
    let obj = heap.get(string_ref)?;
    let text = obj.string_value.as_deref().unwrap_or("null");
    write!(out, "{text}").ok();
    Ok(None)
}

fn native_print_int(args: &[Slot], _heap: &mut Heap, out: &mut dyn Write) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => return Err(VmError::TypeMismatch { expected: "Int", got: "other" }),
    };
    write!(out, "{val}").ok();
    Ok(None)
}
```

### Step 3: Add ALL integration tests

**InterfaceTest tests:**
```rust
#[test] fn interface_call_simple() { /* callSimple → Int(7) */ }
#[test] fn interface_call_double() { /* callDouble → Int(14) */ }
#[test] fn interface_polymorphic_zero() { /* polymorphic(0) → Int(8) */ }
#[test] fn interface_polymorphic_one() { /* polymorphic(1) → Int(16) */ }
```

**MultiArray tests:**
```rust
#[test] fn multi_array_sum_2d() { /* sum2d → Int(21) */ }
#[test] fn multi_array_dimensions() { /* dimensions → Int(12) */ }
```

**MoreNatives tests:**
```rust
#[test] fn object_hash_code_nonzero() { /* objectHashCode → Int(1) */ }
#[test] fn string_value_of_int() { /* valueOfInt → Int(2) */ }
#[test] fn print_no_newline() { /* printNoNewline → Int(1), check output "ABCD\n" */ }
```

### Step 4: Full verification

```bash
cargo test && cargo clippy --workspace -- -W clippy::pedantic -W clippy::nursery && cargo fmt --check
```

Target: 185+ tests passing.

---

## Wave Execution Strategy

**Wave 1 (parallel):**
- Agent A: Task 1 (fixtures — InterfaceTest, MultiArray, MoreNatives)
- Agent B: Task 2 (invokeinterface + multianewarray + resolve_methodref extension)

**Wave 2 (sequential):**
- Agent C: Task 3 (native methods + ALL integration tests)

**Wave 3 (leader):**
- Verify, commit, update memory
