//! Switch-dispatch JVM bytecode interpreter for Duke Phase 4.
//!
//! Executes decoded instruction streams for methods containing integer, long,
//! float, and double arithmetic, control flow, and local variables. Heap
//! allocation, field access, and method invocation are implemented in the `execute` module.

pub mod context;
pub mod execute;
pub mod natives;
pub mod registry;

pub use context::*;
#[allow(unused_imports)]
pub use execute::*;
#[allow(ambiguous_glob_reexports)]
pub use natives::*;
#[allow(unused_imports)]
pub use execute::*;
pub use registry::*;

#[cfg(test)]
mod tests;
