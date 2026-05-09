#![allow(clippy::cast_possible_truncation)]

#[test]
fn test_annotation_stack_overflow() {
    let mut data = vec![
        0xca, 0xfe, 0xba, 0xbe, // 0..4 magic
        0x00, 0x00, // 4..6 minor_version
        0x00, 0x34, // 6..8 major_version (52)
        0x00, 0x05, // 8..10 constant_pool_count
        1, 0x00, 0x03, b'F', b'o', b'o', // 10..16 CP[1]: Utf8 "Foo"
        1, 0x00, 0x19, b'R', b'u', b'n', b't', b'i', b'm', b'e', b'V', b'i', b's', b'i', b'b', b'l', b'e', b'A', b'n', b'n', b'o', b't', b'a', b't', b'i', b'o', b'n', b's', // 16..44 CP[2]: Utf8 "RuntimeVisibleAnnotations"
        7, 0x00, 0x01, // 44..47 CP[3]: Class(1)
        7, 0x00, 0x02, // 47..50 CP[4]: Class(2)
        0x00, 0x01, // 50..52 access_flags
        0x00, 0x03, // 52..54 this_class (CP[3])
        0x00, 0x04, // 54..56 super_class (CP[4])
        0x00, 0x00, // 56..58 interfaces_count
        0x00, 0x00, // 58..60 fields_count
        0x00, 0x00, // 60..62 methods_count
        0x00, 0x01, // 62..64 attributes_count
        0x00, 0x02, // 64..66 attribute_name_index (CP[2]) "RuntimeVisibleAnnotations"
        0x00, 0x00, 0x00, 0x00, // 66..70 attribute_length (will patch later)
        0x00, 0x01, // 70..72 num_annotations
        0x00, 0x01, // 72..74 type_index (CP[1]) "Foo"
        0x00, 0x01, // 74..76 num_element_value_pairs
        0x00, 0x01, // 76..78 element_name_index
    ];

    let depth = 500; // Anything above 256 should hit our new limit
    for _ in 0..depth {
        data.push(b'@');
        data.push(0x00);
        data.push(0x01); // type_index
        data.push(0x00);
        data.push(0x01); // num_element_value_pairs
        data.push(0x00);
        data.push(0x01); // element_name_index
    }
    data.push(b'B'); // Base case
    data.push(0x00);
    data.push(0x01); // Const value index

    // Patch attribute_length
    let attr_len = (data.len() - 70) as u32; // 70 is the start of attribute data
    data[66] = (attr_len >> 24) as u8;
    data[67] = (attr_len >> 16) as u8;
    data[68] = (attr_len >> 8) as u8;
    data[69] = attr_len as u8;

    let res = duke_classfile::parse(&data);
    assert!(matches!(res, Err(duke_classfile::Error::AnnotationDepthLimit)), "Expected AnnotationDepthLimit");
}
