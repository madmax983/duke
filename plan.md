1. Refactor `print_report` methods in `duke-telemetry/src/lib.rs` to handle empty states gracefully (Reduces noise from empty reports).
   - Before: Outputs headers and empty tables for empty components.
   - After: Outputs a concise message stating no events were recorded.
2. Complete pre commit steps to make sure proper testing, verifications, reviews and reflections are done.
3. Submit the change using a descriptive title.
