#![allow(clippy::unreadable_literal)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_sign_loss)]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn create_valid_class(name: &str) {
    let magic = 0xCAFEBABE_u32;
    let minor = 0_u16;
    let major = 52_u16;
    let pool_count = 1_u16;
    let access_flags = 0x0021_u16;
    let this_class = 0_u16;
    let super_class = 0_u16;
    let interfaces_count = 0_u16;
    let fields_count = 0_u16;
    let methods_count = 0_u16;
    let attributes_count = 0_u16;

    let mut data = Vec::new();
    data.extend_from_slice(&magic.to_be_bytes());
    data.extend_from_slice(&minor.to_be_bytes());
    data.extend_from_slice(&major.to_be_bytes());
    data.extend_from_slice(&pool_count.to_be_bytes());
    data.extend_from_slice(&access_flags.to_be_bytes());
    data.extend_from_slice(&this_class.to_be_bytes());
    data.extend_from_slice(&super_class.to_be_bytes());
    data.extend_from_slice(&interfaces_count.to_be_bytes());
    data.extend_from_slice(&fields_count.to_be_bytes());
    data.extend_from_slice(&methods_count.to_be_bytes());
    data.extend_from_slice(&attributes_count.to_be_bytes());

    std::fs::write(name, &data).unwrap();
}

#[test]
fn test_jdwp_getvalues_oom() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    create_valid_class("dummy1.class");

    let mut child = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg(&format!(
            "-agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address={}",
            port
        ))
        .arg("dummy1.class")
        .spawn()
        .unwrap();

    let mut stream = loop {
        if let Ok(s) = TcpStream::connect(format!("127.0.0.1:{}", port)) {
            break s;
        }
        thread::sleep(Duration::from_millis(10));
    };

    stream.write_all(b"JDWP-Handshake").unwrap();
    let mut buf = [0u8; 14];
    stream.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b"JDWP-Handshake");

    let mut event_header = [0u8; 11];
    stream.read_exact(&mut event_header).unwrap();
    let length = u32::from_be_bytes([
        event_header[0],
        event_header[1],
        event_header[2],
        event_header[3],
    ]);
    let mut payload = vec![0; length as usize - 11];
    stream.read_exact(&mut payload).unwrap();

    let length = 11 + 4; // 15 bytes
    let mut pkt = Vec::new();
    pkt.extend_from_slice(&(length as u32).to_be_bytes());
    pkt.extend_from_slice(&1_i32.to_be_bytes());
    pkt.push(0); // flags
    pkt.push(9); // cmd_set: ObjectReference
    pkt.push(2); // cmd: GetValues

    let count = 0x0FFFFFFF; // Huge count
    pkt.extend_from_slice(&(count as u32).to_be_bytes());

    stream.write_all(&pkt).unwrap();

    let mut reply_header = [0u8; 11];
    let _ = stream.read_exact(&mut reply_header);

    let length = 11;
    let mut pkt = Vec::new();
    pkt.extend_from_slice(&(length as u32).to_be_bytes());
    pkt.extend_from_slice(&2_i32.to_be_bytes());
    pkt.push(0);
    pkt.push(1); // VirtualMachine
    pkt.push(9); // Resume
    let _ = stream.write_all(&pkt);

    let status = child.wait().unwrap();
    std::fs::remove_file("dummy1.class").unwrap();
    assert!(
        status.success(),
        "Process crashed due to OOM! exit code: {:?}",
        status.code()
    );
}

#[test]
fn test_jdwp_stackframe_getvalues_oom() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    create_valid_class("dummy2.class");

    let mut child = Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg(&format!(
            "-agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address={}",
            port
        ))
        .arg("dummy2.class")
        .spawn()
        .unwrap();

    let mut stream = loop {
        if let Ok(s) = TcpStream::connect(format!("127.0.0.1:{}", port)) {
            break s;
        }
        thread::sleep(Duration::from_millis(10));
    };

    stream.write_all(b"JDWP-Handshake").unwrap();
    let mut buf = [0u8; 14];
    stream.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b"JDWP-Handshake");

    let mut event_header = [0u8; 11];
    stream.read_exact(&mut event_header).unwrap();
    let length = u32::from_be_bytes([
        event_header[0],
        event_header[1],
        event_header[2],
        event_header[3],
    ]);
    let mut payload = vec![0; length as usize - 11];
    stream.read_exact(&mut payload).unwrap();

    let length = 11 + 4; // 15 bytes
    let mut pkt = Vec::new();
    pkt.extend_from_slice(&(length as u32).to_be_bytes());
    pkt.extend_from_slice(&1_i32.to_be_bytes());
    pkt.push(0); // flags
    pkt.push(16); // cmd_set: StackFrame
    pkt.push(1); // cmd: GetValues

    let count = 0x0FFFFFFF; // Huge count
    pkt.extend_from_slice(&(count as u32).to_be_bytes());

    stream.write_all(&pkt).unwrap();

    let mut reply_header = [0u8; 11];
    let _ = stream.read_exact(&mut reply_header);

    let length = 11;
    let mut pkt = Vec::new();
    pkt.extend_from_slice(&(length as u32).to_be_bytes());
    pkt.extend_from_slice(&2_i32.to_be_bytes());
    pkt.push(0);
    pkt.push(1); // VirtualMachine
    pkt.push(9); // Resume
    let _ = stream.write_all(&pkt);

    let status = child.wait().unwrap();
    std::fs::remove_file("dummy2.class").unwrap();
    assert!(
        status.success(),
        "Process crashed due to OOM! exit code: {:?}",
        status.code()
    );
}
