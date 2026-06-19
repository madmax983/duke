#[test]
fn test_complex_regex() {
    let mut large_delim = String::with_capacity(1_000_000);
    for _ in 0..1_000_000 {
        large_delim.push_str("(a|b)");
    }
    let re = regex::Regex::new(&large_delim);
    assert!(re.is_err());
}
