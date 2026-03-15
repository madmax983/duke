use std::collections::{BTreeMap, BTreeSet};

use crate::Instruction;

/// A basic block in the Control Flow Graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    pub start_pc: usize,
    pub end_pc: usize,
    pub instructions: Vec<(usize, Instruction)>,
    pub successors: BTreeSet<usize>,
}

/// Generates a Graphviz DOT representation of the Control Flow Graph for the given instructions.
///
/// # Panics
/// Panics if a PC plus an offset exceeds `usize::MAX` (which is impossible for valid class files),
/// or if an invalid offset converts to `usize` improperly.
#[must_use]
#[allow(clippy::too_many_lines, clippy::cast_sign_loss, clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub fn generate_dot(instructions: &[(usize, Instruction)]) -> String {
    use std::fmt::Write;

    if instructions.is_empty() {
        return "digraph CFG {\n}\n".to_string();
    }

    // 1. Find all basic block leaders (start PCs of basic blocks).
    let mut leaders = BTreeSet::new();
    leaders.insert(instructions[0].0); // The first instruction is always a leader.

    for (i, &(pc, ref instr)) in instructions.iter().enumerate() {
        // A branch instruction makes its targets leaders, and the instruction *after* it a leader.
        let mut is_branch = false;

        match instr {
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
            | Instruction::Goto(offset)
            | Instruction::Jsr(offset)
            | Instruction::Ifnull(offset)
            | Instruction::Ifnonnull(offset) => {
                let target_pc = usize::try_from(i64::try_from(pc).unwrap() + i64::from(*offset)).unwrap();
                leaders.insert(target_pc);
                is_branch = true;
            }
            Instruction::GotoW(offset) | Instruction::JsrW(offset) => {
                let target_pc = usize::try_from(i64::try_from(pc).unwrap() + i64::from(*offset)).unwrap();
                leaders.insert(target_pc);
                is_branch = true;
            }
            Instruction::Tableswitch {
                default,
                low: _,
                high: _,
                offsets,
            } => {
                leaders.insert(usize::try_from(i64::try_from(pc).unwrap() + i64::from(*default)).unwrap());
                for offset in offsets {
                    leaders.insert(usize::try_from(i64::try_from(pc).unwrap() + i64::from(*offset)).unwrap());
                }
                is_branch = true;
            }
            Instruction::Lookupswitch {
                default,
                pairs,
            } => {
                leaders.insert(usize::try_from(i64::try_from(pc).unwrap() + i64::from(*default)).unwrap());
                for (_, offset) in pairs {
                    leaders.insert(usize::try_from(i64::try_from(pc).unwrap() + i64::from(*offset)).unwrap());
                }
                is_branch = true;
            }
            Instruction::Ireturn
            | Instruction::Lreturn
            | Instruction::Freturn
            | Instruction::Dreturn
            | Instruction::Areturn
            | Instruction::Return
            | Instruction::Athrow
            | Instruction::Ret(_) => {
                is_branch = true; // These terminate a basic block.
            }
            _ => {}
        }

        if is_branch && i + 1 < instructions.len() {
            leaders.insert(instructions[i + 1].0);
        }
    }

    // 2. Construct basic blocks.
    let mut blocks: BTreeMap<usize, BasicBlock> = BTreeMap::new();
    let mut current_block_start = instructions[0].0;
    let mut current_block_instrs: Vec<(usize, Instruction)> = Vec::new();

    for &(pc, ref instr) in instructions {
        if leaders.contains(&pc) && pc != current_block_start {
            // Finish current block.
            blocks.insert(
                current_block_start,
                BasicBlock {
                    start_pc: current_block_start,
                    end_pc: current_block_instrs.last().unwrap().0,
                    instructions: std::mem::take(&mut current_block_instrs),
                    successors: BTreeSet::new(),
                },
            );
            current_block_start = pc;
        }
        current_block_instrs.push((pc, instr.clone()));
    }
    // Finish the last block.
    if !current_block_instrs.is_empty() {
        blocks.insert(
            current_block_start,
            BasicBlock {
                start_pc: current_block_start,
                end_pc: current_block_instrs.last().unwrap().0,
                instructions: current_block_instrs,
                successors: BTreeSet::new(),
            },
        );
    }

    // 3. Connect basic blocks (compute successors).
    let block_starts: Vec<usize> = blocks.keys().copied().collect();
    for (i, start_pc) in block_starts.iter().enumerate() {
        let block = blocks.get_mut(start_pc).unwrap();
        let last_instr = &block.instructions.last().unwrap().1;
        let last_pc = block.instructions.last().unwrap().0;

        let mut falls_through = true;
        match last_instr {
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
            | Instruction::Ifnonnull(offset) => {
                let target_pc = usize::try_from(i64::try_from(last_pc).unwrap() + i64::from(*offset)).unwrap();
                block.successors.insert(target_pc);
            }
            Instruction::Goto(offset) | Instruction::Jsr(offset) => {
                let target_pc = usize::try_from(i64::try_from(last_pc).unwrap() + i64::from(*offset)).unwrap();
                block.successors.insert(target_pc);
                falls_through = false;
            }
            Instruction::GotoW(offset) | Instruction::JsrW(offset) => {
                let target_pc = usize::try_from(i64::try_from(last_pc).unwrap() + i64::from(*offset)).unwrap();
                block.successors.insert(target_pc);
                falls_through = false;
            }
            Instruction::Tableswitch {
                default,
                offsets,
                ..
            } => {
                block
                    .successors
                    .insert(usize::try_from(i64::try_from(last_pc).unwrap() + i64::from(*default)).unwrap());
                for offset in offsets {
                    block
                        .successors
                        .insert(usize::try_from(i64::try_from(last_pc).unwrap() + i64::from(*offset)).unwrap());
                }
                falls_through = false;
            }
            Instruction::Lookupswitch {
                default,
                pairs,
                ..
            } => {
                block
                    .successors
                    .insert(usize::try_from(i64::try_from(last_pc).unwrap() + i64::from(*default)).unwrap());
                for (_, offset) in pairs {
                    block
                        .successors
                        .insert(usize::try_from(i64::try_from(last_pc).unwrap() + i64::from(*offset)).unwrap());
                }
                falls_through = false;
            }
            Instruction::Ireturn
            | Instruction::Lreturn
            | Instruction::Freturn
            | Instruction::Dreturn
            | Instruction::Areturn
            | Instruction::Return
            | Instruction::Athrow
            | Instruction::Ret(_) => {
                falls_through = false;
            }
            _ => {}
        }

        if falls_through && i + 1 < block_starts.len() {
            block.successors.insert(block_starts[i + 1]);
        }
    }

    // 4. Generate DOT output.
    let mut dot = String::new();
    dot.push_str("digraph CFG {\n");
    dot.push_str("    node [shape=box, fontname=\"Courier\"];\n");

    for block in blocks.values() {
        let mut label = String::new();
        let _ = writeln!(label, "Block {}", block.start_pc);
        for (pc, instr) in &block.instructions {
            let _ = write!(label, "{pc}: {instr:?}\\l");
        }
        let _ = writeln!(dot, "    block{} [label=\"{label}\"];", block.start_pc);

        for succ in &block.successors {
            let _ = writeln!(dot, "    block{} -> block{succ};", block.start_pc);
        }
    }

    dot.push_str("}\n");
    dot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_dot_empty() {
        let dot = generate_dot(&[]);
        assert_eq!(dot, "digraph CFG {\n}\n");
    }

    #[test]
    fn test_generate_dot_simple() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Istore1),
            (2, Instruction::Return),
        ];
        let dot = generate_dot(&instructions);
        assert!(dot.contains("digraph CFG"));
        assert!(dot.contains("block0 [label=\"Block 0\n0: Iconst0\\l1: Istore1\\l2: Return\\l\"];"));
        assert!(!dot.contains("->")); // No edges
    }

    #[test]
    fn test_generate_dot_branch() {
        let instructions = vec![
            (0, Instruction::Iload1),
            (1, Instruction::Ifeq(5)), // jump to 6
            (4, Instruction::Iconst1),
            (5, Instruction::Return),
            (6, Instruction::Iconst0),
            (7, Instruction::Return),
        ];
        let dot = generate_dot(&instructions);
        assert!(dot.contains("block0 -> block4")); // fallthrough
        assert!(dot.contains("block0 -> block6")); // branch target
        assert!(dot.contains("block4 [label=\"Block 4"));
        assert!(dot.contains("block6 [label=\"Block 6"));
        // block4 doesn't fall through to block6 because of Return
        assert!(!dot.contains("block4 -> block6"));
    }
}
