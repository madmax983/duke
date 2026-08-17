//! Method Metrics Analyzer.
//!
//! Gathers various statistics and metrics about a sequence of JVM instructions.

#[cfg(feature = "nova")]
use crate::Instruction;
#[cfg(feature = "nova")]
use crate::basic_block::build_basic_blocks;
#[cfg(feature = "nova")]
use crate::cfg::cyclomatic_complexity;
#[cfg(feature = "nova")]
use crate::reachability::find_dead_blocks;

/// A summary of metrics for a method's bytecode.
#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodMetrics {
    /// Total number of instructions.
    pub instruction_count: usize,
    /// Total number of basic blocks.
    pub basic_block_count: usize,
    /// Cyclomatic complexity of the method.
    pub cyclomatic_complexity: usize,
    /// Number of unreachable (dead) basic blocks.
    pub dead_block_count: usize,
    /// Number of return instructions.
    pub return_count: usize,
}

/// Analyzes a sequence of instructions and returns a summary of metrics.
#[cfg(feature = "nova")]
#[must_use]
pub fn analyze_method_metrics(instructions: &[(usize, Instruction)]) -> MethodMetrics {
    let instruction_count = instructions.len();
    let blocks = build_basic_blocks(instructions);
    let basic_block_count = blocks.len();

    let cyclomatic_complexity = cyclomatic_complexity(instructions);

    let dead_blocks = if blocks.is_empty() {
        Vec::new()
    } else {
        find_dead_blocks(&blocks, blocks[0].start_pc)
    };
    let dead_block_count = dead_blocks.len();

    let return_count = instructions
        .iter()
        .filter(|(_, instr)| instr.is_return())
        .count();

    MethodMetrics {
        instruction_count,
        basic_block_count,
        cyclomatic_complexity,
        dead_block_count,
        return_count,
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;

    #[test]
    fn test_analyze_method_metrics() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Ifeq(5)),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Goto(10)),
            (9, Instruction::Ireturn),
            (10, Instruction::Ireturn),
        ];

        let metrics = analyze_method_metrics(&instructions);

        assert_eq!(metrics.instruction_count, 7);
        assert_eq!(metrics.return_count, 3);
        assert_eq!(metrics.dead_block_count, 3);
        assert_eq!(metrics.basic_block_count, 6);
        assert_eq!(metrics.cyclomatic_complexity, 2); // 1 + 1 branch
    }

    #[test]
    fn test_analyze_method_metrics_empty() {
        let metrics = analyze_method_metrics(&[]);
        assert_eq!(metrics.instruction_count, 0);
        assert_eq!(metrics.basic_block_count, 0);
        assert_eq!(metrics.cyclomatic_complexity, 1);
        assert_eq!(metrics.dead_block_count, 0);
        assert_eq!(metrics.return_count, 0);
    }
}
