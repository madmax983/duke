#[cfg(feature = "nova")]
use duke_bytecode::{build_basic_blocks, compute_dominators, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use std::fs;

#[cfg(feature = "nova")]
fn cp_str(cf: &duke_classfile::ClassFile, idx: CpIndex) -> Option<&str> {
    cf.constant_pool
        .get(idx.0 as usize)
        .and_then(|slot: &Option<CpEntry>| slot.as_ref())
        .and_then(|entry| {
            if let CpEntry::Utf8(s) = entry {
                Some(s.as_str())
            } else {
                None
            }
        })
}

#[cfg(feature = "nova")]
pub fn dump_dom(class_file_path: &str, target_method: &str) {
    let bytes = fs::read(class_file_path).unwrap_or_else(|_| {
        eprintln!("Error: Could not read file '{class_file_path}'");
        std::process::exit(1);
    });

    let class_file = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("Error parsing class file: {e}");
        std::process::exit(1);
    });

    let mut method_found = false;

    for method in &class_file.methods {
        let name = cp_str(&class_file, method.name_index).unwrap_or("<invalid>");

        if name != target_method {
            continue;
        }

        method_found = true;

        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                let instructions = decode(&code.code).unwrap_or_else(|e| {
                    eprintln!("Error decoding bytecode: {e}");
                    std::process::exit(1);
                });

                if instructions.is_empty() {
                    println!("Method {target_method} has no instructions.");
                    return;
                }

                let blocks = build_basic_blocks(&instructions);
                let doms = compute_dominators(&blocks, 0);

                println!("Dominator Tree for method {target_method}:");

                let mut sorted_pcs: Vec<usize> = doms.keys().copied().collect();
                sorted_pcs.sort_unstable();

                for pc in sorted_pcs {
                    if let Some(dominators) = doms.get(&pc) {
                        let mut sorted_doms: Vec<usize> = dominators.iter().copied().collect();
                        sorted_doms.sort_unstable();

                        let dom_strs: Vec<String> =
                            sorted_doms.iter().map(|d| format!("PC {d}")).collect();
                        println!(
                            "Block at PC {:>3} is dominated by: {}",
                            pc,
                            dom_strs.join(", ")
                        );
                    }
                }
            }
        }
    }

    if !method_found {
        eprintln!("Error: Method '{target_method}' not found in class.");
        std::process::exit(1);
    }
}
