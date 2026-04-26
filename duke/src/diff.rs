use duke_bytecode::decode;
use duke_classfile::{
    parse,
    types::{AttributeData, ClassFile, CpEntry, CpIndex},
};
use std::collections::HashSet;

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
pub fn dump_diff(path1: &str, path2: &str) {
    let bytes1 = std::fs::read(path1).unwrap();
    let bytes2 = std::fs::read(path2).unwrap();

    let cf1 = parse(&bytes1).unwrap();
    let cf2 = parse(&bytes2).unwrap();

    let get_methods = |cf: &ClassFile| {
        let mut methods = std::collections::HashMap::new();
        for m in &cf.methods {
            let name = cp_str(cf, m.name_index).unwrap_or("<invalid>");
            let desc = cp_str(cf, m.descriptor_index).unwrap_or("<invalid>");
            let mut instructions = Vec::new();
            for attr in &m.attributes {
                if let AttributeData::Code(code) = &attr.data {
                    if let Ok(instrs) = decode(&code.code) {
                        for (_, i) in instrs {
                            instructions.push(i.mnemonic().to_string());
                        }
                    }
                }
            }
            methods.insert(format!("{}{}", name, desc), instructions);
        }
        methods
    };

    let m1 = get_methods(&cf1);
    let m2 = get_methods(&cf2);

    let k1: HashSet<_> = m1.keys().cloned().collect();
    let k2: HashSet<_> = m2.keys().cloned().collect();

    for added in k2.difference(&k1) {
        println!("+ Method added: {}", added);
    }
    for removed in k1.difference(&k2) {
        println!("- Method removed: {}", removed);
    }

    for common in k1.intersection(&k2) {
        let i1 = m1.get(common).unwrap();
        let i2 = m2.get(common).unwrap();

        if i1 != i2 {
            println!("~ Method changed: {}", common);
            let max_len = std::cmp::max(i1.len(), i2.len());
            for i in 0..max_len {
                let inst1 = i1.get(i).map_or("<none>", |s| s.as_str());
                let inst2 = i2.get(i).map_or("<none>", |s| s.as_str());
                if inst1 != inst2 {
                    println!("  - {}", inst1);
                    println!("  + {}", inst2);
                } else {
                    println!("    {}", inst1);
                }
            }
        }
    }
}

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
        0x01, 0x00, 0x03, 0x28, 0x29, 0x56, // #3: Utf8 "Code"
        0x01, 0x00, 0x04, 0x43, 0x6f, 0x64, 0x65, 0x00, 0x01, // access_flags (public)
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
        0x01, 0x00, 0x03, 0x28, 0x29, 0x56, // #3: Utf8 "Code"
        0x01, 0x00, 0x04, 0x43, 0x6f, 0x64, 0x65, 0x00, 0x01, // access_flags (public)
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
        0x01, 0x00, 0x0d, 0x63, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x64, 0x4d, 0x65, 0x74, 0x68, 0x6f,
        0x64, // #2: Utf8 "()V"
        0x01, 0x00, 0x03, 0x28, 0x29, 0x56, // #3: Utf8 "Code"
        0x01, 0x00, 0x04, 0x43, 0x6f, 0x64, 0x65, 0x00, 0x01, // access_flags (public)
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
        0x01, 0x00, 0x0d, 0x63, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x64, 0x4d, 0x65, 0x74, 0x68, 0x6f,
        0x64, // #2: Utf8 "()V"
        0x01, 0x00, 0x03, 0x28, 0x29, 0x56, // #3: Utf8 "Code"
        0x01, 0x00, 0x04, 0x43, 0x6f, 0x64, 0x65, 0x00, 0x01, // access_flags (public)
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
