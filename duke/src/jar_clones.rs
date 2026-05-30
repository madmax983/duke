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
use duke_classfile::{AttributeData, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
#[cfg(feature = "nova")]
use std::path::Path;
#[cfg(feature = "nova")]
use std::process;

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
#[cfg(feature = "nova")]
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
#[cfg(feature = "nova")]
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

/// Prints a code clone analysis summary of an entire JAR file to standard output.
///
/// **Why it exists:** Provides developers with insights into duplicated logic
/// across different methods and classes in a JAR, helping identify areas for
/// refactoring and cleanup.
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
#[cfg(feature = "nova")]
pub fn dump_jar_clones(jar_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    #[cfg(not(tarpaulin_include))]
    #[allow(unexpected_cfgs)]
    struct MethodRecord {
        class_name: String,
        method_name: String,
        instructions: Vec<Instruction>,
    }
    let mut all_methods = Vec::new();

    let reader = loader.reader();

    for entry_name in reader.entry_names() {
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
                    let full_name = format!("{name_str}{desc_str}");

                    let mut extracted_instructions = None;
                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                extracted_instructions = Some(
                                    instructions.into_iter().map(|(_, i)| i).collect::<Vec<_>>(),
                                );
                            }
                        }
                    }

                    if let Some(instructions) = extracted_instructions {
                        // Filter out very short methods to avoid noise (e.g., getters/setters)
                        if instructions.len() >= 15 {
                            all_methods.push(MethodRecord {
                                class_name: class_name.clone(),
                                method_name: full_name,
                                instructions,
                            });
                        }
                    }
                }
            }
        }
    }

    println!("======================================");
    println!(" Code Clone Analysis");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!(
        "Analyzed Methods:     {} (>= 15 instructions)",
        all_methods.len()
    );
    println!();

    let mut clones_found = 0;

    // O(N^2) comparison - fine for small/medium jars in an analysis tool
    for i in 0..all_methods.len() {
        for j in (i + 1)..all_methods.len() {
            let m1 = &all_methods[i];
            let m2 = &all_methods[j];

            let similarity = calculate_similarity(&m1.instructions, &m2.instructions);

            // 0.90 is a good threshold for "basically identical logic"
            if similarity >= 0.90 {
                if clones_found == 0 {
                    println!("Potential Clones Found:");
                }
                clones_found += 1;
                println!(
                    "\n  Match {clones_found} (Similarity: {:.1}%):",
                    similarity * 100.0
                );
                println!("    A: {}::{}", m1.class_name, m1.method_name);
                println!("    B: {}::{}", m2.class_name, m2.method_name);
            }
        }
    }

    if clones_found == 0 {
        println!("No potential clones found (threshold 90%).");
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    #[cfg(feature = "nova")]
    fn test_dump_jar_clones_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path = path.to_str().unwrap();

        // Run the analyzer to ensure it doesn't crash
        super::dump_jar_clones(path);
    }
}
