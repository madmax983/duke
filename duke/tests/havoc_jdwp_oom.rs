use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::time::Duration;

#[test]
fn havoc_jdwp_oom() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let temp_dir = std::env::temp_dir();
    let java_file = temp_dir.join("HelloWorld.java");
    std::fs::write(&java_file, b"public class HelloWorld { public static void main(String[] args) { while(true) { try { Thread.sleep(1000); } catch (Exception e) {} } } }").unwrap();
    Command::new("javac").arg(&java_file).status().unwrap();

    let exe = env!("CARGO_BIN_EXE_duke");
    let mut child = Command::new(exe)
        .arg("run")
        .arg(format!(
            "-agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address={port}"
        ))
        .arg("HelloWorld")
        .current_dir(&temp_dir)
        .spawn()
        .expect("Failed to start Duke JVM");

    std::thread::sleep(Duration::from_millis(1500));

    let mut stream =
        TcpStream::connect(format!("127.0.0.1:{port}")).expect("Failed to connect to JDWP server");

    stream.write_all(b"JDWP-Handshake").unwrap();
    let mut buf = [0u8; 14];
    stream.read_exact(&mut buf).unwrap();

    // Malicious ObjectReference.GetValues
    let length: u32 = 11 + 12; // payload len is 12 so that payload[8..12] works
    stream.write_all(&length.to_be_bytes()).unwrap();
    stream.write_all(&1u32.to_be_bytes()).unwrap();
    stream.write_all(&[0u8]).unwrap();
    stream.write_all(&[9u8, 2u8]).unwrap();

    // Payload bytes 0..8
    stream.write_all(&[0u8; 8]).unwrap();

    // Payload bytes 8..12 (the count)
    // Send a huge allocation size
    let count: u32 = 0x3FFF_FFFF;
    stream.write_all(&count.to_be_bytes()).unwrap();

    // Try to read header response
    let mut header = [0u8; 11];
    let _res = stream.read_exact(&mut header);
    // Read might fail because the server may disconnect immediately due to unparsable/unhandled protocol things, but it definitely should NOT have been killed by OOM.

    // Need to read the rest of the payload from the response, otherwise next write/read will fail
    let length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    let mut resp_payload = vec![0u8; (length - 11) as usize];
    stream.read_exact(&mut resp_payload).unwrap();

    // Send StackFrame.GetValues (16, 1)
    let length: u32 = 11 + 12; // payload len is 12 so that payload[8..12] works
    let req2 = stream.write_all(&length.to_be_bytes());
    if req2.is_ok() {
        let _ = stream.write_all(&2u32.to_be_bytes());
        let _ = stream.write_all(&[0u8]);
        let _ = stream.write_all(&[16u8, 1u8]);

        // Payload bytes 0..8
        let _ = stream.write_all(&[0u8; 8]);

        // Payload bytes 8..12 (the count)
        let count: u32 = 0x3FFF_FFFF;
        let _ = stream.write_all(&count.to_be_bytes());

        // Try to read header response
        let mut header = [0u8; 11];
        let _res2 = stream.read_exact(&mut header);
        // Not asserting on res2 because stream might have closed.
    }

    child.kill().unwrap();
    let _status = child.wait().unwrap();

    let _ = std::fs::remove_file(java_file);
    let _ = std::fs::remove_file(temp_dir.join("HelloWorld.class"));
}
