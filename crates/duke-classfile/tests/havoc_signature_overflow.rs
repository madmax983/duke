use duke_classfile::signature::parse_field_signature;

#[test]
fn test_signature_stack_overflow() {
    let mut sig = String::new();
    for _ in 0..10000 {
        sig.push_str("Ljava/util/List<");
    }
    sig.push_str("Ljava/lang/String;");
    for _ in 0..10000 {
        sig.push_str(">;");
    }
    let res = parse_field_signature(&sig);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(err.message, "signature recursion depth limit exceeded");
}
