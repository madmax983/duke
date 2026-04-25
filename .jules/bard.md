## 2024-04-22 - [LambdaInfo Documentation]
**Confusion:** The purpose of `LambdaInfo` and its fields were completely undocumented, making it difficult to understand how dynamic lambda generation works.
**Clarification:** Added module-level documentation and executable doctests explaining how `LambdaInfo` bridges the gap between functional interfaces and implementation methods during `invokedynamic` instructions.
## 2024-05-18 - [Reachability Documentation]
**Confusion:** The `duke-bytecode/src/reachability.rs` module lacked module-level documentation and executable doctests for its functions (`get_successors`, `find_dead_blocks`, `find_shortest_path`). This caused confusion around how to use the functions and what their purpose was in analyzing JVM basic block connectivity, triggering a documentation error.
**Clarification:** Added comprehensive module-level documentation (`//!`) explaining the context of the module, and updated the three core functions with `/// # Examples` containing executable doctests, explicitly linking to related concepts and providing copy-pasteable context.
