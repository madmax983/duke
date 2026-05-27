use std::process::Command;

#[test]
fn test_dump_search_invalid_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("search")
        .arg("non_existent_file.class")
        .arg("invokevirtual")
        .output()
        .expect("Failed to execute duke");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot read 'non_existent_file.class'"));
}

#[test]
fn test_dump_search_invalid_classfile() {
    let tmp = std::env::temp_dir().join("invalid_search.class");
    std::fs::write(&tmp, b"not a class file").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("search")
        .arg(tmp.to_str().unwrap())
        .arg("invokevirtual")
        .output()
        .expect("Failed to execute duke");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duke: parse error:"));

    let _ = std::fs::remove_file(&tmp);
}
