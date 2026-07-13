#![allow(clippy::items_after_statements)]
#![allow(
    clippy::case_sensitive_file_extension_comparisons,
    clippy::collapsible_if,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]
use duke_bytecode::{cyclomatic_complexity, decode};
use duke_classfile::{
    parse, AttributeData, CpEntry, CpIndex,
};
use duke_loader::{ClassLoader, ZipLoader};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process;

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

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
pub fn dump_metrics_csv(jar_path: &str, output_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let mut out_file = File::create(output_path).unwrap_or_else(|e| {
        eprintln!("duke: failed to create output file '{output_path}': {e}");
        process::exit(1);
    });

    writeln!(out_file, "Class,Method,Complexity,BytecodeSize").unwrap();

    let reader = loader.reader();

    for entry_name in reader.entry_names() {
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        if !entry_name.ends_with(".class") {
            continue;
        }
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let class_name = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{name_str}{desc_str}");

                    let mut complexity = 0;
                    let mut size = 0;
                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            size += code.code.len();
                            if let Ok(instructions) = decode(&code.code) {
                                complexity = cyclomatic_complexity(&instructions);
                            }
                        }
                    }

                    writeln!(out_file, "{class_name},{full_name},{complexity},{size}").unwrap();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_dump_metrics_csv() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path = path.to_str().unwrap();

        let temp_file = NamedTempFile::new().unwrap();
        let out_path = temp_file.path().to_str().unwrap();

        super::dump_metrics_csv(path, out_path);

        let content = fs::read_to_string(out_path).unwrap();
        assert!(content.contains("Class,Method,Complexity,BytecodeSize"));
    }
}
