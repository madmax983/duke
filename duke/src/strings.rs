#![allow(clippy::items_after_statements)]

#[cfg(feature = "nova")]
use duke_bytecode::decode;
#[cfg(feature = "nova")]
use duke_bytecode::Instruction;
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

/// Analyzes a JAR file to extract all meaningful hardcoded strings (`LDC/LDC_W`).
///
/// **Why it exists:** Reverse-engineering, auditing, or translating a Java app often requires
/// dumping all textual strings baked into the bytecode (e.g., UI labels, error messages, SQL queries).
/// This provides an instant "strings" equivalent specifically for Java constant pools, ignoring
/// structural strings like method names and focusing only on runtime values loaded into the stack.
#[cfg(feature = "nova")]
#[allow(
    clippy::print_stdout,
    clippy::collapsible_if,
    clippy::use_debug,
    clippy::items_after_statements,
    clippy::single_match
)]
pub fn dump_jar_strings(jar_path: &str) {
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

    let mut all_strings = Vec::new();

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
                                for (pc, instr) in instructions {
                                    let mut string_idx: Option<CpIndex> = None;

                                    match instr {
                                        Instruction::Ldc(idx) => {
                                            string_idx = Some(CpIndex(idx.into()));
                                        }
                                        Instruction::LdcW(idx) => {
                                            string_idx = Some(idx);
                                        }
                                        _ => {}
                                    }

                                    if let Some(idx) = string_idx {
                                        if let Some(CpEntry::String { string_index }) = cf.constant_pool.get(idx.0 as usize).and_then(|slot| slot.as_ref()) {
                                            if let Some(s) = cp_str(&cf, *string_index) {
                                                if !s.is_empty() { // Meaningful strings only
                                                    all_strings.push((s.to_string(), full_name.clone(), pc));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("======================================");
    println!(" JAR Hardcoded Strings Extractor");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Total Strings Found:  {}", all_strings.len());
    println!();

    if all_strings.is_empty() {
        println!("No hardcoded strings found.");
    } else {
        for (text, location, pc) in all_strings {
            println!("  \"{}\"", text.replace('\n', "\\n").replace('\r', "\\r"));
            println!("    └─ {location} @ {pc}");
        }
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_jar_strings_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        // Just ensure it runs without panicking on a valid JAR.
        dump_jar_strings(path_str);
    }
}
