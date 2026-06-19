#![allow(clippy::items_after_statements)]
#![allow(clippy::case_sensitive_file_extension_comparisons)]

#[cfg(feature = "nova")]
use duke_classfile::{
    parse,
    AttributeData, CpEntry, CpIndex,
};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn cp_str(cf: &duke_classfile::ClassFile, idx: CpIndex) -> Option<&str> {
    cf.constant_pool
        .get(idx.0 as usize)
        .and_then(|slot: &Option<CpEntry>| slot.as_ref())
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
        .and_then(|s: &Option<CpEntry>| s.as_ref());
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
pub fn dump_jar_diff(jar1_path: &str, jar2_path: &str, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
    let map1 = load_jar_methods(jar1_path);
    let map2 = load_jar_methods(jar2_path);

    let keys1: HashSet<_> = map1.keys().collect();
    let keys2: HashSet<_> = map2.keys().collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();
    let mut unchanged = 0;

    for k in keys2.difference(&keys1) {
        added.push((*k).clone());
    }

    for k in keys1.difference(&keys2) {
        removed.push((*k).clone());
    }

    for k in keys1.intersection(&keys2) {
        if map1.get(*k) == map2.get(*k) {
            unchanged += 1;
        } else {
            modified.push((*k).clone());
        }
    }

    added.sort();
    removed.sort();
    modified.sort();

    writeln!(writer, "======================================")?;
    writeln!(writer, " JAR Bytecode Diff Analysis")?;
    writeln!(writer, "======================================")?;
    writeln!(writer, "File 1:               {jar1_path}")?;
    writeln!(writer, "File 2:               {jar2_path}")?;
    writeln!(writer, "Methods Added:        {}", added.len())?;
    writeln!(writer, "Methods Removed:      {}", removed.len())?;
    writeln!(writer, "Methods Modified:     {}", modified.len())?;
    writeln!(writer, "Methods Unchanged:    {unchanged}")?;
    writeln!(writer)?;

    if !added.is_empty() {
        writeln!(writer, "--- Top 10 Added Methods ---")?;
        for m in added.iter().take(10) {
            writeln!(writer, "  + {m}")?;
        }
        if added.len() > 10 {
            writeln!(writer, "  ... and {} more", added.len() - 10)?;
        }
        writeln!(writer)?;
    }

    if !removed.is_empty() {
        writeln!(writer, "--- Top 10 Removed Methods ---")?;
        for m in removed.iter().take(10) {
            writeln!(writer, "  - {m}")?;
        }
        if removed.len() > 10 {
            writeln!(writer, "  ... and {} more", removed.len() - 10)?;
        }
        writeln!(writer)?;
    }

    if !modified.is_empty() {
        writeln!(writer, "--- Top 10 Modified Methods ---")?;
        for m in modified.iter().take(10) {
            writeln!(writer, "  ~ {m}")?;
        }
        if modified.len() > 10 {
            writeln!(writer, "  ... and {} more", modified.len() - 10)?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
pub fn load_jar_methods(jar_path: &str) -> HashMap<String, u64> {
    let Ok(loader) = ZipLoader::open(Path::new(jar_path)) else {
        return HashMap::new(); // If we cannot load, treat as empty
    };

    let mut method_hashes = HashMap::new();
    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(std::string::ToString::to_string)
        .collect();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            #[allow(clippy::collapsible_if)]
            if let Ok(cf) = parse(&bytes) {
                let class_name = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{class_name}::{name_str}{desc_str}");

                    let mut hasher = DefaultHasher::new();
                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            code.code.hash(&mut hasher);
                        }
                    }
                    method_hashes.insert(full_name, hasher.finish());
                }
            }
        }
    }
    method_hashes
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_jar_diff_dummy() {
        let path1 = "dummy1.jar";
        let path2 = "dummy2.jar";

        let mut buf = Vec::new();
        dump_jar_diff(path1, path2, &mut buf).unwrap();

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("File 1:               dummy1.jar"));
        assert!(output.contains("Methods Added:        0"));
    }

    #[test]
    fn test_load_jar_methods_invalid_path() {
        let result = load_jar_methods("invalid_path_does_not_exist.jar");
        assert!(result.is_empty());
    }

    #[test]
    fn test_cp_str_invalid() {
        // Provide enough context to reach branch statements on resolving class name and invalid cp elements
        use duke_classfile::{ClassFile, CpIndex, CpEntry};
        let cf = ClassFile {
            minor_version: 0,
            major_version: 0,
            constant_pool: vec![
                None, // 0
                Some(CpEntry::Utf8("test".to_string())), // 1
                Some(CpEntry::Integer(42)), // 2
                Some(CpEntry::Class { name_index: CpIndex(1) }), // 3
                Some(CpEntry::Class { name_index: CpIndex(2) }), // 4
            ],
            access_flags: duke_classfile::ClassAccessFlags::empty(),
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        assert_eq!(cp_str(&cf, CpIndex(1)), Some("test"));
        assert_eq!(cp_str(&cf, CpIndex(2)), None);
        assert_eq!(cp_str(&cf, CpIndex(99)), None);

        assert_eq!(resolve_class_name(&cf, CpIndex(0)), "<none>");
        assert_eq!(resolve_class_name(&cf, CpIndex(2)), "<not a class ref>");
        assert_eq!(resolve_class_name(&cf, CpIndex(3)), "test");
        assert_eq!(resolve_class_name(&cf, CpIndex(4)), "<invalid utf8>");
    }

    struct FailingWriter;
    impl std::io::Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("mock error"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_dump_jar_diff_io_error() {
        let mut writer = FailingWriter;
        let res = dump_jar_diff("a", "b", &mut writer);
        assert!(res.is_err());
    }
}
