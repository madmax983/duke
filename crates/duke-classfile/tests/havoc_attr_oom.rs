use duke_classfile::parse;

#[test]
fn havoc_test_attr_oom() {
    // 0xCAFEBABE, java 21
    let mut prefix = vec![0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x00, 0x00, 0x41];

    // CP count 3
    prefix.push(0x00);
    prefix.push(0x03);

    // CP entry 1: UTF8 "LineNumberTable"
    prefix.push(0x01);
    prefix.push(0x00);
    prefix.push(15);
    prefix.extend(b"LineNumberTable");

    // CP entry 2: Class, name_index 1
    prefix.push(0x07);
    prefix.push(0x00);
    prefix.push(0x01);

    // Access flags
    prefix.push(0x00);
    prefix.push(0x00);

    // This class
    prefix.push(0x00);
    prefix.push(0x02);
    // Super class
    prefix.push(0x00);
    prefix.push(0x00);

    // Interfaces count
    prefix.push(0x00);
    prefix.push(0x00);

    // Fields count
    prefix.push(0x00);
    prefix.push(0x00);

    // Methods count
    prefix.push(0x00);
    prefix.push(0x00);

    // Attributes count (1)
    prefix.push(0x00);
    prefix.push(0x01);

    // Attribute 1: LineNumberTable
    // name_index = 1
    prefix.push(0x00);
    prefix.push(0x01);

    // length = 100
    prefix.push(0x00);
    prefix.push(0x00);
    prefix.push(0x00);
    prefix.push(100);

    // number of entries = 0xFFFF
    prefix.push(0xFF);
    prefix.push(0xFF);

    let result = parse(&prefix);
    // Ensure it doesn't OOM, it should fail parsing correctly because EOF
    assert!(result.is_err());
}
