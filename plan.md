1.  **Refactor `parse_class_members` in `duke-classfile/src/parser.rs`**:
    *   **Smell**: The `parse_class_members` function still contains inline loops to build `interfaces`, `fields`, and `methods` vectors. This mixes high-level structural parsing with low-level array population and bounds checking.
    *   **Solution**: Extract the inline array building loops into dedicated helper functions: `parse_interfaces`, `parse_fields`, and `parse_methods`.
    *   **Benefit**: Flattens the `parse_class_members` function, clearly separating the high-level class structure definition from the repeated element parsing logic, adhering to the "Forge" persona's directive to extract large sequential parsing chunks into small helper functions.
2.  **Verify and test**:
    *   Run `cargo test --all-targets --all-features` to ensure no runtime behavior has changed.
    *   Run `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --all`.
3.  **Complete pre commit steps**
    *   Complete pre commit steps to make sure proper testing, verifications, reviews and reflections are done.
4.  **Submit PR**:
    *   Create a PR with the title "⚒️ Forge: Extract class member array parsers".
