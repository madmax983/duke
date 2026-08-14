#![allow(clippy::items_after_statements)]
#[cfg(feature = "nova")]
use duke_bytecode::cyclomatic_complexity;
#[cfg(feature = "nova")]
use duke_bytecode::decode;
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, ClassFile, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::path::Path;

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
#[allow(clippy::print_stdout, clippy::collapsible_if, clippy::use_debug)]
pub fn dump_jar_complexity(jar_path: &str) -> Result<(), String> {
    let loader = ZipLoader::open(Path::new(jar_path))
        .map_err(|e| format!("duke: failed to open JAR '{jar_path}': {e}"))?;

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

    let mut method_complexities = Vec::new();

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
                                let cc = cyclomatic_complexity(&instructions);
                                method_complexities.push((
                                    full_name.clone(),
                                    cc,
                                    instructions.len(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    method_complexities.sort_by(|a, b| b.1.cmp(&a.1));

    println!("======================================");
    println!(" JAR Cyclomatic Complexity Analysis");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!();

    if method_complexities.is_empty() {
        println!("No methods found.");
    } else {
        println!("Top 10 Most Complex Methods:");
        for (i, (method, cc, instr_len)) in method_complexities.iter().take(10).enumerate() {
            println!("  {}. {} (CC: {}, {} instrs)", i + 1, method, cc, instr_len);
        }
    }
    Ok(())
}

#[cfg(feature = "nova")]
#[allow(clippy::print_stdout, clippy::collapsible_if, clippy::use_debug)]
pub fn dump_complexity(path: &str) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("duke: cannot read '{path}': {e}"))?;
    let cf = parse(&bytes).map_err(|e| format!("duke: parse error: {e}"))?;

    let mut method_complexities = Vec::new();
    let this_class = resolve_class_name(&cf, cf.this_class);

    for method in &cf.methods {
        let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
        let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
        let full_name = format!("{this_class}::{name_str}{desc_str}");

        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                if let Ok(instructions) = decode(&code.code) {
                    let cc = cyclomatic_complexity(&instructions);
                    method_complexities.push((full_name.clone(), cc, instructions.len()));
                }
            }
        }
    }

    method_complexities.sort_by(|a, b| b.1.cmp(&a.1));

    println!("======================================");
    println!(" Class Cyclomatic Complexity Analysis");
    println!("======================================");
    println!("File:                 {path}");
    println!();

    if method_complexities.is_empty() {
        println!("No methods found.");
    } else {
        println!("Top 10 Most Complex Methods:");
        for (i, (method, cc, instr_len)) in method_complexities.iter().take(10).enumerate() {
            println!("  {}. {} (CC: {}, {} instrs)", i + 1, method, cc, instr_len);
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_jar_complexity_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../spring-boot-loader-3.5.12.jar");
        let path_str = path.to_str().unwrap();

        assert!(dump_jar_complexity(path_str).is_ok());
    }

    #[test]
    fn test_dump_complexity_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/MainHello.class");
        let path_str = path.to_str().unwrap();

        assert!(dump_complexity(path_str).is_ok());
    }
}
