#![allow(clippy::items_after_statements)]
#![allow(
    clippy::case_sensitive_file_extension_comparisons,
    clippy::collapsible_if,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]
use duke_bytecode::{cyclomatic_complexity, decode};
use duke_classfile::{
    parse, {AttributeData, CpEntry, CpIndex},
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

/// Prints a static analysis summary of an entire JAR file to standard output.
///
/// **Why it exists:** When auditing large third-party JARs or legacy codebases,
/// it's crucial to identify the most problematic areas. This function acts as a
/// "heatmap" by finding and highlighting the most complex methods across the entire archive.
///
/// This iterates over all classes and methods in the JAR, computing code size
/// and cyclomatic complexity, then prints a summary including the top 10 most
/// complex methods.
///
/// # Examples
///
/// ```ignore
/// // Assumes 'legacy_lib.jar' is available on disk
/// use duke::jar_analyze::dump_jar_analyze;
///
/// dump_jar_analyze("legacy_lib.jar");
/// ```
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
pub fn dump_jar_analyze(jar_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let mut total_classes = 0;
    let mut total_methods = 0;
    let mut total_bytecode_bytes = 0;

    #[cfg(not(tarpaulin_include))]
    #[allow(unexpected_cfgs)]
    struct MethodRecord {
        class_name: String,
        method_name: String,
        complexity: usize,
    }
    let mut all_methods = Vec::new();

    let reader = loader.reader();

    for entry_name in reader.entry_names() {
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        if !entry_name.ends_with(".class") {
            continue;
        }
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        let Ok(bytes) = loader.find_class(class_name_internal) else {
            continue;
        };
        let Ok(cf) = parse(&bytes) else {
            continue;
        };

        total_classes += 1;
        let class_name = resolve_class_name(&cf, cf.this_class);

        for method in &cf.methods {
            total_methods += 1;
            let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
            let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
            let full_name = format!("{name_str}{desc_str}");

            let mut complexity = 0;
            for attr in &method.attributes {
                let AttributeData::Code(code) = &attr.data else {
                    continue;
                };
                total_bytecode_bytes += code.code.len();
                let Ok(instructions) = decode(&code.code) else {
                    continue;
                };
                complexity = cyclomatic_complexity(&instructions);
            }

            all_methods.push(MethodRecord {
                class_name: class_name.clone(),
                method_name: full_name,
                complexity,
            });
        }
    }

    all_methods.sort_by_key(|b| std::cmp::Reverse(b.complexity));

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
    use duke_classfile::ClassAccessFlags;
    use duke_classfile::ClassFile;

    #[test]
    fn test_dump_jar_analyze_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path = path.to_str().unwrap();

        super::dump_jar_analyze(path);
    }

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
