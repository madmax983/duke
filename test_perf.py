import time
import os
import subprocess

with open("crates/duke-bytecode/src/basic_block.rs", "r") as f:
    original = f.read()

replacement = original.replace("use std::collections::BTreeSet;", "")
replacement = replacement.replace(
"""fn find_leaders(instructions: &[(usize, Instruction)]) -> BTreeSet<usize> {
    let mut leaders = BTreeSet::new();""",
"""fn find_leaders(instructions: &[(usize, Instruction)]) -> Vec<usize> {
    let mut leaders = Vec::with_capacity(instructions.len() / 4 + 2);"""
)
replacement = replacement.replace(
"""    leaders
}""",
"""    leaders.sort_unstable();
    leaders.dedup();
    leaders
}"""
)
replacement = replacement.replace(
"""fn construct_blocks(
    instructions: &[(usize, Instruction)],
    leaders: &BTreeSet<usize>,
) -> Vec<BasicBlock> {""",
"""fn construct_blocks(
    instructions: &[(usize, Instruction)],
    leaders: &[usize],
) -> Vec<BasicBlock> {"""
)
replacement = replacement.replace("leaders.contains(pc)", "leaders.binary_search(pc).is_ok()")

with open("crates/duke-bytecode/src/basic_block.rs", "w") as f:
    f.write(replacement)

os.system("cargo test -p duke-bytecode")

with open("crates/duke-bytecode/src/basic_block.rs", "w") as f:
    f.write(original)
