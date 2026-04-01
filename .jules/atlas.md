**[Title]
**Tangle:** [The Structural Mess]
**Blueprint:** [The Structural Fix]

**Refactoring the Duke Interpreter Blob**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file had grown into a massive "Blob" anti-pattern (over 23,000 lines). It mixed JVM data structures, the registry, execution engine logic, and an enormous amount of standard library bootstrapping/tests, making it hard to navigate and violating the single responsibility principle.
**Blueprint:** Extracted the core execution context data structures (`MethodEntry`, `FieldEntry`, `ExceptionEntry`, `ClassContext`) into a new `context` module, and the registry types (`ClassRegistry`, `NativeRegistry`, handler definitions) into a new `registry` module. These are now re-exported from `lib.rs` to maintain a clean public API while breaking up the physical file bloat.

**Extract Native Handlers**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file contained roughly 5,000 lines of `bootstrap_stdlib` logic and standard library native implementations. This created a massive "Blob" anti-pattern, violating separation of concerns by mixing execution engine core logic with Java stdlib native implementations.
**Blueprint:** Extracted `bootstrap_stdlib` and all native handler implementations (e.g. `native_println_string`, `native_file_init`, etc) into a dedicated `crates/duke-interpreter/src/natives.rs` module. Re-exported `bootstrap_stdlib` from `lib.rs` to maintain the public API while severely reducing file bloat.
