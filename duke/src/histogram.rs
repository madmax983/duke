use duke_bytecode::decode;
use duke_classfile::{AttributeData, parse};
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
        for method in &cf.methods {
            for attr in &method.attributes {
                let AttributeData::Code(code) = &attr.data else {
                    continue;
                };
                let Ok(instructions) = decode(&code.code) else {
                    continue;
                };
                for (_, instr) in instructions {
                    let name = instr.mnemonic();
                    *histogram.entry(name).or_insert(0) += 1;
                    total_instructions += 1;
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
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path = path.to_str().unwrap();

        // Ensure it doesn't panic on a valid JAR
        dump_histogram(path);
    }
}
