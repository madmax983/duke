#![allow(
    clippy::items_after_statements,
    clippy::collapsible_if,
    clippy::uninlined_format_args,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::print_stdout,
    dead_code
)]
#[cfg(feature = "nova")]
use duke_bytecode::{Instruction, calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, ClassFile, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
#[cfg(feature = "nova")]
use std::path::Path;
#[cfg(feature = "nova")]
#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs, dead_code)]
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
#[allow(unexpected_cfgs, dead_code)]
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
#[cfg(feature = "nova")]
pub fn dump_clone_detect(jar_path: &str) {
    let Ok(loader) = ZipLoader::open(Path::new(jar_path)) else {
        eprintln!("duke: failed to open JAR '{jar_path}'");
        return;
    };
    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(std::string::ToString::to_string)
        .collect();
    // Store (Method Name, Vec<Instruction>)
    let mut methods_code: Vec<(String, Vec<Instruction>)> = Vec::new();
    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let this_class = resolve_class_name(&cf, cf.this_class);
                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{}::{}{}", this_class, name_str, desc_str);
                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                let instrs: Vec<Instruction> =
                                    instructions.into_iter().map(|(_, i)| i).collect();
                                if instrs.len() >= 10 {
                                    // Only consider non-trivial methods
                                    methods_code.push((full_name.clone(), instrs));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    println!("======================================");
    println!(" Structural Clone Detection");
    println!("======================================");
    println!("File:                 {}", jar_path);
    println!("Methods analyzed:     {}", methods_code.len());
    println!();
    let mut clones_found = 0;
    for i in 0..methods_code.len() {
        for j in (i + 1)..methods_code.len() {
            let (name1, seq1) = &methods_code[i];
            let (name2, seq2) = &methods_code[j];
            let sim = calculate_similarity(seq1, seq2);
            if sim >= 0.95 {
                clones_found += 1;
                println!("🚨 Clone Found (Similarity: {:.2}):", sim);
                println!("  - {}", name1);
                println!("  - {}", name2);
                println!();
            }
        }
    }
    if clones_found == 0 {
        println!("✅ No significant structural clones detected.");
    } else {
        println!("Total Clones Found: {}", clones_found);
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
        dump_clone_detect(path_str);
    }
    #[test]
    fn test_dump_clone_detect_clones() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/clones/clones.jar");
        let path_str = path.to_str().unwrap();
        if std::path::Path::new(path_str).exists() {
            dump_clone_detect(path_str);
        }
    }
}
