#![allow(clippy::items_after_statements)]

#[cfg(feature = "nova")]
use duke_bytecode::{Instruction, calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, ClassFile, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::path::Path;
use std::process;

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn cp_str(cf: &ClassFile, idx: CpIndex) -> Option<&str> {
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
fn resolve_class_name(cf: &ClassFile, idx: CpIndex) -> String {
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

/// Detects structural bytecode clones across a JAR file.
///
/// **Why it exists:** Copy-pasting code is a common anti-pattern that leads to
/// unmaintainable codebases. This utility uses Levenshtein distance on decoded
/// bytecode instructions (ignoring local variable indices/constants) to find
/// structurally similar methods.
#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if,
    clippy::cast_precision_loss
)]
pub fn dump_jar_clone_detect(jar_path: &str, min_similarity: f64) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    struct MethodData {
        class_name: String,
        method_name: String,
        instructions: Vec<Instruction>,
    }

    let mut all_methods = Vec::new();
    let reader = loader.reader();

    for entry_name in reader.entry_names() {
        let Some(class_name_internal) = entry_name.strip_suffix(".class") else {
            continue;
        };
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let this_class = resolve_class_name(&cf, cf.this_class);
                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{name_str}{desc_str}");

                    // Skip trivial methods (like empty constructors) to avoid noise
                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                let instrs: Vec<Instruction> =
                                    instructions.into_iter().map(|(_, i)| i).collect();
                                if instrs.len() >= 10 {
                                    // Only consider non-trivial methods
                                    all_methods.push(MethodData {
                                        class_name: this_class.clone(),
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
    println!(" JAR Clone Detection");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Threshold:            {:.0}%", min_similarity * 100.0);
    println!("Methods analyzed:     {}", all_methods.len());
    println!();

    let mut clones_found = 0;

    for i in 0..all_methods.len() {
        for j in (i + 1)..all_methods.len() {
            let m1 = &all_methods[i];
            let m2 = &all_methods[j];

            // Quick heuristic: length must be somewhat similar
            let len1 = m1.instructions.len() as f64;
            let len2 = m2.instructions.len() as f64;

            let min_len = len1.min(len2);
            let max_len = len1.max(len2);

            if (min_len / max_len) < min_similarity {
                continue;
            }

            let sim = calculate_similarity(&m1.instructions, &m2.instructions);
            if sim >= min_similarity {
                if sim > 0.99 && m1.class_name == m2.class_name && m1.method_name == m2.method_name
                {
                    continue;
                }

                println!("🚨 Clone Detected (Similarity: {:.2}%)", sim * 100.0);
                println!("  - {}::{}", m1.class_name, m1.method_name);
                println!("  - {}::{}", m2.class_name, m2.method_name);
                println!();
                clones_found += 1;
            }
        }
    }

    if clones_found == 0 {
        println!("✅ No clones detected above threshold.");
    } else {
        println!("Total clone pairs found: {clones_found}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "nova")]
    fn test_dump_jar_clone_detect_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        // Run with a very high threshold to ensure it doesn't find false positives and works.
        // It's mostly about covering the parsing and execution.
        dump_jar_clone_detect(path_str, 0.99);
    }
}
