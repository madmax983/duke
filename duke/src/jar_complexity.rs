#[cfg(feature = "nova")]
use duke_bytecode::{cyclomatic_complexity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{
    parse,
    types::{AttributeData, CpEntry, CpIndex},
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

/// Computes cyclomatic complexity for all methods in a JAR.
///
/// **Why it exists:** Provides insight into the maintainability and testing
/// burden of code by calculating `McCabe`'s Cyclomatic Complexity per method.
#[cfg(feature = "nova")]
#[allow(clippy::print_stdout, clippy::collapsible_if, clippy::use_debug, clippy::doc_markdown, clippy::cast_precision_loss, clippy::cast_lossless)]
pub fn dump_jar_complexity(jar_path: &str) {
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

    let mut complexities = Vec::new();
    let mut total_complexity = 0;
    let mut method_count = 0;

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
                                let c = cyclomatic_complexity(&instructions);
                                complexities.push((full_name.clone(), c));
                                total_complexity += c;
                                method_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    complexities.sort_by(|a, b| b.1.cmp(&a.1));

    println!("======================================");
    println!(" JAR Cyclomatic Complexity Analysis");
    println!("======================================");
    println!("File:                 {jar_path}");
    if method_count > 0 {
        println!("Average Complexity:   {:.2}", (total_complexity as f64) / f64::from(method_count));
    } else {
        println!("Average Complexity:   N/A");
    }
    println!();

    let top_n = std::cmp::min(10, complexities.len());
    if top_n > 0 {
        println!("Top {top_n} most complex methods:");
        for (name, score) in complexities.iter().take(top_n) {
            let bar: String = "█".repeat(*score);
            println!("{score:>5} | {name} {bar}");
        }
    } else {
        println!("No methods found.");
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_jar_complexity() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        dump_jar_complexity(path_str);
    }
}
