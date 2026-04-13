use duke_bytecode::decode;
use duke_classfile::{parse, types::AttributeData};
use duke_loader::{ClassLoader, ZipLoader};
use std::collections::HashMap;
use std::path::Path;

/// Analyzes a JAR file and prints a histogram of JVM opcode frequencies.
///
/// **Why it exists:** Provides developers with insights into what instructions
/// are most commonly used in their codebase, helping identify potential areas
/// for bytecode optimization or analyzing compiler behavior.
#[allow(
    clippy::cast_precision_loss,
    clippy::collapsible_if,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::cast_lossless
)]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
pub fn dump_histogram(jar_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        panic!("duke: failed to open JAR '{jar_path}': {e}");
    });

    let mut histogram: HashMap<&'static str, usize> = HashMap::new();
    let mut total_instructions = 0;

    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(std::string::ToString::to_string)
        .collect();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                for method in &cf.methods {
                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                for (_, instr) in instructions {
                                    let name = instr.mnemonic();
                                    *histogram.entry(name).or_insert(0) += 1;
                                    total_instructions += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut sorted_histogram: Vec<(&'static str, usize)> = histogram.into_iter().collect();
    sorted_histogram.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));

    println!("======================================");
    println!(" Bytecode Histogram Analysis");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Total Instructions:   {total_instructions}");
    println!();
    println!("Opcode Frequencies:");
    for (name, count) in sorted_histogram {
        let percentage = if total_instructions > 0 {
            (count as f64 / total_instructions as f64) * 100.0
        } else {
            0.0
        };
        println!("  {name:<20} {count:>8} ({percentage:>5.2}%)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dump_histogram_missing_file() {
        let result = std::panic::catch_unwind(|| {
            dump_histogram("non_existent_file_for_histogram.jar");
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_dump_histogram_valid_file() {
        let path1 = "../tests/fixtures/hello.jar";
        let path2 = "tests/fixtures/hello.jar";
        let path = if std::path::Path::new(path1).exists() {
            path1
        } else if std::path::Path::new(path2).exists() {
            path2
        } else {
            panic!("Could not find hello.jar fixture");
        };

        // Ensure it doesn't panic on a valid JAR
        dump_histogram(path);
    }
}
