#![allow(
    clippy::items_after_statements,
    clippy::cast_precision_loss,
    clippy::collapsible_if,
    clippy::print_stdout,
    clippy::case_sensitive_file_extension_comparisons
)]
#[cfg(feature = "nova")]
use duke_bytecode::{calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, CpEntry, CpIndex};
#[cfg(feature = "nova")]
use duke_loader::ZipLoader;
use std::path::Path;
use std::process;

#[cfg(feature = "nova")]
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
pub fn dump_clone_hunt(jar_path: &str, threshold: f64) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let mut methods = Vec::new();

    for name in loader.reader().entry_names() {
        if !name.ends_with(".class") {
            continue;
        }
        let Ok(class_bytes) = loader.reader().read_entry(name) else {
            continue;
        };
        let Ok(cf) = duke_classfile::parse(&class_bytes) else {
            continue;
        };
        let class_name = resolve_class_name(&cf, cf.this_class);

        for method in &cf.methods {
            let method_name = cp_str(&cf, method.name_index).unwrap_or("<unknown>");
            for attr in &method.attributes {
                if let AttributeData::Code(code) = &attr.data {
                    if let Ok(decoded) = decode(&code.code) {
                        let instrs: Vec<_> = decoded.into_iter().map(|(_, i)| i).collect();
                        if instrs.len() >= 10 {
                            // Only consider methods with >= 10 instructions
                            methods.push((class_name.clone(), method_name.to_string(), instrs));
                        }
                    }
                }
            }
        }
    }

    println!("Found {} methods with >= 10 instructions.", methods.len());
    println!("Hunting for clones with similarity >= {threshold:.2}...");

    let mut found = 0;
    for i in 0..methods.len() {
        for j in (i + 1)..methods.len() {
            let m1 = &methods[i];
            let m2 = &methods[j];
            // Quick length filter to avoid expensive Levenshtein if sizes are vastly different
            let len1 = m1.2.len() as f64;
            let len2 = m2.2.len() as f64;
            if len1 > len2 * (2.0 - threshold) || len2 > len1 * (2.0 - threshold) {
                continue; // Cannot be similar
            }

            let sim = calculate_similarity(&m1.2, &m2.2);
            if sim >= threshold {
                println!("Match: {:.2}% similarity", sim * 100.0);
                println!("  1. {}::{}", m1.0, m1.1);
                println!("  2. {}::{}", m2.0, m2.1);
                found += 1;
            }
        }
    }

    if found == 0 {
        println!("No clones found above threshold.");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dummy() {}
}
