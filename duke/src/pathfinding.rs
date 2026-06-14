//! Finds the shortest path between two points in the control flow graph.

#![allow(clippy::items_after_statements)]
#[cfg(feature = "nova")]
use duke_bytecode::{build_basic_blocks, decode, find_shortest_path};
#[cfg(feature = "nova")]
use duke_classfile::{
    parse, {AttributeData, CpEntry},
};
use std::process;

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if
)]
pub fn dump_shortest_path(path: &str, method_name: &str, start_pc: usize, target_pc: usize) {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("duke: cannot read '{path}': {e}");
            process::exit(1);
        }
    };
    let cf = match parse(&bytes) {
        Ok(cf) => cf,
        Err(e) => {
            eprintln!("duke: parse error in '{path}': {e}");
            process::exit(1);
        }
    };

    let target = cf
        .methods
        .iter()
        .find(|m| {
            let Some(Some(CpEntry::Utf8(s))) = cf.constant_pool.get(m.name_index.0 as usize) else {
                return false;
            };
            s.as_str() == method_name
        })
        .unwrap_or_else(|| {
            eprintln!("duke: method '{method_name}' not found");
            process::exit(1);
        });

    for attr in &target.attributes {
        if let AttributeData::Code(code) = &attr.data {
            let instructions = match decode(&code.code) {
                Ok(inst) => inst,
                Err(e) => {
                    eprintln!("duke: decode error: {e}");
                    process::exit(1);
                }
            };
            let blocks = build_basic_blocks(&instructions);

            // First we need to find which basic block contains the start_pc and target_pc
            let mut start_block_pc = None;
            let mut target_block_pc = None;

            for block in &blocks {
                if block.start_pc <= start_pc && block.end_pc > start_pc {
                    start_block_pc = Some(block.start_pc);
                }
                if block.start_pc <= target_pc && block.end_pc > target_pc {
                    target_block_pc = Some(block.start_pc);
                }
            }

            let Some(actual_start_pc) = start_block_pc else {
                eprintln!("duke: start_pc {start_pc} is not within any basic block");
                process::exit(1);
            };

            let Some(actual_target_pc) = target_block_pc else {
                eprintln!("duke: target_pc {target_pc} is not within any basic block");
                process::exit(1);
            };

            println!("=== Shortest Path Analysis ===");
            println!("Class File: {path}");
            println!("Method:     {method_name}");
            println!("Start PC:   {start_pc} (Block starts at {actual_start_pc})");
            println!("Target PC:  {target_pc} (Block starts at {actual_target_pc})");
            println!();

            match find_shortest_path(&blocks, actual_start_pc, actual_target_pc) {
                Some(path) => {
                    println!("Shortest path found ({} basic blocks):", path.len());
                    let path_str: Vec<String> =
                        path.iter().map(std::string::ToString::to_string).collect();
                    println!("  {}", path_str.join(" -> "));
                }
                None => {
                    println!("No execution path found from PC {start_pc} to {target_pc}.");
                }
            }
            return;
        }
    }
    eprintln!("duke: method '{method_name}' has no code attribute");
    process::exit(1);
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_shortest_path_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/HelloWorld.class");
        let path_str = path.to_str().unwrap();

        // HelloWorld main is very simple, start to end should be one block or straightforward.
        // It doesn't really matter as long as it doesn't panic.
        dump_shortest_path(path_str, "main", 0, 8);
    }
}
