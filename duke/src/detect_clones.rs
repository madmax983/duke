#![allow(clippy::items_after_statements)]

#[cfg(feature = "nova")]
use duke_bytecode::{calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, CpEntry, CpIndex, parse};
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

#[cfg(feature = "nova")]
#[allow(
    clippy::print_stdout,
    clippy::collapsible_if,
    clippy::use_debug,
    clippy::cast_precision_loss
)]
pub fn dump_clones(jar_path: &str) {
    let loader = match ZipLoader::open(Path::new(jar_path)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("duke: failed to open JAR '{jar_path}': {e}");
            return;
        }
    };

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

    // extract all methods and their instructions
    let mut all_methods = Vec::new();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let class_name = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{class_name}.{name_str}{desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                // ignore tiny methods
                                if instructions.len() > 5 {
                                    let instrs: Vec<_> =
                                        instructions.into_iter().map(|(_, i)| i).collect();
                                    all_methods.push((full_name.clone(), instrs));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("======================================");
    println!(" Bytecode Clone Detection ");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Methods analyzed:     {}", all_methods.len());
    println!();

    let mut clones_found = 0;

    // pairwise similarity comparison
    for i in 0..all_methods.len() {
        for j in (i + 1)..all_methods.len() {
            let (name1, instrs1) = &all_methods[i];
            let (name2, instrs2) = &all_methods[j];
            let sim = calculate_similarity(instrs1, instrs2);
            if sim > 0.9 && sim < 1.0 {
                println!("🚨 Potential Clone Detected (Similarity: {sim:.2}):");
                println!("  - {name1}");
                println!("  - {name2}");
                println!();
                clones_found += 1;
            } else if (sim - 1.0).abs() < f64::EPSILON {
                println!("🚨 Exact Clone Detected (Similarity: 1.00):");
                println!("  - {name1}");
                println!("  - {name2}");
                println!();
                clones_found += 1;
            }
        }
    }

    if clones_found == 0 {
        println!("✅ No clones detected.");
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_clones_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        dump_clones(path_str);
    }
}
