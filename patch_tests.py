import re

with open("duke/src/main.rs", "r") as f:
    content = f.read()

test_code = """
    #[test]
    fn test_dump_uml() {
        // Find HelloWorld.class in tests/fixtures
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../tests/fixtures/HelloWorld.class");

        // Ensure it doesn't panic on a valid class
        super::dump_uml(p.to_str().unwrap());
    }
"""

if "fn test_dump_uml" not in content:
    content = content.replace("    fn test_dump_html() {", test_code + "\n    #[test]\n    fn test_dump_html() {")

with open("duke/src/main.rs", "w") as f:
    f.write(content)
