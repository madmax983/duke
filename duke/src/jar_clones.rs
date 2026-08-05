#![allow(clippy::items_after_statements)]
#![allow(
    clippy::case_sensitive_file_extension_comparisons,
    clippy::collapsible_if,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]

#[cfg(feature = "nova")]
use duke_bytecode::{Instruction, calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{
    parse, {AttributeData, CpEntry, CpIndex},
};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::path::Path;
use std::process;

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn cp_str(cf: &duke_classfile::ClassFile, idx: CpIndex) -> Option<&str> {
    cf.constant_pool
        .get(idx.0 as usize)
        .and_then(|slot| slot.as_ref())
        .and_then(|entry| {
            if let CpEntry::Utf8(s) = entry {
                Some(s.as_str())
            } else {
                None
            }
        })
}

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn resolve_class_name(cf: &duke_classfile::ClassFile, idx: CpIndex) -> String {
    if idx.0 == 0 {
        return "<none>".to_string();
    }
    let class_entry = cf
        .constant_pool
        .get(idx.0 as usize)
        .and_then(|s| s.as_ref());
    if let Some(CpEntry::Class { name_index }) = class_entry {
        cp_str(cf, *name_index)
            .unwrap_or("<invalid utf8>")
            .to_string()
    } else {
        "<not a class ref>".to_string()
    }
}

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs, clippy::print_stdout, clippy::use_debug)]
pub fn dump_jar_clones(jar_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    struct MethodRecord {
        full_name: String,
        instructions: Vec<Instruction>,
    }

    let mut all_methods = Vec::new();
    let reader = loader.reader();

    let entry_names: Vec<String> = reader
        .entry_names()
        .map(std::string::ToString::to_string)
        .collect();

    for entry_name in entry_names {
        if !entry_name.ends_with(".class") {
            continue;
        }
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let class_name = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{class_name}::{name_str}{desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(decoded) = decode(&code.code) {
                                let instructions: Vec<Instruction> =
                                    decoded.into_iter().map(|(_, instr)| instr).collect();
                                // Filter out very small methods (like getters/setters) to avoid noise
                                if instructions.len() >= 10 {
                                    all_methods.push(MethodRecord {
                                        full_name: full_name.clone(),
                                        instructions,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("======================================");
    println!(" JAR Code Clone Detection");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Methods analyzed:     {}", all_methods.len());
    println!();

    let mut clones_found = 0;

    for i in 0..all_methods.len() {
        for j in (i + 1)..all_methods.len() {
            let m1 = &all_methods[i];
            let m2 = &all_methods[j];

            // Optimization: if lengths differ by more than 20%, they are unlikely to be >90% similar
            let len1 = m1.instructions.len() as f64;
            let len2 = m2.instructions.len() as f64;
            let min_len = len1.min(len2);
            let max_len = len1.max(len2);
            if min_len / max_len < 0.8 {
                continue;
            }

            let sim = calculate_similarity(&m1.instructions, &m2.instructions);
            if sim >= 0.90 {
                println!("Clone detected (Similarity: {:.1}%):", sim * 100.0);
                println!("  - {}", m1.full_name);
                println!("  - {}", m2.full_name);
                println!();
                clones_found += 1;
            }
        }
    }

    if clones_found == 0 {
        println!("No significant clones found.");
    } else {
        println!("Found {clones_found} similar method pairs.");
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_jar_clones_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path = path.to_str().unwrap();

        dump_jar_clones(path);
    }
}
