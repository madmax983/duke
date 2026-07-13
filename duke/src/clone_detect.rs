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
use duke_classfile::{parse, AttributeData, CpEntry, CpIndex};
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

/// Identifies heavily duplicated bytecode logic across an entire JAR.
///
/// **Why it exists:** Provides insight into structural code duplication.
#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::too_many_lines
)]
pub fn dump_clone_detect(jar_path: &str) -> Result<(), String> {
    let loader = ZipLoader::open(Path::new(jar_path)).map_err(|e| {
        format!("duke: failed to open JAR '{jar_path}': {e}")
    })?;

    struct MethodInfo {
        full_name: String,
        instructions: Vec<Instruction>,
    }

    let mut methods = Vec::new();
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
                    let full_name = format!("{class_name}::{name_str}{desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(decoded) = decode(&code.code) {
                                let instrs: Vec<_> = decoded.into_iter().map(|(_, i)| i).collect();
                                // Filter out trivial methods to avoid noise (e.g. getters/setters)
                                if instrs.len() >= 15 {
                                    methods.push(MethodInfo {
                                        full_name: full_name.clone(),
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
    println!(" Code Clone Detection");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!(
        "Analyzed Methods:     {} (>= 15 instructions)",
        methods.len()
    );
    println!();

    let mut clones = Vec::new();

    // O(N^2) search. Let's do a heuristic pre-filter on length to avoid running O(M^2) Levenshtein unnecessarily
    for i in 0..methods.len() {
        for j in (i + 1)..methods.len() {
            let m1 = &methods[i];
            let m2 = &methods[j];

            // Heuristic: If lengths differ by more than 20%, they are unlikely to be strong clones
            let len1 = m1.instructions.len() as f64;
            let len2 = m2.instructions.len() as f64;
            if (len1 - len2).abs() / f64::max(len1, len2) > 0.20 {
                continue;
            }

            let sim = calculate_similarity(&m1.instructions, &m2.instructions);
            if sim >= 0.85 {
                clones.push((sim, m1.full_name.clone(), m2.full_name.clone()));
            }
        }
    }

    clones.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    if clones.is_empty() {
        println!("✅ No significant code clones found.");
    } else {
        println!("🚨 Top Code Clones Detected:");
        for (i, (sim, m1, m2)) in clones.iter().take(20).enumerate() {
            println!("  {}. Similarity: {:.1}%", i + 1, sim * 100.0);
            println!("     - {m1}");
            println!("     - {m2}");
            println!();
        }
    }
    Ok(())
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

        assert!(dump_clone_detect(path_str).is_ok());
    }

    #[test]
    fn test_dump_clone_detect_invalid() {
        assert!(dump_clone_detect("nonexistent.jar").is_err());
    }
}
