1. **Refactor `parse_class_members` in `crates/duke-classfile/src/parser.rs`**: Extract the inline loops for parsing interfaces, fields, and methods into separate helper functions (`parse_interfaces`, `parse_fields`, `parse_methods`) to flatten the structure and improve readability.
2. Verify changes with `cargo clippy`, `cargo test`, and `cargo fmt`.
3. Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
4. Call the submit tool to create a PR titled "⚒️ Forge: Refactor parse_class_members" detailing the smell, solution, benefit, and verification.
