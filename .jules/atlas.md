# Atlas Journal - Critical Architectural Decisions

**[Facade over stdlib]**
**Tangle:** `duke-interpreter` leaked standard library registration internals to the launcher.
**Blueprint:** Encapsulated in `stdlib.rs`, exported single `bootstrap_stdlib` facade.

**[Extract 20k-line Test Module from Interpreter]**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file had grown into a massive "Blob" anti-pattern, encompassing over 34,000 lines of code. This was primarily driven by an inline `#[cfg(test)] mod tests { ... }` block that contained over 20,000 lines of unit and integration tests. This extreme file size impaired navigation, code comprehension, and general maintainability, violating high cohesion boundaries.
**Blueprint:** Extracted the entire inline test module into a dedicated `crates/duke-interpreter/src/tests.rs` file, leaving behind a clean `#[cfg(test)] mod tests;` declaration in `lib.rs`. This correctly isolated test boundaries, significantly reducing the bloat of the core interpreter module without altering internal behavior.
