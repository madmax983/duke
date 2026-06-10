//! Integration tests for the jar-analyze and jar-scan CLI commands.
use std::process::Command;
#[test]
fn test_dump_jar_analyze_and_scan() {
    let mut p = std::env::current_dir().unwrap();
    p.push("spring-boot-loader-3.5.12.jar");

    let jar_path = p.to_str().unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--bin", "duke", "--", "jar-analyze", jar_path])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("JAR Analysis Summary"));

    let output_scan = Command::new("cargo")
        .args(&["run", "--bin", "duke", "--", "jar-scan", jar_path])
        .output()
        .expect("failed to execute process");

    assert!(output_scan.status.success());
    let stdout_scan = String::from_utf8_lossy(&output_scan.stdout);
    assert!(stdout_scan.contains("JAR Security Scan Report"));
}
