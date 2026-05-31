#[cfg(feature = "nova")]
use duke_bytecode::calculate_similarity;
#[cfg(feature = "nova")]
use duke_bytecode::decode;
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, ClassFile, CpEntry, CpIndex};

#[cfg(feature = "nova")]
#[allow(unexpected_cfgs)]
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
#[allow(unexpected_cfgs, clippy::print_stdout)]
pub fn dump_duplicate_code(cf: &ClassFile) {
    let mut method_codes = Vec::new();

    for method in &cf.methods {
        let name_str = cp_str(cf, method.name_index).unwrap_or("<invalid>");
        let desc_str = cp_str(cf, method.descriptor_index).unwrap_or("<invalid>");
        let full_name = format!("{name_str}{desc_str}");

        for attr in &method.attributes {
            #[allow(clippy::collapsible_if)]
            if let AttributeData::Code(code) = &attr.data {
                if let Ok(decoded) = decode(&code.code) {
                    let instructions: Vec<_> = decoded.into_iter().map(|(_, i)| i).collect();
                    method_codes.push((full_name.clone(), instructions));
                }
            }
        }
    }

    let mut found_duplicates = false;
    let threshold = 0.85;

    for i in 0..method_codes.len() {
        for j in (i + 1)..method_codes.len() {
            let (name1, instrs1) = &method_codes[i];
            let (name2, instrs2) = &method_codes[j];

            if instrs1.len() < 5 || instrs2.len() < 5 {
                continue; // Ignore very short methods
            }

            let sim = calculate_similarity(instrs1, instrs2);
            if sim >= threshold {
                found_duplicates = true;
                println!(
                    "Duplicate detected: {} and {} are {:.0}% similar.",
                    name1,
                    name2,
                    sim * 100.0
                );
            }
        }
    }

    if !found_duplicates {
        println!("No significant duplication detected.");
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {

    #[test]
    fn test_dump_duplicate_code_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/HelloWorld.class");
        let path = path.to_str().unwrap();

        let bytes = std::fs::read(path).unwrap();
        let cf = duke_classfile::parse(&bytes).unwrap();
        super::dump_duplicate_code(&cf);
    }
}
