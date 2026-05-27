use std::process::Command;

#[test]
fn test_main_run_jar_invalid() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("-jar")
        .arg("non_existent_file.jar")
        .output()
        .expect("Failed to execute duke");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot open JAR"));
}
