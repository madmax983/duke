#![allow(clippy::items_after_statements)]
#[cfg(feature = "nova")]
use duke_bytecode::{calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, ClassFile, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::path::Path;

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs, dead_code)]
fn cp_str(cf: &ClassFile, idx: CpIndex) -> Option<&str> {
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
fn resolve_class_name(cf: &ClassFile, idx: CpIndex) -> String {
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
#[derive(Debug, Clone)]
struct MethodInfo {
    class_name: String,
    method_name: String,
    descriptor: String,
    instructions: Vec<duke_bytecode::Instruction>,
}

#[cfg(feature = "nova")]
impl MethodInfo {
    fn full_name(&self) -> String {
        format!(
            "{}::{}{}",
            self.class_name, self.method_name, self.descriptor
        )
    }
}

#[cfg(feature = "nova")]
#[allow(clippy::print_stdout, clippy::collapsible_if, clippy::use_debug)]
pub fn dump_clone_detect(path: &str, threshold: f64) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        panic!("duke: cannot read '{path}': {e}");
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        panic!("duke: parse error: {e}");
    });

    let methods = extract_methods(&cf);
    analyze_clones(&methods, threshold);
}

#[cfg(feature = "nova")]
#[allow(clippy::print_stdout, clippy::collapsible_if, clippy::use_debug)]
pub fn dump_jar_clone_detect(jar_path: &str, threshold: f64) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        panic!("duke: failed to open JAR '{jar_path}': {e}");
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

    let mut all_methods = Vec::new();

    for entry_name in class_entries {
        let class_name_internal = &entry_name[..entry_name.len() - 6];
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                all_methods.extend(extract_methods(&cf));
            }
        }
    }

    analyze_clones(&all_methods, threshold);
}

#[cfg(feature = "nova")]
fn extract_methods(cf: &ClassFile) -> Vec<MethodInfo> {
    let mut extracted = Vec::new();
    let this_class = resolve_class_name(cf, cf.this_class);

    for method in &cf.methods {
        let name_str = cp_str(cf, method.name_index).unwrap_or("<invalid>");
        let desc_str = cp_str(cf, method.descriptor_index).unwrap_or("<invalid>");

        for attr in &method.attributes {
            #[allow(clippy::collapsible_if)]
            if let AttributeData::Code(code) = &attr.data {
                if let Ok(instructions_with_pc) = decode(&code.code) {
                    // Only consider methods with at least a few instructions to avoid trivial clones
                    if instructions_with_pc.len() > 3 {
                        let instructions: Vec<_> =
                            instructions_with_pc.into_iter().map(|(_, i)| i).collect();
                        extracted.push(MethodInfo {
                            class_name: this_class.clone(),
                            method_name: name_str.to_string(),
                            descriptor: desc_str.to_string(),
                            instructions,
                        });
                    }
                }
            }
        }
    }
    extracted
}

#[cfg(feature = "nova")]
#[allow(clippy::print_stdout, clippy::cast_precision_loss)]
fn analyze_clones(methods: &[MethodInfo], threshold: f64) {
    println!("======================================");
    println!(" Bytecode Clone Detection");
    println!("======================================");
    println!("Methods analyzed: {}", methods.len());
    println!("Similarity threshold: {threshold:.2}");
    println!();

    let mut found_clones = false;
    let len = methods.len();

    for i in 0..len {
        for j in (i + 1)..len {
            let m1 = &methods[i];
            let m2 = &methods[j];

            let similarity = calculate_similarity(&m1.instructions, &m2.instructions);

            if similarity >= threshold {
                found_clones = true;
                println!("🚨 Clone Detected (Similarity: {similarity:.2}):");
                println!("  - {}", m1.full_name());
                println!("  - {}", m2.full_name());
                println!();
            }
        }
    }

    if !found_clones {
        println!("✅ No clones detected above the threshold.");
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_clone_detect_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/HelloWorld.class");
        let path_str = path.to_str().unwrap();

        dump_clone_detect(path_str, 0.9);
    }

    #[test]
    fn test_dump_jar_clone_detect_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        dump_jar_clone_detect(path_str, 0.8);
    }
}
