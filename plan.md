1. **Optimize String concatenation in `native_string_concat`**
   - The function currently uses `format!("{s1}{s2}")` to concatenate two strings, which allocates a new string buffer inside `format!`, formats the arguments, and returns it.
   - We can optimize this by pre-allocating a `String` with the exact required capacity and appending the strings:
     ```rust
     let mut combined = String::with_capacity(s1.len() + s2.len());
     combined.push_str(&s1);
     combined.push_str(&s2);
     let r = heap.allocate_string(combined);
     ```
   - This eliminates the intermediate allocation and parsing overhead of `format!`, which is a common hotspot in interpreters.
   - Add `// ⚡ Bolt: Eliminate intermediate format! allocation` comment.
   - Ensure the tests pass.
1. Refactor `print_report` methods in `duke-telemetry/src/lib.rs` to handle empty states gracefully (Reduces noise from empty reports).
   - Before: Outputs headers and empty tables for empty components.
   - After: Outputs a concise message stating no events were recorded.
2. Complete pre commit steps to make sure proper testing, verifications, reviews and reflections are done.
3. Submit the change using a descriptive title.
1. **Optimize Vector Allocations in Execution (Vec::with_capacity)**: Pre-allocate vectors in `crates/duke-interpreter/src/execution.rs` for `Multianewarray` `dims` array and lambda args `impl_args` to avoid unnecessary dynamic heap reallocations.
2. Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
3. **Submit the PR**: Present PR titled '⚡ Bolt: Optimize Vector Allocations in Execution & Native' detailing 💡 What, 🎯 Why, 📊 Impact, and 🔭 Measurement. I will use `run_in_bash_session` to execute `git commit` with the requested PR details.
