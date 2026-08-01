/// Native: `ServerSocket.<init>(int port)` — binds to 0.0.0.0:{port}.
pub(crate) fn native_server_socket_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let port = extract_int_arg(args, 1)?;
    let addr = format!("0.0.0.0:{port}");
    let server_id = heap.bind_server_socket(&addr)?;
    let actual_port = heap.server_socket_local_port(server_id)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 2 {
        return Err(Error::InvalidRef { address: this_ref });
    }
    obj.fields[0] = Slot::Int(server_id);
    obj.fields[1] = Slot::Int(actual_port);
    Ok(None)
}
/// Native: `ServerSocket.accept()` — blocks until a client connects, returns a Socket.
pub(crate) fn native_server_socket_accept(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let server_fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let (reader_id, writer_id) = heap.accept_connection(server_fd)?;
    // Allocate a new Socket object with fdRead=reader_id, fdWrite=writer_id
    let socket_ref = heap.allocate("java/net/Socket".to_string(), 2);
    heap.get_mut(socket_ref)?.fields[0] = Slot::Int(reader_id);
    heap.get_mut(socket_ref)?.fields[1] = Slot::Int(writer_id);
    Ok(Some(Slot::Reference(Some(socket_ref))))
}
/// Native: `ServerSocket.getLocalPort()` — returns the bound port.
pub(crate) fn native_server_socket_get_local_port(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(port)) => Ok(Some(Slot::Int(*port))),
        _ => Err(Error::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}
/// Native: `ServerSocket.close()` — closes the OS listener and zeros the fd field.
pub(crate) fn native_server_socket_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        Some(Slot::Int(_)) => return Ok(None), // already closed — idempotent
        _ => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    heap.close_host_file(fd);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    obj.fields[1] = Slot::Int(0); // also zero cached port so getLocalPort() returns 0 after close
    Ok(None)
}
/// Native: `Socket.<init>(String host, int port)` — connects to host:port.
pub(crate) fn native_socket_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let host_ref = extract_ref_arg(args, 1)?;
    let host = string_value_from_ref(heap, host_ref)?;
    let port = extract_int_arg(args, 2)?;
    let addr = format!("{host}:{port}");
    let (reader_id, writer_id) = heap.connect_socket(&addr)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 2 {
        return Err(Error::InvalidRef { address: this_ref });
    }
    obj.fields[0] = Slot::Int(reader_id);
    obj.fields[1] = Slot::Int(writer_id);
    Ok(None)
}
/// Native: `Socket.getInputStream()` — allocates a `SocketInputStream` wrapping `fdRead`.
pub(crate) fn native_socket_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd_read = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let stream_ref = heap.allocate("duke/net/SocketInputStream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(fd_read);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
/// Native: `Socket.getOutputStream()` — allocates a `SocketOutputStream` wrapping `fdWrite`.
pub(crate) fn native_socket_get_output_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd_write = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let stream_ref = heap.allocate("duke/net/SocketOutputStream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(fd_write);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
/// Native: `Socket.close()` — closes both OS handles (fdRead and fdWrite).
pub(crate) fn native_socket_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd_read = extract_io_fd(heap, this_ref)?;
    let fd_write = extract_io_fd_at(heap, this_ref, 1)?;
    heap.close_host_file(fd_read);
    heap.close_host_file(fd_write);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    obj.fields[1] = Slot::Int(0);
    Ok(None)
}
/// Native: `URL.openConnection()` — mints a spec-backed `java/net/URLConnection`
/// carrying the same resource spec String as this URL, mirroring the URL's
/// 1-slot spec-backed layout. The connection defers resource I/O until
/// `getInputStream()` is called.
pub(crate) fn native_url_open_connection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let spec = string_backed_object_value(heap, this_ref)?;
    let conn_ref = allocate_string_backed_object(heap, "java/net/URLConnection", spec)?;
    Ok(Some(Slot::Reference(Some(conn_ref))))
}
/// Native: `URL.getUserInfo()` — returns null. Duke's synthetic URLs name
/// classpath/jar/file resources and never carry a `user:password@` userinfo
/// component, so Spring's `AbstractFileResolvingResource.customizeConnection`
/// correctly skips its Basic-auth header path.
#[allow(clippy::unnecessary_wraps)] // signature must match `NativeHandler`
pub(crate) fn native_url_get_user_info(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Reference(None)))
}
/// Native: `URLConnection.getInputStream()` — reads the resource named by the
/// connection's spec String and returns a `duke/io/ResourceInputStream`,
/// mirroring `native_url_open_stream`.
pub(crate) fn native_url_connection_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let spec = string_backed_object_value(heap, this_ref)?;
    let bytes = read_resource_bytes_from_url_spec(&spec)?;
    let stream_ref = allocate_resource_input_stream(heap, bytes)?;
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
/// Native: `URLConnection.setUseCaches(boolean)` — no-op; Duke's resource
/// streams are read fresh on each `getInputStream()` call.
#[allow(clippy::unnecessary_wraps)] // signature must match `NativeHandler`
pub(crate) fn native_url_connection_set_use_caches(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}
/// `application/x-www-form-urlencoded` decoding, shared by both `URLDecoder.decode`
/// overloads. `+` becomes a space; each `%XX` becomes the raw byte `0xXX`; every
/// other char contributes its own UTF-8 bytes. The accumulated byte sequence is
/// then interpreted as UTF-8 (the only charset Spring passes here — the charset
/// argument is UTF-8, and Duke's Strings are UTF-8/Latin-1). A malformed `%`
/// escape raises `IllegalArgumentException`, matching the JDK.
fn www_form_url_decode(encoded: &str) -> Result<String> {
    let bytes = encoded.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                decoded.push(b' ');
                i += 1;
            }
            b'%' => {
                if i + 2 >= bytes.len() {
                    return Err(Error::JavaException {
                        class_name: "java/lang/IllegalArgumentException".into(),
                    });
                }
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                match (hi, lo) {
                    (Some(hi), Some(lo)) => {
                        decoded.push(u8::try_from(hi * 16 + lo).unwrap_or(0));
                        i += 3;
                    }
                    _ => {
                        return Err(Error::JavaException {
                            class_name: "java/lang/IllegalArgumentException".into(),
                        });
                    }
                }
            }
            b => {
                decoded.push(b);
                i += 1;
            }
        }
    }
    Ok(String::from_utf8_lossy(&decoded).into_owned())
}
/// Native: `URLDecoder.decode(String, Charset)` — decodes an
/// `application/x-www-form-urlencoded` string. Spring's `UrlResource.getFilename`
/// calls this with `StandardCharsets.UTF_8` to un-escape a URL path component.
/// The charset argument is accepted and treated as UTF-8 (see
/// `www_form_url_decode`).
pub(crate) fn native_url_decoder_decode_charset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let encoded_ref = extract_ref_arg(args, 0)?;
    let encoded = string_value_from_ref(heap, encoded_ref)?;
    let decoded = www_form_url_decode(&encoded)?;
    Ok(Some(Slot::Reference(Some(heap.allocate_string(decoded)))))
}
#[cfg(test)]
mod java_net_tests {
    use super::*;
    use std::io::sink;
    use std::sync::mpsc;
    use std::thread;

    fn create_test_heap() -> duke_gc::Heap {
        duke_gc::Heap::new()
    }

    fn create_test_control() -> NativeControl {
        NativeControl::default()
    }

    #[test]
    fn test_server_socket_lifecycle() {
        let mut heap = create_test_heap();
        let mut control = create_test_control();
        let mut out = sink();

        // Allocate a dummy ServerSocket object with 2 fields
        let server_ref = heap.allocate("java/net/ServerSocket".to_string(), 2);
        let args = vec![Slot::Reference(Some(server_ref)), Slot::Int(0)]; // Port 0 for ephemeral

        // 1. Init
        let res = native_server_socket_init(&args, &mut heap, &mut out, &mut control).unwrap();
        assert!(res.is_none());

        // Verify fields are populated
        let obj = heap.get(server_ref).unwrap();
        let Slot::Int(server_fd) = obj.fields[0] else { panic!("Expected Int") };
        assert!(server_fd > 0);

        // 2. Get local port
        let args_get_port = vec![Slot::Reference(Some(server_ref))];
        let port_slot = native_server_socket_get_local_port(&args_get_port, &mut heap, &mut out, &mut control).unwrap().unwrap();
        let Slot::Int(port) = port_slot else { panic!("Expected Int") };
        assert!(port > 0);

        // 3. Connect a client in a background thread to unblock accept
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _client = std::net::TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
            tx.send(()).unwrap();
        });

        // 4. Accept
        let accept_args = vec![Slot::Reference(Some(server_ref))];
        let socket_slot = native_server_socket_accept(&accept_args, &mut heap, &mut out, &mut control).unwrap().unwrap();
        let Slot::Reference(Some(socket_ref)) = socket_slot else { panic!("Expected Reference") };

        rx.recv().unwrap(); // Ensure client connected

        // Verify socket fields
        let socket_obj = heap.get(socket_ref).unwrap();
        assert_eq!(socket_obj.fields.len(), 2);
        match socket_obj.fields[0] {
            Slot::Int(fd) => assert!(fd > 0),
            _ => panic!("Expected Int for read fd"),
        }
        match socket_obj.fields[1] {
            Slot::Int(fd) => assert!(fd > 0),
            _ => panic!("Expected Int for write fd"),
        }

        // 5. Get InputStream
        let socket_args = vec![Slot::Reference(Some(socket_ref))];
        let is_slot = native_socket_get_input_stream(&socket_args, &mut heap, &mut out, &mut control).unwrap().unwrap();
        let Slot::Reference(Some(is_ref)) = is_slot else { panic!("Expected Reference") };
        let is_obj = heap.get(is_ref).unwrap();
        assert_eq!(is_obj.fields.len(), 1);

        // 6. Get OutputStream
        let os_slot = native_socket_get_output_stream(&socket_args, &mut heap, &mut out, &mut control).unwrap().unwrap();
        let Slot::Reference(Some(os_ref)) = os_slot else { panic!("Expected Reference") };
        let os_obj = heap.get(os_ref).unwrap();
        assert_eq!(os_obj.fields.len(), 1);

        // 7. Close Socket
        native_socket_close(&socket_args, &mut heap, &mut out, &mut control).unwrap();
        let socket_obj_closed = heap.get(socket_ref).unwrap();
        assert_eq!(socket_obj_closed.fields[0], Slot::Int(0));
        assert_eq!(socket_obj_closed.fields[1], Slot::Int(0));

        // 8. Close ServerSocket
        native_server_socket_close(&accept_args, &mut heap, &mut out, &mut control).unwrap();
        let server_obj_closed = heap.get(server_ref).unwrap();
        assert_eq!(server_obj_closed.fields[0], Slot::Int(0));
        assert_eq!(server_obj_closed.fields[1], Slot::Int(0));

        // Idempotent close
        native_server_socket_close(&accept_args, &mut heap, &mut out, &mut control).unwrap();
    }

    #[test]
    fn test_socket_lifecycle() {
        let mut heap = create_test_heap();
        let mut control = create_test_control();
        let mut out = sink();

        // Start a real dummy TCP server to connect to
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            tx.send(()).unwrap();
            drop(stream);
        });

        // 1. Allocate Socket
        let socket_ref = heap.allocate("java/net/Socket".to_string(), 2);
        let host_ref = heap.allocate_string("127.0.0.1".to_string());
        let args = vec![
            Slot::Reference(Some(socket_ref)),
            Slot::Reference(Some(host_ref)),
            Slot::Int(i32::from(port)),
        ];

        // 2. Init
        let res = native_socket_init(&args, &mut heap, &mut out, &mut control).unwrap();
        assert!(res.is_none());
        rx.recv().unwrap(); // Wait for server to accept

        // 3. Verify fields
        let socket_obj = heap.get(socket_ref).unwrap();
        let Slot::Int(fd_read) = socket_obj.fields[0] else { panic!("Expected Int") };
        assert!(fd_read > 0);

        // 4. Close
        let close_args = vec![Slot::Reference(Some(socket_ref))];
        native_socket_close(&close_args, &mut heap, &mut out, &mut control).unwrap();

        let socket_obj_closed = heap.get(socket_ref).unwrap();
        assert_eq!(socket_obj_closed.fields[0], Slot::Int(0));
        assert_eq!(socket_obj_closed.fields[1], Slot::Int(0));
    }

    #[test]
    fn test_server_socket_accept_closed() {
        let mut heap = create_test_heap();
        let mut control = create_test_control();
        let mut out = sink();

        let server_ref = heap.allocate("java/net/ServerSocket".to_string(), 2);
        heap.get_mut(server_ref).unwrap().fields[0] = Slot::Int(0); // closed

        let args = vec![Slot::Reference(Some(server_ref))];
        let err = native_server_socket_accept(&args, &mut heap, &mut out, &mut control).unwrap_err();
        assert!(matches!(err, Error::JavaException { class_name } if class_name == "java/io/IOException"));
    }
}
