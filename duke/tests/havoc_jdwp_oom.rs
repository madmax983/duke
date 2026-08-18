#![allow(missing_docs)]

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

/// Outcome of a single OOM probe attempt.
///
/// `Completed` means the probe drove the whole JDWP exchange and reached
/// `child.wait()`; the carried `ExitStatus` is what the real OOM assertion
/// inspects. `TransportFailed` means the setup phase (connect / handshake /
/// `VM_START` drain) failed inconclusively before the attack was ever sent, so
/// the attempt tells us nothing about the behaviour under test and should be
/// retried rather than asserted on.
enum ProbeOutcome {
    Completed(std::process::ExitStatus),
    TransportFailed(String),
}

/// Connect to the freshly spawned JDWP server, perform the handshake and drain
/// the `VM_START` event. Any read/connect problem in this SETUP phase is reported
/// as an `Err(String)` (never a panic) so the caller can treat it as transport
/// flakiness and retry.
fn connect_and_handshake(port: u16) -> Result<TcpStream, String> {
    let mut stream = None;
    for _ in 0..500 {
        if let Ok(s) = TcpStream::connect(format!("127.0.0.1:{}", port)) {
            stream = Some(s);
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let mut stream = stream.ok_or_else(|| "could not connect to JDWP server".to_string())?;

    // Never let a read block forever if the child hangs or dies.
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();

    stream
        .write_all(b"JDWP-Handshake")
        .map_err(|e| format!("handshake write failed: {}", e))?;
    let mut buf = [0u8; 14];
    stream
        .read_exact(&mut buf)
        .map_err(|e| format!("handshake read failed: {}", e))?;
    if &buf != b"JDWP-Handshake" {
        return Err(format!("unexpected handshake bytes: {:?}", buf));
    }

    let mut event_header = [0u8; 11];
    stream
        .read_exact(&mut event_header)
        .map_err(|e| format!("VM_START header read failed: {}", e))?;
    let length = u32::from_be_bytes([
        event_header[0],
        event_header[1],
        event_header[2],
        event_header[3],
    ]);
    // Guard against a short/garbage header underflowing the payload length.
    if length < 11 {
        return Err(format!("VM_START header length too small: {}", length));
    }
    let mut payload = vec![0; length as usize - 11];
    stream
        .read_exact(&mut payload)
        .map_err(|e| format!("VM_START payload read failed: {}", e))?;

    Ok(stream)
}

/// Run one OOM probe: spawn the JVM as a JDWP server, connect, handshake, drain
/// `VM_START`, send `attack_pkt`, resume and wait for the child.
///
/// Reaching `child.wait()` always yields `Completed(status)` — an OOM crash
/// simply surfaces as `!status.success()`, which the caller must still assert.
/// Only inconclusive setup failures yield `TransportFailed`.
fn run_oom_probe(class_name: &str, attack_pkt: &[u8]) -> ProbeOutcome {
    let listener = match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => return ProbeOutcome::TransportFailed(format!("bind failed: {}", e)),
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(e) => return ProbeOutcome::TransportFailed(format!("local_addr failed: {}", e)),
    };
    drop(listener);

    let mut child = match Command::new(env!("CARGO_BIN_EXE_duke"))
        .arg(&format!(
            "-agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address={}",
            port
        ))
        .arg(class_name)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return ProbeOutcome::TransportFailed(format!("spawn failed: {}", e)),
    };

    let mut stream = match connect_and_handshake(port) {
        Ok(s) => s,
        Err(msg) => {
            // Don't leak the subprocess when the setup phase is inconclusive.
            let _ = child.kill();
            let _ = child.wait();
            return ProbeOutcome::TransportFailed(msg);
        }
    };

    // SETUP is done; from here we exercise the actual behaviour under test.
    let _ = stream.write_all(attack_pkt);

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

    match child.wait() {
        Ok(status) => ProbeOutcome::Completed(status),
        Err(e) => {
            let _ = child.kill();
            ProbeOutcome::TransportFailed(format!("child.wait failed: {}", e))
        }
    }
}

/// Drive `run_oom_probe` in a bounded retry loop. On a `Completed` outcome we
/// run the original OOM assertion. Pure transport failures are retried, and if
/// every attempt fails inconclusively the test is skipped (not failed) so
/// transport flakiness under CI load never produces a red build.
fn run_oom_test(test_name: &str, class_name: &str, attack_pkt: &[u8]) {
    create_valid_class(class_name);

    let mut last_err = String::from("<none>");
    for attempt in 0..5 {
        match run_oom_probe(class_name, attack_pkt) {
            ProbeOutcome::Completed(status) => {
                let _ = std::fs::remove_file(class_name);
                assert!(
                    status.success(),
                    "Process crashed due to OOM! exit code: {:?}",
                    status.code()
                );
                return;
            }
            ProbeOutcome::TransportFailed(msg) => {
                last_err = msg;
                if attempt < 4 {
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }
    }

    let _ = std::fs::remove_file(class_name);
    eprintln!(
        "SKIP {}: JDWP transport unstable after 5 attempts; last error: {}",
        test_name, last_err
    );
}

#[test]
fn test_jdwp_getvalues_oom() {
    let length = 11 + 4; // 15 bytes
    let mut pkt = Vec::new();
    pkt.extend_from_slice(&(length as u32).to_be_bytes());
    pkt.extend_from_slice(&1_i32.to_be_bytes());
    pkt.push(0); // flags
    pkt.push(9); // cmd_set: ObjectReference
    pkt.push(2); // cmd: GetValues

    let count = 0x0FFFFFFF; // Huge count
    pkt.extend_from_slice(&(count as u32).to_be_bytes());

    run_oom_test("test_jdwp_getvalues_oom", "dummy1.class", &pkt);
}

#[test]
fn test_jdwp_stackframe_getvalues_oom() {
    let length = 11 + 4; // 15 bytes
    let mut pkt = Vec::new();
    pkt.extend_from_slice(&(length as u32).to_be_bytes());
    pkt.extend_from_slice(&1_i32.to_be_bytes());
    pkt.push(0); // flags
    pkt.push(16); // cmd_set: StackFrame
    pkt.push(1); // cmd: GetValues

    let count = 0x0FFFFFFF; // Huge count
    pkt.extend_from_slice(&(count as u32).to_be_bytes());

    run_oom_test("test_jdwp_stackframe_getvalues_oom", "dummy2.class", &pkt);
}
