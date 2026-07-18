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