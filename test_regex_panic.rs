fn main() {
    let delim = "a".repeat(10_000_000);
    let _re = regex::Regex::new(&delim)
        .unwrap_or_else(|_| regex::Regex::new(&regex::escape(&delim)).unwrap());
    println!("OK");
}
