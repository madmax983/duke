#![allow(clippy::items_after_statements)]
#![allow(clippy::case_sensitive_file_extension_comparisons)]

#[cfg(feature = "nova")]
use duke_bytecode::{Instruction, calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::path::Path;

#[cfg(feature = "nova")]
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

#[cfg(feature = "nova")]
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
#[allow(unexpected_cfgs)]
struct MethodBody {
    class_name: String,
    method_name: String,
    instructions: Vec<Instruction>,
}

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if,
    clippy::cast_precision_loss
)]
pub fn dump_jar_clone_detect(jar_path: &str, threshold: f64) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        // We avoid process::exit to prevent test runner death; instead panic (which can be caught)
        panic!("duke: failed to open JAR");
    });

    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(std::string::ToString::to_string)
        .collect();

    let mut methods = Vec::new();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let class_name = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{name_str}{desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(decoded) = decode(&code.code) {
                                // Extract instructions, ignoring the PC
                                let instrs: Vec<Instruction> =
                                    decoded.into_iter().map(|(_, i)| i).collect();

                                // Only consider methods with a meaningful amount of instructions
                                // to avoid false positives on trivial getters/setters/constructors
                                if instrs.len() >= 5 {
                                    methods.push(MethodBody {
                                        class_name: class_name.clone(),
                                        method_name: full_name.clone(),
                                        instructions: instrs,
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
    println!(" Bytecode Plagiarism & Clone Detection");
    println!("======================================");
    println!("File:               {jar_path}");
    println!("Methods Analyzed:   {}", methods.len());
    println!("Similarity Threshold: {:.0}%", threshold * 100.0);
    println!();

    let mut found_clones = false;

    // O(N^2) comparison. This is acceptable for a static analysis tool,
    // and `calculate_similarity` uses dynamic programming.
    for i in 0..methods.len() {
        for j in (i + 1)..methods.len() {
            let m1 = &methods[i];
            let m2 = &methods[j];

            let similarity = calculate_similarity(&m1.instructions, &m2.instructions);

            if similarity >= threshold {
                found_clones = true;
                println!(
                    "🚨 Clone Detected! (Similarity: {:.1}%)",
                    similarity * 100.0
                );
                println!("  1. {}::{}", m1.class_name, m1.method_name);
                println!("  2. {}::{}", m2.class_name, m2.method_name);
                println!();
            }
        }
    }

    if !found_clones {
        println!("✅ No clones detected above the threshold. Code is DRY!");
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_jar_clone_detect_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        // Should just print results for a real JAR without panicking
        dump_jar_clone_detect(path_str, 0.9);
    }

    #[test]
    fn test_dump_jar_clone_detect_missing() {
        let result = std::panic::catch_unwind(|| {
            dump_jar_clone_detect("non_existent_file_for_clone_detect.jar", 0.9);
        });
        assert!(result.is_err());
    }
}
