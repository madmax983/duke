#![allow(clippy::items_after_statements)]

#[cfg(feature = "nova")]
use duke_bytecode::{cyclomatic_complexity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
#[cfg(feature = "nova")]
use std::fs::File;
#[cfg(feature = "nova")]
use std::io::Write;
#[cfg(feature = "nova")]
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
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if
)]
pub fn export_jar_metrics_csv(jar_path: &str, out_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        panic!("duke: failed to open JAR '{jar_path}': {e}");
    });

    let mut out_file = File::create(out_path).unwrap_or_else(|e| {
        panic!("duke: failed to create output file '{out_path}': {e}");
    });

    writeln!(out_file, "Class,Method,Complexity,CodeSize,IsPure").unwrap();

    let reader = loader.reader();
    for entry_name in reader.entry_names() {
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        if !entry_name.ends_with(".class") {
            continue;
        }
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let class_name = class_name_internal.replace('/', ".");

                for method in &cf.methods {
                    let method_name = cp_str(&cf, method.name_index).unwrap_or("<unknown>");

                    let mut complexity = 0;
                    let mut code_size = 0;
                    let mut is_pure = true;

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            code_size = code.code.len();
                            if let Ok(instructions) = decode(&code.code) {
                                complexity = cyclomatic_complexity(&instructions);

                                for (_, instr) in &instructions {
                                    let mnemonic = instr.mnemonic();
                                    if mnemonic == "putstatic"
                                        || mnemonic == "putfield"
                                        || mnemonic == "invokevirtual"
                                        || mnemonic == "invokespecial"
                                        || mnemonic == "invokestatic"
                                        || mnemonic == "invokeinterface"
                                        || mnemonic == "invokedynamic"
                                    {
                                        is_pure = false;
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    writeln!(
                        out_file,
                        "{class_name},{method_name},{complexity},{code_size},{is_pure}"
                    )
                    .unwrap();
                }
            }
        }
    }

    println!("Exported metrics to {out_path}");
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_export_jar_metrics_csv_missing_jar() {
        let result = std::panic::catch_unwind(|| {
            export_jar_metrics_csv("non_existent.jar", "out.csv");
        });
        assert!(result.is_err());
    }
}
