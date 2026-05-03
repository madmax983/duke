**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Title]** 2025-05-02 - Splitting the Monolithic native.rs Module
**Tangle:** `native.rs` had grown to an excessive 37,000 lines ("The Bloat"), acting as a god module containing core JVM execution state (like `ExecutionState`, `CachedDispatch`), reflection context building, and over 1,200 native standard library handlers ("The Sprawl"). It was dumped into the root namespace via `include!("native.rs");`.
**Blueprint:** To preserve high cohesion without tangling module visibility and triggering 500+ private field access errors, `native.rs` was logically split into three cleanly separated files (`native_handlers_a.rs`, `native_engine.rs`, and `native_handlers_b.rs`), breaking apart the massive file while remaining within the same root namespace layer. This immediately solved the physical file bloat and separated the domains of JVM internals and JDK natives.
