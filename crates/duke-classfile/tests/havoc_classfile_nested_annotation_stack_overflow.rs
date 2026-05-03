#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation)]
use duke_classfile::parse;

#[test]
fn test_classfile_nested_annotation_stack_overflow() {
    let mut data = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 65]; // magic & version
    data.push(0); data.push(3); // pool count (idx 1, 2)
    data.push(1); // Utf8 #1
    let s = b"RuntimeVisibleAnnotations";
    data.push(u8::try_from(s.len() >> 8).unwrap());
    data.push(u8::try_from(s.len() & 0xFF).unwrap());
    data.extend_from_slice(s);

    data.push(1); // Utf8 #2 (dummy type name)
    let s2 = b"dummy";
    data.push(u8::try_from(s2.len() >> 8).unwrap());
    data.push(u8::try_from(s2.len() & 0xFF).unwrap());
    data.extend_from_slice(s2);


    // access flags, this class, super class
    data.extend_from_slice(&[0, 0, 0, 1, 0, 2]); // valid cp indexes
    // interfaces, fields, methods
    data.extend_from_slice(&[0, 0, 0, 0, 0, 0]);

    // attributes count
    data.extend_from_slice(&[0, 1]); // 1 attribute

    // attribute RuntimeVisibleAnnotations (name_index = 1)
    data.extend_from_slice(&[0, 1]);

    data.extend_from_slice(&[0, 0, 200, 0]); // Length

    data.extend_from_slice(&[0, 1]); // 1 annotation
    // Create deeply nested annotation to cause stack overflow
    for _ in 0..100 {
        data.extend_from_slice(&[0, 2]); // type index
        data.extend_from_slice(&[0, 1]); // 1 pair
        data.extend_from_slice(&[0, 2]); // element name
        data.push(b'@'); // annotation element
    }

    // append some dummy bytes so it doesn't EOF
    data.extend_from_slice(&[0; 1000]);

    let result = parse(&data);

    assert!(result.is_err(), "It should fail.");
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("maximum recursion depth exceeded") || err_str.contains("unexpected end of input"), "Expected RecursionLimitExceeded error, got: {err_str}");
}
