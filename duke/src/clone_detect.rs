#![allow(clippy::items_after_statements)]
#[cfg(feature = "nova")]
use duke_bytecode::{Instruction, calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{
    parse, AttributeData, CpEntry, CpIndex,
};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::path::Path;
use std::process;

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs, dead_code)]
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
#[allow(unexpected_cfgs, dead_code)]
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
struct MethodDef {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

#[cfg(feature = "nova")]
#[allow(
    clippy::print_stdout,
    clippy::collapsible_if,
    clippy::use_debug,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::items_after_statements,
    clippy::cast_lossless,
    clippy::cast_precision_loss
)]
pub fn dump_clone_detect(jar_path: &str, threshold: f64) {
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

    let mut all_methods = Vec::new();

    for entry_name in class_entries {
        let class_name_internal = &entry_name[..entry_name.len() - 6];
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let this_class = resolve_class_name(&cf, cf.this_class);
                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{this_class}::{name_str}{desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                let instrs: Vec<Instruction> =
                                    instructions.into_iter().map(|(_, i)| i).collect();
                                if instrs.len() > 5 {
                                    // Ignore trivial methods
                                    all_methods.push(MethodDef {
                                        name: full_name.clone(),
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
    println!(" Clone Detection Analysis");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Threshold:            {threshold}");
    println!();

    let mut found = false;
    for i in 0..all_methods.len() {
        for j in (i + 1)..all_methods.len() {
            let sim =
                calculate_similarity(&all_methods[i].instructions, &all_methods[j].instructions);
            if sim >= threshold {
                found = true;
                println!("🚨 Clone Found (Similarity: {sim:.2}):");
                println!("   - {}", all_methods[i].name);
                println!("   - {}", all_methods[j].name);
            }
        }
    }

    if !found {
        println!("✅ No clones found above threshold.");
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

        dump_clone_detect(path_str, 0.9);
    }
}
