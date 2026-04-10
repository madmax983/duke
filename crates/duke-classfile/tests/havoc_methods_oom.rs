use duke_classfile::parser;

#[test]
fn test_methods_oom() {
    let mut data = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 0];
    // cp count
    data.extend([0, 1]);
    // flags
    data.extend([0, 0]);
    // this class
    data.extend([0, 0]);
    // super class
    data.extend([0, 0]);
    // interfaces count
    data.extend([0, 0]);
    // fields count
    data.extend([0, 0]);
    // methods count
    data.extend([0xFF, 0xFF]);

    let _ = parser::parse(&data);
}
