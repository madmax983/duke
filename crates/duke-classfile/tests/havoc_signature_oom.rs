use duke_classfile::signature::*;

#[test]
fn test_deep_array_signature() {
    let mut sig = String::from("");
    for _ in 0..10000 {
        sig.push_str("[");
    }
    sig.push_str("I");

    let res = parse_field_signature(&sig);
    assert!(res.is_err());
}
