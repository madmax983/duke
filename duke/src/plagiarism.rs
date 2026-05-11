#![allow(clippy::items_after_statements)]
#![allow(
    unused_imports,
    clippy::uninlined_format_args,
    clippy::redundant_closure_for_method_calls,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::collapsible_if
)]

#[cfg(feature = "nova")]
use duke_bytecode::{Instruction, calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, ClassFile, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::path::Path;
use std::process;

#[cfg(feature = "nova")]
#[allow(unexpected_cfgs)]
fn cp_str(cf: &ClassFile, idx: CpIndex) -> Option<&str> {
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
#[allow(unexpected_cfgs)]
fn resolve_class_name(cf: &ClassFile, idx: CpIndex) -> String {
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
struct MethodData {
    class_name: String,
    method_name: String,
    instructions: Vec<Instruction>,
}

#[cfg(feature = "nova")]
fn load_methods(jar_path: &str) -> Vec<MethodData> {
    let Ok(loader) = ZipLoader::open(Path::new(jar_path)) else {
        eprintln!("duke: failed to open JAR '{}'", jar_path);
        process::exit(1);
    };

    let mut methods = Vec::new();
    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(|s| s.to_string())
        .collect();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let class_name = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(decoded) = decode(&code.code) {
                                let instrs: Vec<_> = decoded.into_iter().map(|(_, i)| i).collect();
                                if instrs.len() >= 15 {
                                    // Only non-trivial methods
                                    methods.push(MethodData {
                                        class_name: class_name.clone(),
                                        method_name: format!("{}{}", name_str, desc_str),
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
    methods
}

#[cfg(feature = "nova")]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    unused_imports,
    clippy::uninlined_format_args,
    clippy::redundant_closure_for_method_calls,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::collapsible_if
)]
pub fn dump_plagiarism_check(jar1_path: &str, jar2_path: &str, threshold: f64) {
    println!("Loading methods from {}...", jar1_path);
    let methods1 = load_methods(jar1_path);
    println!("Loaded {} non-trivial methods.", methods1.len());

    println!("Loading methods from {}...", jar2_path);
    let methods2 = load_methods(jar2_path);
    println!("Loaded {} non-trivial methods.", methods2.len());

    println!("Comparing for plagiarism (threshold: {})...", threshold);

    let mut clones = Vec::new();

    for m1 in &methods1 {
        for m2 in &methods2 {
            let len1 = m1.instructions.len() as f64;
            let len2 = m2.instructions.len() as f64;

            let min_len = len1.min(len2);
            let max_len = len1.max(len2);
            if min_len / max_len < threshold {
                continue;
            }

            // Exclude identical names (might just be the same library version)
            if m1.class_name == m2.class_name && m1.method_name == m2.method_name {
                continue;
            }

            let sim = calculate_similarity(&m1.instructions, &m2.instructions);
            if sim >= threshold {
                clones.push((
                    sim,
                    format!("{}::{}", m1.class_name, m1.method_name),
                    format!("{}::{}", m2.class_name, m2.method_name),
                ));
            }
        }
    }

    clones.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    println!("======================================");
    println!(" Structural Plagiarism Found");
    println!("======================================");
    if clones.is_empty() {
        println!("No suspicious structural clones found.");
    } else {
        for (sim, n1, n2) in clones.iter().take(20) {
            println!("Similarity: {:.2}%", sim * 100.0);
            println!("  JAR 1: {}", n1);
            println!("  JAR 2: {}", n2);
            println!();
        }
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_plagiarism_check_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path = path.to_str().unwrap();

        // Compare same jar, but since it excludes exact name matches, it should find nothing or other methods
        super::dump_plagiarism_check(path, path, 0.9);
    }
}
