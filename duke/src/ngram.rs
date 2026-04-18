//! N-Gram Analysis for Bytecode Instructions.

use duke_bytecode::decode;
use duke_classfile::{parse, types::AttributeData};
use duke_loader::{ClassLoader, ZipLoader};
use std::collections::HashMap;
use std::path::Path;

/// Analyzes a JAR file and prints an n-gram frequency analysis of JVM opcodes.
///
/// **Why it exists:** Analyzing single opcode frequency is useful, but identifying
/// common sequences of instructions (e.g. `aload_0` -> `getfield` -> `ireturn`)
/// provides deeper insights. This can guide the creation of "super-instructions"
/// to optimize the JVM interpreter, or detect common code generation patterns.
#[allow(
    clippy::cast_precision_loss,
    clippy::collapsible_if,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::cast_lossless
)]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
#[cfg(feature = "nova")]
pub fn dump_ngram(jar_path: &str, n: usize) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        panic!("duke: failed to open JAR '{jar_path}': {e}");
    });

    let mut ngrams: HashMap<Vec<&'static str>, usize> = HashMap::new();
    let mut total_ngrams = 0;

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
                                let mnemonics: Vec<&'static str> = instructions
                                    .iter()
                                    .map(|(_, instr)| instr.mnemonic())
                                    .collect();

                                if mnemonics.len() >= n {
                                    for window in mnemonics.windows(n) {
                                        *ngrams.entry(window.to_vec()).or_insert(0) += 1;
                                        total_ngrams += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut sorted_ngrams: Vec<(Vec<&'static str>, usize)> = ngrams.into_iter().collect();
    sorted_ngrams.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    println!("======================================");
    println!(" Bytecode {n}-Gram Analysis");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Total {n}-Grams:        {total_ngrams}");
    println!();
    println!("Top 10 Sequences:");
    for (seq, count) in sorted_ngrams.into_iter().take(10) {
        let seq_str = seq.join(" -> ");
        let percentage = if total_ngrams > 0 {
            (count as f64 / total_ngrams as f64) * 100.0
        } else {
            0.0
        };
        println!("  {seq_str:<40} {count:>8} ({percentage:>5.2}%)");
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_ngram_missing_file() {
        let result = std::panic::catch_unwind(|| {
            dump_ngram("non_existent_file_for_ngram.jar", 2);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_dump_ngram_valid_file() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path = path.to_str().unwrap();

        dump_ngram(path, 2);
    }
}
