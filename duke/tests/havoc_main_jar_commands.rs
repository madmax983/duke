use std::process::Command;

#[test]
fn test_jar_search_invalid_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("jar-search")
        .output()
        .expect("Failed to execute duke");
    assert!(!output.status.success());
}

#[test]
fn test_jar_diff_invalid_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("jar-diff")
        .output()
        .expect("Failed to execute duke");
    assert!(!output.status.success());
}

#[test]
fn test_jar_dead_code_invalid_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("jar-dead-code")
        .output()
        .expect("Failed to execute duke");
    assert!(!output.status.success());
}

#[test]
fn test_audit_invalid_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("audit")
        .output()
        .expect("Failed to execute duke");
    assert!(!output.status.success());
}

#[test]
fn test_exec_missing_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("exec")
        .output()
        .expect("Failed to execute duke");
    assert!(!output.status.success());
}
