//! Control Flow Graph construction from decoded JVM bytecode.

use std::collections::{HashMap, HashSet};

use crate::instruction::Instruction;

/// A single basic block in the control flow graph.
#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub id: usize,
    pub start_pc: usize,
    pub end_pc: usize, // inclusive
    pub instructions: Vec<(usize, Instruction)>,
    pub successors: Vec<usize>,   // BasicBlock IDs
    pub predecessors: Vec<usize>, // BasicBlock IDs
}

/// The Control Flow Graph of a method.
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    pub blocks: HashMap<usize, BasicBlock>,
    pub entry_block: usize,
}

impl ControlFlowGraph {
    /// Builds a CFG from a sequence of decoded instructions.
    pub fn build(instructions: &[(usize, Instruction)]) -> Self {
        if instructions.is_empty() {
            return Self {
                blocks: HashMap::new(),
                entry_block: 0,
            };
        }

        let leaders = Self::find_leaders(instructions);
        let blocks = Self::build_blocks(instructions, &leaders);

        let mut cfg = Self {
            blocks,
            entry_block: instructions[0].0,
        };

        cfg.connect_edges();

        cfg
    }

    fn find_leaders(instructions: &[(usize, Instruction)]) -> HashSet<usize> {
        let mut leaders = HashSet::new();

        if !instructions.is_empty() {
            leaders.insert(instructions[0].0);
        }

        let mut next_is_leader = false;

        for (pc, instr) in instructions.iter() {
            if next_is_leader {
                leaders.insert(*pc);
                next_is_leader = false;
            }

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
                | Instruction::Ifnull(offset)
                | Instruction::Ifnonnull(offset) => {
                    leaders.insert((*pc as isize + *offset as isize) as usize);
                    next_is_leader = true;
                }
                Instruction::Goto(offset) | Instruction::Jsr(offset) => {
                    leaders.insert((*pc as isize + *offset as isize) as usize);
                    next_is_leader = true;
                }
                Instruction::GotoW(offset) | Instruction::JsrW(offset) => {
                    leaders.insert((*pc as isize + *offset as isize) as usize);
                    next_is_leader = true;
                }
                Instruction::Tableswitch {
                    default, offsets, ..
                } => {
                    leaders.insert((*pc as isize + *default as isize) as usize);
                    for offset in offsets {
                        leaders.insert((*pc as isize + *offset as isize) as usize);
                    }
                    next_is_leader = true;
                }
                Instruction::Lookupswitch { default, pairs } => {
                    leaders.insert((*pc as isize + *default as isize) as usize);
                    for (_, offset) in pairs {
                        leaders.insert((*pc as isize + *offset as isize) as usize);
                    }
                    next_is_leader = true;
                }
                Instruction::Ireturn
                | Instruction::Lreturn
                | Instruction::Freturn
                | Instruction::Dreturn
                | Instruction::Areturn
                | Instruction::Return
                | Instruction::Athrow
                | Instruction::Ret(_)
                | Instruction::RetW(_) => {
                    next_is_leader = true;
                }
                _ => {}
            }
        }

        leaders
    }

    fn build_blocks(
        instructions: &[(usize, Instruction)],
        leaders: &HashSet<usize>,
    ) -> HashMap<usize, BasicBlock> {
        let mut blocks = HashMap::new();

        if instructions.is_empty() {
            return blocks;
        }

        let mut current_block: Option<BasicBlock> = None;

        for (pc, instr) in instructions {
            if leaders.contains(pc) {
                if let Some(block) = current_block.take() {
                    blocks.insert(block.id, block);
                }
                current_block = Some(BasicBlock {
                    id: *pc,
                    start_pc: *pc,
                    end_pc: *pc,
                    instructions: Vec::new(),
                    successors: Vec::new(),
                    predecessors: Vec::new(),
                });
            }

            if let Some(ref mut block) = current_block {
                block.instructions.push((*pc, instr.clone()));
                block.end_pc = *pc;
            }
        }

        if let Some(block) = current_block {
            blocks.insert(block.id, block);
        }

        blocks
    }

    fn connect_edges(&mut self) {
        let mut edges = Vec::new(); // (from, to)
        let block_ids: Vec<usize> = self.blocks.keys().copied().collect();

        for block_id in &block_ids {
            let block = self.blocks.get(block_id).unwrap();
            let (last_pc, last_instr) = block.instructions.last().unwrap();

            let mut falls_through = true;
            let mut branch_targets = Vec::new();

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
                    branch_targets.push((*last_pc as isize + *offset as isize) as usize);
                }
                Instruction::Goto(offset) | Instruction::Jsr(offset) => {
                    branch_targets.push((*last_pc as isize + *offset as isize) as usize);
                    falls_through = false;
                }
                Instruction::GotoW(offset) | Instruction::JsrW(offset) => {
                    branch_targets.push((*last_pc as isize + *offset as isize) as usize);
                    falls_through = false;
                }
                Instruction::Tableswitch {
                    default, offsets, ..
                } => {
                    branch_targets.push((*last_pc as isize + *default as isize) as usize);
                    for offset in offsets {
                        branch_targets.push((*last_pc as isize + *offset as isize) as usize);
                    }
                    falls_through = false;
                }
                Instruction::Lookupswitch { default, pairs } => {
                    branch_targets.push((*last_pc as isize + *default as isize) as usize);
                    for (_, offset) in pairs {
                        branch_targets.push((*last_pc as isize + *offset as isize) as usize);
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
                | Instruction::Ret(_)
                | Instruction::RetW(_) => {
                    falls_through = false;
                }
                _ => {}
            }

            for target in branch_targets {
                if self.blocks.contains_key(&target) {
                    edges.push((*block_id, target));
                }
            }

            if falls_through {
                // Find next block sequentially
                let mut next_start_pc = usize::MAX;
                for &other_id in &block_ids {
                    if other_id > *block_id && other_id < next_start_pc {
                        next_start_pc = other_id;
                    }
                }
                if next_start_pc != usize::MAX {
                    edges.push((*block_id, next_start_pc));
                }
            }
        }

        // Apply edges
        for (from, to) in edges {
            self.blocks.get_mut(&from).unwrap().successors.push(to);
            self.blocks.get_mut(&to).unwrap().predecessors.push(from);
        }

        // Deduplicate
        for block in self.blocks.values_mut() {
            block.successors.sort_unstable();
            block.successors.dedup();
            block.predecessors.sort_unstable();
            block.predecessors.dedup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfg_basic() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Istore0),
            (2, Instruction::Iload0),
            (3, Instruction::Iconst5),
            (4, Instruction::IfIcmpge(9)), // Branches to 13
            (7, Instruction::Iload0),
            (8, Instruction::Iconst1),
            (9, Instruction::Iadd),
            (10, Instruction::Istore0),
            (11, Instruction::Goto(-9)), // Branches to 2
            (13, Instruction::Return),
        ];

        let cfg = ControlFlowGraph::build(&instructions);

        assert_eq!(cfg.entry_block, 0);
        assert_eq!(cfg.blocks.len(), 4);

        let block_0 = cfg.blocks.get(&0).unwrap();
        assert_eq!(block_0.start_pc, 0);
        assert_eq!(block_0.end_pc, 1);
        assert_eq!(block_0.successors, vec![2]);

        let block_2 = cfg.blocks.get(&2).unwrap();
        assert_eq!(block_2.start_pc, 2);
        assert_eq!(block_2.end_pc, 4);
        assert_eq!(block_2.successors, vec![7, 13]);

        let block_7 = cfg.blocks.get(&7).unwrap();
        assert_eq!(block_7.start_pc, 7);
        assert_eq!(block_7.end_pc, 11);
        assert_eq!(block_7.successors, vec![2]);

        let block_13 = cfg.blocks.get(&13).unwrap();
        assert_eq!(block_13.start_pc, 13);
        assert_eq!(block_13.end_pc, 13);
        assert!(block_13.successors.is_empty());
    }
}
