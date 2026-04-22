1. **The Weak Point (IDENTIFY)**: In `crates/duke-interpreter/src/native.rs`, the `native_string_repeat` function handles the implementation of `String.repeat(int)`. It unconditionally uses `s.repeat(n)` where `n` can be up to `i32::MAX`. If the string is sufficiently large, or if `n` is `i32::MAX`, this will attempt an unbounded memory allocation (e.g., 40GB+), causing the standard library to trigger a fatal `capacity overflow` panic (SIGABRT) that brings down the entire JVM instead of gracefully raising a `java/lang/OutOfMemoryError`.

2. **The Harness (ATTACK)**: Create a new test file `crates/duke-interpreter/tests/havoc_string_repeat_oom.rs` to reproduce the failure. The test will initialize the heap, allocate a string `1234567890`, and call `native_string_repeat` with `n = i32::MAX`. We will use a custom global allocator (like the one in `havoc_bytecode_oom.rs`) to track allocations and ensure it doesn't actually try to allocate 40GB in the test runner, or we can just assert that it returns an error instead of panicking. Wait, using the custom allocator will gracefully intercept it, returning a null pointer or failing, but `Vec::with_capacity` via `String::repeat` panics directly on capacity overflow *before* even calling the allocator if the size exceeds `isize::MAX`. Thus, we must fix the code to prevent the panic.

3. **The Fix (Refactor)**:
Update `native_string_repeat` in `crates/duke-interpreter/src/native.rs`:
```rust
<<<<<<< SEARCH
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.repeat(n));
    Ok(Some(Slot::Reference(Some(r))))
=======
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();

    // Prevent capacity overflow panics and unbounded allocation.
    // JVM strings max length are practically limited. We set a 128MB limit for repeated strings.
    let max_size = 1024 * 1024 * 128;
    if n.checked_mul(s.len()).is_none_or(|len| len > max_size) {
        return Err(VmError::JavaException {
            class_name: "java/lang/OutOfMemoryError".to_string(),
        });
    }

    let r = heap.allocate_string(s.repeat(n));
    Ok(Some(Slot::Reference(Some(r))))
>>>>>>> REPLACE
```

4. **Verify (DETONATE)**: Run `cargo test -p duke-interpreter --test havoc_string_repeat_oom` to ensure the test passes gracefully without aborting. Then run the workspace tests.
5. Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
6. **Submit (PRESENT)**: Submit the PR with Havoc's specific title and formatting requirements.
