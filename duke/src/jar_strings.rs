#![allow(clippy::items_after_statements)]
#[cfg(feature = "nova")]
use duke_classfile::{CpEntry, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process;

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if
)]
pub fn dump_jar_strings(jar_path: &str, min_length: usize) {
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

    let mut string_map: HashMap<String, HashSet<String>> = HashMap::new();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                for entry in &cf.constant_pool {
                    if let Some(CpEntry::String { string_index }) = entry {
                        if let Some(Some(CpEntry::Utf8(s))) =
                            cf.constant_pool.get(string_index.0 as usize)
                        {
                            if s.len() >= min_length {
                                string_map
                                    .entry(s.clone())
                                    .or_default()
                                    .insert(class_name_internal.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    let mut sorted_strings: Vec<_> = string_map.into_iter().collect();
    sorted_strings.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));

    println!("======================================");
    println!(" JAR String Constant Extractor");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Min Length:           {min_length}");
    println!("Unique Strings Found: {}", sorted_strings.len());
    println!();

    if sorted_strings.is_empty() {
        println!("No string constants found matching criteria.");
    } else {
        for (s, classes) in sorted_strings {
            let mut class_list: Vec<_> = classes.into_iter().collect();
            class_list.sort(); // Predictable order for tests
            let mut class_display = class_list.join(", ");
            if class_display.len() > 60 {
                class_display.truncate(57);
                class_display.push_str("...");
            }
            let display_s = s.replace('\n', "\\n").replace('\r', "\\r");
            println!("\"{display_s}\"");
            println!("    (Found in: {class_display})");
            println!();
        }
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_jar_strings() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        dump_jar_strings(path_str, 0);
    }
}
