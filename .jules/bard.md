## 2024-05-24 - The Missing ClassLoader Guides
**Confusion:** The `ClassLoader` implementations (`BootstrapLoader`, `DirectoryLoader`, and `JImageReader`) had no executable examples explaining how to instantiate them. Users were left guessing how `java/lang/Object` translates to a path under the hood.
**Clarification:** Added executable `/// # Examples` doc-tests for `new` and `open` methods on each struct demonstrating correct usage and error handling.
## 2024-05-24 - The Missing Host OS API Examples
**Confusion:** The `duke-gc` crate's host file and networking API methods (like `open_host_zip`, `bind_server_socket`, etc.) lacked executable examples, leaving users to guess the correct usage of file handles (`id`s) and error types.
**Clarification:** Added executable `/// # Examples` doc-tests for all host OS API methods and `dump_mermaid`. Also ensured module-level `//!` documentation was present for all fuzzing, benchmarking, and integration test files to avoid warnings and ensure consistent generated documentation.
