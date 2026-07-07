use std::process::Command;

#[test]
fn test_extract_jdk_flag_long() {
    let bin = env!("CARGO_BIN_EXE_duke");

    let output = Command::new(bin)
        .arg("--jdk=/fake/path")
        .arg("run")
        .arg("NonExistentClass")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No main class found") || stderr.contains("No such file") || stderr.contains("not found") || stderr.contains("Cannot load main class"));

    let output2 = Command::new(bin)
        .arg("--jdk")
        .arg("/fake/path2")
        .arg("run")
        .arg("NonExistentClass")
        .output()
        .unwrap();
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    assert!(stderr2.contains("No main class found") || stderr2.contains("No such file") || stderr2.contains("not found") || stderr2.contains("Cannot load main class"));

    let output3 = Command::new(bin)
        .arg("--telemetry")
        .arg("run")
        .arg("NonExistentClass")
        .output()
        .unwrap();
    let stderr3 = String::from_utf8_lossy(&output3.stderr);
    assert!(stderr3.contains("No main class found") || stderr3.contains("No such file") || stderr3.contains("not found") || stderr3.contains("Cannot load main class"));

    let output4 = Command::new(bin)
        .arg("--telemetry=file:test.json")
        .arg("run")
        .arg("NonExistentClass")
        .output()
        .unwrap();
    let stderr4 = String::from_utf8_lossy(&output4.stderr);
    assert!(stderr4.contains("No main class found") || stderr4.contains("No such file") || stderr4.contains("not found") || stderr4.contains("Cannot load main class"));

    let output5 = Command::new(bin)
        .arg("--telemetry=markdown:test.md")
        .arg("run")
        .arg("NonExistentClass")
        .output()
        .unwrap();
    let stderr5 = String::from_utf8_lossy(&output5.stderr);
    assert!(stderr5.contains("No main class found") || stderr5.contains("No such file") || stderr5.contains("not found") || stderr5.contains("Cannot load main class"));
}
