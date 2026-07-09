//! CLI-level integration tests for the `duke -jar` Spring Boot launch path.
//!
//! These drive the real `duke` binary through `run_jar`, exercising both a
//! genuine fat JAR (`Main-Class: JarLauncher` + `Start-Class` + `BOOT-INF`) and
//! the launcher-detection fallback (an archive that bundles `JarLauncher`
//! without declaring a `Main-Class`).

use std::path::{Path, PathBuf};
use std::process::Command;

use duke_loader::ZipReader;

const JAR_LAUNCHER_ENTRY: &str = "org/springframework/boot/loader/launch/JarLauncher.class";

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at the `duke` crate; the workspace root is its parent.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn spring_boot_loader_jar() -> PathBuf {
    repo_root().join("spring-boot-loader-3.5.12.jar")
}

fn hello_world_class() -> Vec<u8> {
    std::fs::read(repo_root().join("tests/fixtures/HelloWorld.class"))
        .expect("read HelloWorld.class fixture")
}

/// Standard CRC-32 (matching the zip spec), used to build STORED zip entries.
fn zip_crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Build a minimal uncompressed (STORED) zip from the given entries.
fn build_stored_zip(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
    const CD_SIGNATURE: u32 = 0x0201_4b50;
    const EOCD_SIGNATURE: u32 = 0x0605_4b50;
    const METHOD_STORED: u16 = 0;

    let count = u16::try_from(entries.len()).expect("entry count fits in u16");
    let mut zip = Vec::new();
    let mut local_offsets = Vec::with_capacity(entries.len());

    for (name, content) in entries {
        let crc = zip_crc32(content);
        let size = u32::try_from(content.len()).expect("entry size fits in u32");
        let name_bytes = name.as_bytes();

        local_offsets.push(u32::try_from(zip.len()).expect("offset fits in u32"));
        zip.extend_from_slice(&LOCAL_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&20_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&METHOD_STORED.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&size.to_le_bytes());
        zip.extend_from_slice(&size.to_le_bytes());
        zip.extend_from_slice(
            &u16::try_from(name_bytes.len())
                .expect("name length fits in u16")
                .to_le_bytes(),
        );
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(name_bytes);
        zip.extend_from_slice(content);
    }

    let cd_offset = u32::try_from(zip.len()).expect("central directory offset fits in u32");
    for (index, (name, content)) in entries.iter().enumerate() {
        let crc = zip_crc32(content);
        let size = u32::try_from(content.len()).expect("entry size fits in u32");
        let name_bytes = name.as_bytes();

        zip.extend_from_slice(&CD_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&20_u16.to_le_bytes());
        zip.extend_from_slice(&20_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&METHOD_STORED.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&size.to_le_bytes());
        zip.extend_from_slice(&size.to_le_bytes());
        zip.extend_from_slice(
            &u16::try_from(name_bytes.len())
                .expect("name length fits in u16")
                .to_le_bytes(),
        );
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u32.to_le_bytes());
        zip.extend_from_slice(&local_offsets[index].to_le_bytes());
        zip.extend_from_slice(name_bytes);
    }
    let cd_size = u32::try_from(zip.len()).expect("central directory size fits in u32") - cd_offset;

    zip.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&count.to_le_bytes());
    zip.extend_from_slice(&count.to_le_bytes());
    zip.extend_from_slice(&cd_size.to_le_bytes());
    zip.extend_from_slice(&cd_offset.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());

    zip
}

/// Read every entry of the repo-root Spring Boot loader library, decompressed.
fn spring_boot_loader_entries() -> Vec<(String, Vec<u8>)> {
    let reader = ZipReader::open(&spring_boot_loader_jar()).expect("open spring-boot-loader jar");
    reader
        .entry_names()
        .filter(|name| !name.ends_with('/'))
        .map(|name| {
            let bytes = reader.read_entry(name).expect("read loader entry");
            (name.to_string(), bytes)
        })
        .collect()
}

/// Write `bytes` to a uniquely named temp file with a `.jar` suffix.
fn write_temp_jar(tag: &str, bytes: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "duke_cli_boot_{tag}_{}.jar",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::write(&path, bytes).expect("write temp jar");
    path
}

/// A real fat JAR launched through the CLI boots its `Start-Class` main.
#[test]
fn real_fat_jar_launches_via_cli() {
    let mut entries = spring_boot_loader_entries();
    entries.push((
        "META-INF/MANIFEST.MF".to_string(),
        b"Manifest-Version: 1.0\r\nMain-Class: org.springframework.boot.loader.launch.JarLauncher\r\nStart-Class: HelloWorld\r\n\r\n"
            .to_vec(),
    ));
    entries.push((
        "BOOT-INF/classes/HelloWorld.class".to_string(),
        hello_world_class(),
    ));

    let jar_path = write_temp_jar("fat", &build_stored_zip(&entries));
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("-jar")
        .arg(&jar_path)
        .output()
        .expect("run duke -jar on synthetic fat jar");
    std::fs::remove_file(&jar_path).ok();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("Hello, World!"),
        "expected fat jar to boot Start-Class main; stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(
        !stderr.contains("no Main-Class attribute"),
        "fat jar with explicit Main-Class must not hit the no-Main-Class path; stderr={stderr:?}"
    );
}

/// An archive bundling `JarLauncher` but declaring no `Main-Class` is routed to
/// the launcher via the detection fallback, rather than aborting with the
/// `no Main-Class attribute` error.
#[test]
fn launcher_detection_fallback_routes_without_main_class() {
    let entries = spring_boot_loader_entries();
    assert!(
        entries.iter().any(|(name, _)| name == JAR_LAUNCHER_ENTRY),
        "loader library should contain the JarLauncher class"
    );

    // Manifest deliberately omits Main-Class and Start-Class.
    let mut with_manifest = entries;
    with_manifest.push((
        "META-INF/MANIFEST.MF".to_string(),
        b"Manifest-Version: 1.0\r\n\r\n".to_vec(),
    ));

    let jar_path = write_temp_jar("fallback", &build_stored_zip(&with_manifest));
    let output = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg("-jar")
        .arg(&jar_path)
        .output()
        .expect("run duke -jar on launcher-only jar");
    std::fs::remove_file(&jar_path).ok();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // The fallback must engage: we should never see the no-Main-Class abort, and
    // we should never see a "cannot load class JarLauncher" error (it loads fine).
    assert!(
        !stderr.contains("no Main-Class attribute"),
        "launcher detection should route to JarLauncher, not abort; stderr={stderr:?}"
    );
    assert!(
        !stderr.contains("cannot load class"),
        "JarLauncher should load from the archive; stderr={stderr:?}"
    );
}
