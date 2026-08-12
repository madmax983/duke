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

#[cfg(feature = "nova")]
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
#[allow(
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::collapsible_if,
    clippy::too_many_lines,
    unexpected_cfgs,
    clippy::items_after_statements
)]
pub fn dump_clone_detect(jar_path: &str) {
    struct MethodInfo {
        class_name: String,
        method_name: String,
        desc: String,
        instructions: Vec<Instruction>,
    }

    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let reader = loader.reader();

    let mut methods_data = Vec::new();

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
                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(decoded) = decode(&code.code) {
                                let instrs: Vec<Instruction> =
                                    decoded.into_iter().map(|(_, i)| i).collect();
                                // Only consider methods with at least 10 instructions to avoid trivial clones
                                if instrs.len() >= 10 {
                                    methods_data.push(MethodInfo {
                                        class_name: class_name_internal.to_string(),
                                        method_name: name_str.to_string(),
                                        desc: desc_str.to_string(),
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
    println!(" JAR Clone Detection Analysis");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Methods analyzed:     {}", methods_data.len());
    println!();

    let threshold = 0.85;
    let mut clones_found = 0;

    for i in 0..methods_data.len() {
        for j in (i + 1)..methods_data.len() {
            let m1 = &methods_data[i];
            let m2 = &methods_data[j];

            // Skip same class to avoid matching overloaded methods? Or maybe we want to match them.
            // Let's just compare all.

            let sim = calculate_similarity(&m1.instructions, &m2.instructions);
            if sim >= threshold {
                clones_found += 1;
                println!("🚨 Clone Detected (Similarity: {:.1}%)", sim * 100.0);
                println!("  - {}.{}{}", m1.class_name, m1.method_name, m1.desc);
                println!("  - {}.{}{}", m2.class_name, m2.method_name, m2.desc);
                println!();
            }
        }
    }

    if clones_found == 0 {
        println!(
            "✅ No significant clones detected above {}% threshold.",
            threshold * 100.0
        );
    } else {
        println!("Total clone pairs found: {clones_found}");
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
}
