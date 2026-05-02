#![allow(missing_docs)]

use duke_classfile::parse;

#[test]
fn test_annotation_stack_overflow() {
    let mut data = vec![
        0xca, 0xfe, 0xba, 0xbe, // magic
        0x00, 0x00, // minor version
        0x00, 0x41, // major version (65)
        0x00, 0x05, // constant pool count (5)
        // cp info: 1 - utf8 "RuntimeVisibleAnnotations"
        0x01, 0x00, 0x19, b'R', b'u', b'n', b't', b'i', b'm', b'e', b'V', b'i', b's', b'i', b'b',
        b'l', b'e', b'A', b'n', b'n', b'o', b't', b'a', b't', b'i', b'o', b'n', b's',
        // cp info: 2 - utf8 "foo"
        0x01, 0x00, 0x03, b'f', b'o', b'o', // cp info: 3 - utf8 "bar"
        0x01, 0x00, 0x03, b'b', b'a', b'r', // cp info: 4 - utf8 "baz"
        0x01, 0x00, 0x03, b'b', b'a', b'z', 0x00, 0x00, // access flags
        0x00, 0x00, // this class
        0x00, 0x00, // super class
        0x00, 0x00, // interfaces count
        0x00, 0x00, // fields count
        0x00, 0x00, // methods count
        0x00, 0x01, // attributes count
        // attribute RuntimeVisibleAnnotations
        0x00, 0x01, // name index (1)
    ];

    let mut attr_data = vec![];
    attr_data.extend_from_slice(&[0x00, 0x01]); // num_annotations

    // Let's create deeply nested arrays
    attr_data.extend_from_slice(&[0x00, 0x02]); // annotation.type_index
    attr_data.extend_from_slice(&[0x00, 0x01]); // num_element_value_pairs
    attr_data.extend_from_slice(&[0x00, 0x03]); // element_name_index

    // Value: lots of nested arrays
    let depth = 50_000;
    for _ in 0..depth {
        attr_data.push(b'[');
        attr_data.extend_from_slice(&[0x00, 0x01]); // num_values = 1
    }

    // Final value
    attr_data.push(b'I');
    attr_data.extend_from_slice(&[0x00, 0x04]); // const value index

    // attr length
    let attr_len = u32::try_from(attr_data.len()).unwrap();
    data.extend_from_slice(&attr_len.to_be_bytes());
    data.extend(attr_data);

    // This used to panic with a stack overflow! Now it should return RecursionLimitExceeded.
    let result = parse(&data);
    assert!(
        matches!(
            result,
            Err(duke_classfile::Error::RecursionLimitExceeded { depth: 16 })
        ),
        "Expected RecursionLimitExceeded, got {result:?}",
    );
}
