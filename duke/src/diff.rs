#[cfg(feature = "nova")]
use std::collections::HashMap;
#[cfg(feature = "nova")]
use duke_classfile::{parse, types::{AttributeData, CpEntry, CpIndex}};
#[cfg(feature = "nova")]
use duke_bytecode::decode;

#[cfg(feature = "nova")]
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

#[cfg(feature = "nova")]
#[allow(clippy::print_stdout, clippy::use_debug, clippy::collapsible_if)]
pub fn dump_diff(path1: &str, path2: &str) {
    let bytes1 = match std::fs::read(path1) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("duke: failed to read '{path1}': {e}");
            return;
        }
    };
    let cf1 = match parse(&bytes1) {
        Ok(cf) => cf,
        Err(e) => {
            eprintln!("duke: parse error in '{path1}': {e}");
            return;
        }
    };

    let bytes2 = match std::fs::read(path2) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("duke: failed to read '{path2}': {e}");
            return;
        }
    };
    let cf2 = match parse(&bytes2) {
        Ok(cf) => cf,
        Err(e) => {
            eprintln!("duke: parse error in '{path2}': {e}");
            return;
        }
    };

    let mut methods1 = HashMap::new();
    for m in &cf1.methods {
        let name = cp_str(&cf1, m.name_index).unwrap_or("");
        let desc = cp_str(&cf1, m.descriptor_index).unwrap_or("");
        let mut code_len = 0;
        for attr in &m.attributes {
            if let AttributeData::Code(code) = &attr.data {
                if let Ok(instructions) = decode(&code.code) {
                    code_len = instructions.len();
                }
            }
        }
        methods1.insert(format!("{name}{desc}"), code_len);
    }

    let mut methods2 = HashMap::new();
    for m in &cf2.methods {
        let name = cp_str(&cf2, m.name_index).unwrap_or("");
        let desc = cp_str(&cf2, m.descriptor_index).unwrap_or("");
        let mut code_len = 0;
        for attr in &m.attributes {
            if let AttributeData::Code(code) = &attr.data {
                if let Ok(instructions) = decode(&code.code) {
                    code_len = instructions.len();
                }
            }
        }
        methods2.insert(format!("{name}{desc}"), code_len);
    }

    println!("=== Bytecode Diff ===");
    println!("--- {path1}");
    println!("+++ {path2}");

    for (m, len1) in &methods1 {
        if let Some(len2) = methods2.get(m) {
            if len1 != len2 {
                println!("~ {m} ({len1} instrs -> {len2} instrs)");
            }
        } else {
            println!("- {m}");
        }
    }
    for m in methods2.keys() {
        if !methods1.contains_key(m) {
            println!("+ {m}");
        }
    }
}


#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_diff_handles_files() {
        let valid_bytes1 = vec![
            0xca, 0xfe, 0xba, 0xbe, // magic
            0x00, 0x00, // minor
            0x00, 0x3d, // major (61)
            0x00, 0x01, // constant_pool_count (1)
            0x00, 0x01, // access_flags (public)
            0x00, 0x00, // this_class (0)
            0x00, 0x00, // super_class (0)
            0x00, 0x00, // interfaces_count (0)
            0x00, 0x00, // fields_count (0)
            0x00, 0x00, // methods_count (0)
            0x00, 0x00, // attributes_count (0)
        ];

        let dir = std::env::temp_dir();
        let path1 = dir.join("diff_test_file1.class");
        let path2 = dir.join("diff_test_file2.class");

        std::fs::write(&path1, &valid_bytes1).unwrap();
        std::fs::write(&path2, &valid_bytes1).unwrap();

        // This will print to stdout, we just ensure it doesn't panic
        dump_diff(path1.to_str().unwrap(), path2.to_str().unwrap());

        std::fs::remove_file(&path1).unwrap();
        std::fs::remove_file(&path2).unwrap();
    }

    #[test]
    fn test_dump_diff_added_and_removed_methods() {
        let valid_bytes1 = vec![
            0xca, 0xfe, 0xba, 0xbe, // magic
            0x00, 0x00, // minor
            0x00, 0x3d, // major (61)
            0x00, 0x04, // constant_pool_count (4)
            // #1: Utf8 "oldMethod"
            0x01, 0x00, 0x09, 0x6f, 0x6c, 0x64, 0x4d, 0x65, 0x74, 0x68, 0x6f, 0x64,
            // #2: Utf8 "()V"
            0x01, 0x00, 0x03, 0x28, 0x29, 0x56,
            // #3: Utf8 "Code"
            0x01, 0x00, 0x04, 0x43, 0x6f, 0x64, 0x65,
            0x00, 0x01, // access_flags (public)
            0x00, 0x00, // this_class (0)
            0x00, 0x00, // super_class (0)
            0x00, 0x00, // interfaces_count (0)
            0x00, 0x00, // fields_count (0)
            0x00, 0x01, // methods_count (1)
            0x00, 0x01, // method[0].access_flags
            0x00, 0x01, // method[0].name_index
            0x00, 0x02, // method[0].descriptor_index
            0x00, 0x01, // method[0].attributes_count
            0x00, 0x03, // attr name index
            0x00, 0x00, 0x00, 0x0e, // attr len
            0x00, 0x01, // max stack
            0x00, 0x01, // max locals
            0x00, 0x00, 0x00, 0x02, // code len
            0x03, 0xac, // iconst_0, ireturn
            0x00, 0x00, // exception table len
            0x00, 0x00, // attributes count
            0x00, 0x00, // class attributes_count (0)
        ];

        let valid_bytes2 = vec![
            0xca, 0xfe, 0xba, 0xbe, // magic
            0x00, 0x00, // minor
            0x00, 0x3d, // major (61)
            0x00, 0x04, // constant_pool_count (4)
            // #1: Utf8 "newMethod"
            0x01, 0x00, 0x09, 0x6e, 0x65, 0x77, 0x4d, 0x65, 0x74, 0x68, 0x6f, 0x64,
            // #2: Utf8 "()V"
            0x01, 0x00, 0x03, 0x28, 0x29, 0x56,
            // #3: Utf8 "Code"
            0x01, 0x00, 0x04, 0x43, 0x6f, 0x64, 0x65,
            0x00, 0x01, // access_flags (public)
            0x00, 0x00, // this_class (0)
            0x00, 0x00, // super_class (0)
            0x00, 0x00, // interfaces_count (0)
            0x00, 0x00, // fields_count (0)
            0x00, 0x01, // methods_count (1)
            0x00, 0x01, // method[0].access_flags
            0x00, 0x01, // method[0].name_index
            0x00, 0x02, // method[0].descriptor_index
            0x00, 0x01, // method[0].attributes_count
            0x00, 0x03, // attr name index
            0x00, 0x00, 0x00, 0x0e, // attr len
            0x00, 0x01, // max stack
            0x00, 0x01, // max locals
            0x00, 0x00, 0x00, 0x02, // code len
            0x03, 0xac, // iconst_0, ireturn
            0x00, 0x00, // exception table len
            0x00, 0x00, // attributes count
            0x00, 0x00, // class attributes_count (0)
        ];

        let dir = std::env::temp_dir();
        let path1 = dir.join("diff_test_file3.class");
        let path2 = dir.join("diff_test_file4.class");

        std::fs::write(&path1, &valid_bytes1).unwrap();
        std::fs::write(&path2, &valid_bytes2).unwrap();

        dump_diff(path1.to_str().unwrap(), path2.to_str().unwrap());

        std::fs::remove_file(&path1).unwrap();
        std::fs::remove_file(&path2).unwrap();
    }

    #[test]
    fn test_dump_diff_changed_methods_with_different_instructions() {
        let valid_bytes1 = vec![
            0xca, 0xfe, 0xba, 0xbe, // magic
            0x00, 0x00, // minor
            0x00, 0x3d, // major (61)
            0x00, 0x04, // constant_pool_count (4)
            // #1: Utf8 "changedMethod"
            0x01, 0x00, 0x0d, 0x63, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x64, 0x4d, 0x65, 0x74, 0x68, 0x6f, 0x64,
            // #2: Utf8 "()V"
            0x01, 0x00, 0x03, 0x28, 0x29, 0x56,
            // #3: Utf8 "Code"
            0x01, 0x00, 0x04, 0x43, 0x6f, 0x64, 0x65,
            0x00, 0x01, // access_flags (public)
            0x00, 0x00, // this_class (0)
            0x00, 0x00, // super_class (0)
            0x00, 0x00, // interfaces_count (0)
            0x00, 0x00, // fields_count (0)
            0x00, 0x01, // methods_count (1)
            0x00, 0x01, // method[0].access_flags
            0x00, 0x01, // method[0].name_index
            0x00, 0x02, // method[0].descriptor_index
            0x00, 0x01, // method[0].attributes_count
            0x00, 0x03, // attr name index
            0x00, 0x00, 0x00, 0x0e, // attr len
            0x00, 0x01, // max stack
            0x00, 0x01, // max locals
            0x00, 0x00, 0x00, 0x02, // code len
            0x03, 0xac, // iconst_0, ireturn
            0x00, 0x00, // exception table len
            0x00, 0x00, // attributes count
            0x00, 0x00, // class attributes_count (0)
        ];

        let valid_bytes2 = vec![
            0xca, 0xfe, 0xba, 0xbe, // magic
            0x00, 0x00, // minor
            0x00, 0x3d, // major (61)
            0x00, 0x04, // constant_pool_count (4)
            // #1: Utf8 "changedMethod"
            0x01, 0x00, 0x0d, 0x63, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x64, 0x4d, 0x65, 0x74, 0x68, 0x6f, 0x64,
            // #2: Utf8 "()V"
            0x01, 0x00, 0x03, 0x28, 0x29, 0x56,
            // #3: Utf8 "Code"
            0x01, 0x00, 0x04, 0x43, 0x6f, 0x64, 0x65,
            0x00, 0x01, // access_flags (public)
            0x00, 0x00, // this_class (0)
            0x00, 0x00, // super_class (0)
            0x00, 0x00, // interfaces_count (0)
            0x00, 0x00, // fields_count (0)
            0x00, 0x01, // methods_count (1)
            0x00, 0x01, // method[0].access_flags
            0x00, 0x01, // method[0].name_index
            0x00, 0x02, // method[0].descriptor_index
            0x00, 0x01, // method[0].attributes_count
            0x00, 0x03, // attr name index
            0x00, 0x00, 0x00, 0x0f, // attr len
            0x00, 0x01, // max stack
            0x00, 0x01, // max locals
            0x00, 0x00, 0x00, 0x03, // code len
            0x03, 0x04, 0xac, // iconst_0, iconst_1, ireturn (diff instruction count)
            0x00, 0x00, // exception table len
            0x00, 0x00, // attributes count
            0x00, 0x00, // class attributes_count (0)
        ];

        let dir = std::env::temp_dir();
        let path1 = dir.join("diff_test_file5.class");
        let path2 = dir.join("diff_test_file6.class");

        std::fs::write(&path1, &valid_bytes1).unwrap();
        std::fs::write(&path2, &valid_bytes2).unwrap();

        dump_diff(path1.to_str().unwrap(), path2.to_str().unwrap());

        std::fs::remove_file(&path1).unwrap();
        std::fs::remove_file(&path2).unwrap();
    }
}
