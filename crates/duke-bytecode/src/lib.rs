//! `duke-bytecode` — JVM bytecode definitions, decoder, and structural verifier.
//!
//! # Modules
//!
//! - [`opcodes`] — All JVM opcode byte constants (JVM SE 21 §6.5)
//! - [`instruction`] — Typed [`Instruction`] enum with decoded operands
//! - [`decoder`] — Decode raw `Code` bytes → `Vec<(pc, Instruction)>`
//! - [`verifier`] — Structural pass: stack bounds, local bounds, empty stack on return
//! - [`error`] — [`DecodeError`] and [`VerifyError`]

#[cfg(feature = "nova")]
pub mod basic_block;
pub mod call_graph;
pub mod cfg;
pub mod decoder;
pub mod error;
pub mod instruction;
pub mod opcodes;
pub mod verifier;

#[cfg(feature = "nova")]
pub use basic_block::{BasicBlock, build_basic_blocks};
pub use call_graph::generate_mermaid_call_graph;
pub use cfg::{cyclomatic_complexity, generate_mermaid_cfg};
pub use decoder::decode;
pub use error::{DecodeError, DecodeResult, Error, Result, VerifyError, VerifyResult};
pub use instruction::Instruction;
pub use verifier::verify;

#[cfg(test)]
mod fuzz;
#[cfg(test)]
mod tests;
