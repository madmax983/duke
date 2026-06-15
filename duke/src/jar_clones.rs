#![allow(clippy::items_after_statements)]
#[cfg(feature = "nova")]
use duke_bytecode::{calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{
    parse, {AttributeData, CpEntry, CpIndex},
};
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
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
pub fn dump_jar_clones(jar_path: &str, threshold: f64) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("class"))
        })
        .map(std::string::ToString::to_string)
        .collect();

    // Collect all methods and their instructions
    struct MethodInfo {
        full_name: String,
        instructions: Vec<duke_bytecode::Instruction>,
    }
    let mut all_methods: Vec<MethodInfo> = Vec::new();

    for entry_name in class_entries {
        let class_name_internal = &entry_name[..entry_name.len() - 6];
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{class_name_internal}.{name_str}{desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                // Extract just the instructions without the PC
                                let just_instrs: Vec<_> =
                                    instructions.into_iter().map(|(_, i)| i).collect();
                                // Ignore very small methods (e.g. getters/setters) to avoid noise
                                if just_instrs.len() > 5 {
                                    all_methods.push(MethodInfo {
                                        full_name: full_name.clone(),
                                        instructions: just_instrs,
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
    println!(" JAR Code Clone Detector");
    println!("======================================");
    println!("File:      {jar_path}");
    println!("Threshold: {threshold:.2}");
    println!("Methods:   {}", all_methods.len());
    println!();

    let mut found_clones = false;
    // O(N^2) comparison - fine for moderately sized JARs like our test fixtures
    for i in 0..all_methods.len() {
        for j in (i + 1)..all_methods.len() {
            let sim =
                calculate_similarity(&all_methods[i].instructions, &all_methods[j].instructions);
            if sim >= threshold {
                found_clones = true;
                println!("🚨 Clone Detected! (Similarity: {sim:.2})");
                println!("  A: {}", all_methods[i].full_name);
                println!("  B: {}", all_methods[j].full_name);
                println!();
            }
        }
    }

    if !found_clones {
        println!("✅ No clones detected above threshold.");
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
        let path_str = path.to_str().unwrap();

        dump_jar_clones(path_str, 0.9);
    }
}
