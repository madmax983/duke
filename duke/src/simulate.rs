#[cfg(feature = "nova")]
use duke_bytecode::decode;
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, ClassFile, CpEntry, CpIndex};

#[cfg(feature = "nova")]
#[allow(unexpected_cfgs)]
fn cp_str(cf: &ClassFile, idx: CpIndex) -> Option<&str> {
    cf.constant_pool
        .get(idx.0 as usize)
        .and_then(|slot: &Option<CpEntry>| slot.as_ref())
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
pub fn dump_simulate(path: &str, method_name: &str) {
    let bytes = std::fs::read(path).expect("Failed to read class file");
    let cf = duke_classfile::parse(&bytes).expect("Failed to parse class file");

    let target = cf
        .methods
        .iter()
        .find(|m| cp_str(&cf, m.name_index) == Some(method_name));

    if let Some(target) = target {
        println!("Simulating execution of method '{method_name}'...");
        for attr in &target.attributes {
            #[allow(clippy::collapsible_if)]
            if let AttributeData::Code(code) = &attr.data {
                if let Ok(decoded) = decode(&code.code) {
                    println!("--- Instruction Trace ---");
                    for (pc, instr) in decoded {
                        println!("PC {pc:>3}: Executing {}", instr.mnemonic());
                    }
                    println!("--- End Trace ---");
                }
            }
        }
    } else {
        println!("duke: method '{method_name}' not found");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::{ClassAccessFlags, ClassFile, CpEntry, CpIndex};

    #[test]
    fn test_dump_simulate_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dummy_not_found_parse_error.class");
        // Create a truncated dummy file to force a parse failure at `expect("Failed to parse class file")`
        std::fs::write(&path, b"\xca\xfe\xba\xbe\x00\x00\x00\x34\x00\x00\x00").unwrap();

        let path_str = path.to_str().unwrap();
        let result = std::panic::catch_unwind(|| {
            dump_simulate(path_str, "missing_method");
        });

        assert!(result.is_err(), "Expected parsing to fail and panic");
    }

    #[test]
    fn test_dump_simulate_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does_not_exist_xyz123.class");

        let path_str = path.to_str().unwrap();
        let result = std::panic::catch_unwind(|| {
            dump_simulate(path_str, "missing_method");
        });
        assert!(result.is_err(), "Expected file read to fail and panic");
    }

    #[test]
    fn test_dump_simulate_method_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dummy_valid_class2.class");
        let valid_class_bytes: [u8; 24] = [
            0xca, 0xfe, 0xba, 0xbe, // magic
            0x00, 0x00, // minor
            0x00, 0x34, // major (52)
            0x00, 0x01, // constant pool count (1, so 0 entries)
            0x00, 0x21, // access flags (SUPER | PUBLIC)
            0x00, 0x00, // this class (invalid index, but parser might not care)
            0x00, 0x00, // super class
            0x00, 0x00, // interfaces count
            0x00, 0x00, // fields count
            0x00, 0x00, // methods count
            0x00, 0x00, // attributes count
        ];

        std::fs::write(&path, valid_class_bytes).unwrap();

        let path_str = path.to_str().unwrap();
        let result = std::panic::catch_unwind(|| {
            dump_simulate(path_str, "missing_method");
        });

        // The parser correctly parses this minimal dummy file and returns Ok.
        // It should then print `duke: method 'missing_method' not found` without panicking.
        assert!(
            result.is_ok(),
            "Expected valid empty class to parse successfully and simulate to exit cleanly"
        );
    }

    #[test]
    fn test_cp_str() {
        let cf1 = ClassFile {
            major_version: 52,
            minor_version: 0,
            constant_pool: vec![None, Some(CpEntry::Utf8("test".to_string()))],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        assert_eq!(cp_str(&cf1, CpIndex(1)), Some("test"));
    }
}
