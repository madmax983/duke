1. Refactor `print_report` methods in `duke-telemetry/src/lib.rs` to handle empty states gracefully (Reduces noise from empty reports).
   - Before: Outputs headers and empty tables for empty components.
   - After: Outputs a concise message stating no events were recorded.
2. Complete pre commit steps to make sure proper testing, verifications, reviews and reflections are done.
3. Submit the change using a descriptive title.
1. **Optimize Vector Allocations in Execution (Vec::with_capacity)**: Pre-allocate vectors in `crates/duke-interpreter/src/execution.rs` for `Multianewarray` `dims` array and lambda args `impl_args` to avoid unnecessary dynamic heap reallocations.
2. Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
3. **Submit the PR**: Present PR titled '⚡ Bolt: Optimize Vector Allocations in Execution & Native' detailing 💡 What, 🎯 Why, 📊 Impact, and 🔭 Measurement. I will use `run_in_bash_session` to execute `git commit` with the requested PR details.
