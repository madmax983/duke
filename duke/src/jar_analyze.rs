#![allow(clippy::items_after_statements)]
#![allow(clippy::case_sensitive_file_extension_comparisons, clippy::collapsible_if, clippy::cast_precision_loss, clippy::cast_lossless)]
use duke_bytecode::{cyclomatic_complexity, decode};
use duke_classfile::{
    parse,
    types::{AttributeData, CpEntry, CpIndex},
};
use duke_loader::{ClassLoader, ZipLoader};
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

pub fn dump_jar_analyze(jar_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let mut total_classes = 0;
    let mut total_methods = 0;
    let mut total_bytecode_bytes = 0;

    struct MethodRecord {
        class_name: String,
        method_name: String,
        complexity: usize,
    }
    let mut all_methods = Vec::new();

    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class")) // #[allow(clippy::case_sensitive_file_extension_comparisons)] is cleaner, let us just allow it via macro
        .map(std::string::ToString::to_string)
        .collect();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                total_classes += 1;
                let class_name = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    total_methods += 1;
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{name_str}{desc_str}");

                    let mut complexity = 0;
                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            total_bytecode_bytes += code.code.len();
                            if let Ok(instructions) = decode(&code.code) {
                                complexity = cyclomatic_complexity(&instructions);
                            }
                        }
                    }

                    all_methods.push(MethodRecord {
                        class_name: class_name.clone(),
                        method_name: full_name,
                        complexity,
                    });
                }
            }
        }
    }

    all_methods.sort_by(|a, b| b.complexity.cmp(&a.complexity));

    println!("======================================");
    println!(" JAR Analysis Summary");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Total Classes:        {total_classes}");
    println!("Total Methods:        {total_methods}");
    println!("Total Bytecode Size:  {total_bytecode_bytes} bytes");
    if total_methods > 0 {
        let avg_comp =
            all_methods.iter().map(|m| m.complexity).sum::<usize>() as f64 / total_methods as f64;
        println!("Avg Complexity:       {avg_comp:.2}");
    }
    println!();
    println!("Top 10 Most Complex Methods:");
    for (i, m) in all_methods.iter().take(10).enumerate() {
        let rank = i + 1;
        println!(
            "  {rank:>2}. {}::{} (Complexity: {})",
            m.class_name, m.method_name, m.complexity
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::access_flags::ClassAccessFlags;
    use duke_classfile::types::ClassFile;

    #[test]
    fn test_resolve_class_name_invalid_index() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![None],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(1),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        assert_eq!(resolve_class_name(&cf, CpIndex(1)), "<not a class ref>");
        assert_eq!(resolve_class_name(&cf, CpIndex(0)), "<none>");
    }
}
