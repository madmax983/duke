//! `duke-classfile` — JVM `.class` file parser.
//!
//! Parses `.class` files conforming to JVM SE 21 (class file version 65).
//! All constant pool entries for JDK 21 are supported, including the Java 9+
//! Module and Package entry kinds.
//!
//! # Example
//!
//! ```no_run
//! use duke_classfile::parse;
//!
//! let bytes = std::fs::read("HelloWorld.class").unwrap();
//! let class_file = parse(&bytes).expect("valid .class file");
//! println!("Class: {:?}", class_file.this_class);
//! println!("Methods: {}", class_file.methods.len());
//! ```

pub mod access_flags;
pub mod error;
pub mod parser;
pub mod types;

pub use error::{ParseError, ParseResult};
pub use parser::parse;
pub use types::{
    AttributeData, AttributeInfo, ClassFile, CodeAttribute, CpEntry, CpIndex, ExceptionTableEntry,
    FieldInfo, LineNumberEntry, LocalVariableEntry, MethodInfo,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access_flags::ClassAccessFlags;
    use crate::error::ParseError;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Path to a compiled fixture relative to the workspace root.
    fn fixture(name: &str) -> std::path::PathBuf {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../tests/fixtures");
        p.push(name);
        p
    }

    /// Build a minimal hand-crafted valid class file in memory.
    ///
    /// Structure:
    ///   - Class named "Minimal"
    ///   - Superclass java/lang/Object
    ///   - No fields, no methods, no attributes
    ///   - Java 21 (major 65)
    fn minimal_class_bytes() -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();

        // magic
        v.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]);
        // minor, major (Java 21 = 65)
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x41]);

        // constant_pool_count = 5 (indices 1–4)
        v.extend_from_slice(&[0x00, 0x05]);

        // #1 CONSTANT_Utf8 "Minimal"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x07]);
        v.extend_from_slice(b"Minimal");

        // #2 CONSTANT_Class { name_index = 1 }
        v.push(7);
        v.extend_from_slice(&[0x00, 0x01]);

        // #3 CONSTANT_Utf8 "java/lang/Object"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x10]);
        v.extend_from_slice(b"java/lang/Object");

        // #4 CONSTANT_Class { name_index = 3 }
        v.push(7);
        v.extend_from_slice(&[0x00, 0x03]);

        // access_flags = PUBLIC | SUPER (0x0021)
        v.extend_from_slice(&[0x00, 0x21]);
        // this_class = 2
        v.extend_from_slice(&[0x00, 0x02]);
        // super_class = 4
        v.extend_from_slice(&[0x00, 0x04]);

        // interfaces_count = 0
        v.extend_from_slice(&[0x00, 0x00]);
        // fields_count = 0
        v.extend_from_slice(&[0x00, 0x00]);
        // methods_count = 0
        v.extend_from_slice(&[0x00, 0x00]);
        // attributes_count = 0
        v.extend_from_slice(&[0x00, 0x00]);

        v
    }

    // -----------------------------------------------------------------------
    // Happy-path unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_minimal_hand_crafted() {
        let bytes = minimal_class_bytes();
        let cf = parse(&bytes).expect("minimal class should parse");

        assert_eq!(cf.major_version, 65);
        assert_eq!(cf.minor_version, 0);
        assert!(cf.access_flags.contains(ClassAccessFlags::PUBLIC));
        assert!(cf.access_flags.contains(ClassAccessFlags::SUPER));
        assert_eq!(cf.this_class, CpIndex(2));
        assert_eq!(cf.super_class, CpIndex(4));
        assert!(cf.interfaces.is_empty());
        assert!(cf.fields.is_empty());
        assert!(cf.methods.is_empty());
        assert!(cf.attributes.is_empty());

        // Verify constant pool layout
        // Slot 0 is always None
        assert!(cf.constant_pool[0].is_none());
        assert!(matches!(&cf.constant_pool[1], Some(CpEntry::Utf8(s)) if s == "Minimal"));
        assert!(
            matches!(&cf.constant_pool[2], Some(CpEntry::Class { name_index }) if *name_index == CpIndex(1))
        );
        assert!(matches!(&cf.constant_pool[3], Some(CpEntry::Utf8(s)) if s == "java/lang/Object"));
        assert!(
            matches!(&cf.constant_pool[4], Some(CpEntry::Class { name_index }) if *name_index == CpIndex(3))
        );
    }

    #[test]
    fn parse_hello_world_class_file() {
        let path = fixture("HelloWorld.class");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|_| panic!("fixture not found: {}", path.display()));
        let cf = parse(&bytes).expect("HelloWorld.class should parse");

        assert_eq!(cf.major_version, 65, "should be Java 21");
        assert!(cf.access_flags.contains(ClassAccessFlags::PUBLIC));
        assert!(cf.access_flags.contains(ClassAccessFlags::SUPER));

        // Should have a main method
        let has_main = cf.methods.iter().any(|m| {
            matches!(&cf.constant_pool[m.name_index.0 as usize], Some(CpEntry::Utf8(s)) if s == "main")
        });
        assert!(has_main, "HelloWorld should have a main method");

        // main should have a Code attribute
        let main = cf
            .methods
            .iter()
            .find(|m| {
                matches!(&cf.constant_pool[m.name_index.0 as usize], Some(CpEntry::Utf8(s)) if s == "main")
            })
            .unwrap();
        let has_code = main
            .attributes
            .iter()
            .any(|a| matches!(&a.data, AttributeData::Code(_)));
        assert!(has_code, "main method should have a Code attribute");
    }

    #[test]
    fn parse_counter_class_file() {
        let path = fixture("Counter.class");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|_| panic!("fixture not found: {}", path.display()));
        let cf = parse(&bytes).expect("Counter.class should parse");

        assert_eq!(cf.major_version, 65);

        // Counter has a private static field and a private instance field
        assert!(
            cf.fields.len() >= 2,
            "expected at least 2 fields, got {}",
            cf.fields.len()
        );

        // Should have constructor, increment, getInstanceCount
        let method_names: Vec<&str> = cf
            .methods
            .iter()
            .filter_map(|m| {
                if let Some(Some(CpEntry::Utf8(s))) = cf.constant_pool.get(m.name_index.0 as usize)
                {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            method_names.contains(&"<init>"),
            "Counter should have a constructor"
        );
        assert!(
            method_names.contains(&"increment"),
            "Counter should have increment()"
        );
        assert!(
            method_names.contains(&"getInstanceCount"),
            "Counter should have getInstanceCount()"
        );
    }

    // -----------------------------------------------------------------------
    // Error / edge-case tests
    // -----------------------------------------------------------------------

    #[test]
    fn reject_empty_input() {
        let err = parse(&[]).unwrap_err();
        assert!(
            matches!(err, ParseError::UnexpectedEof { .. }),
            "empty input should be UnexpectedEof, got: {err}"
        );
    }

    #[test]
    fn reject_bad_magic() {
        let mut bytes = minimal_class_bytes();
        bytes[0] = 0xDE; // corrupt magic
        let err = parse(&bytes).unwrap_err();
        assert!(
            matches!(err, ParseError::BadMagic { .. }),
            "bad magic should fail, got: {err}"
        );
    }

    #[test]
    fn reject_truncated_after_magic() {
        // Only 4 bytes (just the magic)
        let bytes = [0xCA, 0xFE, 0xBA, 0xBE];
        let err = parse(&bytes).unwrap_err();
        assert!(
            matches!(err, ParseError::UnexpectedEof { .. }),
            "truncated after magic should be UnexpectedEof, got: {err}"
        );
    }

    #[test]
    fn reject_truncated_mid_constant_pool() {
        let mut bytes = minimal_class_bytes();
        // Truncate partway through — keep magic + version + cp_count only
        bytes.truncate(10);
        let err = parse(&bytes).unwrap_err();
        assert!(
            matches!(err, ParseError::UnexpectedEof { .. }),
            "truncated CP should be UnexpectedEof, got: {err}"
        );
    }

    #[test]
    fn reject_unsupported_version() {
        let mut bytes = minimal_class_bytes();
        // major version = 66 (Java 22)
        bytes[6] = 0x00;
        bytes[7] = 0x42;
        let err = parse(&bytes).unwrap_err();
        assert!(
            matches!(err, ParseError::UnsupportedVersion { major: 66, .. }),
            "version 66 should be unsupported, got: {err}"
        );
    }

    #[test]
    fn reject_unknown_cp_tag() {
        let mut bytes = minimal_class_bytes();
        // Overwrite tag byte of CP entry #1 with an invalid tag (e.g. 99)
        // magic(4) + minor(2) + major(2) + cp_count(2) = offset 10 = tag of first entry
        bytes[10] = 99;
        let err = parse(&bytes).unwrap_err();
        assert!(
            matches!(err, ParseError::UnknownCpTag { tag: 99, .. }),
            "unknown tag should fail, got: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // Long/Double phantom-slot test
    // -----------------------------------------------------------------------

    #[test]
    fn long_constant_inserts_phantom_slot() {
        let mut v: Vec<u8> = Vec::new();
        v.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]); // magic
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x41]); // Java 21

        // cp_count = 6 (indices 1-5; long at 1 eats slot 1+2=phantom)
        v.extend_from_slice(&[0x00, 0x06]);

        // #1 CONSTANT_Long (tag 5) — occupies slots 1 and 2
        v.push(5);
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2A]); // 42L

        // #3 CONSTANT_Utf8 "LongClass"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x09]);
        v.extend_from_slice(b"LongClass");

        // #4 CONSTANT_Class { name_index = 3 }
        v.push(7);
        v.extend_from_slice(&[0x00, 0x03]);

        // #5 CONSTANT_Utf8 "java/lang/Object"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x10]);
        v.extend_from_slice(b"java/lang/Object");

        // --- rest of class file ---
        // access_flags, this_class=4, super_class=5 (we need another Class entry)
        // Actually let's add a #6 CONSTANT_Class for Object — but cp_count is 6 so max idx is 5
        // Let's just not reference a Class entry and see that the CP parses correctly.
        // We only care about validating phantom-slot behavior here, so add minimal footer:
        v.extend_from_slice(&[0x00, 0x00]); // access_flags
        v.extend_from_slice(&[0x00, 0x04]); // this_class = 4 (LongClass)
        v.extend_from_slice(&[0x00, 0x04]); // super_class = 4 (reuse; fine for unit test)
        v.extend_from_slice(&[0x00, 0x00]); // interfaces_count
        v.extend_from_slice(&[0x00, 0x00]); // fields_count
        v.extend_from_slice(&[0x00, 0x00]); // methods_count
        v.extend_from_slice(&[0x00, 0x00]); // attributes_count

        let cf = parse(&v).expect("class with CONSTANT_Long should parse");

        // Slot 1 = Long(42)
        assert!(
            matches!(&cf.constant_pool[1], Some(CpEntry::Long(42))),
            "slot 1 should be Long(42)"
        );
        // Slot 2 = phantom (None)
        assert!(
            cf.constant_pool[2].is_none(),
            "slot 2 should be phantom None after Long"
        );
        // Slot 3 = Utf8("LongClass")
        assert!(
            matches!(&cf.constant_pool[3], Some(CpEntry::Utf8(s)) if s == "LongClass"),
            "slot 3 should be Utf8"
        );
    }

    // -----------------------------------------------------------------------
    // Property tests: parser must never panic on arbitrary input
    // -----------------------------------------------------------------------

    use proptest::prelude::*;

    proptest! {
        /// For any byte sequence, `parse` must return `Ok` or `Err` — never panic.
        ///
        /// This is the core safety property: the cursor-based parser has no
        /// out-of-bounds slice access or integer overflow on untrusted input.
        #[test]
        fn no_panic_on_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
            // We don't care whether it succeeds or fails, only that it doesn't panic.
            let _ = parse(&bytes);
        }

        /// A valid class file with the magic bytes should either parse successfully
        /// or fail with a structured error — never an unstructured panic.
        #[test]
        fn no_panic_on_magic_prefixed_bytes(
            tail in proptest::collection::vec(any::<u8>(), 0..4096)
        ) {
            let mut bytes = vec![0xCA, 0xFE, 0xBA, 0xBE];
            bytes.extend_from_slice(&tail);
            let _ = parse(&bytes);
        }
    }

    #[test]
    fn test_extracted_attribute_parsers() {
        let bytes = b"\xca\xfe\xba\xbe\x00\x00\x00A\x00\n\x01\x00\x07Minimal\x07\x00\x01\x01\x00\x10java/lang/Object\x07\x00\x03\x01\x00\x0fLineNumberTable\x01\x00\x12LocalVariableTable\x01\x00\nExceptions\x01\x00\x10BootstrapMethods\x0f\x06\x00\x04\x00!\x00\x02\x00\x04\x00\x00\x00\x00\x00\x00\x00\x04\x00\x05\x00\x00\x00\x06\x00\x01\x00\x00\x00\n\x00\x06\x00\x00\x00\x0c\x00\x01\x00\x00\x00\n\x00\x01\x00\x02\x00\x00\x00\x07\x00\x00\x00\x04\x00\x01\x00\x02\x00\x08\x00\x00\x00\x08\x00\x01\x00\t\x00\x01\x00\x04";
        let cf = parse(bytes).unwrap();

        assert_eq!(cf.attributes.len(), 4);

        let mut found_lnt = false;
        let mut found_lvt = false;
        let mut found_exc = false;
        let mut found_bm = false;

        for attr in &cf.attributes {
            match &attr.data {
                crate::types::AttributeData::LineNumberTable(entries) => {
                    found_lnt = true;
                    assert_eq!(entries.len(), 1);
                    assert_eq!(entries[0].start_pc, 0);
                    assert_eq!(entries[0].line_number, 10);
                }
                crate::types::AttributeData::LocalVariableTable(entries) => {
                    found_lvt = true;
                    assert_eq!(entries.len(), 1);
                    assert_eq!(entries[0].start_pc, 0);
                    assert_eq!(entries[0].length, 10);
                }
                crate::types::AttributeData::Exceptions {
                    exception_index_table,
                } => {
                    found_exc = true;
                    assert_eq!(exception_index_table.len(), 1);
                    assert_eq!(exception_index_table[0].0, 2);
                }
                crate::types::AttributeData::BootstrapMethods(entries) => {
                    found_bm = true;
                    assert_eq!(entries.len(), 1);
                    assert_eq!(entries[0].method_ref.0, 9);
                    assert_eq!(entries[0].arguments.len(), 1);
                }
                _ => {}
            }
        }

        assert!(found_lnt);
        assert!(found_lvt);
        assert!(found_exc);
        assert!(found_bm);
    }
}
