1. **Explore the Workspace:** Check `crates/` to see which ones have `pub mod` exposing internal details.
2. **Apply the Facade Pattern in `duke-bytecode` and `duke-telemetry`:** Change `pub mod` to `pub(crate) mod` to encapsulate internal modules like `reachability.rs` and `helpers.rs`.
3. **Apply the Facade Pattern in `duke-classfile`:** Make `pub mod types` private (`pub(crate) mod types`). Fix downstream crates (`duke-bytecode`, `duke-interpreter`, `duke`) to import `types` elements directly from `duke_classfile::` instead of `duke_classfile::types::`. Ensure `duke-classfile` re-exports everything required by downstreams.
4. **Update `.jules/atlas.md`:** Add an entry for the structural fix applied to `duke-classfile` and other crates.
5. **Pre-commit Steps:** Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done (e.g. `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo fmt --all`).
6. **Submit PR:** Submit the change with title "🗺️ Atlas: [architectural change]" and appropriate description.
