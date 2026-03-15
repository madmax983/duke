**[Extracting 'The Blob' lib.rs]**
**Tangle:** The `duke-interpreter` crate had a massive 15k+ lines `lib.rs` file containing everything from basic data structures to the core execution loop and registries, demonstrating "The Blob" and "The Sprawl" architectural smells.
**Blueprint:** Extracted basic state representations into `context.rs` (`ClassContext`, `MethodEntry`, etc.) and registry components into `native_registry.rs`. Kept `lib.rs` as the facade providing `pub use` to maintain backward compatibility without breaking existing module structures.
