#![allow(clippy::items_after_statements)]

#[cfg(feature = "nova")]
use duke_bytecode::{Instruction, calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::path::Path;
use std::process;

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
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
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn resolve_class_name(cf: &duke_classfile::ClassFile, idx: CpIndex) -> String {
    if idx.0 == 0 {
        return "<none>".to_string();
    }
    let class_entry = cf
        .constant_pool
        .get(idx.0 as usize)
        .and_then(|s: &Option<CpEntry>| s.as_ref());
    if let Some(CpEntry::Class { name_index }) = class_entry {
        cp_str(cf, *name_index)
            .unwrap_or("<invalid utf8>")
            .to_string()
    } else {
        "<not a class ref>".to_string()
    }
}

/// Detects structurally similar methods (code clones) across a JAR file.
///
/// **Why it exists:** Code duplication is a common source of technical debt.
/// This tool scans all methods in a JAR, computes the Levenshtein distance
/// between their instruction streams (ignoring operands), and highlights
/// highly similar methods. This helps identify refactoring opportunities.
#[cfg(feature = "nova")]
#[allow(
    clippy::print_stdout,
    clippy::collapsible_if,
    clippy::use_debug,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::too_many_lines
)]
pub fn dump_clone_detect(jar_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    struct MethodData {
        full_name: String,
        instructions: Vec<Instruction>,
    }

    let mut all_methods = Vec::new();
    let reader = loader.reader();

    for entry_name in reader.entry_names() {
        if !std::path::Path::new(&entry_name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("class"))
        {
            continue;
        }
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let this_class = resolve_class_name(&cf, cf.this_class);
                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{this_class}::{name_str}{desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(decoded) = decode(&code.code) {
                                // Filter out trivial methods to avoid noise (e.g., empty constructors)
                                if decoded.len() > 10 {
                                    let instructions: Vec<Instruction> =
                                        decoded.into_iter().map(|(_, instr)| instr).collect();
                                    all_methods.push(MethodData {
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

    let mut clones = Vec::new();
    let threshold = 0.85;

    for i in 0..all_methods.len() {
        for j in (i + 1)..all_methods.len() {
            let sim =
                calculate_similarity(&all_methods[i].instructions, &all_methods[j].instructions);
            if sim >= threshold {
                clones.push((
                    sim,
                    all_methods[i].full_name.clone(),
                    all_methods[j].full_name.clone(),
                ));
            }
        }
    }

    clones.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    println!("======================================");
    println!(" JAR Clone Detection Analysis");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Methods analyzed:     {}", all_methods.len());
    println!("Clones detected:      {}", clones.len());
    println!();

    if clones.is_empty() {
        println!("✅ No significant code clones found. Good job!");
    } else {
        println!("🚨 Suspected Code Clones (>= 85% similarity):");
        for (sim, m1, m2) in clones.iter().take(20) {
            println!("  [{sim:.2}]");
            println!("    - {m1}");
            println!("    - {m2}");
        }
        if clones.len() > 20 {
            println!("  ... and {} more", clones.len() - 20);
        }
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_clone_detect_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        // Just ensure it runs without panicking on a valid JAR.
        dump_clone_detect(path_str);
    }
}
