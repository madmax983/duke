#![allow(clippy::items_after_statements)]

#[cfg(feature = "nova")]
use duke_bytecode::{calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, CpEntry, CpIndex, parse};
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
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if
)]
pub fn dump_method_similarity(jar_path: &str, method_a: &str, method_b: &str) {
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

    let mut instructions_a = None;
    let mut instructions_b = None;

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let this_class = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{this_class}::{name_str}{desc_str}");

                    if full_name == method_a || full_name == method_b {
                        for attr in &method.attributes {
                            if let AttributeData::Code(code) = &attr.data {
                                if let Ok(decoded) = decode(&code.code) {
                                    let instrs: Vec<_> =
                                        decoded.into_iter().map(|(_, i)| i).collect();
                                    if full_name == method_a {
                                        instructions_a = Some(instrs.clone());
                                    }
                                    if full_name == method_b {
                                        instructions_b = Some(instrs.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    match (instructions_a, instructions_b) {
        (Some(a), Some(b)) => {
            let sim = calculate_similarity(&a, &b);
            println!("======================================");
            println!(" Method Similarity Analysis");
            println!("======================================");
            println!("Method A: {method_a}");
            println!("Method B: {method_b}");
            println!("Similarity Score: {:.2}%", sim * 100.0);
        }
        (None, _) => {
            eprintln!("duke: method A '{method_a}' not found or has no code attribute");
            process::exit(1);
        }
        (_, None) => {
            eprintln!("duke: method B '{method_b}' not found or has no code attribute");
            process::exit(1);
        }
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_method_similarity_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        dump_method_similarity(
            path_str,
            "HelloWorld::main([Ljava/lang/String;)V",
            "HelloWorld::<init>()V",
        );
    }
}
