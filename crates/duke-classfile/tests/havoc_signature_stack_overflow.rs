use duke_classfile::parse_field_signature;

#[test]
fn test_stack_overflow_array() {
    let s = "[".repeat(10000) + "I";
    let res = parse_field_signature(&s);
    assert!(res.is_err());
}

#[test]
fn test_stack_overflow_generics() {
    let s = "LList<".repeat(10000) + "LString;" + ">;".repeat(10000).as_str();
    let res = parse_field_signature(&s);
    assert!(res.is_err());
}
