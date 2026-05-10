#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation)]
use duke_classfile::{Error, parse};

#[test]
fn test_annotation_bombs_infinite_recursion() {
    let mut data = vec![
        0xca, 0xfe, 0xba, 0xbe, // magic (0)
        0x00, 0x00, 0x00, 0x41, // version (4)
        0x00, 0x05, // cp count (8) - size is 5, meaning indexes 1, 2, 3, 4 are valid
        // cp #1 Utf8 "RuntimeVisibleAnnotations" (10)
        0x01, 0x00, 0x19, // (10)
        b'R', b'u', b'n', b't', b'i', b'm', b'e', b'V', b'i', b's', b'i', b'b', b'l', b'e', b'A',
        b'n', b'n', b'o', b't', b'a', b't', b'i', b'o', b'n', b's', // (13)
        // cp #2 Utf8 "Lcom/example/Annotation;" (38)
        0x01, 0x00, 0x18, b'L', b'c', b'o', b'm', b'/', b'e', b'x', b'a', b'm', b'p', b'l', b'e',
        b'/', b'A', b'n', b'n', b'o', b't', b'a', b't', b'i', b'o', b'n', b';', // (38)
        // cp #3 Utf8 "Foo" (65)
        0x01, 0x00, 0x03, b'F', b'o', b'o', // (65)
        // cp #4 Class (71)
        0x07, 0x00, 0x03, // (71)
        0x00, 0x00, // flags (74)
        0x00, 0x04, // this (76)
        0x00, 0x00, // super (78)
        0x00, 0x00, // interfaces (80)
        0x00, 0x00, // fields (82)
        0x00, 0x00, // methods (84)
        0x00, 0x01, // attrs count (86)
        0x00, 0x01, // name_idx = "RuntimeVisibleAnnotations" (88)
        0x00, 0x00, 0x00,
        0x00, // length (90) - offsets 90,91,92,93 - we will overwrite this
              // annotation data (94)
    ];

    let mut nested_payload = vec![
        0x00, 0x01, // num_annotations (94)
        0x00, 0x02, // type_index (96)
        0x00, 0x01, // num_pairs = 1 (98)
        0x00, 0x03, // element_name_index (100) - "Foo"
        b'@', // value = nested annotation (102)
        0x00, 0x02, // type_index (103)
        0x00, 0x01, // num_pairs (105)
    ];
    for _ in 0..100 {
        nested_payload.extend_from_slice(&[
            0x00, 0x03, // element_name_index
            b'@', // value = nested annotation
            0x00, 0x02, // type_index
            0x00, 0x01, // num_pairs
        ]);
    }
    nested_payload.extend_from_slice(&[
        0x00, 0x03, // element_name_index
        b's', // string
        0x00, 0x03, // const string value index (Foo)
    ]);

    // Patch length
    let payload_len = nested_payload.len();
    data[90] = (payload_len >> 24) as u8;
    data[91] = (payload_len >> 16) as u8;
    data[92] = (payload_len >> 8) as u8;
    data[93] = payload_len as u8;

    data.extend(nested_payload);

    let err = parse(&data).unwrap_err();
    assert!(
        matches!(err, Error::RecursionLimitExceeded { .. }),
        "Expected RecursionLimitExceeded, got: {err:?}"
    );
}
