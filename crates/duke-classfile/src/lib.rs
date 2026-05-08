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

pub(crate) mod access_flags;
pub(crate) mod attributes;
pub(crate) mod class;
pub(crate) mod constant_pool;
pub(crate) mod error;
pub(crate) mod parser;

/// Compatibility module re-exporting the split type structures.
pub mod types {
    pub use crate::attributes::{
        Annotation, AttributeData, AttributeInfo, BootstrapMethodEntry, CodeAttribute,
        ElementValue, ElementValuePair, ExceptionTableEntry, LineNumberEntry, LocalVariableEntry,
    };
    pub use crate::class::{ClassFile, FieldInfo, MethodInfo};
    pub use crate::constant_pool::{CpEntry, CpIndex};
}

pub use access_flags::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
pub use error::{Error, Result};
pub use parser::parse;
pub use types::{
    AttributeData, AttributeInfo, ClassFile, CodeAttribute, CpEntry, CpIndex, ExceptionTableEntry,
    FieldInfo, LineNumberEntry, LocalVariableEntry, MethodInfo,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access_flags::ClassAccessFlags;
    use crate::error::Error;

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
            matches!(err, Error::UnexpectedEof { .. }),
            "empty input should be UnexpectedEof, got: {err}"
        );
    }

    #[test]
    fn reject_bad_magic() {
        let mut bytes = minimal_class_bytes();
        bytes[0] = 0xDE; // corrupt magic
        let err = parse(&bytes).unwrap_err();
        assert!(
            matches!(err, Error::BadMagic { .. }),
            "bad magic should fail, got: {err}"
        );
    }

    #[test]
    fn reject_truncated_after_magic() {
        // Only 4 bytes (just the magic)
        let bytes = [0xCA, 0xFE, 0xBA, 0xBE];
        let err = parse(&bytes).unwrap_err();
        assert!(
            matches!(err, Error::UnexpectedEof { .. }),
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
            matches!(err, Error::UnexpectedEof { .. }),
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
            matches!(err, Error::UnsupportedVersion { major: 66, .. }),
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
            matches!(err, Error::UnknownCpTag { tag: 99, .. }),
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
    // Double phantom-slot (mirrors Long test, kills i += 2 mutants in Double branch)
    // -----------------------------------------------------------------------

    #[test]
    fn double_constant_inserts_phantom_slot() {
        let mut v: Vec<u8> = Vec::new();
        v.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]); // magic
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x41]); // Java 21

        // cp_count = 6 (indices 1-5; Double at 1 eats slots 1+2)
        v.extend_from_slice(&[0x00, 0x06]);

        // #1 CONSTANT_Double (tag 6) = 3.14 — occupies slots 1 and 2
        v.push(6);
        let bits = std::f64::consts::PI.to_bits();
        v.extend_from_slice(&bits.to_be_bytes());

        // #3 CONSTANT_Utf8 "DblClass"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x08]);
        v.extend_from_slice(b"DblClass");

        // #4 CONSTANT_Class { name_index = 3 }
        v.push(7);
        v.extend_from_slice(&[0x00, 0x03]);

        // #5 CONSTANT_Utf8 "java/lang/Object"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x10]);
        v.extend_from_slice(b"java/lang/Object");

        // Footer
        v.extend_from_slice(&[0x00, 0x00]); // access_flags
        v.extend_from_slice(&[0x00, 0x04]); // this_class = 4
        v.extend_from_slice(&[0x00, 0x04]); // super_class = 4 (reuse)
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // ifaces/fields/methods/attrs

        let cf = parse(&v).expect("class with CONSTANT_Double should parse");
        assert!(
            matches!(&cf.constant_pool[1], Some(CpEntry::Double(d)) if (*d - std::f64::consts::PI).abs() < 1e-10),
            "slot 1 should be Double(3.14)"
        );
        assert!(
            cf.constant_pool[2].is_none(),
            "slot 2 should be phantom None after Double"
        );
        assert!(
            matches!(&cf.constant_pool[3], Some(CpEntry::Utf8(s)) if s == "DblClass"),
            "slot 3 should be Utf8"
        );
    }

    // -----------------------------------------------------------------------
    // MethodHandle validation (kills `delete !` on bounds check)
    // -----------------------------------------------------------------------

    #[test]
    fn method_handle_invalid_kind_rejected() {
        // kind=0 is invalid (valid range 1-9); `delete !` mutant would ACCEPT this
        let v = minimal_class_bytes();
        // We need to rebuild with a MethodHandle CP entry (tag=15).
        // Easiest: craft a class with cp_count=4: #1=MethodHandle(kind=0,ref=2), #2=Utf8("X"), #3=Class(2)
        let mut v2: Vec<u8> = Vec::new();
        v2.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]);
        v2.extend_from_slice(&[0x00, 0x00, 0x00, 0x41]); // Java 21
        v2.extend_from_slice(&[0x00, 0x04]); // cp_count = 4

        // #1 CONSTANT_MethodHandle (tag=15) kind=0 (invalid), ref=#2
        v2.push(15);
        v2.push(0); // reference_kind = 0 — INVALID
        v2.extend_from_slice(&[0x00, 0x02]); // reference_index

        // #2 CONSTANT_Utf8 "X"
        v2.push(1);
        v2.extend_from_slice(&[0x00, 0x01]);
        v2.push(b'X');

        // #3 CONSTANT_Class { name_index=2 }
        v2.push(7);
        v2.extend_from_slice(&[0x00, 0x02]);

        // Footer
        v2.extend_from_slice(&[0x00, 0x00, 0x00, 0x03, 0x00, 0x03]); // flags, this=3, super=3
        v2.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let err = parse(&v2).unwrap_err();
        assert!(
            matches!(err, Error::InvalidMethodHandleKind { kind: 0 }),
            "kind=0 should be rejected: {err}"
        );
        // Silence unused warning from variable
        let _ = v;
    }

    #[test]
    fn method_handle_valid_kind_parses() {
        // kind=6 (REF_invokeVirtual) is valid; `delete !` mutant would REJECT this
        let mut v: Vec<u8> = Vec::new();
        v.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]);
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x41]); // Java 21
        v.extend_from_slice(&[0x00, 0x05]); // cp_count = 5

        // #1 CONSTANT_MethodHandle (tag=15) kind=6, ref=#2
        v.push(15);
        v.push(6); // REF_invokeVirtual — valid
        v.extend_from_slice(&[0x00, 0x02]);

        // #2 CONSTANT_Utf8 "Foo"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x03]);
        v.extend_from_slice(b"Foo");

        // #3 CONSTANT_Class { name_index=2 }
        v.push(7);
        v.extend_from_slice(&[0x00, 0x02]);

        // #4 CONSTANT_Utf8 "java/lang/Object"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x10]);
        v.extend_from_slice(b"java/lang/Object");

        // Footer: flags, this=3, super=0 (Object has no super)
        v.extend_from_slice(&[0x00, 0x20]); // ACC_SUPER
        v.extend_from_slice(&[0x00, 0x03]); // this_class
        v.extend_from_slice(&[0x00, 0x03]); // super_class (reuse)
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let cf = parse(&v).expect("valid method handle kind=6 should parse");
        assert!(
            matches!(
                &cf.constant_pool[1],
                Some(CpEntry::MethodHandle {
                    reference_kind: 6,
                    ..
                })
            ),
            "slot 1 should be MethodHandle(kind=6)"
        );
    }

    // -----------------------------------------------------------------------
    // Attribute arm tests — each verifies a specific AttributeData variant
    // is decoded (not left as Raw), killing the "delete match arm" mutants
    // -----------------------------------------------------------------------

    #[test]
    fn source_file_attribute_decoded() {
        let path = fixture("HelloWorld.class");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|_| panic!("fixture not found: {}", path.display()));
        let cf = parse(&bytes).unwrap();
        let has_source_file = cf
            .attributes
            .iter()
            .any(|a| matches!(&a.data, AttributeData::SourceFile { .. }));
        assert!(
            has_source_file,
            "HelloWorld.class should have a decoded SourceFile attribute"
        );
    }

    #[test]
    fn line_number_table_decoded() {
        let path = fixture("HelloWorld.class");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|_| panic!("fixture not found: {}", path.display()));
        let cf = parse(&bytes).unwrap();
        let has_lnt = cf.methods.iter().any(|m| {
            m.attributes.iter().any(|a| {
                if let AttributeData::Code(code) = &a.data {
                    code.attributes
                        .iter()
                        .any(|ca| matches!(&ca.data, AttributeData::LineNumberTable(_)))
                } else {
                    false
                }
            })
        });
        assert!(
            has_lnt,
            "HelloWorld.class should have a decoded LineNumberTable in some method"
        );
    }

    #[test]
    fn local_variable_table_decoded() {
        let path = fixture("HelloWorld.class");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|_| panic!("fixture not found: {}", path.display()));
        let cf = parse(&bytes).unwrap();
        let has_lvt = cf.methods.iter().any(|m| {
            m.attributes.iter().any(|a| {
                if let AttributeData::Code(code) = &a.data {
                    code.attributes
                        .iter()
                        .any(|ca| matches!(&ca.data, AttributeData::LocalVariableTable(_)))
                } else {
                    false
                }
            })
        });
        assert!(
            has_lvt,
            "HelloWorld.class should have a decoded LocalVariableTable in some method"
        );
    }

    #[test]
    fn constant_value_attribute_decoded() {
        // Hand-crafted class with a static final int field (ConstantValue attribute).
        // CP: #1=Utf8("CV"), #2=Utf8("I"), #3=Utf8("ConstantValue"), #4=Integer(99),
        //     #5=Utf8("FooClass"), #6=Class(5), #7=Utf8("java/lang/Object"), #8=Class(7)
        let mut v: Vec<u8> = Vec::new();
        v.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]);
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x41]); // Java 21
        v.extend_from_slice(&[0x00, 0x09]); // cp_count = 9

        // #1 Utf8 "CV"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x02]);
        v.extend_from_slice(b"CV");
        // #2 Utf8 "I"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x01]);
        v.push(b'I');
        // #3 Utf8 "ConstantValue"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x0D]);
        v.extend_from_slice(b"ConstantValue");
        // #4 Integer 99
        v.push(3);
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x63]);
        // #5 Utf8 "FooClass"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x08]);
        v.extend_from_slice(b"FooClass");
        // #6 Class { name_index=5 }
        v.push(7);
        v.extend_from_slice(&[0x00, 0x05]);
        // #7 Utf8 "java/lang/Object"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x10]);
        v.extend_from_slice(b"java/lang/Object");
        // #8 Class { name_index=7 }
        v.push(7);
        v.extend_from_slice(&[0x00, 0x07]);

        // access_flags=PUBLIC|SUPER, this=6, super=8
        v.extend_from_slice(&[0x00, 0x21, 0x00, 0x06, 0x00, 0x08]);
        // interfaces=0, fields=1
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);

        // Field: ACC_PUBLIC|ACC_STATIC|ACC_FINAL=0x0019, name=#1, desc=#2, attrs=1
        v.extend_from_slice(&[0x00, 0x19, 0x00, 0x01, 0x00, 0x02, 0x00, 0x01]);
        // ConstantValue attribute: name=#3, length=2, index=#4
        v.extend_from_slice(&[0x00, 0x03]); // name_index = 3
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x02]); // attribute_length = 2
        v.extend_from_slice(&[0x00, 0x04]); // constant_value_index = 4

        // methods=0, class_attrs=0
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        let cf = parse(&v).expect("class with ConstantValue field should parse");
        assert_eq!(cf.fields.len(), 1);
        let has_cv = cf.fields[0]
            .attributes
            .iter()
            .any(|a| matches!(&a.data, AttributeData::ConstantValue { .. }));
        assert!(
            has_cv,
            "static final field should have a decoded ConstantValue attribute"
        );
    }

    #[test]
    fn exceptions_attribute_decoded() {
        // Hand-crafted method with Exceptions attribute listing one exception class.
        // CP: #1=Utf8("<init>"), #2=Utf8("()V"), #3=Utf8("Exceptions"),
        //     #4=Class(5), #5=Utf8("java/io/IOException"),
        //     #6=Utf8("ThrowsClass"), #7=Class(6), #8=Utf8("java/lang/Object"), #9=Class(8)
        let mut v: Vec<u8> = Vec::new();
        v.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]);
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x41]); // Java 21
        v.extend_from_slice(&[0x00, 0x0A]); // cp_count = 10

        // #1 Utf8 "<init>"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x06]);
        v.extend_from_slice(b"<init>");
        // #2 Utf8 "()V"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x03]);
        v.extend_from_slice(b"()V");
        // #3 Utf8 "Exceptions"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x0A]);
        v.extend_from_slice(b"Exceptions");
        // #4 Class { name_index=5 }
        v.push(7);
        v.extend_from_slice(&[0x00, 0x05]);
        // #5 Utf8 "java/io/IOException"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x13]);
        v.extend_from_slice(b"java/io/IOException");
        // #6 Utf8 "ThrowsClass"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x0B]);
        v.extend_from_slice(b"ThrowsClass");
        // #7 Class { name_index=6 }
        v.push(7);
        v.extend_from_slice(&[0x00, 0x06]);
        // #8 Utf8 "java/lang/Object"
        v.push(1);
        v.extend_from_slice(&[0x00, 0x10]);
        v.extend_from_slice(b"java/lang/Object");
        // #9 Class { name_index=8 }
        v.push(7);
        v.extend_from_slice(&[0x00, 0x08]);

        // access_flags=PUBLIC|SUPER, this=7, super=9
        v.extend_from_slice(&[0x00, 0x21, 0x00, 0x07, 0x00, 0x09]);
        // interfaces=0, fields=0, methods=1
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x01]);

        // Method: ACC_PUBLIC=0x0001, name=#1, desc=#2, attrs=1
        v.extend_from_slice(&[0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x01]);
        // Exceptions attribute: name=#3, length=4 (num=1, one index), exception=#4
        v.extend_from_slice(&[0x00, 0x03]); // name_index = 3
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x04]); // attribute_length = 4
        v.extend_from_slice(&[0x00, 0x01]); // number_of_exceptions = 1
        v.extend_from_slice(&[0x00, 0x04]); // exception_index_table[0] = 4

        // class_attrs=0
        v.extend_from_slice(&[0x00, 0x00]);

        let cf = parse(&v).expect("class with Exceptions method attribute should parse");
        assert_eq!(cf.methods.len(), 1);
        let has_exc = cf.methods[0]
            .attributes
            .iter()
            .any(|a| matches!(&a.data, AttributeData::Exceptions { exception_index_table } if !exception_index_table.is_empty()));
        assert!(
            has_exc,
            "method should have a decoded Exceptions attribute with one entry"
        );
    }

    #[test]
    fn bootstrap_methods_attribute_decoded() {
        let path = fixture("LambdaTest.class");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|_| panic!("fixture not found: {}", path.display()));
        let cf = parse(&bytes).unwrap();
        let has_bsm = cf.attributes.iter().any(
            |a| matches!(&a.data, AttributeData::BootstrapMethods(entries) if !entries.is_empty()),
        );
        assert!(
            has_bsm,
            "LambdaTest.class should have a non-empty BootstrapMethods attribute"
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

        let mut lnt_found = false;
        let mut found_lvt = false;
        let mut found_exc = false;
        let mut found_bm = false;

        for attr in &cf.attributes {
            match &attr.data {
                crate::types::AttributeData::LineNumberTable(entries) => {
                    lnt_found = true;
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

        assert!(lnt_found);
        assert!(found_lvt);
        assert!(found_exc);
        assert!(found_bm);
    }
}
#[cfg(test)]
mod fuzz;
