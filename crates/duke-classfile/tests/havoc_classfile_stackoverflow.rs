use duke_classfile::parse;

#[test]
fn test_annotation_stack_overflow() {
    // Construct a malicious classfile with deeply nested annotations.
    let mut bytes = vec![
        0xCA, 0xFE, 0xBA, 0xBE, // magic
        0x00, 0x00, 0x00, 0x41, // version
    ];

    // Constant pool count: 2 (indices 1)
    bytes.push(0x00);
    bytes.push(0x02);

    // CP 1: Utf8 "RuntimeVisibleAnnotations"
    let attr_name = b"RuntimeVisibleAnnotations";
    bytes.push(1); // tag Utf8
    bytes.push(0);
    bytes.push(u8::try_from(attr_name.len()).unwrap());
    bytes.extend_from_slice(attr_name);

    // access_flags, this_class, super_class
    bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x00, 0x01]);

    // interfaces count
    bytes.extend_from_slice(&[0x00, 0x00]);

    // fields count
    bytes.extend_from_slice(&[0x00, 0x00]);

    // methods count
    bytes.extend_from_slice(&[0x00, 0x00]);

    // attributes count: 1
    bytes.extend_from_slice(&[0x00, 0x01]);

    // Attribute: RuntimeVisibleAnnotations
    bytes.extend_from_slice(&[0x00, 0x01]); // name_index

    // Attribute length: we will fill this later
    let attr_len_idx = bytes.len();
    bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    let attr_start = bytes.len();

    // num_annotations: 1
    bytes.extend_from_slice(&[0x00, 0x01]);

    // Let's create deeply nested arrays
    let depth = 10000; // Deeply nested to cause stack overflow

    // start annotation
    bytes.extend_from_slice(&[0x00, 0x01]); // type_index
    bytes.extend_from_slice(&[0x00, 0x01]); // num_pairs
    bytes.extend_from_slice(&[0x00, 0x01]); // element_name_index

    for _ in 0..depth {
        bytes.push(b'['); // array
        bytes.extend_from_slice(&[0x00, 0x01]); // num_values: 1
    }

    bytes.push(b'B'); // base value
    bytes.extend_from_slice(&[0x00, 0x01]); // const_value_index

    let attr_len = u32::try_from(bytes.len() - attr_start).unwrap();
    bytes[attr_len_idx] = u8::try_from(attr_len >> 24).unwrap();
    bytes[attr_len_idx + 1] = u8::try_from(attr_len >> 16).unwrap();
    bytes[attr_len_idx + 2] = u8::try_from(attr_len >> 8).unwrap();
    bytes[attr_len_idx + 3] = u8::try_from(attr_len & 0xFF).unwrap();

    let _ = parse(&bytes);
}
