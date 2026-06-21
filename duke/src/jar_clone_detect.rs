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

/// Detects structural clones (copy-pasted methods) across an entire JAR file.
///
/// **Why it exists:** Technical debt often accumulates through duplicated code.
/// This tool uses bytecode similarity analysis to find methods that are structurally
/// identical (or very similar) even if variable names or constants changed.
#[cfg(feature = "nova")]
#[allow(
    clippy::print_stdout,
    clippy::collapsible_if,
    clippy::use_debug,
    clippy::cast_precision_loss
)]
pub fn dump_jar_clone_detect(jar_path: &str) {
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

    struct MethodRecord {
        method_name: String,
        instructions: Vec<Instruction>,
    }

    let mut methods = Vec::new();

    for entry_name in class_entries {
        let class_name_internal = &entry_name[..entry_name.len() - 6];
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let this_class = resolve_class_name(&cf, cf.this_class);
                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");

                    // Skip tiny methods (getters/setters/constructors) to avoid noise
                    if name_str == "<init>" || name_str == "<clinit>" {
                        continue;
                    }

                    let full_name = format!("{this_class}::{name_str}{desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(decoded) = decode(&code.code) {
                                let instructions: Vec<Instruction> =
                                    decoded.into_iter().map(|(_, i)| i).collect();

                                // Only compare substantial methods (>= 15 instructions)
                                if instructions.len() >= 15 {
                                    methods.push(MethodRecord {
                                        method_name: full_name.clone(),
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
    println!(" JAR Clone Detection (Similarity > 0.90)");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Substantial Methods:  {}", methods.len());
    println!();

    let mut clones_found = 0;

    // O(N^2) comparison - optimized by checking length differences first
    for i in 0..methods.len() {
        for j in (i + 1)..methods.len() {
            let m1 = &methods[i];
            let m2 = &methods[j];

            // Length optimization: if lengths differ by more than 10%, similarity won't be > 0.90
            let max_len = std::cmp::max(m1.instructions.len(), m2.instructions.len()) as f64;
            let min_len = std::cmp::min(m1.instructions.len(), m2.instructions.len()) as f64;
            if min_len / max_len < 0.85 {
                continue;
            }

            let similarity = calculate_similarity(&m1.instructions, &m2.instructions);
            if similarity > 0.90 {
                println!("🚨 Clone Detected! (Score: {similarity:.2})");
                println!("  - {}", m1.method_name);
                println!("  - {}", m2.method_name);
                println!();
                clones_found += 1;
            }
        }
    }

    if clones_found == 0 {
        println!("✅ No highly similar clones detected.");
    } else {
        println!("Total cloned pairs found: {clones_found}");
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

        dump_jar_clone_detect(path_str);
    }
}
