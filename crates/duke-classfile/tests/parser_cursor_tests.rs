use duke_classfile::parse;

#[test]
fn test_parser_cursor_methods() {
    let empty_class = vec![
        0xca, 0xfe, 0xba, 0xbe, // Magic
        0x00, 0x00, // Minor
        0x00, 0x40, // Major (64)
        0x00, 0x01, // Constant pool count (1)
        0x00, 0x21, // Access flags (super | public)
        0x00, 0x00, // This class
        0x00, 0x00, // Super class
        0x00, 0x00, // Interfaces count
        0x00, 0x00, // Fields count
        0x00, 0x00, // Methods count
        0x00, 0x00, // Attributes count
    ];
    let _ = parse(&empty_class);

    let empty_class2 = vec![
        0xca, 0xfe, 0xba, 0xbe, // Magic
        0x00, 0x00, // Minor
        0x00, 0x40, // Major (64)
        0x00, 0x01, // Constant pool count (1)
        0x00, 0x21, // Access flags
        0x00, 0x00, // This class
        0x00, 0x00, // Super class
        0x00, 0x01, // Interfaces count
        0x00, 0x00, // Fields count
    ];
    let _ = parse(&empty_class2);

    let empty_class3 = vec![
        0xca, 0xfe, 0xba, 0xbe, // Magic
        0x00, 0x00, // Minor
        0x00, 0x40, // Major (64)
        0x00, 0x01, // Constant pool count (1)
        0x00, 0x21, // Access flags
        0x00, 0x00, // This class
        0x00, 0x00, // Super class
        0x00, 0x00, // Interfaces count
        0x00, 0x01, // Fields count
        0x00, 0x00, // Field flags
        0x00, 0x00, // Field name
        0x00, 0x00, // Field desc
    ];
    let _ = parse(&empty_class3);
}
