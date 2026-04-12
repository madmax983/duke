⚒️ Forge: Refactor Repeated Slot Extraction Boilerplate

🚮 Smell: Repeated primitive extraction boilerplate (`args.get(idx).copied().unwrap_or(Slot::Reference(None))`) was scattered across `native.rs` and `stdlib.rs`, causing code bloat, increasing cognitive load, and masking the core logic of the VM native bindings.

✨ Solution: Created a `SlotExt` extension trait in `duke_runtime` to encapsulate these extractions into explicitly named helper methods (`unwrap_or_ref()`, `unwrap_or_int()`, `unwrap_or_long()`, `unwrap_or_double()`). Refactored `native.rs` and `stdlib.rs` to use these helpers.

🧼 Benefit: Dramatically reduces boilerplate, flattens the native functions, and enforces DRY principles, making the native handlers significantly easier to read and maintain.

🛡️ Verification: Tests passed. No runtime logic was altered. Verified via `cargo fmt --all`, `cargo clippy`, and `cargo test`.
