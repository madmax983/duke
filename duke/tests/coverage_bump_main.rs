use std::process::Command;

#[test]
fn test_cli_subcommands() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("unknown-subcommand-12345")
        .arg("dummy.class")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duke: cannot read 'dummy.class'"));
}

#[test]
fn test_cli_load_nonexistent() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("load")
        .arg("NonExistentClass123")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duke: class not found: NonExistentClass123"));
}

#[test]
fn test_cli_dump_nonexistent() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("dump")
        .arg("NonExistentClass123.class")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duke: cannot read 'NonExistentClass123.class'"));
}

#[test]
fn test_cli_jar_dead_code_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("jar-dead-code")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage: duke jar-dead-code <file.jar>"));
}

#[test]
fn test_cli_jar_search_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("jar-search")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage: duke jar-search <file.jar> <query>"));
}

#[test]
fn test_cli_audit_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("audit")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage: duke audit <file.jar>"));
}

#[test]
fn test_cli_help_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke")).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage: duke <classfile.class>"));
}

#[test]
fn test_cli_jar_diff_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("jar-diff")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage: duke jar-diff <file1.jar> <file2.jar>"));
}

#[test]
fn test_cli_html_jar_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("html-jar")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot read"));
}

#[test]
fn test_cli_html_jar_one_arg() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("html-jar")
        .arg("some.jar")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot read"));
}

#[test]
fn test_cli_html_jar_nonexistent() {
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("html-jar")
        .arg("some_non_existent_file.jar")
        .arg("out")
        .output()
        .unwrap();
    assert!(!output.status.success());
}
