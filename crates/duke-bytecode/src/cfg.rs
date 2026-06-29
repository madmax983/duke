//! Control Flow Graph Generation.
//!
//! This module provides utilities to convert decoded JVM instructions into a visual
//! representation using [Mermaid JS](https://mermaid.js.org/). It models branches, gotos,
//! returns, and switch statements to create a comprehensible Control Flow Graph (CFG)
//! of method bytecode logic.

use std::fmt::Write;

use crate::Instruction;

/// Generates a Mermaid control flow graph (CFG) from a list of decoded instructions.
///
/// This is used for visualising the structure of a Java method. It outputs
/// Mermaid.js compatible syntax (using `graph TD`). Each instruction becomes a node,
/// and edges represent the control flow between them (e.g. conditional branches, gotos, returns).
///
/// # Examples
///
/// ```
/// use duke_bytecode::{Instruction, generate_mermaid_cfg};
///
/// let instructions = vec![
///     (0, Instruction::Iconst0),
///     (1, Instruction::Ifeq(5)), // jump to PC=6
///     (4, Instruction::Iconst1),
///     (5, Instruction::Ireturn),
///     (6, Instruction::Iconst2),
///     (7, Instruction::Ireturn),
/// ];
/// let cfg = generate_mermaid_cfg(&instructions);
///
/// assert!(cfg.contains("graph TD"));
/// assert!(cfg.contains("node0[\"0: iconst_0\"]"));
/// assert!(cfg.contains("node0 --> node1"));
/// assert!(cfg.contains("node1 -->|true| node6"));
/// ```
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
#[must_use]
#[allow(missing_docs)]
pub fn generate_mermaid_cfg(instructions: &[(usize, Instruction)]) -> String {
    let mut cfg = String::from("graph TD\n");

    for (i, (pc, instr)) in instructions.iter().enumerate() {
        let mnemonic = instr.mnemonic();
        // Add node
        let _ = writeln!(cfg, "    node{pc}[\"{pc}: {mnemonic}\"]");

        // Add edges
        let next_pc = instructions.get(i + 1).map(|(p, _)| *p);
        for (target, label) in instr.control_flow_edges(*pc, next_pc) {
            if let Some(label) = label {
                let _ = writeln!(cfg, "    node{pc} -->|{label}| node{target}");
            } else {
                let _ = writeln!(cfg, "    node{pc} --> node{target}");
            }
        }
    }

    cfg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Instruction;

    #[test]
    fn test_cfg_generation() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Ifeq(5)),
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst2),
            (7, Instruction::Ireturn),
        ];

        let cfg = generate_mermaid_cfg(&instructions);

        assert!(cfg.contains("graph TD"));
        assert!(cfg.contains("node0[\"0: iconst_0\"]"));
        assert!(cfg.contains("node1[\"1: ifeq\"]"));
        assert!(cfg.contains("node4[\"4: iconst_1\"]"));
        assert!(cfg.contains("node5[\"5: ireturn\"]"));
        assert!(cfg.contains("node6[\"6: iconst_2\"]"));
        assert!(cfg.contains("node7[\"7: ireturn\"]"));

        assert!(cfg.contains("node0 --> node1"));
        assert!(cfg.contains("node1 -->|true| node6"));
        assert!(cfg.contains("node1 -->|false| node4"));
        assert!(cfg.contains("node4 --> node5"));
        assert!(cfg.contains("node6 --> node7"));
    }

    #[test]
    fn test_cfg_goto() {
        let instructions = vec![
            (0, Instruction::Goto(5)),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ];
        let cfg = generate_mermaid_cfg(&instructions);
        assert!(cfg.contains("node0 --> node5"));
    }

    #[test]
    fn test_cfg_gotow() {
        let instructions = vec![(0, Instruction::GotoW(5)), (5, Instruction::Ireturn)];
        let cfg = generate_mermaid_cfg(&instructions);
        assert!(cfg.contains("node0 --> node5"));
    }

    #[test]
    fn test_cfg_conditional_branches() {
        let instructions_to_test = vec![
            Instruction::Ifne(5),
            Instruction::Iflt(5),
            Instruction::Ifge(5),
            Instruction::Ifgt(5),
            Instruction::Ifle(5),
            Instruction::IfIcmpeq(5),
            Instruction::IfIcmpne(5),
            Instruction::IfIcmplt(5),
            Instruction::IfIcmpge(5),
            Instruction::IfIcmpgt(5),
            Instruction::IfIcmple(5),
            Instruction::IfAcmpeq(5),
            Instruction::IfAcmpne(5),
            Instruction::Ifnull(5),
            Instruction::Ifnonnull(5),
        ];

        for branch_instr in instructions_to_test {
            let instructions = vec![
                (0, branch_instr.clone()),
                (4, Instruction::Iconst1),
                (5, Instruction::Ireturn),
            ];
            let cfg = generate_mermaid_cfg(&instructions);
            assert!(
                cfg.contains("node0 -->|true| node5"),
                "Failed for instruction: {branch_instr:?}",
            );
            assert!(
                cfg.contains("node0 -->|false| node4"),
                "Failed for instruction: {branch_instr:?}",
            );
        }
    }

    #[test]
    fn test_cfg_tableswitch() {
        let instructions = vec![
            (
                0,
                Instruction::Tableswitch {
                    default: 10,
                    low: 1,
                    high: 2,
                    offsets: vec![4, 6],
                },
            ),
            (4, Instruction::Ireturn),
            (6, Instruction::Ireturn),
            (10, Instruction::Ireturn),
        ];
        let cfg = generate_mermaid_cfg(&instructions);
        assert!(cfg.contains("node0 -->|default| node10"));
        assert!(cfg.contains("node0 -->|1| node4"));
        assert!(cfg.contains("node0 -->|2| node6"));
    }

    #[test]
    fn test_cfg_lookupswitch() {
        let instructions = vec![
            (
                0,
                Instruction::Lookupswitch {
                    default: 10,
                    pairs: vec![(5, 4), (10, 6)],
                },
            ),
            (4, Instruction::Ireturn),
            (6, Instruction::Ireturn),
            (10, Instruction::Ireturn),
        ];
        let cfg = generate_mermaid_cfg(&instructions);
        assert!(cfg.contains("node0 -->|default| node10"));
        assert!(cfg.contains("node0 -->|5| node4"));
        assert!(cfg.contains("node0 -->|10| node6"));
    }

    #[test]
    fn test_cfg_jsr() {
        let instructions = vec![
            (0, Instruction::Jsr(5)),
            (3, Instruction::Ireturn),
            (5, Instruction::Astore1),
            (6, Instruction::Ret(1)),
        ];
        let cfg = generate_mermaid_cfg(&instructions);
        assert!(cfg.contains("node0 -->|true| node5"));
        assert!(cfg.contains("node0 -->|false| node3"));
    }

    #[test]
    fn test_cfg_jsrw() {
        let instructions = vec![
            (0, Instruction::JsrW(5)),
            (5, Instruction::Astore1),
            (6, Instruction::RetW(1)),
        ];
        let cfg = generate_mermaid_cfg(&instructions);
        assert!(cfg.contains("node0 -->|true| node5"));
        assert!(cfg.contains("node0 -->|false| node5"));
    }
}

/// Computes the `McCabe` Cyclomatic Complexity of a sequence of JVM instructions.
///
/// Cyclomatic complexity (`v(G)`) measures the number of linearly independent paths
/// through a program's source code. For JVM bytecode, this is equivalent to:
/// `v(G) = E - N + 2`, where `E` is the number of edges and `N` is the number of nodes.
///
/// A simplified, common way to calculate this iteratively is:
/// `v(G) = 1 + number_of_decision_points`.
///
/// Each conditional branch (`ifeq`, `if_icmpeq`, etc.) adds 1.
/// Each `tableswitch` or `lookupswitch` branch (excluding the default) adds 1.
/// `goto` statements do not add to complexity, as they don't branch.
///
/// # Examples
///
/// ```
/// use duke_bytecode::{Instruction, cyclomatic_complexity};
///
/// let instructions = vec![
///     (0, Instruction::Iconst0),
///     (1, Instruction::Ifeq(5)), // +1 decision
///     (4, Instruction::Iconst1),
///     (5, Instruction::Ireturn),
/// ];
///
/// assert_eq!(cyclomatic_complexity(&instructions), 2);
/// ```
#[must_use]
#[allow(missing_docs)]
pub fn cyclomatic_complexity(instructions: &[(usize, Instruction)]) -> usize {
    let mut complexity = 1;

    for (_, instr) in instructions {
        if instr.is_conditional_branch() || instr.is_subroutine_call() {
            complexity += 1;
        } else if let Some((_, pairs)) = instr.switch_targets() {
            complexity += pairs.len();
        }
    }

    complexity
}

#[cfg(test)]
mod complexity_tests {
    use super::*;
    use crate::Instruction;

    #[test]
    fn test_cyclomatic_complexity_linear() {
        let instructions = vec![(0, Instruction::Iconst0), (1, Instruction::Ireturn)];
        assert_eq!(cyclomatic_complexity(&instructions), 1);
    }

    #[test]
    fn test_cyclomatic_complexity_branches() {
        let instructions = vec![
            (0, Instruction::Ifeq(5)),
            (3, Instruction::Iconst1),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst2),
            (6, Instruction::Ireturn),
        ];
        assert_eq!(cyclomatic_complexity(&instructions), 2);
    }

    #[test]
    fn test_cyclomatic_complexity_conditional_branches() {
        let instructions_to_test = vec![
            Instruction::Ifne(5),
            Instruction::Iflt(5),
            Instruction::Ifge(5),
            Instruction::Ifgt(5),
            Instruction::Ifle(5),
            Instruction::IfIcmpeq(5),
            Instruction::IfIcmpne(5),
            Instruction::IfIcmplt(5),
            Instruction::IfIcmpge(5),
            Instruction::IfIcmpgt(5),
            Instruction::IfIcmple(5),
            Instruction::IfAcmpeq(5),
            Instruction::IfAcmpne(5),
            Instruction::Ifnull(5),
            Instruction::Ifnonnull(5),
        ];

        for branch_instr in instructions_to_test {
            let instructions = vec![
                (0, branch_instr.clone()),
                (4, Instruction::Iconst1),
                (5, Instruction::Ireturn),
            ];
            assert_eq!(
                cyclomatic_complexity(&instructions),
                2,
                "Failed for instruction: {branch_instr:?}",
            );
        }
    }

    #[test]
    fn test_cyclomatic_complexity_tableswitch() {
        let instructions = vec![(
            0,
            Instruction::Tableswitch {
                default: 10,
                low: 1,
                high: 2,
                offsets: vec![4, 6],
            },
        )];
        assert_eq!(cyclomatic_complexity(&instructions), 3); // 1 + 2 offsets
    }

    #[test]
    fn test_cyclomatic_complexity_lookupswitch() {
        let instructions = vec![(
            0,
            Instruction::Lookupswitch {
                default: 10,
                pairs: vec![(5, 4), (10, 6)],
            },
        )];
        assert_eq!(cyclomatic_complexity(&instructions), 3); // 1 + 2 pairs
    }
}

/// Generates a Mermaid control flow graph (CFG) from a list of basic blocks.
///
/// This is used to visualise the structure of a Java method in terms of basic blocks.
/// It outputs Mermaid.js compatible syntax (using `graph TD`). Each basic block becomes a node,
/// and edges represent the control flow between them (e.g. conditional branches, gotos, returns).
///
/// # Examples
///
/// ```
/// use duke_bytecode::{Instruction, BasicBlock, generate_basic_block_cfg};
///
/// let blocks = vec![
///     BasicBlock {
///         start_pc: 0,
///         end_pc: 2,
///         instructions: vec![
///             (0, Instruction::Iconst1),
///             (1, Instruction::Istore1),
///         ],
///     }
/// ];
///
/// let cfg = generate_basic_block_cfg(&blocks);
/// assert!(cfg.contains("graph TD"));
/// assert!(cfg.contains("Block 0"));
/// ```
#[cfg(feature = "nova")]
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
#[must_use]
#[allow(missing_docs)]
pub fn generate_basic_block_cfg(blocks: &[crate::basic_block::BasicBlock]) -> String {
    use std::fmt::Write;
    let mut cfg = String::with_capacity(1024);
    cfg.push_str("graph TD\n");
    if blocks.is_empty() {
        return cfg;
    }

    // Map PC to Block ID (start_pc) for edge resolution
    let mut pc_to_block = std::collections::HashMap::new();
    for block in blocks {
        pc_to_block.insert(block.start_pc, block.start_pc);
    }

    for block in blocks {
        let block_id = block.start_pc;

        // Node definition
        let mut node_label = String::with_capacity(32 + block.instructions.len() * 16);
        let _ = writeln!(node_label, "Block {block_id}");
        for (pc, instr) in &block.instructions {
            let _ = writeln!(node_label, "{}: {}", pc, instr.mnemonic());
        }
        // Escape quotes
        let node_label = node_label.replace('"', "\\\"");
        let _ = writeln!(cfg, "    block{block_id}[\"{node_label}\"]");

        // Edge definition based on the last instruction
        if let Some((last_pc, last_instr)) = block.instructions.last() {
            let next_block_id = block.end_pc;
            let next_pc = if pc_to_block.contains_key(&next_block_id) {
                Some(next_block_id)
            } else {
                None
            };
            for (target, label) in last_instr.control_flow_edges(*last_pc, next_pc) {
                if let Some(label) = label {
                    let _ = writeln!(cfg, "    block{block_id} -->|{label}| block{target}");
                } else {
                    let _ = writeln!(cfg, "    block{block_id} --> block{target}");
                }
            }
        }
    }

    cfg
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod basic_block_cfg_tests {
    use super::*;
    use crate::Instruction;
    use crate::basic_block::build_basic_blocks;

    #[test]
    fn test_generate_basic_block_cfg() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Ifeq(5)),
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst2),
            (7, Instruction::Ireturn),
        ];

        let blocks = build_basic_blocks(&instructions);
        let cfg = generate_basic_block_cfg(&blocks);

        assert!(cfg.contains("graph TD"));
        // Don't assert the exact string literal because of escapes, just assert the block nodes exist
        assert!(cfg.contains("block0[\"Block 0"));
        assert!(cfg.contains("0: iconst_0"));
        assert!(cfg.contains("1: ifeq"));
        assert!(cfg.contains("block4[\"Block 4"));
        assert!(cfg.contains("4: iconst_1"));
        assert!(cfg.contains("block6[\"Block 6"));
        assert!(cfg.contains("6: iconst_2"));

        assert!(cfg.contains("block0 -->|true| block6"));
        assert!(cfg.contains("block0 -->|false| block4"));
    }

    #[test]
    fn test_generate_basic_block_cfg_empty() {
        let blocks = build_basic_blocks(&[]);
        let cfg = generate_basic_block_cfg(&blocks);
        assert!(cfg.contains("graph TD"));
        assert!(!cfg.contains("block0"));
    }

    #[test]
    fn test_generate_basic_block_cfg_switch() {
        let instructions = vec![(
            0,
            Instruction::Tableswitch {
                default: 10,
                low: 1,
                high: 2,
                offsets: vec![4, 6],
            },
        )];
        let blocks = build_basic_blocks(&instructions);
        let cfg = generate_basic_block_cfg(&blocks);
        assert!(cfg.contains("graph TD"));
        assert!(cfg.contains("block0 -->|default| block10"));
        assert!(cfg.contains("block0 -->|1| block4"));
        assert!(cfg.contains("block0 -->|2| block6"));
    }

    #[test]
    fn test_generate_basic_block_cfg_switch_negative() {
        let instructions = vec![(
            0,
            Instruction::Tableswitch {
                default: 10,
                low: -1,
                high: 0,
                offsets: vec![4, 6],
            },
        )];
        let blocks = build_basic_blocks(&instructions);
        let cfg = generate_basic_block_cfg(&blocks);
        assert!(cfg.contains("graph TD"));
        assert!(cfg.contains("block0 -->|default| block10"));
        assert!(cfg.contains("block0 -->|-1| block4"));
        assert!(cfg.contains("block0 -->|0| block6"));
    }

    #[test]
    fn test_generate_basic_block_cfg_tableswitch_negative() {
        let instructions = vec![(
            0,
            Instruction::Tableswitch {
                default: 10,
                low: -1,
                high: 0,
                offsets: vec![4, 6],
            },
        )];
        let blocks = build_basic_blocks(&instructions);
        let cfg = generate_basic_block_cfg(&blocks);
        assert!(cfg.contains("graph TD"));
        assert!(cfg.contains("block0 -->|default| block10"));
        assert!(cfg.contains("block0 -->|-1| block4"));
        assert!(cfg.contains("block0 -->|0| block6"));
    }

    #[test]
    fn test_generate_basic_block_cfg_lookupswitch() {
        let instructions = vec![(
            0,
            Instruction::Lookupswitch {
                default: 10,
                pairs: vec![(5, 4), (10, 6)],
            },
        )];
        let blocks = build_basic_blocks(&instructions);
        let cfg = generate_basic_block_cfg(&blocks);
        assert!(cfg.contains("graph TD"));
        assert!(cfg.contains("block0 -->|default| block10"));
        assert!(cfg.contains("block0 -->|5| block4"));
        assert!(cfg.contains("block0 -->|10| block6"));
    }

    #[test]
    fn test_generate_basic_block_cfg_goto() {
        let instructions = vec![
            (0, Instruction::Goto(4)),
            (3, Instruction::Iconst1), // Unreachable, but just to have next block
            (4, Instruction::Iconst2),
        ];
        let blocks = build_basic_blocks(&instructions);
        let cfg = generate_basic_block_cfg(&blocks);
        assert!(cfg.contains("graph TD"));
        assert!(cfg.contains("block0 --> block4"));
    }

    #[test]
    fn test_generate_basic_block_cfg_fallthrough() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Nop),
            (2, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);
        let cfg = generate_basic_block_cfg(&blocks);
        assert!(cfg.contains("graph TD"));
        // One big block
        assert!(cfg.contains("block0[\"Block 0"));
        assert!(cfg.contains("0: iconst_0"));
        assert!(cfg.contains("1: nop"));
        assert!(cfg.contains("2: ireturn"));
        assert!(!cfg.contains("-->"));
    }
}
