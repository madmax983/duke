use std::process::Command;
use std::path::PathBuf;

#[test]
fn test_run_main_coverage() {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../tests/fixtures/HelloWorld.class");

    let output = Command::new("cargo")
        .args(&["run", "--bin", "duke", "--", "run", p.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
       println!("stdout: {}", stdout);
       println!("stderr: {}", stderr);
    }

    assert!(output.status.success());
    assert!(stdout.contains("Hello, World!"));
}

#[test]
fn test_run_jar_coverage() {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../spring-boot-loader-3.5.12.jar");

    let output = Command::new("cargo")
        .args(&["run", "--bin", "duke", "--", "-jar", p.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    let _ = output.status;
}
