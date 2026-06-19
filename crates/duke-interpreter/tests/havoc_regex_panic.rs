#[test]
fn test_regex_panic_size_limit() {
    // Generate a massive regex that exceeds the size limit
    let mut delim = String::new();
    for _ in 0..10_000_000 {
        delim.push_str("a|");
    }
    delim.push('b');

    let _re = regex::Regex::new(&delim)
        .unwrap_or_else(|_| regex::Regex::new(&regex::escape(&delim)).unwrap());
    println!("OK");
}
