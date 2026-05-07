use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, mpsc};

const HANDSHAKE: &[u8] = b"JDWP-Handshake";
const REPLY_FLAG: u8 = 0x80;
const ERR_NONE: u16 = 0;
const ERR_NOT_IMPLEMENTED: u16 = 99;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JdwpConfig {
    pub transport: String,
    pub server: bool,
    pub suspend: bool,
    pub address: String,
}

impl Default for JdwpConfig {
    fn default() -> Self {
        Self {
            transport: "dt_socket".to_string(),
            server: true,
            suspend: false,
            address: "127.0.0.1:5005".to_string(),
        }
    }
}

pub fn parse_agentlib_jdwp(arg: &str) -> Option<JdwpConfig> {
    let payload = arg.strip_prefix("-agentlib:jdwp=")?;
    let mut config = JdwpConfig::default();
    for part in payload.split(',') {
        let mut kv = part.splitn(2, '=');
        let key = kv.next().unwrap_or("").trim();
        let value = kv.next().unwrap_or("").trim();
        match key {
            "transport" if !value.is_empty() => config.transport = value.to_string(),
            "server" => config.server = value.eq_ignore_ascii_case("y"),
            "suspend" => config.suspend = value.eq_ignore_ascii_case("y"),
            "address" if !value.is_empty() => {
                config.address = normalize_socket_addr(value);
            }
            _ => {}
        }
    }
    Some(config)
}

fn normalize_socket_addr(raw: &str) -> String {
    if raw.contains(':') {
        raw.to_string()
    } else {
        format!("127.0.0.1:{raw}")
    }
}

pub struct JdwpServer {
    attached_rx: mpsc::Receiver<()>,
}

impl JdwpServer {
    pub fn wait_for_attach(&self) {
        let _ = self.attached_rx.recv();
    }
}

pub fn start(config: &JdwpConfig) -> std::io::Result<JdwpServer> {
    let listener = TcpListener::bind(&config.address)?;
    let (attached_tx, attached_rx) = mpsc::channel();
    let cfg = config.clone();
    std::thread::spawn(move || {
        let next_request_id = Arc::new(AtomicI32::new(1));
        let vm_suspended = Arc::new(AtomicBool::new(cfg.suspend));

        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            if !do_handshake(&mut stream).unwrap_or(false) {
                continue;
            }
            let _ = attached_tx.send(());
            let _ = send_vm_start_event(&mut stream, vm_suspended.load(Ordering::SeqCst));

            let _ = handle_client(&mut stream, &next_request_id, &vm_suspended);
        }
    });
    Ok(JdwpServer { attached_rx })
}

fn do_handshake(stream: &mut TcpStream) -> std::io::Result<bool> {
    let mut buf = [0u8; 14];
    stream.read_exact(&mut buf)?;
    if buf != HANDSHAKE {
        return Ok(false);
    }
    stream.write_all(HANDSHAKE)?;
    Ok(true)
}

fn handle_client(
    stream: &mut TcpStream,
    next_request_id: &AtomicI32,
    vm_suspended: &AtomicBool,
) -> std::io::Result<()> {
    loop {
        let mut header = [0u8; 11];
        if stream.read_exact(&mut header).is_err() {
            return Ok(());
        }
        let length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        if length < 11 {
            return Ok(());
        }
        let id = i32::from_be_bytes([header[4], header[5], header[6], header[7]]);
        let flags = header[8];
        let mut payload = vec![0; length - 11];
        stream.read_exact(&mut payload)?;

        if flags & REPLY_FLAG != 0 {
            continue;
        }

        let cmd_set = header[9];
        let cmd = header[10];
        let (err, data, should_close) =
            dispatch_command(cmd_set, cmd, &payload, next_request_id, vm_suspended);
        send_reply(stream, id, err, &data)?;
        if should_close {
            return Ok(());
        }
    }
}

fn dispatch_command(
    cmd_set: u8,
    cmd: u8,
    payload: &[u8],
    next_request_id: &AtomicI32,
    vm_suspended: &AtomicBool,
) -> (u16, Vec<u8>, bool) {
    match (cmd_set, cmd) {
        (1, 1) => (ERR_NONE, vm_version(), false),
        (1, 4) => (ERR_NONE, one_thread_list(), false),
        (1, 7) => (ERR_NONE, id_sizes(), false),
        (1, 8) => {
            vm_suspended.store(true, Ordering::SeqCst);
            (ERR_NONE, vec![], false)
        }
        (1, 9) => {
            vm_suspended.store(false, Ordering::SeqCst);
            (ERR_NONE, vec![], false)
        }
        (1, 12 | 17) => (ERR_NONE, vec![0; 32], false),
        (1, 13) => (ERR_NONE, class_paths(), false),
        (1, 6) => (ERR_NONE, vec![], true),

        // ReferenceType
        (2, 1) => (ERR_NONE, signature_placeholder(), false),
        (2, 5) => (ERR_NONE, write_u32(0), false),

        // Method
        (6, 1) => (ERR_NONE, method_line_table(), false),

        // ThreadReference.Frames
        (11, 6) => (ERR_NONE, one_frame(), false),

        // ObjectReference.GetValues
        (9, 2) => {
            let count = payload
                .get(8..12)
                .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
            let mut out = Vec::new();
            out.extend_from_slice(&count.to_be_bytes());
            for _ in 0..count {
                out.push(b'L');
                out.extend_from_slice(&0_i64.to_be_bytes());
            }
            (ERR_NONE, out, false)
        }

        // StackFrame.GetValues
        (16, 1) => {
            let slots = payload
                .get(8..12)
                .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
            let mut out = Vec::new();
            out.extend_from_slice(&slots.to_be_bytes());
            for _ in 0..slots {
                out.push(b'I');
                out.extend_from_slice(&0_i32.to_be_bytes());
            }
            (ERR_NONE, out, false)
        }

        // EventRequest.Set (BREAKPOINT / STEP)
        (15, 1) => {
            let req_id = next_request_id.fetch_add(1, Ordering::SeqCst);
            (ERR_NONE, req_id.to_be_bytes().to_vec(), false)
        }

        _ => (ERR_NOT_IMPLEMENTED, vec![], false),
    }
}

fn send_reply(stream: &mut dyn Write, id: i32, err: u16, data: &[u8]) -> std::io::Result<()> {
    let len = 11_u32 + to_u32_len(data.len());
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(&id.to_be_bytes())?;
    stream.write_all(&[REPLY_FLAG])?;
    stream.write_all(&err.to_be_bytes())?;
    stream.write_all(data)
}

fn send_vm_start_event(stream: &mut dyn Write, suspended: bool) -> std::io::Result<()> {
    let mut data = Vec::new();
    data.push(if suspended { 2 } else { 0 });
    data.extend_from_slice(&1_u32.to_be_bytes());
    data.push(90); // VM_START
    data.extend_from_slice(&1_i32.to_be_bytes());
    data.extend_from_slice(&1_i64.to_be_bytes());

    let len = 11_u32 + to_u32_len(data.len());
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(&1_i32.to_be_bytes())?;
    stream.write_all(&[0])?;
    stream.write_all(&[64, 100])?;
    stream.write_all(&data)
}

fn vm_version() -> Vec<u8> {
    let mut out = Vec::new();
    write_string(&mut out, "Duke JDWP");
    out.extend_from_slice(&1_i32.to_be_bytes());
    out.extend_from_slice(&8_i32.to_be_bytes());
    write_string(&mut out, "1.8");
    write_string(&mut out, "DukeVM");
    out
}

fn one_thread_list() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&1_u32.to_be_bytes());
    out.extend_from_slice(&1_i64.to_be_bytes());
    out
}

fn id_sizes() -> Vec<u8> {
    let mut out = Vec::new();
    for _ in 0..5 {
        out.extend_from_slice(&8_i32.to_be_bytes());
    }
    out
}

fn class_paths() -> Vec<u8> {
    let mut out = Vec::new();
    write_string(&mut out, ".");
    out.extend_from_slice(&0_u32.to_be_bytes());
    out.extend_from_slice(&0_u32.to_be_bytes());
    out
}

fn signature_placeholder() -> Vec<u8> {
    let mut out = Vec::new();
    write_string(&mut out, "Ljava/lang/Object;");
    out
}

fn method_line_table() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0_i64.to_be_bytes());
    out.extend_from_slice(&0_i64.to_be_bytes());
    out.extend_from_slice(&0_u32.to_be_bytes());
    out
}

fn one_frame() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&1_u32.to_be_bytes());
    out.extend_from_slice(&1_i64.to_be_bytes()); // frame id
    out.push(1); // type tag class
    out.extend_from_slice(&1_i64.to_be_bytes()); // class id
    out.extend_from_slice(&1_i64.to_be_bytes()); // method id
    out.extend_from_slice(&0_u64.to_be_bytes()); // index
    out
}

fn to_u32_len(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&to_u32_len(value.len()).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn write_u32(value: u32) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_agentlib_config() {
        let cfg = parse_agentlib_jdwp(
            "-agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address=5005",
        )
        .expect("config should parse");
        assert_eq!(cfg.transport, "dt_socket");
        assert!(cfg.server);
        assert!(cfg.suspend);
        assert_eq!(cfg.address, "127.0.0.1:5005");
    }

    #[test]
    fn normalize_address() {
        assert_eq!(normalize_socket_addr("5005"), "127.0.0.1:5005");
        assert_eq!(normalize_socket_addr("0.0.0.0:5005"), "0.0.0.0:5005");
    }

    #[test]
    fn parse_agentlib_returns_none_if_no_prefix() {
        assert!(parse_agentlib_jdwp("invalid_arg").is_none());
    }

    #[test]
    fn test_dispatch_command_vm_version() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, data, close) = dispatch_command(1, 1, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_dispatch_command_unimplemented() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, _, close) = dispatch_command(99, 99, &[], &req_id, &suspended);
        assert_eq!(err, 99);
        assert!(!close);
    }

    #[test]
    fn test_dispatch_command_suspend() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, _, close) = dispatch_command(1, 8, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert!(suspended.load(Ordering::SeqCst));
    }

    #[test]
    fn test_dispatch_command_resume() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(true);
        let (err, _, close) = dispatch_command(1, 9, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert!(!suspended.load(Ordering::SeqCst));
    }

    #[test]
    fn test_dispatch_command_event_request_set() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, data, close) = dispatch_command(15, 1, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert_eq!(data, 1_i32.to_be_bytes().to_vec());
        assert_eq!(req_id.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_dispatch_command_id_sizes() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, data, close) = dispatch_command(1, 7, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert_eq!(data.len(), 20); // 5 sizes * 4 bytes
    }

    #[test]
    fn test_dispatch_command_class_paths() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, data, close) = dispatch_command(1, 13, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_dispatch_command_reference_type_signature() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, data, close) = dispatch_command(2, 1, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_dispatch_command_thread_reference_frames() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, data, close) = dispatch_command(11, 6, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_dispatch_command_one_thread_list() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, data, close) = dispatch_command(1, 4, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert_eq!(data.len(), 12);
    }

    #[test]
    fn test_dispatch_command_method_line_table() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, data, close) = dispatch_command(6, 1, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert_eq!(data.len(), 20);
    }

    #[test]
    fn test_dispatch_command_capabilities() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, data, close) = dispatch_command(1, 12, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert_eq!(data.len(), 32);

        let (err, data, close) = dispatch_command(1, 17, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert_eq!(data.len(), 32);
    }

    #[test]
    fn test_dispatch_command_close() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, data, close) = dispatch_command(1, 6, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(close);
        assert!(data.is_empty());
    }

    #[test]
    fn test_dispatch_command_object_reference_get_values() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let mut payload = vec![0; 12];
        payload[8..12].copy_from_slice(&2_u32.to_be_bytes());
        let (err, data, close) = dispatch_command(9, 2, &payload, &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert_eq!(data.len(), 4 + 2 * (1 + 8)); // count + 2 * (tag + id)
    }

    #[test]
    fn test_dispatch_command_stack_frame_get_values() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let mut payload = vec![0; 12];
        payload[8..12].copy_from_slice(&3_u32.to_be_bytes());
        let (err, data, close) = dispatch_command(16, 1, &payload, &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert_eq!(data.len(), 4 + 3 * (1 + 4)); // slots + 3 * (tag + i32)
    }

    #[test]
    fn test_dispatch_command_reference_type_modifiers() {
        let req_id = AtomicI32::new(1);
        let suspended = AtomicBool::new(false);
        let (err, data, close) = dispatch_command(2, 5, &[], &req_id, &suspended);
        assert_eq!(err, 0);
        assert!(!close);
        assert_eq!(data, vec![0, 0, 0, 0]);
    }

    #[test]
    fn test_normalize_address_edge_cases() {
        assert_eq!(normalize_socket_addr(""), "127.0.0.1:");
    }

    #[test]
    fn test_send_reply() {
        let mut buf = Vec::new();
        let res = send_reply(&mut buf, 42, 0, b"data");
        assert!(res.is_ok());
        assert_eq!(buf.len(), 11 + 4);
    }

    #[test]
    fn test_send_vm_start_event() {
        let mut buf = Vec::new();
        let res = send_vm_start_event(&mut buf, true);
        assert!(res.is_ok());
        assert_eq!(buf.len(), 11 + 18);
    }

    #[test]
    fn test_jdwp_server_wait() {
        let (tx, rx) = std::sync::mpsc::channel();
        let server = JdwpServer { attached_rx: rx };
        tx.send(()).unwrap();
        server.wait_for_attach();
    }
}
