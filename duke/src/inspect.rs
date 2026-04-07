use duke_bytecode::{cyclomatic_complexity, decode};
use duke_classfile::{parse, types::AttributeData};
use duke_loader::ZipReader;
use std::fs;

#[allow(clippy::cast_precision_loss, clippy::case_sensitive_file_extension_comparisons, clippy::collapsible_if)]
pub fn inspect_jar(path: &str) {
    let bytes = fs::read(path).expect("Failed to read JAR file");
    let zip = ZipReader::from_bytes(bytes).expect("Failed to parse JAR/ZIP");

    let mut total_classes = 0;
    let mut total_methods = 0;
    let mut total_fields = 0;
    let mut total_instructions = 0;
    let mut total_complexity = 0;

    let mut max_complexity = 0;
    let mut max_complex_method = String::new();

    let mut names: Vec<&str> = zip.entry_names().collect();
    names.sort_unstable(); // For deterministic parsing in tests

    for name in names {
        if name.ends_with(".class") {
            if let Ok(class_bytes) = zip.read_entry(name) {
                if let Ok(cf) = parse(&class_bytes) {
                    total_classes += 1;
                    total_fields += cf.fields.len();
                    total_methods += cf.methods.len();

                    let class_name = name.replace(".class", "").replace('/', ".");

                    for method in &cf.methods {
                        let method_name = cf
                            .constant_pool
                            .get(method.name_index.0 as usize)
                            .and_then(|e| e.as_ref())
                            .and_then(|e| {
                                if let duke_classfile::types::CpEntry::Utf8(s) = e {
                                    Some(s)
                                } else {
                                    None
                                }
                            })
                            .map_or("<unknown>", std::string::String::as_str);

                        for attr in &method.attributes {
                            if let AttributeData::Code(code) = &attr.data {
                                if let Ok(instructions) = decode(&code.code) {
                                    total_instructions += instructions.len();
                                    let comp = cyclomatic_complexity(&instructions);
                                    total_complexity += comp;

                                    if comp > max_complexity {
                                        max_complexity = comp;
                                        max_complex_method = format!("{class_name}::{method_name}");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("=== JAR Analysis Report: {path} ===");
    println!("Total Classes:      {total_classes}");
    println!("Total Methods:      {total_methods}");
    println!("Total Fields:       {total_fields}");
    println!("Total Instructions: {total_instructions}");
    println!("Total Complexity:   {total_complexity}");
    if total_methods > 0 {
        let avg = total_complexity as f64 / total_methods as f64;
        println!("Average Complexity: {avg:.2}");
    }
    println!("Most Complex Method: {max_complex_method} (Complexity: {max_complexity})");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_jar_missing_file() {
        let result = std::panic::catch_unwind(|| {
            inspect_jar("non_existent_file.jar");
        });
        assert!(result.is_err());
    }
}
