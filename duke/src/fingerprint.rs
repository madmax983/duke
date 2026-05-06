#![allow(clippy::items_after_statements)]
#[cfg(feature = "nova")]
use duke_bytecode::decode;
#[cfg(feature = "nova")]
use duke_classfile::{
    parse,
    types::{AttributeData, CpEntry, CpIndex},
};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::process;

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

/// Fingerprints methods in a JAR file based on their bytecode structure.
///
/// **Why it exists:** When auditing large third-party JARs, you often find vendored
/// or duplicated logic (e.g. shadowed libraries or copy-pasted utility classes).
/// This function generates a hash based *only* on the sequence of instruction mnemonics
/// (ignoring constant pool indices, branch offsets, and local variables). Methods with
/// identical structural hashes are highly likely to be duplicates, even if compiled
/// under different circumstances or obfuscated.
///
/// # Examples
///
/// ```ignore
/// use duke::fingerprint::dump_fingerprints;
///
/// dump_fingerprints("legacy_lib.jar");
/// ```
#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if,
    clippy::cast_precision_loss
)]
pub fn dump_fingerprints(jar_path: &str) {
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

    let mut fingerprints: HashMap<u64, Vec<String>> = HashMap::new();
    let mut total_methods = 0;

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");

                    // Ignore very short methods (like getters/setters/empty constructors) to reduce noise
                    if name_str == "<init>" || name_str == "<clinit>" {
                        continue;
                    }

                    let full_name = format!("{class_name_internal}.{name_str}{desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                // Filter out extremely short methods (less than 10 instructions)
                                if instructions.len() < 10 {
                                    continue;
                                }

                                total_methods += 1;
                                let mut hasher = DefaultHasher::new();
                                for (_, instr) in &instructions {
                                    instr.mnemonic().hash(&mut hasher);
                                }
                                let hash = hasher.finish();

                                fingerprints
                                    .entry(hash)
                                    .or_default()
                                    .push(full_name.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    println!("======================================");
    println!(" Structural Method Fingerprinting");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Methods analyzed:     {total_methods} (ignoring short/init methods)");
    println!();

    let mut found_duplicates = false;
    let mut entries: Vec<_> = fingerprints
        .into_iter()
        .filter(|(_, methods)| methods.len() > 1)
        .collect();
    // Sort by number of duplicates (descending)
    entries.sort_by_key(|(_, methods)| std::cmp::Reverse(methods.len()));

    for (hash, methods) in entries {
        found_duplicates = true;
        println!("Hash {hash:016x} appears {} times:", methods.len());
        for method in methods {
            println!("  - {method}");
        }
        println!();
    }

    if !found_duplicates {
        println!("✅ No structurally identical methods found (no duplication detected).");
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_fingerprints_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        dump_fingerprints(path_str);
    }
}
