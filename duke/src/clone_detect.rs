#[cfg(feature = "nova")]
use duke_bytecode::{decode, similarity::calculate_similarity};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, ClassFile, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
#[cfg(feature = "nova")]
use std::path::Path;
#[cfg(feature = "nova")]
use std::process;

#[cfg(feature = "nova")]
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

/// Detects structurally similar methods across a JAR file.
///
/// **Why it exists:** Copy-pasting code is a common source of technical debt.
/// This tool allows developers to scan a JAR and find methods that have nearly
/// identical bytecode structures, even if the variables or constants are different.
#[cfg(feature = "nova")]
#[allow(clippy::print_stdout, clippy::cast_precision_loss)]
pub fn dump_clone_detect(jar_path: &str, threshold: f64) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let reader = loader.reader();
    let mut methods_data = Vec::new();

    for entry_name in reader.entry_names() {
        if !std::path::Path::new(&entry_name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("class"))
        {
            continue;
        }
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        #[allow(clippy::collapsible_if)]
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let this_name = resolve_class_name(&cf, cf.this_class);
                for method in &cf.methods {
                    let method_name = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    for attr in &method.attributes {
                        #[allow(clippy::collapsible_if)]
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(decoded) = decode(&code.code) {
                                let instrs: Vec<_> =
                                    decoded.into_iter().map(|(_, instr)| instr).collect();
                                if instrs.len() >= 10 {
                                    // Ignore small methods
                                    methods_data
                                        .push((format!("{this_name}.{method_name}{desc}"), instrs));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("======================================");
    println!(" JAR Clone Detection Analysis");
    println!("======================================");
    println!("File:      {jar_path}");
    println!("Threshold: {threshold}");
    println!("Methods:   {}", methods_data.len());
    println!();

    let mut clones_found = 0;
    for i in 0..methods_data.len() {
        for j in (i + 1)..methods_data.len() {
            let sim = calculate_similarity(&methods_data[i].1, &methods_data[j].1);
            if sim >= threshold {
                println!("🚨 Clone Found (Similarity: {sim:.2}):");
                println!("   - {}", methods_data[i].0);
                println!("   - {}", methods_data[j].0);
                println!();
                clones_found += 1;
            }
        }
    }

    if clones_found == 0 {
        println!("✅ No clones detected at this threshold.");
    } else {
        println!("Total Clones Found: {clones_found}");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "nova")]
    fn test_dump_clone_detect_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        // Just ensure it runs without panicking on a valid JAR.
        super::dump_clone_detect(path_str, 0.9);
    }
}
