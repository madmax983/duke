#![allow(clippy::items_after_statements)]
#[cfg(feature = "nova")]
use duke_classfile::{CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::path::Path;
use std::process;

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn cp_str(cf: &duke_classfile::ClassFile, idx: CpIndex) -> Option<&str> {
    cf.constant_pool
        .get(idx.0 as usize)
        .and_then(|slot: &Option<CpEntry>| slot.as_ref())
        .and_then(|entry: &CpEntry| {
            if let CpEntry::Utf8(s) = entry {
                Some(s.as_str())
            } else {
                None
            }
        })
}

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if
)]
pub fn dump_jar_strings(jar_path: &str, query: &str) {
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

    let query_lower = query.to_lowercase();
    let mut found_any = false;

    println!("Searching for literal strings containing '{query}' in {jar_path}");

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let mut found_in_class = false;

                for (i, slot) in cf.constant_pool.iter().enumerate() {
                    if let Some(entry) = slot {
                        // Check if it's a CONSTANT_String and resolve its UTF-8 value
                        if let CpEntry::String { string_index } = entry {
                            if let Some(s) = cp_str(&cf, *string_index) {
                                if s.to_lowercase().contains(&query_lower) {
                                    if !found_in_class {
                                        println!("\nClass: {class_name_internal}");
                                        found_in_class = true;
                                        found_any = true;
                                    }
                                    println!("  [CP #{i}] CONSTANT_String: \"{s}\"");
                                }
                            }
                        }
                        // Also check for raw Utf8 strings
                        else if let CpEntry::Utf8(s) = entry {
                            if s.to_lowercase().contains(&query_lower) {
                                if !found_in_class {
                                    println!("\nClass: {class_name_internal}");
                                    found_in_class = true;
                                    found_any = true;
                                }
                                println!("  [CP #{i}] CONSTANT_Utf8: \"{s}\"");
                            }
                        }
                    }
                }
            }
        }
    }

    if !found_any {
        println!("\nNo strings matching '{query}' found.");
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

        dump_jar_strings(path_str, "hello");
    }
}
