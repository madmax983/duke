#![allow(clippy::items_after_statements)]

#[cfg(feature = "nova")]
use duke_bytecode::{calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::ZipLoader;
#[cfg(feature = "nova")]
use std::collections::HashMap;
#[cfg(feature = "nova")]
use std::path::Path;
#[cfg(feature = "nova")]
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

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::cast_precision_loss
)]
pub fn dump_jar_similarity(jar_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let mut methods: HashMap<String, Vec<duke_bytecode::Instruction>> = HashMap::new();
    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(std::string::ToString::to_string)
        .collect();

    for entry_name in class_entries {
        if let Ok(bytes) = reader.read_entry(&entry_name) {
            if let Ok(cf) = parse(&bytes) {
                let class_name = resolve_class_name(&cf, cf.this_class);
                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{class_name}::{name_str}{desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                let instrs: Vec<_> =
                                    instructions.into_iter().map(|(_, i)| i).collect();
                                if instrs.len() > 10 {
                                    // Only consider non-trivial methods
                                    methods.insert(full_name.clone(), instrs);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut clones = Vec::new();
    let method_names: Vec<_> = methods.keys().collect();
    let total_methods = method_names.len();

    for i in 0..total_methods {
        for j in (i + 1)..total_methods {
            let name1 = method_names[i];
            let name2 = method_names[j];
            let seq1 = methods.get(name1).unwrap();
            let seq2 = methods.get(name2).unwrap();

            let sim = calculate_similarity(seq1, seq2);
            if sim > 0.85 {
                clones.push((name1.clone(), name2.clone(), sim));
            }
        }
    }

    clones.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    println!("======================================");
    println!(" JAR Bytecode Clone Analysis");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Analyzed Methods:     {total_methods} (length > 10)");
    println!("Potential Clones:     {}", clones.len());
    println!();

    if clones.is_empty() {
        println!("✅ No structural clones detected. Great codebase!");
    } else {
        println!("--- Top Clones (Similarity > 85%) ---");
        for (i, (m1, m2, sim)) in clones.iter().take(15).enumerate() {
            let percent = sim * 100.0;
            println!("  {}. {percent:.1}% match", i + 1);
            println!("       A: {m1}");
            println!("       B: {m2}");
        }
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_jar_similarity_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();
        dump_jar_similarity(path_str);
    }
}
