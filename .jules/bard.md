## 2024-04-22 - [LambdaInfo Documentation]
**Confusion:** The purpose of `LambdaInfo` and its fields were completely undocumented, making it difficult to understand how dynamic lambda generation works.
**Clarification:** Added module-level documentation and executable doctests explaining how `LambdaInfo` bridges the gap between functional interfaces and implementation methods during `invokedynamic` instructions.
## 2024-04-25 - [reachability.rs Documentation]
**Confusion:** The `duke-bytecode::reachability` module was entirely missing module-level documentation (`//!`) and functional executable examples (doc tests) for `get_successors`, `find_dead_blocks`, and `find_shortest_path`, making it difficult for downstream consumers to understand how to correctly perform CFG and path-based analysis.
**Clarification:** Added module-level documentation and executable doc tests using `BasicBlock` initialization and `Instruction` variants to clearly demonstrate how control flow transitions and analysis work.
