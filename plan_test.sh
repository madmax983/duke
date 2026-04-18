sed -i 's/fn test_extract_bbcfg() {/fn test_dump_bbcfg_valid() {\n        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));\n        p.push("..\/tests\/fixtures\/HelloWorld.class");\n        dump_bbcfg(p.to_str().unwrap(), "main");\n    }\n\n    #[test]\n    fn test_extract_bbcfg_old() {/g' duke/src/main.rs
cargo test -p duke
