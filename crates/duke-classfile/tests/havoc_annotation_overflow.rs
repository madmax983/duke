#![allow(clippy::cast_possible_truncation)]
use duke_classfile::{parse, Error};

#[test]
fn test_annotation_stack_overflow() {
    let mut class_bytes = vec![
        0xCA, 0xFE, 0xBA, 0xBE, // magic
        0x00, 0x00,             // minor
        0x00, 0x41,             // major (65)
        0x00, 0x04,             // constant pool count (4)
        // 1: Utf8 "RuntimeVisibleAnnotations"
        0x01, 0x00, 0x19, b'R', b'u', b'n', b't', b'i', b'm', b'e', b'V', b'i', b's', b'i', b'b', b'l', b'e', b'A', b'n', b'n', b'o', b't', b'a', b't', b'i', b'o', b'n', b's',
        // 2: Utf8 "Lfoo;"
        0x01, 0x00, 0x05, b'L', b'f', b'o', b'o', b';',
        // 3: Utf8 "value"
        0x01, 0x00, 0x05, b'v', b'a', b'l', b'u', b'e',
        // Access flags, this_class, super_class
        0x00, 0x01, 0x00, 0x02, 0x00, 0x02,
        // Interfaces count
        0x00, 0x00,
        // Fields count
        0x00, 0x00,
        // Methods count
        0x00, 0x00,
        // Attributes count
        0x00, 0x01,
        // Attribute 1: RuntimeVisibleAnnotations
        0x00, 0x01, // name_index = 1
        0x00, 0x00, 0x00, 0x00, // attr_length (will patch later)
        // num_annotations
        0x00, 0x01,
    ];

    // We appended 6 bytes for the attribute: 2 for name_index, 4 for attr_length.
    // Wait, the bytes above added name_index (2) + attr_length (4) + num_annotations (2) = 8 bytes.
    // The attr_length field is at index: class_bytes.len() - 6
    let attr_length_offset = class_bytes.len() - 6;
    let attr_start = class_bytes.len() - 2; // the start of the data inside the attribute

    let depth = 500_000;
    for _ in 0..depth {
        class_bytes.extend_from_slice(&[
            0x00, 0x02, // type_index = 2
            0x00, 0x01, // num_element_value_pairs = 1
            0x00, 0x03, // element_name_index = 3
            b'@',       // ElementValue tag = annotation
        ]);
    }

    // Bottom of recursion
    class_bytes.extend_from_slice(&[
        0x00, 0x02, // type_index = 2
        0x00, 0x00, // num_element_value_pairs = 0
    ]);

    let attr_end = class_bytes.len();
    let attr_length = u32::try_from(attr_end - attr_start).unwrap();
    class_bytes[attr_length_offset] = u8::try_from(attr_length >> 24).unwrap();
    class_bytes[attr_length_offset + 1] = u8::try_from((attr_length >> 16) & 0xFF).unwrap();
    class_bytes[attr_length_offset + 2] = u8::try_from((attr_length >> 8) & 0xFF).unwrap();
    class_bytes[attr_length_offset + 3] = u8::try_from(attr_length & 0xFF).unwrap();

    let result = parse(&class_bytes);
    assert!(matches!(result, Err(Error::RecursionLimitExceeded)), "Expected RecursionLimitExceeded, got {:?}", result);
}
