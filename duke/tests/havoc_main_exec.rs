use std::process::Command;

#[test]
fn test_main_exec_invalid_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("exec")
        .output()
        .expect("Failed to execute duke");
    assert!(!output.status.success());
}

#[test]
fn test_main_run_invalid_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("run")
        .output()
        .expect("Failed to execute duke");
    assert!(!output.status.success());
}
