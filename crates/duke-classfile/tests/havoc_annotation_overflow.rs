#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation)]
use std::thread;

#[test]
fn test_annotation_stack_overflow() {
    let depth = 50000;
    let mut raw = Vec::new();

    // Classfile Header
    raw.extend_from_slice(&[0xca, 0xfe, 0xba, 0xbe]); // Magic
    raw.extend_from_slice(&[0x00, 0x00, 0x00, 0x34]); // Version
    raw.extend_from_slice(&[0x00, 0x02]); // CP Count = 2
    raw.extend_from_slice(&[0x01, 0x00, 0x19]); // CP[1]: Utf8 "RuntimeVisibleAnnotations"
    raw.extend_from_slice(b"RuntimeVisibleAnnotations");
    raw.extend_from_slice(&[0x00, 0x00]); // Access Flags
    raw.extend_from_slice(&[0x00, 0x00]); // This Class
    raw.extend_from_slice(&[0x00, 0x00]); // Super Class
    raw.extend_from_slice(&[0x00, 0x00]); // Interfaces Count
    raw.extend_from_slice(&[0x00, 0x00]); // Fields Count
    raw.extend_from_slice(&[0x00, 0x00]); // Methods Count
    raw.extend_from_slice(&[0x00, 0x01]); // Attributes Count
    raw.extend_from_slice(&[0x00, 0x01]); // attr_name_index = 1 ("RuntimeVisibleAnnotations")

    let attr_len = (6_usize + depth * 7) as u32;
    raw.extend_from_slice(&attr_len.to_be_bytes());

    raw.extend_from_slice(&1_u16.to_be_bytes()); // num_annotations = 1

    for _ in 0..depth {
        raw.extend_from_slice(&1_u16.to_be_bytes()); // type_index
        raw.extend_from_slice(&1_u16.to_be_bytes()); // num_pairs = 1
        raw.extend_from_slice(&1_u16.to_be_bytes()); // element_name_index
        raw.push(b'@'); // tag
    }

    raw.extend_from_slice(&1_u16.to_be_bytes());
    raw.extend_from_slice(&0_u16.to_be_bytes());

    let handle = thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let res = duke_classfile::parse(&raw);
            assert!(res.is_err());
            // It will fail during lazy parsing? No, lazy parsing was removed or what?
            // Actually `decode_runtime_visible_annotations` is called within `parse_class_file` -> `decode_known_attribute`!
            assert!(res.unwrap_err().to_string().contains("annotation recursion depth exceeded"));
        })
        .unwrap();

    handle.join().unwrap();
}
