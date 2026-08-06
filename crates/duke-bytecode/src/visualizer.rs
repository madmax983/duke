//! Annotated CFG Visualizer.

#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use crate::{find_dead_blocks, find_shortest_path, generate_basic_block_cfg};
use std::fmt::Write;

/// Generates a Mermaid control flow graph with annotations for shortest path and dead blocks.
#[cfg(feature = "nova")]
#[must_use]
pub fn generate_annotated_cfg(blocks: &[BasicBlock], start_pc: usize, target_pc: usize) -> String {
    let mut cfg = generate_basic_block_cfg(blocks);
    if cfg.is_empty() || !cfg.contains("graph TD") {
        return cfg;
    }

    cfg.push_str("\n    classDef dead fill:#ff9999,stroke:#333,stroke-width:2px;\n");
    cfg.push_str("    classDef path fill:#99ff99,stroke:#333,stroke-width:4px;\n\n");

    let dead_blocks = find_dead_blocks(blocks, start_pc);
    for dead_pc in dead_blocks {
        let _ = writeln!(cfg, "    class block{dead_pc} dead;");
    }

    if let Some(path) = find_shortest_path(blocks, start_pc, target_pc) {
        for pc in path {
            let _ = writeln!(cfg, "    class block{pc} path;");
        }
    }

    cfg
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;
    use crate::basic_block::build_basic_blocks;

    #[test]
    fn test_generate_annotated_cfg() {
        let instructions = vec![
            (0, Instruction::Goto(6)),
            (3, Instruction::Iconst1), // dead
            (4, Instruction::Ireturn),
            (6, Instruction::Iconst2), // target
            (7, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);
        let cfg = generate_annotated_cfg(&blocks, 0, 7);

        assert!(cfg.contains("class block3 dead;"));

        // Shortest path logic returns target_pc as the end of the path.
        // It should contain block0 and block6 since 0 goes to 6, and 6 contains 7.
        assert!(cfg.contains("class block0 path;"));
        assert!(cfg.contains("class block6 path;"));
    }
}
