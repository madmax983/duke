1. Modify `crates/duke-interpreter/src/lib.rs`
   - Remove wildcard exports `pub use context::*;` and `pub use registry::*;`
   - Replace them with explicit exports for `MethodEntry`, `FieldEntry`, `ExceptionEntry`, `ClassLoadSource`, `ClassContext` from `context` and `LambdaInfo`, `NativeThreadAction`, `NativeStackFrame`, `NativeControl`, `ReflectedMethodInfo`, `ReflectedFieldInfo`, `ReflectedAnnotation`, `ReflectedAnnotationElement`, `ReflectedAnnotationValue`, `ReflectedAnnotationConst`, `ReflectedClassInfo`, `ClassRegistry`, `NativeHandler`, `CallbackOps`, `CallbackNativeHandler`, `HandlerKind`, `NativeRegistry` from `registry`.

2. Modify `crates/duke-telemetry/src/lib.rs`
   - Remove wildcard exports `pub use bytecode_cost::*;`, `pub use object_lineage::*;`, `pub use class_init_dag::*;`, `pub use exception_flow::*;`, `pub use dispatch_resolution::*;`, `pub use native_boundary::*;`
   - Replace them with explicit exports for `OpcodeStat`, `BytecodeCostStore` from `bytecode_cost`; `AllocationSite`, `ObjectLineageStore` from `object_lineage`; `ClinitEvent`, `ClassInitDagStore` from `class_init_dag`; `ExceptionEvent`, `ExceptionFlowStore` from `exception_flow`; `DispatchStat`, `DispatchResolutionStore` from `dispatch_resolution`; `NativeStat`, `NativeBoundaryStore` from `native_boundary`.

3. Ensure no regressions
   - Run tests for all crates `cargo test` and `cargo clippy --all-targets --all-features -- -D warnings`.

4. Journal Learning
   - Add journal entry to `.jules/atlas.md` about wildcard exports causing leaky abstractions and breaking API boundaries.

5. Pre-commit
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

6. Submit
   - Commit and submit.
