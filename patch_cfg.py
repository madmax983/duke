import re

with open('crates/duke-bytecode/src/cfg.rs', 'r') as f:
    content = f.read()

replacement = """    // ⚡ Bolt Optimization:
    // Removed O(N) HashMap allocation for basic block lookup.
    // The `blocks` slice is already sorted by `start_pc`.
    // Lookups now use zero-allocation binary_search_by_key.
    debug_assert!(blocks.windows(2).all(|w| w[0].start_pc < w[1].start_pc), "Basic blocks must be sorted by start_pc");

    for block in blocks {
        let block_id = block.start_pc;"""

content = re.sub(r'    // Map PC to Block ID \(start_pc\) for edge resolution\n    let mut pc_to_block = std::collections::HashMap::new\(\);\n    for block in blocks {\n        pc_to_block\.insert\(block\.start_pc, block\.start_pc\);\n    }\n\n    for block in blocks {', replacement, content)

replacement2 = """            let next_pc = if blocks.binary_search_by_key(&next_block_id, |b| b.start_pc).is_ok() {
                Some(next_block_id)
            } else {"""

content = re.sub(r'            let next_pc = if pc_to_block\.contains_key\(&next_block_id\) {\n                Some\(next_block_id\)\n            } else {', replacement2, content)

with open('crates/duke-bytecode/src/cfg.rs', 'w') as f:
    f.write(content)
