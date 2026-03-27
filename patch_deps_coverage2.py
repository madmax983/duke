import re
with open("duke/src/deps.rs", "r") as f:
    code = f.read()

test_search = r'''    #[test]
    fn test_cp_str_invalid() {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../tests/fixtures/HelloWorld.class");
        let bytes = std::fs::read(&p).unwrap();
        let cf = parse(&bytes).unwrap();
        // Out of bounds
        assert_eq!(cp_str(&cf, CpIndex(999)), None);
        // Not a utf8 entry
        assert_eq!(cp_str(&cf, CpIndex(1)), None);
    }
}'''

test_replace = r'''    #[test]
    fn test_cp_str_invalid() {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../tests/fixtures/HelloWorld.class");
        let bytes = std::fs::read(&p).unwrap();
        let cf = parse(&bytes).unwrap();
        // Out of bounds
        assert_eq!(cp_str(&cf, CpIndex(999)), None);
        // Not a utf8 entry
        assert_eq!(cp_str(&cf, CpIndex(1)), None);
    }
}'''

# I'm actually not going to write a test here that exits the process,
# but maybe we can refactor dump_dependencies to return Result<(), String>
# so it's fully testable.
