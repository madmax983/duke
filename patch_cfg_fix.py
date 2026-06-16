import re

with open('crates/duke-bytecode/src/cfg.rs', 'r') as f:
    content = f.read()

# Replace block_id properly
content = content.replace("            let next_pc = if blocks.binary_search_by_key(&next_block_id, |b| b.start_pc).is_ok() {", "            let next_pc = if blocks.binary_search_by_key(&next_block_id, |b| b.start_pc).is_ok() {")

with open('crates/duke-bytecode/src/cfg.rs', 'w') as f:
    f.write(content)
