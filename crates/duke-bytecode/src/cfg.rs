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
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
#[must_use]
pub fn generate_mermaid_cfg(instructions: &[(usize, Instruction)]) -> String {
    let mut cfg = String::from("graph TD\n");

    for (i, (pc, instr)) in instructions.iter().enumerate() {
        let mnemonic = instr.mnemonic();
        // Add node
        let _ = writeln!(cfg, "    node{pc}[\"{pc}: {mnemonic}\"]");

        // Add edges
        match instr {
            // Absolute fall-through stops
            Instruction::Return
            | Instruction::Ireturn
            | Instruction::Lreturn
            | Instruction::Freturn
            | Instruction::Dreturn
            | Instruction::Areturn
            | Instruction::Athrow
            | Instruction::Ret(_)
            | Instruction::RetW(_) => {
                // No fall-through
            }
            Instruction::Goto(offset) => {
                let target = (*pc as isize + isize::from(*offset)) as usize;
                let _ = writeln!(cfg, "    node{pc} --> node{target}");
            }
            Instruction::GotoW(offset) => {
                let target = (*pc as isize + *offset as isize) as usize;
                let _ = writeln!(cfg, "    node{pc} --> node{target}");
            }
            Instruction::Tableswitch {
                default, offsets, ..
            } => {
                let default_target = (*pc as isize + *default as isize) as usize;
                let _ = writeln!(cfg, "    node{pc} -->|default| node{default_target}");
                for (idx, offset) in offsets.iter().enumerate() {
                    let target = (*pc as isize + *offset as isize) as usize;
                    let _ = writeln!(cfg, "    node{pc} -->|{idx}| node{target}");
                }
            }
            Instruction::Lookupswitch { default, pairs } => {
                let default_target = (*pc as isize + *default as isize) as usize;
                let _ = writeln!(cfg, "    node{pc} -->|default| node{default_target}");
                for (match_val, offset) in pairs {
                    let target = (*pc as isize + *offset as isize) as usize;
                    let _ = writeln!(cfg, "    node{pc} -->|{match_val}| node{target}");
                }
            }
            // Conditional branches
            Instruction::Ifeq(offset)
            | Instruction::Ifne(offset)
            | Instruction::Iflt(offset)
            | Instruction::Ifge(offset)
            | Instruction::Ifgt(offset)
            | Instruction::Ifle(offset)
            | Instruction::IfIcmpeq(offset)
            | Instruction::IfIcmpne(offset)
            | Instruction::IfIcmplt(offset)
            | Instruction::IfIcmpge(offset)
            | Instruction::IfIcmpgt(offset)
            | Instruction::IfIcmple(offset)
            | Instruction::IfAcmpeq(offset)
            | Instruction::IfAcmpne(offset)
            | Instruction::Ifnull(offset)
            | Instruction::Ifnonnull(offset)
            | Instruction::Jsr(offset) => {
                let target = (*pc as isize + isize::from(*offset)) as usize;
                let _ = writeln!(cfg, "    node{pc} -->|true| node{target}");
                if i + 1 < instructions.len() {
                    let next_pc = instructions[i + 1].0;
                    let _ = writeln!(cfg, "    node{pc} -->|false| node{next_pc}");
                }
            }
            Instruction::JsrW(offset) => {
                let target = (*pc as isize + *offset as isize) as usize;
                let _ = writeln!(cfg, "    node{pc} -->|true| node{target}");
                if i + 1 < instructions.len() {
                    let next_pc = instructions[i + 1].0;
                    let _ = writeln!(cfg, "    node{pc} -->|false| node{next_pc}");
                }
            }
            // Everything else falls through
            _ => {
                if i + 1 < instructions.len() {
                    let next_pc = instructions[i + 1].0;
                    let _ = writeln!(cfg, "    node{pc} --> node{next_pc}");
                }
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
        assert!(cfg.contains("node0 -->|0| node4"));
        assert!(cfg.contains("node0 -->|1| node6"));
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
/// use duke_bytecode::{Instruction, cfg::cyclomatic_complexity};
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
pub fn cyclomatic_complexity(instructions: &[(usize, Instruction)]) -> usize {
    let mut complexity = 1;

    for (_, instr) in instructions {
        match instr {
            // Conditional branches
            Instruction::Ifeq(_)
            | Instruction::Ifne(_)
            | Instruction::Iflt(_)
            | Instruction::Ifge(_)
            | Instruction::Ifgt(_)
            | Instruction::Ifle(_)
            | Instruction::IfIcmpeq(_)
            | Instruction::IfIcmpne(_)
            | Instruction::IfIcmplt(_)
            | Instruction::IfIcmpge(_)
            | Instruction::IfIcmpgt(_)
            | Instruction::IfIcmple(_)
            | Instruction::IfAcmpeq(_)
            | Instruction::IfAcmpne(_)
            | Instruction::Ifnull(_)
            | Instruction::Ifnonnull(_)
            | Instruction::Jsr(_)
            | Instruction::JsrW(_) => {
                complexity += 1;
            }
            // Switch statements
            Instruction::Tableswitch { offsets, .. } => {
                // Number of possible paths = offsets.len() + 1 (for default).
                // Subtract 1 because we start at 1 complexity inherently.
                complexity += offsets.len();
            }
            Instruction::Lookupswitch { pairs, .. } => {
                // Number of possible paths = pairs.len() + 1 (for default).
                complexity += pairs.len();
            }
            _ => {}
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
