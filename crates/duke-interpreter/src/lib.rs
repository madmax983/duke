//! Switch-dispatch JVM bytecode interpreter for Duke Phase 4.
//!
//! Executes decoded instruction streams for methods containing integer, long,
//! float, and double arithmetic, control flow, and local variables.  Heap
//! allocation, field access, and method invocation are not yet implemented.

/// Core execution context types for methods and classes.
pub mod context;
/// Repositories for loaded classes and registered native methods.
pub mod registry;
pub mod stdlib;
mod threading;

pub use context::*;
pub use registry::*;
pub use stdlib::bootstrap_stdlib;

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;

use duke_bytecode::Instruction;
use duke_bytecode::instruction::ArrayType;
use duke_classfile::types::CpEntry;
use duke_loader::ClassLoader;
use duke_runtime::{Frame, Slot, VmError, VmResult};

/// Registry of loaded classes — maps class name to its `ClassContext`.
///
/// Used by `execute_class` for cross-class method dispatch.
///
/// # Examples
///
/// A native handler that can call back into the interpreter to invoke Java methods.
///
/// The `invoke` closure takes `heap` and `output` as *parameters* (not captured),
/// using the "loan" pattern: the handler passes its borrows through each call and
/// gets them back when the call returns. Sequential reborrows — no unsafe required.
/// Bootstrap minimal JDK standard library classes for native method support.
///
/// Creates synthetic `java/lang/System` and `java/io/PrintStream` classes and
/// registers native `println` handlers for `(Ljava/lang/String;)V`, `(I)V`,
#[inline]
fn extract_slot_arg(args: &[Slot], idx: usize) -> Slot {
    args.get(idx).copied().unwrap_or(Slot::Reference(None))
}

#[inline]
fn extract_ref_arg(args: &[Slot], idx: usize) -> VmResult<u64> {
    match args.get(idx) {
        Some(Slot::Reference(Some(r))) => Ok(*r),
        _ => Err(VmError::NullPointerException),
    }
}

#[inline]
fn extract_io_fd(heap: &duke_gc::Heap, obj_ref: u64) -> VmResult<i32> {
    match heap.get(obj_ref)?.fields.first() {
        Some(Slot::Int(id)) => Ok(*id),
        _ => Err(VmError::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

#[inline]
fn extract_io_fd_at(heap: &duke_gc::Heap, obj_ref: u64, idx: usize) -> VmResult<i32> {
    match heap.get(obj_ref)?.fields.get(idx) {
        Some(Slot::Int(id)) => Ok(*id),
        _ => Err(VmError::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

#[inline]
fn extract_int_arg(args: &[Slot], idx: usize) -> VmResult<i32> {
    match args.get(idx) {
        Some(Slot::Int(v)) => Ok(*v),
        _ => Err(VmError::TypeMismatch {
            expected: "Int",
            got: "other",
        }),
    }
}

#[inline]
fn extract_long_arg(args: &[Slot], idx: usize) -> VmResult<i64> {
    match args.get(idx) {
        Some(Slot::Long(v)) => Ok(*v),
        _ => Err(VmError::TypeMismatch {
            expected: "Long",
            got: "other",
        }),
    }
}

#[inline]
fn extract_float_arg(args: &[Slot], idx: usize) -> VmResult<f32> {
    match args.get(idx) {
        Some(Slot::Float(v)) => Ok(*v),
        _ => Err(VmError::TypeMismatch {
            expected: "Float",
            got: "other",
        }),
    }
}

#[inline]
fn extract_double_arg(args: &[Slot], idx: usize) -> VmResult<f64> {
    match args.get(idx) {
        Some(Slot::Double(v)) => Ok(*v),
        _ => Err(VmError::TypeMismatch {
            expected: "Double",
            got: "other",
        }),
    }
}

macro_rules! extract_print_arg {
    ($args:expr, $pat:pat => $expr:expr, $expected:literal) => {
        match $args.get(1) {
            Some($pat) => $expr,
            _ => {
                return Err(VmError::TypeMismatch {
                    expected: $expected,
                    got: "other",
                });
            }
        }
    };
}

pub(crate) fn native_println_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let string_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => {
            writeln!(out, "null").ok();
            return Ok(None);
        }
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let obj = heap.get(string_ref)?;
    let text = obj.string_value.as_deref().unwrap_or("null");
    writeln!(out, "{text}").ok();
    Ok(None)
}

pub(crate) fn native_println_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Int(v) => *v, "Int");
    writeln!(out, "{val}").ok();
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_println_void(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    writeln!(out).ok();
    Ok(None)
}

fn path_from_string_slot(
    args: &[Slot],
    idx: usize,
    heap: &duke_gc::Heap,
) -> VmResult<std::path::PathBuf> {
    let path_ref = extract_ref_arg(args, idx)?;
    let path = heap
        .get(path_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)?;
    Ok(std::path::PathBuf::from(path))
}

fn string_value_from_ref(heap: &duke_gc::Heap, string_ref: u64) -> VmResult<String> {
    heap.get(string_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)
}

fn file_path_from_ref(file_ref: u64, heap: &duke_gc::Heap) -> VmResult<std::path::PathBuf> {
    let path_ref = match heap.get(file_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    Ok(std::path::PathBuf::from(string_value_from_ref(
        heap, path_ref,
    )?))
}

fn file_path_from_this(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<std::path::PathBuf> {
    let this_ref = extract_ref_arg(args, 0)?;
    file_path_from_ref(this_ref, heap)
}

fn archive_path_from_slot(
    heap: &duke_gc::Heap,
    archive_ref: u64,
    slot_idx: usize,
) -> VmResult<Option<String>> {
    let Some(file_ref) = archive_ref_from_slot(heap, archive_ref, slot_idx)? else {
        return Ok(None);
    };
    Ok(Some(
        file_path_from_ref(file_ref, heap)?
            .to_string_lossy()
            .to_string(),
    ))
}

fn boot_archive_path_from_ref(
    registry: &ClassRegistry,
    heap: &duke_gc::Heap,
    archive_ref: u64,
) -> VmResult<Option<String>> {
    let archive_class = heap.get(archive_ref)?.class_name.clone();
    match archive_class.as_str() {
        "org/springframework/boot/loader/launch/JarFileArchive" => {
            let file_slot = field_slot_idx(registry, &archive_class, "file")?;
            archive_path_from_slot(heap, archive_ref, file_slot)
        }
        "org/springframework/boot/loader/launch/ExplodedArchive" => {
            let root_slot = field_slot_idx(registry, &archive_class, "rootDirectory")?;
            archive_path_from_slot(heap, archive_ref, root_slot)
        }
        _ => Ok(None),
    }
}

fn launched_class_loader_archive_path(
    registry: &ClassRegistry,
    heap: &duke_gc::Heap,
    loader_ref: u64,
) -> VmResult<Option<String>> {
    if heap.get(loader_ref)?.class_name
        != "org/springframework/boot/loader/launch/LaunchedClassLoader"
    {
        return Ok(None);
    }
    let root_archive_slot = field_slot_idx(
        registry,
        "org/springframework/boot/loader/launch/LaunchedClassLoader",
        "rootArchive",
    )?;
    let Some(archive_ref) = archive_ref_from_slot(heap, loader_ref, root_archive_slot)? else {
        return Ok(None);
    };
    boot_archive_path_from_ref(registry, heap, archive_ref)
}

fn archive_ref_from_slot(
    heap: &duke_gc::Heap,
    obj_ref: u64,
    slot_idx: usize,
) -> VmResult<Option<u64>> {
    match heap.get(obj_ref)?.fields.get(slot_idx).copied() {
        Some(Slot::Reference(Some(r))) => Ok(Some(r)),
        Some(Slot::Reference(None)) | None => Ok(None),
        Some(_) => Err(VmError::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

pub(crate) fn native_file_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path_slot = extract_slot_arg(args, 1);
    let file_obj = heap.get_mut(this_ref)?;
    let Some(path_field) = file_obj.fields.first_mut() else {
        return Err(VmError::InvalidRef { address: this_ref });
    };
    *path_field = path_slot;
    Ok(None)
}

pub(crate) fn native_file_exists(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.exists()))))
}

pub(crate) fn native_file_is_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.is_file()))))
}

pub(crate) fn native_file_is_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.is_dir()))))
}

fn file_stream_id_from_this(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<i32> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(VmError::JavaException {
            class_name: "java/io/IOException".to_string(),
        }),
    }
}

pub(crate) fn native_file_input_stream_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path = path_from_string_slot(args, 1, heap)?;
    let file_id = heap.open_host_input_file(&path)?;
    let stream_obj = heap.get_mut(this_ref)?;
    let Some(fd_field) = stream_obj.fields.first_mut() else {
        return Err(VmError::InvalidRef { address: this_ref });
    };
    *fd_field = Slot::Int(file_id);
    Ok(None)
}

pub(crate) fn native_file_input_stream_read(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let file_id = file_stream_id_from_this(args, heap)?;
    Ok(Some(Slot::Int(heap.read_host_file_byte(file_id)?)))
}

pub(crate) fn native_file_input_stream_read_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let file_id = file_stream_id_from_this(args, heap)?;
    let array_ref = extract_ref_arg(args, 1)?;
    let len = heap.get(array_ref)?.fields.len();
    if len == 0 {
        return Ok(Some(Slot::Int(0)));
    }

    let mut count = 0_usize;
    for idx in 0..len {
        let next = heap.read_host_file_byte(file_id)?;
        if next < 0 {
            break;
        }
        heap.get_mut(array_ref)?.fields[idx] = Slot::Int(next);
        count += 1;
    }

    if count == 0 {
        Ok(Some(Slot::Int(-1)))
    } else {
        Ok(Some(Slot::Int(i32::try_from(count).unwrap_or(i32::MAX))))
    }
}

pub(crate) fn native_file_input_stream_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let file_id = extract_io_fd(heap, this_ref)?;
    heap.close_host_file(file_id);
    let stream_obj = heap.get_mut(this_ref)?;
    let Some(fd_field) = stream_obj.fields.first_mut() else {
        return Err(VmError::InvalidRef { address: this_ref });
    };
    *fd_field = Slot::Int(0);
    Ok(None)
}

pub(crate) fn native_file_output_stream_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path = path_from_string_slot(args, 1, heap)?;
    let file_id = heap.open_host_output_file(&path)?;
    let stream_obj = heap.get_mut(this_ref)?;
    let Some(fd_field) = stream_obj.fields.first_mut() else {
        return Err(VmError::InvalidRef { address: this_ref });
    };
    *fd_field = Slot::Int(file_id);
    Ok(None)
}

pub(crate) fn native_file_output_stream_write(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let file_id = file_stream_id_from_this(args, heap)?;
    let value = extract_int_arg(args, 1)?;
    heap.write_host_file_byte(file_id, value)?;
    Ok(None)
}

pub(crate) fn native_file_output_stream_write_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let file_id = file_stream_id_from_this(args, heap)?;
    let array_ref = extract_ref_arg(args, 1)?;
    let bytes = heap.get(array_ref)?.fields.clone();
    for byte in bytes {
        let Slot::Int(value) = byte else {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        };
        heap.write_host_file_byte(file_id, value)?;
    }
    Ok(None)
}

pub(crate) fn native_file_output_stream_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let file_id = extract_io_fd(heap, this_ref)?;
    heap.close_host_file(file_id);
    let stream_obj = heap.get_mut(this_ref)?;
    let Some(fd_field) = stream_obj.fields.first_mut() else {
        return Err(VmError::InvalidRef { address: this_ref });
    };
    *fd_field = Slot::Int(0);
    Ok(None)
}

// ── Networking natives ────────────────────────────────────────────────────

/// Native: `ServerSocket.<init>(int port)` — binds to 0.0.0.0:{port}.
pub(crate) fn native_server_socket_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let port = extract_int_arg(args, 1)?;
    let addr = format!("0.0.0.0:{port}");
    let server_id = heap.bind_server_socket(&addr)?;
    let actual_port = heap.server_socket_local_port(server_id)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 2 {
        return Err(VmError::InvalidRef { address: this_ref });
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
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let server_fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(VmError::JavaException {
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
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(port)) => Ok(Some(Slot::Int(*port))),
        _ => Err(VmError::JavaException {
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
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        Some(Slot::Int(_)) => return Ok(None), // already closed — idempotent
        _ => {
            return Err(VmError::JavaException {
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
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let host_ref = extract_ref_arg(args, 1)?;
    let host = heap
        .get(host_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)?;
    let port = extract_int_arg(args, 2)?;
    let addr = format!("{host}:{port}");
    let (reader_id, writer_id) = heap.connect_socket(&addr)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 2 {
        return Err(VmError::InvalidRef { address: this_ref });
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
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd_read = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(VmError::JavaException {
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
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd_write = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(VmError::JavaException {
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
) -> VmResult<Option<Slot>> {
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

// ── ZIP / JAR natives ──────────────────────────────────────────────────

/// Native: `ZipFile.<init>(String)` — open and index a ZIP/JAR archive.
pub(crate) fn native_zip_file_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path_ref = extract_ref_arg(args, 1)?;
    let path_str = heap
        .get(path_ref)?
        .string_value
        .as_deref()
        .ok_or(VmError::NullPointerException)?
        .to_string();
    let fd = heap.open_host_zip(std::path::Path::new(&path_str))?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(fd);
    Ok(None)
}

/// Native: `JarFile.<init>(File)` — open and index a JAR archive from a File object.
pub(crate) fn native_jar_file_init_from_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let file_ref = extract_ref_arg(args, 1)?;
    let path = file_path_from_ref(file_ref, heap)?;
    let fd = heap.open_host_zip(&path)?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(fd);
    Ok(None)
}

pub(crate) fn native_jar_file_init_with_mode_and_version(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let forwarded_args = match args {
        [this_slot, file_slot, ..] => [*this_slot, *file_slot],
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "this,file",
                got: "other",
            });
        }
    };
    native_jar_file_init_from_file(&forwarded_args, heap, out, control)
}

pub(crate) fn native_jar_file_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let Ok(manifest_bytes) = heap.zip_read_entry(fd, "META-INF/MANIFEST.MF") else {
        return Ok(Some(Slot::Reference(None)));
    };
    let manifest_ref = allocate_manifest_from_bytes(heap, &manifest_bytes)?;
    Ok(Some(Slot::Reference(Some(manifest_ref))))
}

pub(crate) fn native_boot_nested_jar_file_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_jar_file_get_manifest(args, heap, out, control)
}

const BOOT_JAR_FILE_ARCHIVE_JAR_FILE_SLOT: usize = 1;
const BOOT_EXPLODED_ARCHIVE_ROOT_DIRECTORY_SLOT: usize = 0;
const BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT: usize = 2;

fn allocate_manifest_from_bytes(heap: &mut duke_gc::Heap, bytes: &[u8]) -> VmResult<u64> {
    let raw_ref = heap.allocate_string(String::from_utf8_lossy(bytes).into_owned());
    let manifest_ref = heap.allocate("java/util/jar/Manifest".to_string(), 1);
    heap.get_mut(manifest_ref)?.fields[0] = Slot::Reference(Some(raw_ref));
    Ok(manifest_ref)
}

pub(crate) fn native_boot_jar_file_archive_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let jar_file_ref = match heap
        .get(this_ref)?
        .fields
        .get(BOOT_JAR_FILE_ARCHIVE_JAR_FILE_SLOT)
        .copied()
    {
        Some(Slot::Reference(Some(jar_file_ref))) => jar_file_ref,
        Some(Slot::Reference(None)) | None => return Err(VmError::NullPointerException),
        Some(_) => {
            return Err(VmError::TypeMismatch {
                expected: "reference",
                got: "other",
            });
        }
    };
    native_jar_file_get_manifest(&[Slot::Reference(Some(jar_file_ref))], heap, out, control)
}

pub(crate) fn native_boot_archive_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.class_name.as_str() {
        "org/springframework/boot/loader/launch/JarFileArchive" => {
            native_boot_jar_file_archive_get_manifest(args, heap, out, control)
        }
        "org/springframework/boot/loader/launch/ExplodedArchive" => {
            native_boot_exploded_archive_get_manifest(args, heap, out, control)
        }
        _ => Err(VmError::MethodNotFound {
            name: format!("{}.getManifest", heap.get(this_ref)?.class_name),
            descriptor: "()Ljava/util/jar/Manifest;".to_string(),
        }),
    }
}

pub(crate) fn native_boot_exploded_archive_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    if let Some(slot @ Slot::Reference(Some(_))) = heap
        .get(this_ref)?
        .fields
        .get(BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT)
        .copied()
    {
        return Ok(Some(slot));
    }

    let root_directory_ref = match heap
        .get(this_ref)?
        .fields
        .get(BOOT_EXPLODED_ARCHIVE_ROOT_DIRECTORY_SLOT)
        .copied()
    {
        Some(Slot::Reference(Some(root_directory_ref))) => root_directory_ref,
        Some(Slot::Reference(None)) | None => return Err(VmError::NullPointerException),
        Some(_) => {
            return Err(VmError::TypeMismatch {
                expected: "reference",
                got: "other",
            });
        }
    };
    let manifest_path = file_path_from_ref(root_directory_ref, heap)?.join("META-INF/MANIFEST.MF");
    let manifest_bytes = match std::fs::read(&manifest_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(Slot::Reference(None)));
        }
        Err(_) => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
        }
    };
    let manifest_ref = allocate_manifest_from_bytes(heap, &manifest_bytes)?;
    heap.get_mut(this_ref)?.fields[BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT] =
        Slot::Reference(Some(manifest_ref));
    Ok(Some(Slot::Reference(Some(manifest_ref))))
}

/// Native: `ZipFile.getEntry(String) -> ZipEntry` — look up an entry by name.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_zip_file_get_entry(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let entry_name = heap
        .get(name_ref)?
        .string_value
        .as_deref()
        .ok_or(VmError::NullPointerException)?
        .to_string();
    let info = heap.zip_get_entry_info(fd, &entry_name)?;
    let Some(info) = info else {
        return Ok(Some(Slot::Reference(None)));
    };
    // Allocate a ZipEntry HeapObject with 6 fields.
    let name_heap_ref = heap.allocate_string(info.name);
    let entry_ref = heap.allocate("java/util/zip/ZipEntry".to_string(), 6);
    let entry_obj = heap.get_mut(entry_ref)?;
    entry_obj.fields[0] = Slot::Reference(Some(name_heap_ref));
    entry_obj.fields[1] = Slot::Int(info.compressed_size as i32);
    entry_obj.fields[2] = Slot::Int((info.compressed_size >> 32) as i32);
    entry_obj.fields[3] = Slot::Int(info.uncompressed_size as i32);
    entry_obj.fields[4] = Slot::Int((info.uncompressed_size >> 32) as i32);
    entry_obj.fields[5] = Slot::Int(i32::from(info.compression_method));
    Ok(Some(Slot::Reference(Some(entry_ref))))
}

/// Native: `ZipFile.getInputStream(ZipEntry) -> InputStream`
pub(crate) fn native_zip_file_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let entry_ref = extract_ref_arg(args, 1)?;
    let fd = extract_io_fd(heap, this_ref)?;
    // Get entry name from the ZipEntry object.
    let name_slot_ref = match heap.get(entry_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let entry_name = heap
        .get(name_slot_ref)?
        .string_value
        .as_deref()
        .ok_or(VmError::NullPointerException)?
        .to_string();
    // Decompress the entry and wrap in a ByteBuffer.
    let data = heap.zip_read_entry(fd, &entry_name)?;
    let buf_fd = heap.open_host_byte_buffer(data);
    let is_ref = heap.allocate("duke/zip/ByteBufferInputStream".to_string(), 1);
    let is_obj = heap.get_mut(is_ref)?;
    is_obj.fields[0] = Slot::Int(buf_fd);
    Ok(Some(Slot::Reference(Some(is_ref))))
}

/// Native: `ZipFile.close()`
pub(crate) fn native_zip_file_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) => *id,
        _ => return Ok(None),
    };
    heap.close_host_file(fd);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ZipFile.size() -> int`
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_zip_file_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let count = heap.zip_entry_count(fd)?;
    Ok(Some(Slot::Int(count as i32)))
}

/// Native: `ZipEntry.getName() -> String`
pub(crate) fn native_zip_entry_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(heap.get(this_ref)?.fields[0]))
}

/// Native: `ZipEntry.getCompressedSize() -> long`
pub(crate) fn native_zip_entry_get_compressed_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let lo = match fields[1] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let hi = match fields[2] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let val = (i64::from(hi) << 32) | (i64::from(lo) & 0xFFFF_FFFF);
    Ok(Some(Slot::Long(val)))
}

/// Native: `ZipEntry.getSize() -> long`
pub(crate) fn native_zip_entry_get_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let lo = match fields[3] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let hi = match fields[4] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let val = (i64::from(hi) << 32) | (i64::from(lo) & 0xFFFF_FFFF);
    Ok(Some(Slot::Long(val)))
}

/// Native: `ZipEntry.getMethod() -> int`
pub(crate) fn native_zip_entry_get_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(heap.get(this_ref)?.fields[5]))
}

/// Native: `String.length()` — returns string length as int.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_string_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let len = obj.string_value.as_ref().map_or(0, String::len);
    Ok(Some(Slot::Int(len as i32)))
}

/// Native: `String.equals(Object)` — compares string content.
pub(crate) fn native_string_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    // Fetch objects from heap in one go to keep borrows short
    let this_obj = heap.get(this_ref)?;
    let other_obj = heap.get(other_ref)?;
    let this_str = this_obj.string_value.as_deref().unwrap_or_default();
    let other_str = other_obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(this_str == other_str))))
}

/// Native: `String.charAt(int)` — returns char at index as int.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_string_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let index = extract_int_arg(args, 1)?;
    let obj = heap.get(this_ref)?;
    let s = obj.string_value.as_deref().unwrap_or("");
    let ch = s
        .chars()
        .nth(index as usize)
        .ok_or(VmError::ArrayIndexOutOfBounds {
            index,
            length: s.len(),
        })?;
    Ok(Some(Slot::Int(ch as i32)))
}

/// Native: `Object.<init>()V` - root constructor is a no-op after null-checking `this`.
pub(crate) fn native_object_init(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(None)
}

/// Native: `Object.getClass()` — returns a lightweight `Class` object for the runtime type.
pub(crate) fn native_object_get_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let class_name = heap.get(this_ref)?.class_name.clone();
    let class_ref = allocate_class_object(heap, &class_name)?;
    Ok(Some(Slot::Reference(Some(class_ref))))
}

/// Native: `Object.equals(Object)` — default Java object identity comparison.
pub(crate) fn native_object_equals(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let equal = extract_ref_arg(args, 1) == Ok(this_ref);
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `Object.hashCode()` — returns heap address as hash.
pub(crate) fn native_object_hashcode(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(Slot::Reference(Some(r))) => Ok(Some(Slot::Int(*r as i32))),
        _ => Err(VmError::NullPointerException),
    }
}

/// Native: `Object.toString()` — delegates to `heap_object_to_string` so String,
/// boxed primitives, and opaque objects all produce the correct Java representation.
pub(crate) fn native_object_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap_object_to_string(heap.get(this_ref)?, this_ref);
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Object.clone()` — shallow-copies a heap object.
pub(crate) fn native_object_clone(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let cloned_class = obj.class_name.clone();
    let cloned_fields = obj.fields.clone();
    let cloned_string = obj.string_value.clone();
    let new_ref = heap.allocate(cloned_class, 0);
    let dest = heap.get_mut(new_ref)?;
    dest.fields = cloned_fields;
    dest.string_value = cloned_string;
    Ok(Some(Slot::Reference(Some(new_ref))))
}

/// `Throwable.addSuppressed(Throwable suppressed)V`
///
/// No-op stub. Control flow is handled entirely by the bytecode desugaring —
/// `addSuppressed` only affects what `getSuppressed()` returns, which is not
/// yet implemented. Suppressed exception is silently dropped.
///
/// Signature: `args[0]` = this (Throwable), `args[1]` = suppressed (Throwable)
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_throwable_add_suppressed(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _stdout: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `Throwable.<init>(String)V` — stores detail message in `string_value`.
pub(crate) fn native_throwable_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let msg = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = msg;
    Ok(None)
}

/// Native: `Throwable.getMessage()String` — returns the stored detail message.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_throwable_get_message(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let msg = heap.get(*r)?.string_value.clone();
            let slot = msg.map_or(Slot::Reference(None), |s| {
                Slot::Reference(Some(heap.allocate_string(s)))
            });
            Ok(Some(slot))
        }
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Throwable.toString()String` — returns `"ClassName: message"` or just class name.
pub(crate) fn native_throwable_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let class_name = obj.class_name.replace('/', ".");
    let s = match &obj.string_value {
        Some(msg) => format!("{class_name}: {msg}"),
        None => class_name,
    };
    let _ = obj;
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

// ---- List.of / Set.of / Map.of factory methods ----

/// Helper: create an `ArrayList` from a slice of `Slot`s.
fn make_list_from_slots(
    elems: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<u64> {
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    for &elem in elems {
        native_arraylist_add(&[Slot::Reference(Some(list_ref)), elem], heap, out, control)?;
    }
    Ok(list_ref)
}

/// Native: `List.of(Object...)List` — all args (fixed-arity or varargs) become a new `ArrayList`.
///
/// Handles descriptors with 0–6+ fixed args and the varargs `([O)List` form.
pub(crate) fn native_list_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // Varargs form: single arg that is an Object[] array.
    let elems: Vec<Slot> = if args.len() == 1 {
        if let Some(Slot::Reference(Some(arr_ref))) = args.first() {
            let obj = heap.get(*arr_ref)?;
            if obj.class_name.starts_with('[') {
                let fields = obj.fields.clone();
                let _ = obj;
                fields
            } else {
                args.to_vec()
            }
        } else {
            Vec::new()
        }
    } else {
        args.to_vec()
    };
    let list_ref = make_list_from_slots(&elems, heap, out, control)?;
    Ok(Some(Slot::Reference(Some(list_ref))))
}

/// Helper: create a `HashSet` from a slice of `Slot`s.
fn make_set_from_slots(
    elems: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<u64> {
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    for &elem in elems {
        native_hashset_add(&[Slot::Reference(Some(set_ref)), elem], heap, out, control)?;
    }
    Ok(set_ref)
}

/// Native: `Set.of(Object...)Set` — all args (fixed-arity or varargs) become a new `HashSet`.
///
/// Handles both fixed-arity descriptors (multiple direct element args)
/// and the single-array varargs form `([O)Set`.
pub(crate) fn native_set_of_factory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let elems: Vec<Slot> = if args.len() == 1 {
        if let Some(Slot::Reference(Some(arr_ref))) = args.first() {
            let obj = heap.get(*arr_ref)?;
            if obj.class_name.starts_with('[') {
                let fields = obj.fields.clone();
                let _ = obj;
                fields
            } else {
                args.to_vec()
            }
        } else {
            Vec::new()
        }
    } else {
        args.to_vec()
    };
    let set_ref = make_set_from_slots(&elems, heap, out, control)?;
    Ok(Some(Slot::Reference(Some(set_ref))))
}

/// Native: `Map.of(K,V,...)Map` — pairs of args become entries in a new `HashMap`.
pub(crate) fn native_map_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(map_ref))], heap, out, control)?;
    let mut i = 0;
    while i + 1 < args.len() {
        let k = args[i];
        let v = args[i + 1];
        native_hashmap_put(&[Slot::Reference(Some(map_ref)), k, v], heap, out, control)?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(map_ref))))
}

// ---- Optional<T> natives ----

/// Native: `Optional.empty()Optional` — returns an Optional with no value.
pub(crate) fn native_optional_empty(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(None);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Optional.of(T)Optional` — wraps value; throws NPE if null.
pub(crate) fn native_optional_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = args.first().copied().unwrap_or(Slot::Reference(None));
    if matches!(val, Slot::Reference(None)) {
        return Err(VmError::NullPointerException);
    }
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r)?.fields[0] = val;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Optional.ofNullable(T)Optional` — wraps value or empty if null.
pub(crate) fn native_optional_of_nullable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r)?.fields[0] = val;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Optional.get()T` — returns value or throws `NoSuchElementException`.
pub(crate) fn native_optional_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    if matches!(val, Slot::Reference(None)) {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(val))
}

/// Native: `Optional.isPresent()Z` — true if a value is present.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let present = !matches!(
        heap.get(this_ref)?.fields.first(),
        Some(Slot::Reference(None)) | None
    );
    Ok(Some(Slot::Int(i32::from(present))))
}

/// Native: `Optional.isEmpty()Z` — true if no value is present (Java 11+).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let empty = matches!(
        heap.get(this_ref)?.fields.first(),
        Some(Slot::Reference(None)) | None
    );
    Ok(Some(Slot::Int(i32::from(empty))))
}

/// Native: `Optional.orElse(T)T` — returns value if present, else the argument.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let result = if matches!(val, Slot::Reference(None)) {
        args.get(1).copied().unwrap_or(Slot::Reference(None))
    } else {
        val
    };
    Ok(Some(result))
}

/// Native: `Optional.orElseThrow()T` — returns value or throws `NoSuchElementException`.
pub(crate) fn native_optional_or_else_throw(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_optional_get(args, heap, out, control)
}

// ---- ArrayList / HashMap bulk operations ----

/// Native: `ArrayList.addAll(Collection)Z` — appends all elements from a compatible collection.
pub(crate) fn native_arraylist_add_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_ref = extract_ref_arg(args, 1)?;
    let src_size = match heap.get(src_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let elems: Vec<Slot> = heap.get(src_ref)?.fields[1..=src_size].to_vec();
    let modified = !elems.is_empty();
    for elem in elems {
        native_arraylist_add(&[Slot::Reference(Some(this_ref)), elem], heap, out, control)?;
    }
    Ok(Some(Slot::Int(i32::from(modified))))
}

/// Native: `HashMap.putAll(Map)V` — copies all entries from the source `HashMap`.
pub(crate) fn native_hashmap_put_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_ref = extract_ref_arg(args, 1)?;
    let fields = heap.get(src_ref)?.fields.clone();
    // fields[0] = size, fields[1..] = k0, v0, k1, v1, ...
    let mut i = 1;
    while i + 1 < fields.len() {
        let k = fields[i];
        let v = fields[i + 1];
        native_hashmap_put(&[Slot::Reference(Some(this_ref)), k, v], heap, out, control)?;
        i += 2;
    }
    Ok(None)
}

/// Native: `HashMap.computeIfAbsent(K, Function)V` — returns existing value or computes and stores it.
pub(crate) fn native_hashmap_compute_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    // Check if key already present.
    let existing = native_hashmap_get(&[Slot::Reference(Some(this_ref)), key], heap, out, control)?;
    if let Some(v) = existing
        && !matches!(v, Slot::Reference(None))
    {
        return Ok(Some(v));
    }
    // Key absent — invoke the mapping function (lambda / SAM).
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let computed = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![Slot::Reference(Some(fn_ref)), key],
    )?;
    if let Some(value) = computed
        && !matches!(value, Slot::Reference(None))
    {
        native_hashmap_put(
            &[Slot::Reference(Some(this_ref)), key, value],
            heap,
            out,
            control,
        )?;
        return Ok(Some(value));
    }
    Ok(Some(Slot::Reference(None)))
}

// ---- LinkedList natives (field layout identical to ArrayList: fields[0]=size, fields[1..]=elements) ----

/// Native: `LinkedList.<init>()V` — same initialisation as `ArrayList`.
pub(crate) fn native_linked_list_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `LinkedList.size()I`
pub(crate) fn native_linked_list_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `LinkedList.add(Object)Z` — appends to tail.
pub(crate) fn native_linked_list_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)
}

/// Native: `LinkedList.get(I)Object`
pub(crate) fn native_linked_list_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_get(args, heap, out, control)
}

/// Native: `LinkedList.addFirst(Object)V` — inserts at index 0.
/// Field layout: fields[0]=Int(size), fields[1..size]=elements.
pub(crate) fn native_linked_list_add_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let obj = heap.get_mut(this_ref)?;
    // Shift existing elements right by one.
    obj.fields.insert(1, elem);
    obj.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    Ok(None)
}

/// Native: `LinkedList.addLast(Object)V` — appends to tail (same as add).
pub(crate) fn native_linked_list_add_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(None)
}

/// Native: `LinkedList.peekFirst()Object` — returns head without removal, or null if empty.
pub(crate) fn native_linked_list_peek_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `LinkedList.peekLast()Object` — returns tail without removal, or null if empty.
pub(crate) fn native_linked_list_peek_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[size]))
}

/// Native: `LinkedList.removeFirst()Object` — removes and returns head.
pub(crate) fn native_linked_list_remove_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let obj = heap.get_mut(this_ref)?;
    let elem = obj.fields.remove(1);
    obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    Ok(Some(elem))
}

/// Native: `LinkedList.removeLast()Object` — removes and returns tail.
pub(crate) fn native_linked_list_remove_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let obj = heap.get_mut(this_ref)?;
    let elem = obj.fields.remove(size);
    obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    Ok(Some(elem))
}

/// Native: `LinkedList.poll()Object` — removes and returns head, or null if empty.
pub(crate) fn native_linked_list_poll(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    let obj = heap.get_mut(this_ref)?;
    let elem = obj.fields.remove(1);
    obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    Ok(Some(elem))
}

/// Native: `LinkedList.offer(Object)Z` — appends to tail, returns true.
pub(crate) fn native_linked_list_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)
}

/// Native: `LinkedList.isEmpty()Z`
pub(crate) fn native_linked_list_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

/// Native: `LinkedList.iterator()Iterator` — returns an ArrayList-compatible iterator.
pub(crate) fn native_linked_list_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_iterator(args, heap, out, control)
}

// ---- HashMap.forEach callback ----

/// Native: `HashMap.forEach(BiConsumer)V` — iterates key-value pairs, invoking `accept(k, v)`.
pub(crate) fn native_hashmap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let consumer_ref = extract_ref_arg(args, 1)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    // Snapshot key-val pairs (fields[1,2], fields[3,4], ...)
    let pairs: Vec<(Slot, Slot)> = (0..size)
        .map(|i| {
            let key = heap
                .get(this_ref)
                .map(|o| o.fields[1 + i * 2])
                .unwrap_or(Slot::Reference(None));
            let val = heap
                .get(this_ref)
                .map(|o| o.fields[2 + i * 2])
                .unwrap_or(Slot::Reference(None));
            (key, val)
        })
        .collect();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for (key, val) in pairs {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;Ljava/lang/Object;)V",
            vec![Slot::Reference(Some(consumer_ref)), key, val],
        )?;
    }
    let _ = control;
    Ok(None)
}

// ---- TreeMap natives (field layout: fields[0]=Int(size), fields[1,2]=k0/v0 sorted by String key) ----
// Keys are stored sorted in ascending lexicographic order for O(n) insert / O(1) first&last.

/// Native: `TreeMap.<init>()V`
pub(crate) fn native_treemap_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `TreeMap.put(K,V)V` — inserts in sorted key order.
pub(crate) fn native_treemap_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let val = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let key_str = match &key {
        Slot::Reference(Some(r)) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    // Check for existing key — update in place.
    for i in 0..size {
        let existing_key = heap.get(this_ref)?.fields[1 + i * 2];
        let existing_str = match &existing_key {
            Slot::Reference(Some(r)) => heap.get(*r).ok().and_then(|o| o.string_value.clone()),
            _ => None,
        };
        if existing_str == key_str {
            heap.get_mut(this_ref)?.fields[2 + i * 2] = val;
            return Ok(Some(Slot::Reference(None)));
        }
    }
    // Find sorted insert position.
    let insert_pos = {
        let mut pos = size;
        for i in 0..size {
            let ek = heap.get(this_ref)?.fields[1 + i * 2];
            let ek_str: Option<String> = match &ek {
                Slot::Reference(Some(r)) => heap.get(*r).ok().and_then(|o| o.string_value.clone()),
                _ => None,
            };
            let cmp = key_str
                .as_deref()
                .unwrap_or("")
                .cmp(ek_str.as_deref().unwrap_or(""));
            if cmp == std::cmp::Ordering::Less {
                pos = i;
                break;
            }
        }
        pos
    };
    let obj = heap.get_mut(this_ref)?;
    obj.fields.insert(1 + insert_pos * 2, val);
    obj.fields.insert(1 + insert_pos * 2, key);
    obj.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    Ok(Some(Slot::Reference(None)))
}

/// Native: `TreeMap.get(K)V`
pub(crate) fn native_treemap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let key_str = match &key {
        Slot::Reference(Some(r)) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    for i in 0..size {
        let ek = heap.get(this_ref)?.fields[1 + i * 2];
        let ek_str = match &ek {
            Slot::Reference(Some(r)) => heap.get(*r).ok().and_then(|o| o.string_value.clone()),
            _ => None,
        };
        if ek_str == key_str {
            let v = heap.get(this_ref)?.fields[2 + i * 2];
            return Ok(Some(v));
        }
    }
    Ok(Some(Slot::Reference(None)))
}

/// Native: `TreeMap.containsKey(K)Z`
pub(crate) fn native_treemap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let found = !matches!(
        native_treemap_get(args, heap, out, control)?,
        Some(Slot::Reference(None)) | None
    );
    Ok(Some(Slot::Int(i32::from(found))))
}

/// Native: `TreeMap.size()I`
pub(crate) fn native_treemap_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Slot::Int(*n),
        _ => Slot::Int(0),
    }))
}

/// Native: `TreeMap.firstKey()K` — returns the smallest key (index 0 in sorted list).
pub(crate) fn native_treemap_first_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `TreeMap.lastKey()K` — returns the largest key.
pub(crate) fn native_treemap_last_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(heap.get(this_ref)?.fields[1 + (size - 1) * 2]))
}

/// Native: `TreeMap.remove(K)V`
pub(crate) fn native_treemap_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let key_str = match &key {
        Slot::Reference(Some(r)) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    for i in 0..size {
        let ek = heap.get(this_ref)?.fields[1 + i * 2];
        let ek_str = match &ek {
            Slot::Reference(Some(r)) => heap.get(*r).ok().and_then(|o| o.string_value.clone()),
            _ => None,
        };
        if ek_str == key_str {
            let obj = heap.get_mut(this_ref)?;
            let v = obj.fields.remove(2 + i * 2);
            obj.fields.remove(1 + i * 2);
            obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
            return Ok(Some(v));
        }
    }
    Ok(Some(Slot::Reference(None)))
}

/// Native: `TreeMap.isEmpty()Z`
pub(crate) fn native_treemap_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) | None => 1,
        _ => 0,
    })))
}

// ---- Stack natives (LIFO backed by ArrayList: push=add, pop=removeLast, peek=peekLast) ----

/// Native: `Stack.<init>()V`
pub(crate) fn native_stack_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `Stack.push(E)E` — appends to tail, returns the element.
pub(crate) fn native_stack_push(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(elem))
}

/// Native: `Stack.pop()E` — removes and returns the top element.
pub(crate) fn native_stack_pop(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_linked_list_remove_last(args, heap, out, control)
}

/// Native: `Stack.peek()E` — returns the top element without removal.
pub(crate) fn native_stack_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_linked_list_peek_last(args, heap, out, control)
}

/// Native: `Stack.empty()Z` — returns true if the stack is empty.
pub(crate) fn native_stack_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

/// Native: `Stack.size()I`
pub(crate) fn native_stack_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

// ---- TreeSet natives (sorted unique elements, backed by sorted Vec<Slot>) ----
// fields[0] = Int(size), fields[1..] = unique elements in sorted String order

/// Native: `TreeSet.<init>()V`
pub(crate) fn native_treeset_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `TreeSet.add(E)Z` — inserts in sorted order; returns false if already present.
pub(crate) fn native_treeset_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let elem_str = match &elem {
        Slot::Reference(Some(r)) => heap.get(*r)?.string_value.clone(),
        Slot::Int(v) => Some(v.to_string()),
        _ => None,
    };
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    // Check for duplicate.
    for i in 0..size {
        let ex = heap.get(this_ref)?.fields[1 + i];
        let ex_str = match &ex {
            Slot::Reference(Some(r)) => heap.get(*r).ok().and_then(|o| o.string_value.clone()),
            Slot::Int(v) => Some(v.to_string()),
            _ => None,
        };
        if ex_str == elem_str {
            return Ok(Some(Slot::Int(0))); // false — no change
        }
    }
    // Find sorted insert position.
    let insert_pos = {
        let mut pos = size;
        for i in 0..size {
            let ex = heap.get(this_ref)?.fields[1 + i];
            let ex_str: Option<String> = match &ex {
                Slot::Reference(Some(r)) => heap.get(*r).ok().and_then(|o| o.string_value.clone()),
                Slot::Int(v) => Some(v.to_string()),
                _ => None,
            };
            if elem_str.as_deref().unwrap_or("") < ex_str.as_deref().unwrap_or("") {
                pos = i;
                break;
            }
        }
        pos
    };
    let obj = heap.get_mut(this_ref)?;
    obj.fields.insert(1 + insert_pos, elem);
    obj.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(1))) // true — element added
}

/// Native: `TreeSet.contains(E)Z`
pub(crate) fn native_treeset_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let elem_str = match &elem {
        Slot::Reference(Some(r)) => heap.get(*r)?.string_value.clone(),
        Slot::Int(v) => Some(v.to_string()),
        _ => None,
    };
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    for i in 0..size {
        let ex = heap.get(this_ref)?.fields[1 + i];
        let ex_str = match &ex {
            Slot::Reference(Some(r)) => heap.get(*r).ok().and_then(|o| o.string_value.clone()),
            Slot::Int(v) => Some(v.to_string()),
            _ => None,
        };
        if ex_str == elem_str {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `TreeSet.size()I`
pub(crate) fn native_treeset_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Slot::Int(*n),
        _ => Slot::Int(0),
    }))
}

/// Native: `TreeSet.first()E` — returns smallest element.
pub(crate) fn native_treeset_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first().copied() {
        Some(Slot::Int(0)) | None => Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        }),
        _ => Ok(Some(heap.get(this_ref)?.fields[1])),
    }
}

/// Native: `TreeSet.last()E` — returns largest element.
pub(crate) fn native_treeset_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(heap.get(this_ref)?.fields[size]))
}

/// Native: `TreeSet.isEmpty()Z`
pub(crate) fn native_treeset_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) | None => 1,
        _ => 0,
    })))
}

/// Native: `TreeSet.iterator()Iterator` — returns an ArrayList-compatible iterator over sorted elements.
pub(crate) fn native_treeset_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_iterator(args, heap, out, control)
}

// ---- Collections.min / max / shuffle ----

/// Native: `Collections.min(Collection)T` — returns minimum element via `compareTo`.
pub(crate) fn native_collections_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let coll_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(coll_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let Slot::Reference(Some(mut min_ref)) = heap.get(coll_ref)?.fields[1] else {
        return Ok(Some(Slot::Reference(None)));
    };
    for i in 2..=size {
        let Slot::Reference(Some(candidate)) = heap.get(coll_ref)?.fields[i] else {
            continue;
        };
        let class_name = heap.get(candidate)?.class_name.clone();
        let cmp = ops.invoke(
            heap,
            out,
            &class_name,
            "compareTo",
            "(Ljava/lang/Object;)I",
            vec![
                Slot::Reference(Some(candidate)),
                Slot::Reference(Some(min_ref)),
            ],
        )?;
        let _ = control;
        if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
            min_ref = candidate;
        }
    }
    Ok(Some(Slot::Reference(Some(min_ref))))
}

/// Native: `Collections.max(Collection)T` — returns maximum element via `compareTo`.
pub(crate) fn native_collections_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let coll_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(coll_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let Slot::Reference(Some(mut max_ref)) = heap.get(coll_ref)?.fields[1] else {
        return Ok(Some(Slot::Reference(None)));
    };
    for i in 2..=size {
        let Slot::Reference(Some(candidate)) = heap.get(coll_ref)?.fields[i] else {
            continue;
        };
        let class_name = heap.get(candidate)?.class_name.clone();
        let cmp = ops.invoke(
            heap,
            out,
            &class_name,
            "compareTo",
            "(Ljava/lang/Object;)I",
            vec![
                Slot::Reference(Some(candidate)),
                Slot::Reference(Some(max_ref)),
            ],
        )?;
        let _ = control;
        if matches!(cmp, Some(Slot::Int(n)) if n > 0) {
            max_ref = candidate;
        }
    }
    Ok(Some(Slot::Reference(Some(max_ref))))
}

/// Native: `Collections.shuffle(List)V` — no-op (deterministic test environments).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_shuffle(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

// ---- Stream natives ----
// duke/util/Stream: fields[0]=Int(size), fields[1..]=element refs

/// Native: `Stream.of(Object[])Stream` — create stream from varargs array.
pub(crate) fn native_stream_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let elems: Vec<Slot> = heap.get(arr_ref)?.fields.clone();
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for elem in elems {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `ArrayList.stream()` — wrap `ArrayList` elements into a `Stream`.
pub(crate) fn native_arraylist_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    let elems: Vec<Slot> =
        heap.get(list_ref)?.fields[1..=usize::try_from(size).unwrap_or(0)].to_vec();
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(size);
    for elem in elems {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `Stream.count()J` — returns the number of elements as a long.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let n = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => i64::from(*n),
        _ => 0,
    };
    Ok(Some(Slot::Long(n)))
}

/// Native: `Stream.filter(Predicate)Stream` — keeps elements where `predicate.test()` returns true.
pub(crate) fn native_stream_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied();
    let Slot::Reference(Some(pred_ref)) = pred_slot.unwrap_or(Slot::Reference(None)) else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept: Vec<Slot> = Vec::new();
    for elem in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(pred_ref)), elem],
        )?;
        if matches!(result, Some(Slot::Int(1))) {
            kept.push(elem);
        }
    }
    let new_size = i32::try_from(kept.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in kept {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

/// Native: `Stream.map(Function)Stream` — transforms each element via `function.apply()`.
pub(crate) fn native_stream_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied();
    let Slot::Reference(Some(fn_ref)) = fn_slot.unwrap_or(Slot::Reference(None)) else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut mapped: Vec<Slot> = Vec::with_capacity(elems.len());
    for elem in elems {
        let result = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![Slot::Reference(Some(fn_ref)), elem],
        )?;
        mapped.push(result.unwrap_or(Slot::Reference(None)));
    }
    let new_size = i32::try_from(mapped.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in mapped {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

/// Native: `Stream.forEach(Consumer)V` — calls `consumer.accept()` on each element.
pub(crate) fn native_stream_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = args.get(1).copied();
    let Slot::Reference(Some(consumer_ref)) = consumer_slot.unwrap_or(Slot::Reference(None)) else {
        return Ok(None);
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for elem in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![Slot::Reference(Some(consumer_ref)), elem],
        )?;
    }
    Ok(None)
}

/// Native: `Stream.collect(Collector)Object` — collects to list (only toList collector supported).
#[allow(clippy::too_many_lines, clippy::only_used_in_recursion)]
pub(crate) fn native_stream_collect(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    let elems: Vec<Slot> =
        heap.get(stream_ref)?.fields[1..=usize::try_from(size).unwrap_or(0)].to_vec();

    // Dispatch on collector type.
    let collector_class = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.class_name.clone(),
        _ => "duke/util/ToListCollector".to_string(),
    };

    if collector_class == "duke/util/JoiningCollector" {
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let read_str_field = |heap: &duke_gc::Heap, idx: usize| -> String {
            match heap
                .get(collector_ref)
                .ok()
                .and_then(|o| o.fields.get(idx).copied())
            {
                Some(Slot::Reference(Some(dr))) => heap
                    .get(dr)
                    .ok()
                    .and_then(|o| o.string_value.clone())
                    .unwrap_or_default(),
                _ => String::new(),
            }
        };
        let delim = read_str_field(heap, 0);
        let prefix = read_str_field(heap, 1);
        let suffix = read_str_field(heap, 2);
        let parts: Vec<String> = elems
            .iter()
            .filter_map(|s| match s {
                Slot::Reference(Some(r)) => heap.get(*r).ok()?.string_value.clone(),
                _ => None,
            })
            .collect();
        let joined = format!("{}{}{}", prefix, parts.join(&delim), suffix);
        let result_ref = heap.allocate_string(joined);
        Ok(Some(Slot::Reference(Some(result_ref))))
    } else if collector_class == "duke/util/CountingCollector" {
        // collect() returns Object; box the Long so bytecode can checkcast/invokevirtual it.
        let boxed = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Long(i64::from(size));
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/GroupingByCollector" {
        // GroupingByCollector: fields[0] = key function slot
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let fn_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // Build a HashMap: key → ArrayList of values
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let key = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![fn_slot, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            // find existing bucket or create new list
            let fields = heap.get(map_ref)?.fields.clone();
            let size_n = match fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let mut found_ki = None;
            for i in 0..size_n {
                let ki = 1 + i * 2;
                if fields.get(ki).is_some_and(|k| slots_equal(k, &key, heap)) {
                    found_ki = Some(ki);
                    break;
                }
            }
            if let Some(ki) = found_ki {
                // Append elem to existing list
                let list_slot = heap
                    .get(map_ref)?
                    .fields
                    .get(ki + 1)
                    .copied()
                    .unwrap_or(Slot::Reference(None));
                if let Slot::Reference(Some(list_ref)) = list_slot {
                    let list_size = match heap.get(list_ref)?.fields.first() {
                        Some(Slot::Int(n)) => *n,
                        _ => 0,
                    };
                    heap.get_mut(list_ref)?.fields.push(elem);
                    heap.get_mut(list_ref)?.fields[0] = Slot::Int(list_size + 1);
                }
            } else {
                // New key — create list with one element
                let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
                heap.get_mut(list_ref)?.fields[0] = Slot::Int(1);
                heap.get_mut(list_ref)?.fields.push(elem);
                heap.get_mut(map_ref)?.fields.push(key);
                heap.get_mut(map_ref)?
                    .fields
                    .push(Slot::Reference(Some(list_ref)));
                heap.get_mut(map_ref)?.fields[0] =
                    Slot::Int(i32::try_from(size_n + 1).unwrap_or(i32::MAX));
            }
        }
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/ToSetCollector" {
        // Collect into HashSet (deduplicates).
        let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
        heap.get_mut(set_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            // Check for duplicate before inserting
            let set_fields = heap.get(set_ref)?.fields.clone();
            let set_size = match set_fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let already = set_fields[1..=set_size]
                .iter()
                .any(|s| slots_equal(s, &elem, heap));
            if !already {
                let cur_size = match heap.get(set_ref)?.fields.first() {
                    Some(Slot::Int(n)) => *n,
                    _ => 0,
                };
                heap.get_mut(set_ref)?.fields.push(elem);
                heap.get_mut(set_ref)?.fields[0] = Slot::Int(cur_size + 1);
            }
        }
        Ok(Some(Slot::Reference(Some(set_ref))))
    } else if collector_class == "duke/util/ToMapCollector" {
        // Collect into HashMap using key/val extractor functions.
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let key_fn = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let val_fn = heap
            .get(collector_ref)?
            .fields
            .get(1)
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(key_ref)) = key_fn else {
            return Err(VmError::NullPointerException);
        };
        let Slot::Reference(Some(val_ref)) = val_fn else {
            return Err(VmError::NullPointerException);
        };
        let key_class = heap.get(key_ref)?.class_name.clone();
        let val_class = heap.get(val_ref)?.class_name.clone();
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let k = ops
                .invoke(
                    heap,
                    out,
                    &key_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![key_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let v = ops
                .invoke(
                    heap,
                    out,
                    &val_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![val_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let cur_size = match heap.get(map_ref)?.fields.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            heap.get_mut(map_ref)?.fields.push(k);
            heap.get_mut(map_ref)?.fields.push(v);
            heap.get_mut(map_ref)?.fields[0] = Slot::Int(cur_size + 1);
        }
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/ToMapMergeCollector" {
        // Collect into HashMap with merge function for duplicate keys.
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let key_fn = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let val_fn = heap
            .get(collector_ref)?
            .fields
            .get(1)
            .copied()
            .unwrap_or(Slot::Reference(None));
        let merge_fn = heap
            .get(collector_ref)?
            .fields
            .get(2)
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(key_ref)) = key_fn else {
            return Err(VmError::NullPointerException);
        };
        let Slot::Reference(Some(val_ref)) = val_fn else {
            return Err(VmError::NullPointerException);
        };
        let Slot::Reference(Some(merge_ref)) = merge_fn else {
            return Err(VmError::NullPointerException);
        };
        let key_class = heap.get(key_ref)?.class_name.clone();
        let val_class = heap.get(val_ref)?.class_name.clone();
        let merge_class = heap.get(merge_ref)?.class_name.clone();
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let k = ops
                .invoke(
                    heap,
                    out,
                    &key_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![key_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let v = ops
                .invoke(
                    heap,
                    out,
                    &val_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![val_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let fields = heap.get(map_ref)?.fields.clone();
            if let Some(i) = find_hashmap_entry_index(&fields, &k, heap) {
                // Duplicate key — apply merge function: merge(existing, new)
                let existing = fields[i + 1];
                let merged = ops
                    .invoke(
                        heap,
                        out,
                        &merge_class,
                        "apply",
                        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                        vec![merge_fn, existing, v],
                    )?
                    .unwrap_or(Slot::Reference(None));
                heap.get_mut(map_ref)?.fields[i + 1] = merged;
            } else {
                let cur_size = match heap.get(map_ref)?.fields.first() {
                    Some(Slot::Int(n)) => *n,
                    _ => 0,
                };
                heap.get_mut(map_ref)?.fields.push(k);
                heap.get_mut(map_ref)?.fields.push(v);
                heap.get_mut(map_ref)?.fields[0] = Slot::Int(cur_size + 1);
            }
        }
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/PartitioningByCollector" {
        // Collect into a Map<Boolean, List> partitioned by predicate.
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let pred_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(pred_ref)) = pred_slot else {
            return Err(VmError::NullPointerException);
        };
        let pred_class = heap.get(pred_ref)?.class_name.clone();
        // Create two lists and the result map.
        let true_list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(true_list)?.fields[0] = Slot::Int(0);
        let false_list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(false_list)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![Slot::Reference(Some(pred_ref)), elem],
            )?;
            let is_true = matches!(result, Some(Slot::Int(n)) if n != 0);
            let target = if is_true { true_list } else { false_list };
            let cur_size = match heap.get(target)?.fields.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            heap.get_mut(target)?.fields.push(elem);
            heap.get_mut(target)?.fields[0] = Slot::Int(cur_size + 1);
        }
        // Build HashMap: Boolean(1)→trueList, Boolean(0)→falseList
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(2);
        let bool_true = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(bool_true)?.fields[0] = Slot::Int(1);
        let bool_false = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(bool_false)?.fields[0] = Slot::Int(0);
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(bool_true)));
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(true_list)));
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(bool_false)));
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(false_list)));
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/SummingIntCollector" {
        // Sum via applyAsInt(elem) for each element.
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let fn_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(VmError::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i32;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(Ljava/lang/Object;)I",
                vec![fn_slot, elem],
            )?;
            if let Some(Slot::Int(n)) = result {
                sum = sum.wrapping_add(n);
            }
        }
        // Return boxed Integer
        let boxed = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Int(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingIntCollector" {
        // Average via applyAsInt(elem) for each element.
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let fn_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(VmError::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        let mut count = 0_usize;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(Ljava/lang/Object;)I",
                vec![fn_slot, elem],
            )?;
            if let Some(Slot::Int(n)) = result {
                sum += i64::from(n);
                count += 1;
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let avg = if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        };
        // Return boxed Double
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/SummingLongCollector" {
        // Sum via applyAsLong(elem) for each element.
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let fn_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(VmError::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(Ljava/lang/Object;)J",
                vec![fn_slot, elem],
            )?;
            match result {
                Some(Slot::Long(n)) => sum = sum.wrapping_add(n),
                Some(Slot::Int(n)) => sum = sum.wrapping_add(i64::from(n)),
                _ => {}
            }
        }
        // Return boxed Long
        let boxed = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Long(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingDoubleCollector" {
        // Average via applyAsDouble(elem) for each element.
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let fn_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(VmError::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0.0_f64;
        let mut count = 0_usize;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(Ljava/lang/Object;)D",
                vec![fn_slot, elem],
            )?;
            match result {
                Some(Slot::Double(d)) => {
                    sum += d;
                    count += 1;
                }
                Some(Slot::Float(f)) => {
                    sum += f64::from(f);
                    count += 1;
                }
                Some(Slot::Int(n)) => {
                    sum += f64::from(n);
                    count += 1;
                }
                _ => {}
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let avg = if count == 0 { 0.0 } else { sum / count as f64 };
        // Return boxed Double
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/MappingCollector" {
        // MappingCollector: fields[0]=mapper fn, fields[1]=downstream collector
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let mapper_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let downstream_slot = heap
            .get(collector_ref)?
            .fields
            .get(1)
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(mapper_ref)) = mapper_slot else {
            return Err(VmError::NullPointerException);
        };
        let mapper_class = heap.get(mapper_ref)?.class_name.clone();
        // Map each element through the mapper function
        let mut mapped_elems = Vec::with_capacity(elems.len());
        for elem in elems {
            let mapped = ops
                .invoke(
                    heap,
                    out,
                    &mapper_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![mapper_slot, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            mapped_elems.push(mapped);
        }
        // Build a temporary stream from mapped elements and collect with downstream
        let mapped_size = i32::try_from(mapped_elems.len()).unwrap_or(0);
        let tmp_stream = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(tmp_stream)?.fields[0] = Slot::Int(mapped_size);
        for elem in mapped_elems {
            heap.get_mut(tmp_stream)?.fields.push(elem);
        }
        let tmp_args = vec![Slot::Reference(Some(tmp_stream)), downstream_slot];
        native_stream_collect(&tmp_args, heap, out, control, ops)
    } else if collector_class == "duke/util/GroupingBy2Collector" {
        // groupingBy(keyFn, downstream): fields[0]=keyFn, fields[1]=downstream collector
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let fn_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let downstream_slot = heap
            .get(collector_ref)?
            .fields
            .get(1)
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(VmError::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // First pass: group raw elements by key into HashMap<key, ArrayList<elem>>
        let raw_map = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(raw_map)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let key = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![fn_slot, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let fields = heap.get(raw_map)?.fields.clone();
            let size_n = match fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let mut found_ki = None;
            for i in 0..size_n {
                let ki = 1 + i * 2;
                if fields.get(ki).is_some_and(|k| slots_equal(k, &key, heap)) {
                    found_ki = Some(ki);
                    break;
                }
            }
            if let Some(ki) = found_ki {
                let list_slot = heap
                    .get(raw_map)?
                    .fields
                    .get(ki + 1)
                    .copied()
                    .unwrap_or(Slot::Reference(None));
                if let Slot::Reference(Some(list_ref)) = list_slot {
                    let list_size = match heap.get(list_ref)?.fields.first() {
                        Some(Slot::Int(n)) => *n,
                        _ => 0,
                    };
                    heap.get_mut(list_ref)?.fields.push(elem);
                    heap.get_mut(list_ref)?.fields[0] = Slot::Int(list_size + 1);
                }
            } else {
                let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
                heap.get_mut(list_ref)?.fields[0] = Slot::Int(1);
                heap.get_mut(list_ref)?.fields.push(elem);
                heap.get_mut(raw_map)?.fields.push(key);
                heap.get_mut(raw_map)?
                    .fields
                    .push(Slot::Reference(Some(list_ref)));
                heap.get_mut(raw_map)?.fields[0] =
                    Slot::Int(i32::try_from(size_n + 1).unwrap_or(i32::MAX));
            }
        }
        // Second pass: apply downstream collector to each group's ArrayList
        let result_map = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(result_map)?.fields[0] = Slot::Int(0);
        let raw_fields = heap.get(raw_map)?.fields.clone();
        let group_count = match raw_fields.first() {
            Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
            _ => 0,
        };
        for i in 0..group_count {
            let key = raw_fields
                .get(1 + i * 2)
                .copied()
                .unwrap_or(Slot::Reference(None));
            let list_slot = raw_fields
                .get(2 + i * 2)
                .copied()
                .unwrap_or(Slot::Reference(None));
            let Slot::Reference(Some(list_ref)) = list_slot else {
                continue;
            };
            let group_size_field = heap
                .get(list_ref)?
                .fields
                .first()
                .copied()
                .unwrap_or(Slot::Int(0));
            let group_size = match group_size_field {
                Slot::Int(n) => n,
                _ => 0,
            };
            let tmp_stream = heap.allocate("duke/util/Stream".to_string(), 1);
            heap.get_mut(tmp_stream)?.fields[0] = group_size_field;
            let group_elems: Vec<Slot> =
                heap.get(list_ref)?.fields[1..=usize::try_from(group_size).unwrap_or(0)].to_vec();
            for e in group_elems {
                heap.get_mut(tmp_stream)?.fields.push(e);
            }
            let tmp_args = vec![Slot::Reference(Some(tmp_stream)), downstream_slot];
            let collected = native_stream_collect(&tmp_args, heap, out, control, ops)?
                .unwrap_or(Slot::Reference(None));
            let cur_result_size = match heap.get(result_map)?.fields.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            heap.get_mut(result_map)?.fields.push(key);
            heap.get_mut(result_map)?.fields.push(collected);
            heap.get_mut(result_map)?.fields[0] = Slot::Int(cur_result_size + 1);
        }
        Ok(Some(Slot::Reference(Some(result_map))))
    } else if collector_class == "duke/util/MinByCollector" {
        // minBy(comparator): fields[0] = comparator
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let cmp_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(cmp_ref)) = cmp_slot else {
            let r = make_optional(heap, None);
            return Ok(Some(Slot::Reference(Some(r))));
        };
        let cmp_class = heap.get(cmp_ref)?.class_name.clone();
        let mut min: Option<Slot> = None;
        for elem in elems {
            let is_less = if let Some(ref cur) = min {
                let result = ops
                    .invoke(
                        heap,
                        out,
                        &cmp_class,
                        "compare",
                        "(Ljava/lang/Object;Ljava/lang/Object;)I",
                        vec![cmp_slot, elem, *cur],
                    )?
                    .unwrap_or(Slot::Int(0));
                matches!(result, Slot::Int(n) if n < 0)
            } else {
                true
            };
            if is_less {
                min = Some(elem);
            }
        }
        let r = make_optional(heap, min);
        Ok(Some(Slot::Reference(Some(r))))
    } else if collector_class == "duke/util/MaxByCollector" {
        // maxBy(comparator): fields[0] = comparator
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let cmp_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(cmp_ref)) = cmp_slot else {
            let r = make_optional(heap, None);
            return Ok(Some(Slot::Reference(Some(r))));
        };
        let cmp_class = heap.get(cmp_ref)?.class_name.clone();
        let mut max: Option<Slot> = None;
        for elem in elems {
            let is_greater = if let Some(ref cur) = max {
                let result = ops
                    .invoke(
                        heap,
                        out,
                        &cmp_class,
                        "compare",
                        "(Ljava/lang/Object;Ljava/lang/Object;)I",
                        vec![cmp_slot, elem, *cur],
                    )?
                    .unwrap_or(Slot::Int(0));
                matches!(result, Slot::Int(n) if n > 0)
            } else {
                true
            };
            if is_greater {
                max = Some(elem);
            }
        }
        let r = make_optional(heap, max);
        Ok(Some(Slot::Reference(Some(r))))
    } else if collector_class == "duke/util/SummingDoubleCollector" {
        // summingDouble: fields[0] = ToDoubleFunction
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let fn_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(VmError::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0.0_f64;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(Ljava/lang/Object;)D",
                vec![fn_slot, elem],
            )?;
            match result {
                Some(Slot::Double(d)) => sum += d,
                Some(Slot::Float(f)) => sum += f64::from(f),
                Some(Slot::Int(n)) => sum += f64::from(n),
                _ => {}
            }
        }
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingLongCollector" {
        // averagingLong: fields[0] = ToLongFunction
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let fn_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(VmError::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        let mut count = 0_usize;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(Ljava/lang/Object;)J",
                vec![fn_slot, elem],
            )?;
            match result {
                Some(Slot::Long(n)) => {
                    sum = sum.wrapping_add(n);
                    count += 1;
                }
                Some(Slot::Int(n)) => {
                    sum = sum.wrapping_add(i64::from(n));
                    count += 1;
                }
                _ => {}
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let avg = if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        };
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/ReducingNoIdentityCollector" {
        // reducing(BinaryOperator) → Optional<T>
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let op_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(bop_ref)) = op_slot else {
            return Err(VmError::NullPointerException);
        };
        let op_class = heap.get(bop_ref)?.class_name.clone();
        let result = if elems.is_empty() {
            None
        } else {
            let mut acc = elems[0];
            for elem in elems.into_iter().skip(1) {
                acc = ops
                    .invoke(
                        heap,
                        out,
                        &op_class,
                        "apply",
                        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                        vec![op_slot, acc, elem],
                    )?
                    .unwrap_or(Slot::Reference(None));
            }
            Some(acc)
        };
        let opt_ref = make_optional(heap, result);
        Ok(Some(Slot::Reference(Some(opt_ref))))
    } else if collector_class == "duke/util/ReducingCollector" {
        // reducing(identity, BinaryOperator) → T
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let identity_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let op_slot = heap
            .get(collector_ref)?
            .fields
            .get(1)
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(bop_ref)) = op_slot else {
            return Err(VmError::NullPointerException);
        };
        let op_class = heap.get(bop_ref)?.class_name.clone();
        let mut acc = identity_slot;
        for elem in elems {
            acc = ops
                .invoke(
                    heap,
                    out,
                    &op_class,
                    "apply",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![op_slot, acc, elem],
                )?
                .unwrap_or(Slot::Reference(None));
        }
        Ok(Some(acc))
    } else if collector_class == "duke/util/ReducingMappingCollector" {
        // reducing(identity, mapper, BinaryOperator) → U
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let identity_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let mapper_slot = heap
            .get(collector_ref)?
            .fields
            .get(1)
            .copied()
            .unwrap_or(Slot::Reference(None));
        let op_slot = heap
            .get(collector_ref)?
            .fields
            .get(2)
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(mapper_ref)) = mapper_slot else {
            return Err(VmError::NullPointerException);
        };
        let Slot::Reference(Some(bop_ref)) = op_slot else {
            return Err(VmError::NullPointerException);
        };
        let mapper_class = heap.get(mapper_ref)?.class_name.clone();
        let op_class = heap.get(bop_ref)?.class_name.clone();
        let mut acc = identity_slot;
        for elem in elems {
            let mapped = ops
                .invoke(
                    heap,
                    out,
                    &mapper_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![mapper_slot, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            acc = ops
                .invoke(
                    heap,
                    out,
                    &op_class,
                    "apply",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![op_slot, acc, mapped],
                )?
                .unwrap_or(Slot::Reference(None));
        }
        Ok(Some(acc))
    } else if collector_class == "duke/util/CollectingAndThenCollector" {
        // collectingAndThen(downstream, finisher): fields[0]=downstream, fields[1]=finisher
        let collector_ref = match args.get(1) {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let downstream_slot = heap
            .get(collector_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let finisher_slot = heap
            .get(collector_ref)?
            .fields
            .get(1)
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(finisher_ref)) = finisher_slot else {
            return Err(VmError::NullPointerException);
        };
        let finisher_class = heap.get(finisher_ref)?.class_name.clone();
        // First collect with downstream
        let tmp_args = vec![Slot::Reference(Some(stream_ref)), downstream_slot];
        let intermediate = native_stream_collect(&tmp_args, heap, out, control, ops)?
            .unwrap_or(Slot::Reference(None));
        // Then apply finisher
        let result = ops.invoke(
            heap,
            out,
            &finisher_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![finisher_slot, intermediate],
        )?;
        Ok(result.or(Some(Slot::Reference(None))))
    } else {
        // ToListCollector (default): collect into ArrayList.
        let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(list_ref)?.fields[0] = Slot::Int(size);
        for elem in elems {
            heap.get_mut(list_ref)?.fields.push(elem);
        }
        Ok(Some(Slot::Reference(Some(list_ref))))
    }
}

/// Native: `Stream.distinct()Stream` — removes duplicate elements (by `slots_equal`).
pub(crate) fn native_stream_distinct(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut seen: Vec<Slot> = Vec::new();
    for elem in elems {
        if !seen.iter().any(|s| slots_equal(s, &elem, heap)) {
            seen.push(elem);
        }
    }
    let new_size = i32::try_from(seen.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in seen {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

/// Native: `Stream.sorted()Stream` — sorts elements by natural order via `compareTo`.
pub(crate) fn native_stream_sorted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    // Insertion sort via compareTo callbacks (stable, O(n²) — fine for test sizes).
    for i in 1..elems.len() {
        let mut j = i;
        while j > 0 {
            let Slot::Reference(Some(a)) = elems[j - 1] else {
                break;
            };
            let Slot::Reference(Some(b)) = elems[j] else {
                break;
            };
            let class_a = heap.get(a)?.class_name.clone();
            let cmp = ops.invoke(
                heap,
                out,
                &class_a,
                "compareTo",
                "(Ljava/lang/Object;)I",
                vec![Slot::Reference(Some(a)), Slot::Reference(Some(b))],
            )?;
            if matches!(cmp, Some(Slot::Int(n)) if n > 0) {
                elems.swap(j - 1, j);
                j -= 1;
            } else {
                break;
            }
        }
    }
    let new_size = i32::try_from(elems.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in elems {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

/// Native: `Stream.anyMatch(Predicate)Z` — true if any element satisfies predicate.
pub(crate) fn native_stream_any_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied();
    let Slot::Reference(Some(pred_ref)) = pred_slot.unwrap_or(Slot::Reference(None)) else {
        return Ok(Some(Slot::Int(0)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for elem in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(pred_ref)), elem],
        )?;
        if matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `Stream.allMatch(Predicate)Z` — true if all elements satisfy predicate.
pub(crate) fn native_stream_all_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied();
    let Slot::Reference(Some(pred_ref)) = pred_slot.unwrap_or(Slot::Reference(None)) else {
        return Ok(Some(Slot::Int(1)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for elem in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(pred_ref)), elem],
        )?;
        if !matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `Stream.noneMatch(Predicate)Z` — true if no element satisfies predicate.
pub(crate) fn native_stream_none_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied();
    let Slot::Reference(Some(pred_ref)) = pred_slot.unwrap_or(Slot::Reference(None)) else {
        return Ok(Some(Slot::Int(1)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for elem in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(pred_ref)), elem],
        )?;
        if matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `Stream.findFirst()Optional` — returns Optional of first element, or empty.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_find_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let opt_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if size > 0 {
        let first = heap.get(stream_ref)?.fields[1];
        heap.get_mut(opt_ref)?.fields[0] = first;
    } else {
        heap.get_mut(opt_ref)?.fields[0] = Slot::Reference(None);
    }
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `Stream.reduce(BinaryOperator)Optional` — folds elements left via binary op.
pub(crate) fn native_stream_reduce(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let op_slot = args.get(1).copied();
    let Slot::Reference(Some(op_ref)) = op_slot.unwrap_or(Slot::Reference(None)) else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let op_class = heap.get(op_ref)?.class_name.clone();
    let reduce_result_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if elems.is_empty() {
        heap.get_mut(reduce_result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(reduce_result_ref))));
    }
    let mut acc = elems[0];
    for elem in elems.into_iter().skip(1) {
        let result = ops.invoke(
            heap,
            out,
            &op_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![Slot::Reference(Some(op_ref)), acc, elem],
        )?;
        acc = result.unwrap_or(Slot::Reference(None));
    }
    heap.get_mut(reduce_result_ref)?.fields[0] = acc;
    Ok(Some(Slot::Reference(Some(reduce_result_ref))))
}

/// Native: `Stream.reduce(identity, BinaryOperator)Object` — fold with initial value.
pub(crate) fn native_stream_reduce_with_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let identity = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let fn_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(identity));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut acc = identity;
    for elem in elems {
        acc = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                vec![fn_slot, acc, elem],
            )?
            .unwrap_or(Slot::Reference(None));
    }
    Ok(Some(acc))
}

/// Native: `Stream.toList()List` — terminal op returning an unmodifiable list (same as collect).
pub(crate) fn native_stream_to_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    native_stream_collect(args, heap, out, control, ops)
}

/// Helper: allocate a `JoiningCollector` with 3 fields: delimiter, prefix, suffix.
fn make_joining_collector(
    heap: &mut duke_gc::Heap,
    delimiter: &str,
    prefix: &str,
    suffix: &str,
) -> u64 {
    let collector_ref = heap.allocate("duke/util/JoiningCollector".to_string(), 3);
    let delim_ref = heap.allocate_string(delimiter.to_string());
    let prefix_ref = heap.allocate_string(prefix.to_string());
    let suffix_ref = heap.allocate_string(suffix.to_string());
    let obj = heap.get_mut(collector_ref).expect("just allocated");
    obj.fields[0] = Slot::Reference(Some(delim_ref));
    obj.fields[1] = Slot::Reference(Some(prefix_ref));
    obj.fields[2] = Slot::Reference(Some(suffix_ref));
    collector_ref
}

/// Native: `Collectors.joining(delim)Collector` — returns a joining collector with delimiter.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_joining(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let delim = match args.first() {
        Some(Slot::Reference(Some(r))) => heap
            .get(*r)
            .ok()
            .and_then(|o| o.string_value.clone())
            .unwrap_or_default(),
        _ => String::new(),
    };
    let collector_ref = make_joining_collector(heap, &delim, "", "");
    Ok(Some(Slot::Reference(Some(collector_ref))))
}

/// Native: `Collectors.joining()Collector` — no-arg version (empty delimiter).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_joining_no_arg(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let collector_ref = make_joining_collector(heap, "", "", "");
    Ok(Some(Slot::Reference(Some(collector_ref))))
}

/// Native: `Collectors.joining(delim, prefix, suffix)Collector` — full 3-arg joining collector.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_joining_full(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let read_str = |heap: &duke_gc::Heap, idx: usize| -> String {
        match args.get(idx) {
            Some(Slot::Reference(Some(r))) => heap
                .get(*r)
                .ok()
                .and_then(|o| o.string_value.clone())
                .unwrap_or_default(),
            _ => String::new(),
        }
    };
    let delim = read_str(heap, 0);
    let prefix = read_str(heap, 1);
    let suffix = read_str(heap, 2);
    let collector_ref = make_joining_collector(heap, &delim, &prefix, &suffix);
    Ok(Some(Slot::Reference(Some(collector_ref))))
}

/// Native: `Collectors.toList()Collector` — returns a sentinel collector object.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let collector_ref = heap.allocate("duke/util/ToListCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(collector_ref))))
}

/// Native: `Collectors.counting()Collector` — returns a counting collector sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_counting(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("duke/util/CountingCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.groupingBy(Function)Collector` — returns a grouping-by collector.
/// Stores `fn_slot` in `fields[0]`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_grouping_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/GroupingByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Stream.peek(Consumer)Stream` — side-effect each element, returns same stream.
pub(crate) fn native_stream_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(Some(Slot::Reference(Some(stream_ref))));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for elem in &elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, *elem],
        )?;
    }
    // Return a new stream with same elements (consumer may have GC'd things)
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(elems.len()).unwrap_or(0));
    for elem in elems {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `Stream.toArray()Object[]` — materializes stream into an Object array.
pub(crate) fn native_stream_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    // Arrays use fields directly (no length header); arraylength returns fields.len().
    let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 0);
    for elem in elems {
        heap.get_mut(arr_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

// ---------------------------------------------------------------------------
// Stream.limit / Stream.skip / Stream.flatMap
// ---------------------------------------------------------------------------

/// Native: `Stream.limit(long)Stream` — keeps first N elements.
pub(crate) fn native_stream_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let max_size = match args.get(1).copied() {
        Some(Slot::Long(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        _ => 0,
    };
    let class_name = heap.get(stream_ref)?.class_name.clone();

    // Lazy generators: materialise N elements on limit().
    if class_name == "duke/util/GeneratorStream" {
        let supplier_slot = heap
            .get(stream_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(sup_ref)) = supplier_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let sup_class = heap.get(sup_ref)?.class_name.clone();
        let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(max_size).unwrap_or(0));
        for _ in 0..max_size {
            let elem = ops
                .invoke(
                    heap,
                    out,
                    &sup_class,
                    "get",
                    "()Ljava/lang/Object;",
                    vec![Slot::Reference(Some(sup_ref))],
                )?
                .unwrap_or(Slot::Reference(None));
            heap.get_mut(out_ref)?.fields.push(elem);
        }
        return Ok(Some(Slot::Reference(Some(out_ref))));
    }

    if class_name == "duke/util/IteratorStream" {
        // fields[0] = current seed, fields[1] = UnaryOperator fn
        let seed = heap
            .get(stream_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        let fn_slot = heap
            .get(stream_ref)?
            .fields
            .get(1)
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(max_size).unwrap_or(0));
        let mut current = seed;
        for _ in 0..max_size {
            heap.get_mut(out_ref)?.fields.push(current);
            current = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![fn_slot, current],
                )?
                .unwrap_or(Slot::Reference(None));
        }
        return Ok(Some(Slot::Reference(Some(out_ref))));
    }

    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let take = size.min(max_size);
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(take).unwrap_or(0));
    for elem in elems.into_iter().take(take) {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `Stream.skip(long)Stream` — skips first N elements.
pub(crate) fn native_stream_skip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let skip_n = match args.get(1).copied() {
        Some(Slot::Long(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        _ => 0,
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let skipped: Vec<Slot> = elems.into_iter().skip(skip_n).collect();
    let new_size = i32::try_from(skipped.len()).unwrap_or(0);
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(new_size);
    for elem in skipped {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `Stream.flatMap(Function)Stream` — maps each element to a Stream and flattens.
pub(crate) fn native_stream_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut flat: Vec<Slot> = Vec::new();
    for elem in elems {
        let inner = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, elem],
        )?;
        if let Some(Slot::Reference(Some(inner_ref))) = inner {
            // inner should be a duke/util/Stream — flatten its elements
            let inner_size = match heap.get(inner_ref)?.fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let inner_elems: Vec<Slot> = heap.get(inner_ref)?.fields[1..=inner_size].to_vec();
            flat.extend(inner_elems);
        }
    }
    let new_size = i32::try_from(flat.len()).unwrap_or(0);
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(new_size);
    for elem in flat {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

// ---------------------------------------------------------------------------
// IntStream — duke/util/IntStream
// fields[0]=Int(size), fields[1..n]=Int(value) elements (unboxed ints)
// ---------------------------------------------------------------------------

/// Build an `IntStream` heap object from a `Vec<i32>`.
fn make_int_stream(heap: &mut duke_gc::Heap, values: Vec<i32>) -> u64 {
    let n = i32::try_from(values.len()).unwrap_or(0);
    let r = heap.allocate("duke/util/IntStream".to_string(), 1);
    heap.get_mut(r).expect("fresh").fields[0] = Slot::Int(n);
    for v in values {
        heap.get_mut(r).expect("fresh").fields.push(Slot::Int(v));
    }
    r
}

/// Create an `OptionalInt` heap object. `None` = empty, `Some(v)` = present.
fn make_optional_int(heap: &mut duke_gc::Heap, value: Option<i32>) -> u64 {
    let r = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    if let Some(v) = value {
        heap.get_mut(r).expect("fresh").fields[0] = Slot::Int(v);
        heap.get_mut(r).expect("fresh").fields[1] = Slot::Int(1);
    } else {
        heap.get_mut(r).expect("fresh").fields[1] = Slot::Int(0);
    }
    r
}

/// Create a `java/util/Optional` heap object. `None` = empty, `Some(slot)` = present.
fn make_optional(heap: &mut duke_gc::Heap, value: Option<Slot>) -> u64 {
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r).expect("fresh").fields[0] = value.unwrap_or(Slot::Reference(None));
    r
}

/// Extract int elements from an `IntStream` heap object.
fn int_stream_elems(heap: &duke_gc::Heap, ref_: u64) -> Vec<i32> {
    let size = match heap.get(ref_).ok().and_then(|o| o.fields.first().copied()) {
        Some(Slot::Int(n)) => usize::try_from(n).unwrap_or(0),
        _ => 0,
    };
    heap.get(ref_)
        .ok()
        .map(|o| {
            o.fields[1..=size]
                .iter()
                .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
                .collect()
        })
        .unwrap_or_default()
}

/// Native: `IntStream.range(int,int)IntStream` — half-open range [start, end).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let start = match args.first().copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    let end = match args.get(1).copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    let values: Vec<i32> = (start..end).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

/// Native: `IntStream.rangeClosed(int,int)IntStream` — inclusive range [start, end].
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_range_closed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let start = match args.first().copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    let end = match args.get(1).copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    let values: Vec<i32> = (start..=end).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

/// Native: `IntStream.of(int...)IntStream` — from an int array argument.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // args[0] is the int[] array ref
    let arr_ref = extract_ref_arg(args, 0)?;
    let values: Vec<i32> = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
        .collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

/// Native: `IntStream.count()J`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match heap.get(r)?.fields.first() {
        Some(Slot::Int(n)) => i64::from(*n),
        _ => 0,
    };
    Ok(Some(Slot::Long(n)))
}

/// Native: `IntStream.sum()I`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_sum(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let sum: i32 = int_stream_elems(heap, r)
        .iter()
        .copied()
        .fold(0_i32, i32::wrapping_add);
    Ok(Some(Slot::Int(sum)))
}

/// Native: `IntStream.min()OptionalInt`
pub(crate) fn native_int_stream_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    // OptionalInt: fields[0]=Int(value), fields[1]=Int(1=present/0=empty)
    let opt_ref = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    if let Some(&v) = elems.iter().min() {
        heap.get_mut(opt_ref)?.fields[0] = Slot::Int(v);
        heap.get_mut(opt_ref)?.fields[1] = Slot::Int(1);
    } else {
        heap.get_mut(opt_ref)?.fields[1] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `IntStream.max()OptionalInt`
pub(crate) fn native_int_stream_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let opt_ref = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    if let Some(&v) = elems.iter().max() {
        heap.get_mut(opt_ref)?.fields[0] = Slot::Int(v);
        heap.get_mut(opt_ref)?.fields[1] = Slot::Int(1);
    } else {
        heap.get_mut(opt_ref)?.fields[1] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `IntStream.average()OptionalDouble`
pub(crate) fn native_int_stream_average(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    // OptionalDouble: fields[0]=Double(value), fields[1]=Int(1=present/0=empty)
    let opt_ref = heap.allocate("duke/util/OptionalDouble".to_string(), 2);
    if elems.is_empty() {
        heap.get_mut(opt_ref)?.fields[1] = Slot::Int(0);
    } else {
        let sum: i64 = elems.iter().map(|&n| i64::from(n)).sum();
        #[allow(clippy::cast_precision_loss)]
        let avg = sum as f64 / elems.len() as f64;
        heap.get_mut(opt_ref)?.fields[0] = Slot::Double(avg);
        heap.get_mut(opt_ref)?.fields[1] = Slot::Int(1);
    }
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `IntStream.toArray()int[]`
pub(crate) fn native_int_stream_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let arr_ref = heap.allocate("[I".to_string(), elems.len());
    for (i, v) in elems.into_iter().enumerate() {
        heap.get_mut(arr_ref)?.fields[i] = Slot::Int(v);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `IntStream.filter(IntPredicate)IntStream`
pub(crate) fn native_int_stream_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::new();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(I)Z",
            vec![pred_slot, Slot::Int(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, kept)))))
}

/// Native: `IntStream.map(IntUnaryOperator)IntStream`
pub(crate) fn native_int_stream_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::new();
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(I)I",
            vec![fn_slot, Slot::Int(v)],
        )?;
        result.push(match r {
            Some(Slot::Int(n)) => n,
            _ => 0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}

/// Native: `IntStream.forEach(IntConsumer)V`
pub(crate) fn native_int_stream_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let consumer_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(None);
    };
    let elems = int_stream_elems(heap, r);
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for v in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(I)V",
            vec![consumer_slot, Slot::Int(v)],
        )?;
    }
    Ok(None)
}

/// Native: `IntStream.boxed()Stream` — wraps each int into `java/lang/Integer`.
pub(crate) fn native_int_stream_boxed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for v in elems {
        let boxed_ref = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(v);
        heap.get_mut(stream_ref)?
            .fields
            .push(Slot::Reference(Some(boxed_ref)));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `IntStream.mapToObj(IntFunction)Stream` — maps ints to objects.
pub(crate) fn native_int_stream_map_to_obj(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut mapped = Vec::new();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(I)Ljava/lang/Object;",
            vec![fn_slot, Slot::Int(v)],
        )?;
        mapped.push(result.unwrap_or(Slot::Reference(None)));
    }
    let new_size = i32::try_from(mapped.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(new_size);
    for elem in mapped {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `IntStream.distinct()IntStream` — removes duplicate int values.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_distinct(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let mut seen: Vec<i32> = Vec::new();
    for v in elems {
        if !seen.contains(&v) {
            seen.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, seen)))))
}

/// Native: `String.<init>(String)V` — copy constructor: copies `string_value` from source.
pub(crate) fn native_string_init_copy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_val = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = src_val;
    Ok(None)
}

/// Native: `OptionalInt.getAsInt()I`
pub(crate) fn native_optional_int_get_as_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let v = match heap.get(r)?.fields.first().copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}

/// Native: `OptionalInt.isPresent()Z`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_int_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}

/// Native: `OptionalDouble.getAsDouble()D`
pub(crate) fn native_optional_double_get_as_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let v = match heap.get(r)?.fields.first().copied() {
        Some(Slot::Double(d)) => d,
        _ => 0.0,
    };
    Ok(Some(Slot::Double(v)))
}

// ---- ArrayDeque natives ----
// fields[0]=Int(size), fields[1..size]=elements (front at index 1)

/// Native: `ArrayDeque.<init>()V` — same layout as `ArrayList`.
pub(crate) fn native_arraydeque_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `ArrayDeque.push(Object)V` — push to front (stack: LIFO).
pub(crate) fn native_arraydeque_push(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    heap.get_mut(this_ref)?.fields.insert(1, elem);
    let new_size = i32::try_from(size + 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(None)
}

/// Native: `ArrayDeque.pop()Object` — pop from front (stack: LIFO).
pub(crate) fn native_arraydeque_pop(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let elem = heap.get_mut(this_ref)?.fields.remove(1);
    let new_size = i32::try_from(size - 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(Some(elem))
}

/// Native: `ArrayDeque.offer(Object)Z` — enqueue at back (queue: FIFO).
pub(crate) fn native_arraydeque_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(Slot::Int(1))) // always succeeds
}

/// Native: `ArrayDeque.add(Object)Z` — same as offer (appends to back).
pub(crate) fn native_arraydeque_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `ArrayDeque.poll()Object` — dequeue from front (queue: FIFO); null if empty.
pub(crate) fn native_arraydeque_poll(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    let elem = heap.get_mut(this_ref)?.fields.remove(1);
    let new_size = i32::try_from(size - 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(Some(elem))
}

/// Native: `ArrayDeque.peek()Object` — peek at front; null if empty.
pub(crate) fn native_arraydeque_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `ArrayDeque.size()I`
pub(crate) fn native_arraydeque_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `ArrayDeque.isEmpty()Z`
pub(crate) fn native_arraydeque_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

// ---- PriorityQueue natives ----
// fields[0]=Int(size), fields[1..size]=elements; maintained as a min-heap (String key order for Strings, value for boxed ints).

/// Native: `PriorityQueue.<init>()V` — same init as `ArrayList`.
pub(crate) fn native_priorityqueue_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `PriorityQueue.offer(Object)Z` — inserts in heap order via `compareTo`.
pub(crate) fn native_priorityqueue_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    // Append, then sift up.
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    heap.get_mut(this_ref)?.fields.push(elem);
    let new_size = size + 1;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(new_size).unwrap_or(0));
    // Sift up from last position (1-indexed in fields).
    let mut i = new_size; // fields index of newly added element
    while i > 1 {
        // fields are 1-indexed: child at fields[i], parent at fields[i/2]
        let parent_idx = i / 2;
        let child_slot = heap.get(this_ref)?.fields[i];
        let parent_slot = heap.get(this_ref)?.fields[parent_idx];
        let (Slot::Reference(Some(child_ref)), Slot::Reference(Some(parent_ref))) =
            (child_slot, parent_slot)
        else {
            break;
        };
        let class_child = heap.get(child_ref)?.class_name.clone();
        let cmp = ops.invoke(
            heap,
            out,
            &class_child,
            "compareTo",
            "(Ljava/lang/Object;)I",
            vec![
                Slot::Reference(Some(child_ref)),
                Slot::Reference(Some(parent_ref)),
            ],
        )?;
        if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
            // child < parent: swap
            heap.get_mut(this_ref)?.fields.swap(i, parent_idx);
            i = parent_idx;
        } else {
            break;
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `PriorityQueue.add(Object)Z` — same as offer.
pub(crate) fn native_priorityqueue_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    native_priorityqueue_offer(args, heap, out, control, ops)
}

/// Native: `PriorityQueue.peek()Object` — returns minimum element without removing.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_priorityqueue_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `PriorityQueue.poll()Object` — removes and returns minimum; sifts down.
pub(crate) fn native_priorityqueue_poll(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    let min = heap.get(this_ref)?.fields[1];
    if size == 1 {
        heap.get_mut(this_ref)?.fields.pop();
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
        return Ok(Some(min));
    }
    // Move last element to root, sift down.
    let last = heap.get(this_ref)?.fields[size];
    heap.get_mut(this_ref)?.fields[1] = last;
    heap.get_mut(this_ref)?.fields.pop();
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    let new_size = size - 1;
    let mut i = 1usize;
    loop {
        let left = 2 * i;
        let right = 2 * i + 1;
        let mut smallest = i;
        if left <= new_size {
            let (cur_slot, left_slot) = (
                heap.get(this_ref)?.fields[smallest],
                heap.get(this_ref)?.fields[left],
            );
            let (Slot::Reference(Some(cur_ref)), Slot::Reference(Some(left_ref))) =
                (cur_slot, left_slot)
            else {
                break;
            };
            let class_left = heap.get(left_ref)?.class_name.clone();
            let cmp = ops.invoke(
                heap,
                out,
                &class_left,
                "compareTo",
                "(Ljava/lang/Object;)I",
                vec![
                    Slot::Reference(Some(left_ref)),
                    Slot::Reference(Some(cur_ref)),
                ],
            )?;
            if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
                smallest = left;
            }
        }
        if right <= new_size {
            let (small_slot, right_slot) = (
                heap.get(this_ref)?.fields[smallest],
                heap.get(this_ref)?.fields[right],
            );
            let (Slot::Reference(Some(small_ref)), Slot::Reference(Some(right_ref))) =
                (small_slot, right_slot)
            else {
                break;
            };
            let class_right = heap.get(right_ref)?.class_name.clone();
            let cmp = ops.invoke(
                heap,
                out,
                &class_right,
                "compareTo",
                "(Ljava/lang/Object;)I",
                vec![
                    Slot::Reference(Some(right_ref)),
                    Slot::Reference(Some(small_ref)),
                ],
            )?;
            if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
                smallest = right;
            }
        }
        if smallest == i {
            break;
        }
        heap.get_mut(this_ref)?.fields.swap(i, smallest);
        i = smallest;
    }
    Ok(Some(min))
}

/// Native: `PriorityQueue.size()I`
pub(crate) fn native_priorityqueue_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `PriorityQueue.isEmpty()Z`
pub(crate) fn native_priorityqueue_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

// ---- Comparator natives ----

/// Native: `Comparator.naturalOrder()Comparator` — returns a singleton synthetic comparator.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_natural_order(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("duke/util/NaturalOrderComparator".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Comparator.reverseOrder()Comparator` — returns a singleton reverse comparator.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_reverse_order(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("duke/util/ReverseOrderComparator".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `NaturalOrderComparator.compare(O,O)I` — delegates to `o1.compareTo(o2)`.
pub(crate) fn native_natural_order_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // args: [this, o1, o2]
    let o1 = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let o2 = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let o1_class = match &o1 {
        Slot::Reference(Some(r)) => heap.get(*r)?.class_name.clone(),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let result = ops.invoke(
        heap,
        out,
        &o1_class,
        "compareTo",
        "(Ljava/lang/Object;)I",
        vec![o1, o2],
    )?;
    let _ = control;
    Ok(Some(result.unwrap_or(Slot::Int(0))))
}

/// Native: `ReverseOrderComparator.compare(O,O)I` — negates natural order.
pub(crate) fn native_reverse_order_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let result = native_natural_order_compare(args, heap, out, control, ops)?;
    Ok(Some(match result {
        Some(Slot::Int(v)) => Slot::Int(-v),
        other => other.unwrap_or(Slot::Int(0)),
    }))
}

/// Native: `Comparator.comparingInt(ToIntFunction)Comparator` — wraps key extractor.
/// Creates a `duke/util/ComparingIntComparator` with `fields[0] = fn_ref`.
pub(crate) fn native_comparator_comparing_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_ref = extract_ref_arg(args, 0)?;
    let r = heap.allocate("duke/util/ComparingIntComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(Some(fn_ref));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ComparingIntComparator.compare(O,O)I` — calls `fn.applyAsInt(o)` for each element.
pub(crate) fn native_comparing_int_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(Ljava/lang/Object;)I",
            vec![Slot::Reference(Some(fn_ref)), a],
        )?
        .unwrap_or(Slot::Int(0));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(Ljava/lang/Object;)I",
            vec![Slot::Reference(Some(fn_ref)), b],
        )?
        .unwrap_or(Slot::Int(0));
    let result = match (ka, kb) {
        (Slot::Int(ia), Slot::Int(ib)) => ia.cmp(&ib) as i32,
        _ => 0,
    };
    let _ = control;
    Ok(Some(Slot::Int(result)))
}

/// Native: `Collections.sort(List, Comparator)V` — 2-arg sort with explicit comparator.
pub(crate) fn native_collections_sort_with_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let comparator = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let class_name = heap.get(list_ref)?.class_name.clone();
    ops.invoke(
        heap,
        output,
        &class_name,
        "sort",
        "(Ljava/util/Comparator;)V",
        vec![Slot::Reference(Some(list_ref)), comparator],
    )?;
    Ok(None)
}

/// Native: `Enum.<init>(Ljava/lang/String;I)V` — stores name + ordinal.
/// args: `[this_ref, name_ref, ordinal_int]`
pub(crate) fn native_enum_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let name_slot = extract_slot_arg(args, 1);
    let ordinal = match args.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() >= 2 {
        obj.fields[0] = name_slot;
        obj.fields[1] = Slot::Int(ordinal);
    }
    Ok(None)
}

/// Native: `Enum.ordinal()I`
pub(crate) fn native_enum_ordinal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.get(1) {
        Some(Slot::Int(v)) => Ok(Some(Slot::Int(*v))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `Enum.name()Ljava/lang/String;`
pub(crate) fn native_enum_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Enum.valueOf(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;`
/// Searches heap for enum constants of the given class matching the name.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_enum_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let target_name = heap.get(name_ref)?.string_value.clone().unwrap_or_default();
    let enum_class_name = heap
        .get(class_ref)?
        .string_value
        .clone()
        .unwrap_or_default();

    let obj_count = heap.len();
    for i in 0..obj_count {
        let obj = heap.get(i as u64)?;
        if obj.class_name == enum_class_name
            && obj.fields.len() >= 2
            && let Some(Slot::Reference(Some(name_r))) = obj.fields.first()
            && let Ok(name_obj) = heap.get(*name_r)
            && name_obj.string_value.as_deref() == Some(target_name.as_str())
        {
            return Ok(Some(Slot::Reference(Some(i as u64))));
        }
    }

    Err(VmError::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Reflection natives
// ---------------------------------------------------------------------------

pub(crate) fn native_class_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let name_ref = heap.allocate_string(internal_name_to_binary_name(&internal_name));
    Ok(Some(Slot::Reference(Some(name_ref))))
}

pub(crate) fn native_class_get_package_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let package_name = if internal_name.starts_with('[') {
        String::new()
    } else {
        internal_name
            .rsplit_once('/')
            .map_or_else(String::new, |(package, _)| package.replace('/', "."))
    };
    let package_ref = heap.allocate_string(package_name);
    Ok(Some(Slot::Reference(Some(package_ref))))
}

/// Native: `Class.desiredAssertionStatus()` - Duke currently runs with assertions disabled.
pub(crate) fn native_class_desired_assertion_status(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(0)))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_false_boolean(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(Slot::Int(0)))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_zero_long(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(Slot::Long(0)))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_void_noop(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

pub(crate) fn native_class_for_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let binary_name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)?;
    let internal_name = binary_name_to_internal_name(&binary_name);
    match ops.ensure_loaded(&internal_name) {
        Ok(()) => {
            let class_key = ops.class_key_for_loaded_class(&internal_name)?;
            let class_ref = allocate_class_object(heap, &class_key)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        Err(VmError::ClassNotFound { .. }) => Err(VmError::JavaException {
            class_name: "java/lang/ClassNotFoundException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_class_for_name_with_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let binary_name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)?;
    let internal_name = binary_name_to_internal_name(&binary_name);
    let (load_result, class_key) = match args.get(2) {
        Some(Slot::Reference(Some(loader_ref))) => (
            ops.ensure_loaded_with_runtime_loader(heap, *loader_ref, &internal_name),
            ops.class_key_for_runtime_loader(heap, *loader_ref, &internal_name)?,
        ),
        Some(Slot::Reference(None)) | None => (
            ops.ensure_loaded(&internal_name),
            ops.class_key_for_loaded_class(&internal_name)?,
        ),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    match load_result {
        Ok(()) => {
            let class_ref = allocate_class_object(heap, &class_key)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        Err(VmError::ClassNotFound { .. }) => Err(VmError::JavaException {
            class_name: "java/lang/ClassNotFoundException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_class_get_class_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    Ok(Some(Slot::Reference(
        ops.runtime_loader_for_class(&class_key)?,
    )))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_class_loader_register_as_parallel_capable(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(Slot::Int(1)))
}

fn allocate_string_backed_object(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    value: String,
) -> VmResult<u64> {
    let obj_ref = heap.allocate(class_name.to_string(), 1);
    let value_ref = heap.allocate_string(value);
    heap.get_mut(obj_ref)?.fields[0] = Slot::Reference(Some(value_ref));
    Ok(obj_ref)
}

fn first_reference_field(heap: &duke_gc::Heap, obj_ref: u64) -> VmResult<Option<u64>> {
    match heap.get(obj_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => Ok(Some(*r)),
        Some(Slot::Reference(None)) => Ok(None),
        _ => Err(VmError::NullPointerException),
    }
}

fn string_backed_object_value(heap: &duke_gc::Heap, obj_ref: u64) -> VmResult<String> {
    let Some(value_ref) = first_reference_field(heap, obj_ref)? else {
        return Err(VmError::NullPointerException);
    };
    string_value_from_ref(heap, value_ref)
}

fn path_to_file_url(path: &std::path::Path) -> String {
    let mut normalized = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        if let Some(stripped) = normalized.strip_prefix("//?/UNC/") {
            normalized = format!("//{stripped}");
        } else if let Some(stripped) = normalized.strip_prefix("//?/") {
            normalized = stripped.to_string();
        }
    }
    if cfg!(windows) && !normalized.starts_with('/') {
        normalized.insert(0, '/');
    }
    format!("file://{normalized}")
}

const fn decode_pct_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) =
                (decode_pct_hex(bytes[i + 1]), decode_pct_hex(bytes[i + 2]))
        {
            out.push(char::from((hi << 4) | lo));
            i += 3;
            continue;
        }
        out.push(char::from(bytes[i]));
        i += 1;
    }
    out
}

fn file_url_to_path(url: &str) -> VmResult<std::path::PathBuf> {
    let Some(rest) = url.strip_prefix("file://") else {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalArgumentException".to_string(),
        });
    };
    let decoded = percent_decode(rest);
    if cfg!(windows) {
        let trimmed = decoded.strip_prefix('/').unwrap_or(decoded.as_str());
        Ok(std::path::PathBuf::from(trimmed.replace('/', "\\")))
    } else {
        Ok(std::path::PathBuf::from(decoded))
    }
}

pub(crate) fn native_class_get_protection_domain(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let pd_ref = heap.allocate("java/security/ProtectionDomain".to_string(), 1);
    let code_source_slot = if let Some(path) = ops.code_source_for_class(&class_key)? {
        let url_ref = allocate_string_backed_object(
            heap,
            "java/net/URL",
            path_to_file_url(std::path::Path::new(&path)),
        )?;
        let code_source_ref = heap.allocate("java/security/CodeSource".to_string(), 1);
        heap.get_mut(code_source_ref)?.fields[0] = Slot::Reference(Some(url_ref));
        Slot::Reference(Some(code_source_ref))
    } else {
        Slot::Reference(None)
    };
    heap.get_mut(pd_ref)?.fields[0] = code_source_slot;
    Ok(Some(Slot::Reference(Some(pd_ref))))
}

pub(crate) fn native_protection_domain_get_code_source(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Err(VmError::NullPointerException),
    }
}

pub(crate) fn native_code_source_get_location(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Err(VmError::NullPointerException),
    }
}

pub(crate) fn native_url_to_uri(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let spec = string_backed_object_value(heap, this_ref)?;
    let uri_ref = allocate_string_backed_object(heap, "java/net/URI", spec)?;
    Ok(Some(Slot::Reference(Some(uri_ref))))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_url_set_url_stream_handler_factory(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

pub(crate) fn native_path_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let uri_ref = extract_ref_arg(args, 0)?;
    let uri = string_backed_object_value(heap, uri_ref)?;
    let path = file_url_to_path(&uri)?;
    let path_ref = allocate_string_backed_object(
        heap,
        "java/nio/file/Path",
        path.to_string_lossy().to_string(),
    )?;
    Ok(Some(Slot::Reference(Some(path_ref))))
}

pub(crate) fn native_path_to_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .ok_or(VmError::NullPointerException)?;
    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    heap.get_mut(file_ref)?.fields[0] = path_slot;
    Ok(Some(Slot::Reference(Some(file_ref))))
}

pub(crate) fn native_paths_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let first_ref = extract_ref_arg(args, 0)?;
    let mut path = std::path::PathBuf::from(string_value_from_ref(heap, first_ref)?);
    let more_slot = extract_slot_arg(args, 1);
    match more_slot {
        Slot::Reference(Some(array_ref)) => {
            let segments = heap.get(array_ref)?.fields.clone();
            for segment in segments {
                let Slot::Reference(Some(segment_ref)) = segment else {
                    return Err(VmError::NullPointerException);
                };
                path.push(string_value_from_ref(heap, segment_ref)?);
            }
        }
        Slot::Reference(None) => {}
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    }
    let path_ref = allocate_string_backed_object(
        heap,
        "java/nio/file/Path",
        path.to_string_lossy().into_owned(),
    )?;
    Ok(Some(Slot::Reference(Some(path_ref))))
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_posix_file_permissions_as_file_attribute(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let attribute_ref = heap.allocate("java/nio/file/attribute/FileAttribute".to_string(), 0);
    Ok(Some(Slot::Reference(Some(attribute_ref))))
}

fn manifest_attribute_value(manifest_bytes: &[u8], key: &str) -> Option<String> {
    let text = std::str::from_utf8(manifest_bytes).ok()?;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix(key)
            && let Some(value) = value.strip_prefix(':')
        {
            return Some(value.trim().to_string());
        }
    }
    None
}

pub(crate) fn native_manifest_get_main_attributes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let manifest_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Reference(Some(raw_ref))) = heap.get(manifest_ref)?.fields.first().copied()
    else {
        return Err(VmError::NullPointerException);
    };
    let attributes_ref = heap.allocate("java/util/jar/Attributes".to_string(), 1);
    heap.get_mut(attributes_ref)?.fields[0] = Slot::Reference(Some(raw_ref));
    Ok(Some(Slot::Reference(Some(attributes_ref))))
}

pub(crate) fn native_attributes_get_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let attributes_ref = extract_ref_arg(args, 0)?;
    let key_ref = extract_ref_arg(args, 1)?;
    let Some(Slot::Reference(Some(raw_ref))) = heap.get(attributes_ref)?.fields.first().copied()
    else {
        return Err(VmError::NullPointerException);
    };
    let manifest_text = string_value_from_ref(heap, raw_ref)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let result = manifest_attribute_value(manifest_text.as_bytes(), &key)
        .map_or(Slot::Reference(None), |value| {
            Slot::Reference(Some(heap.allocate_string(value)))
        });
    Ok(Some(result))
}

const BOOT_ARCHIVE_ENTRY_NAME_SLOT: usize = 0;
const BOOT_ARCHIVE_ENTRY_DIRECTORY_SLOT: usize = 1;

#[cfg(test)]
fn boot_archive_entry_name(heap: &duke_gc::Heap, entry_ref: u64) -> VmResult<String> {
    let Some(Slot::Reference(Some(name_ref))) = heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_NAME_SLOT)
        .copied()
    else {
        return Err(VmError::NullPointerException);
    };
    string_value_from_ref(heap, name_ref)
}

fn boot_archive_entry_is_directory_flag(heap: &duke_gc::Heap, entry_ref: u64) -> VmResult<bool> {
    match heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_DIRECTORY_SLOT)
        .copied()
    {
        Some(Slot::Int(value)) => Ok(value != 0),
        _ => Err(VmError::TypeMismatch {
            expected: "Int",
            got: "other",
        }),
    }
}

pub(crate) fn native_boot_archive_entry_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let entry_ref = extract_ref_arg(args, 0)?;
    let Some(slot @ Slot::Reference(Some(_))) = heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_NAME_SLOT)
        .copied()
    else {
        return Err(VmError::NullPointerException);
    };
    Ok(Some(slot))
}

pub(crate) fn native_boot_archive_entry_is_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let entry_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(
        boot_archive_entry_is_directory_flag(heap, entry_ref)?,
    ))))
}

fn boot_archive_hashset_add_url(
    set_ref: u64,
    url_spec: String,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<()> {
    let url_ref = allocate_string_backed_object(heap, "java/net/URL", url_spec)?;
    native_hashset_add(
        &[
            Slot::Reference(Some(set_ref)),
            Slot::Reference(Some(url_ref)),
        ],
        heap,
        out,
        control,
    )?;
    Ok(())
}

fn allocate_boot_archive_entry(
    heap: &mut duke_gc::Heap,
    entry_name: &str,
    is_directory: bool,
) -> VmResult<u64> {
    let entry_ref = heap.allocate("duke/boot/ArchiveEntry".to_string(), 2);
    let name_ref = heap.allocate_string(entry_name.to_string());
    let entry = heap.get_mut(entry_ref)?;
    entry.fields[BOOT_ARCHIVE_ENTRY_NAME_SLOT] = Slot::Reference(Some(name_ref));
    entry.fields[BOOT_ARCHIVE_ENTRY_DIRECTORY_SLOT] = Slot::Int(i32::from(is_directory));
    Ok(entry_ref)
}

fn boot_archive_predicate_accepts(
    predicate_ref: u64,
    entry_name: &str,
    is_directory: bool,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> VmResult<bool> {
    let predicate_class = heap.get(predicate_ref)?.class_name.clone();
    let entry_ref = allocate_boot_archive_entry(heap, entry_name, is_directory)?;
    match ops.invoke(
        heap,
        out,
        &predicate_class,
        "test",
        "(Ljava/lang/Object;)Z",
        vec![
            Slot::Reference(Some(predicate_ref)),
            Slot::Reference(Some(entry_ref)),
        ],
    )? {
        Some(Slot::Int(value)) => Ok(value != 0),
        Some(other) => Err(VmError::TypeMismatch {
            expected: "Int",
            got: match other {
                Slot::Long(_) => "Long",
                Slot::Float(_) => "Float",
                Slot::Double(_) => "Double",
                Slot::Reference(_) => "Reference",
                Slot::ReturnAddress(_) => "ReturnAddress",
                Slot::Int(_) => unreachable!(),
            },
        }),
        None => Ok(false),
    }
}

fn open_boot_archive_reader(path: &std::path::Path) -> VmResult<duke_loader::ZipReader> {
    duke_loader::ZipReader::open(path).map_err(|err| match err {
        duke_loader::LoadError::Io { .. } => VmError::JavaException {
            class_name: "java/io/IOException".to_string(),
        },
        _ => VmError::JavaException {
            class_name: "java/util/zip/ZipException".to_string(),
        },
    })
}

fn archive_file_ref_at(heap: &duke_gc::Heap, archive_ref: u64, slot: usize) -> VmResult<u64> {
    match heap.get(archive_ref)?.fields.get(slot).copied() {
        Some(Slot::Reference(Some(file_ref))) => Ok(file_ref),
        Some(Slot::Reference(None)) => Err(VmError::NullPointerException),
        _ => Err(VmError::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

fn patch_forwarded_slot_if_needed(heap: &duke_gc::Heap, slot: &mut Slot) {
    if heap.has_pending_forwards() {
        heap.apply_forward(slot);
    }
}

fn patch_forwarded_ref_if_needed(heap: &duke_gc::Heap, reference: &mut u64) {
    let mut slot = Slot::Reference(Some(*reference));
    patch_forwarded_slot_if_needed(heap, &mut slot);
    if let Slot::Reference(Some(new_ref)) = slot {
        *reference = new_ref;
    }
}

pub(crate) fn native_boot_jar_file_archive_get_class_path_urls(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let archive_ref = extract_ref_arg(args, 0)?;
    let include_predicate_ref = extract_ref_arg(args, 1)?;
    let _ = extract_ref_arg(args, 2)?;
    let archive_file_ref = archive_file_ref_at(heap, archive_ref, 0)?;
    let archive_path = file_path_from_ref(archive_file_ref, heap)?;
    let reader = open_boot_archive_reader(&archive_path)?;
    let mut entry_names: Vec<String> = reader.entry_names().map(ToOwned::to_owned).collect();
    entry_names.sort_unstable();

    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;

    for entry_name in entry_names {
        let is_directory = entry_name.ends_with('/');
        if boot_archive_predicate_accepts(
            include_predicate_ref,
            &entry_name,
            is_directory,
            heap,
            out,
            ops,
        )? {
            let url_spec = format!("jar:{}!/{}", path_to_file_url(&archive_path), entry_name);
            boot_archive_hashset_add_url(set_ref, url_spec, heap, out, control)?;
        }
    }

    Ok(Some(Slot::Reference(Some(set_ref))))
}

fn list_directory_children_sorted(path: &std::path::Path) -> VmResult<Vec<std::path::PathBuf>> {
    let iter = std::fs::read_dir(path).map_err(|_| VmError::JavaException {
        class_name: "java/io/IOException".to_string(),
    })?;
    let mut children = Vec::new();
    for entry in iter {
        let entry = entry.map_err(|_| VmError::JavaException {
            class_name: "java/io/IOException".to_string(),
        })?;
        children.push(entry.path());
    }
    children.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .cmp(right.file_name().unwrap_or_default())
    });
    Ok(children)
}

fn exploded_archive_relative_entry_name(
    root_path: &std::path::Path,
    entry_path: &std::path::Path,
    is_directory: bool,
) -> String {
    let mut relative = entry_path
        .strip_prefix(root_path)
        .unwrap_or(entry_path)
        .to_string_lossy()
        .replace('\\', "/");
    if is_directory && !relative.ends_with('/') {
        relative.push('/');
    }
    relative
}

pub(crate) fn native_boot_exploded_archive_get_class_path_urls(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let archive_ref = extract_ref_arg(args, 0)?;
    let include_predicate_ref = extract_ref_arg(args, 1)?;
    let search_predicate_ref = extract_ref_arg(args, 2)?;
    let root_directory_ref = archive_file_ref_at(heap, archive_ref, 0)?;
    let root_path = file_path_from_ref(root_directory_ref, heap)?;

    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;

    let mut pending: VecDeque<std::path::PathBuf> = list_directory_children_sorted(&root_path)?
        .into_iter()
        .collect();
    while let Some(entry_path) = pending.pop_front() {
        let is_directory = entry_path.is_dir();
        let entry_name =
            exploded_archive_relative_entry_name(&root_path, &entry_path, is_directory);
        if is_directory
            && boot_archive_predicate_accepts(
                search_predicate_ref,
                &entry_name,
                true,
                heap,
                out,
                ops,
            )?
        {
            let children = list_directory_children_sorted(&entry_path)?;
            for child in children.into_iter().rev() {
                pending.push_front(child);
            }
        }

        if boot_archive_predicate_accepts(
            include_predicate_ref,
            &entry_name,
            is_directory,
            heap,
            out,
            ops,
        )? {
            boot_archive_hashset_add_url(
                set_ref,
                path_to_file_url(&entry_path),
                heap,
                out,
                control,
            )?;
        }
    }

    Ok(Some(Slot::Reference(Some(set_ref))))
}

pub(crate) fn native_boot_launched_class_loader_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let exploded = extract_int_arg(args, 1)?;
    let archive_ref = extract_ref_arg(args, 2)?;
    let _ = extract_ref_arg(args, 3)?;
    match args.get(4) {
        Some(Slot::Reference(_)) => {
            let exploded_slot = ops.instance_field_slot(
                "org/springframework/boot/loader/launch/LaunchedClassLoader",
                "exploded",
            )?;
            let root_archive_slot = ops.instance_field_slot(
                "org/springframework/boot/loader/launch/LaunchedClassLoader",
                "rootArchive",
            )?;
            let loader_obj = heap.get_mut(this_ref)?;
            loader_obj.fields[exploded_slot] = Slot::Int(i32::from(exploded != 0));
            loader_obj.fields[root_archive_slot] = Slot::Reference(Some(archive_ref));
            Ok(None)
        }
        _ => Err(VmError::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

pub(crate) fn native_class_get_declared_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let method_name = string_value_from_ref(heap, name_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 2))?;

    let Some(method) = reflected.methods.into_iter().find(|method| {
        method.name == method_name
            && descriptor_parameter_part(&method.descriptor) == parameter_descriptor
    }) else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let method_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Method",
        &class_key,
        &method.name,
        &method.descriptor,
        method.is_public,
        method.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(method_ref))))
}

pub(crate) fn native_class_get_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let method_name = string_value_from_ref(heap, name_ref)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 2))?;

    let Some((declaring_class, method)) =
        lookup_public_reflected_method(ops, &class_key, &method_name, &parameter_descriptor)?
    else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let method_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Method",
        &declaring_class,
        &method.name,
        &method.descriptor,
        method.is_public,
        method.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(method_ref))))
}

fn lookup_reflected_constructor(
    reflected: ReflectedClassInfo,
    parameter_descriptor: &str,
    public_only: bool,
) -> Option<ReflectedMethodInfo> {
    reflected.methods.into_iter().find(|method| {
        method.name == "<init>"
            && (!public_only || method.is_public)
            && descriptor_parameter_part(&method.descriptor) == parameter_descriptor
    })
}

fn reflected_constructors(
    reflected: ReflectedClassInfo,
    public_only: bool,
) -> Vec<ReflectedMethodInfo> {
    reflected
        .methods
        .into_iter()
        .filter(|method| method.name == "<init>" && (!public_only || method.is_public))
        .collect()
}

pub(crate) fn native_class_get_declared_field(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let field_name = string_value_from_ref(heap, name_ref)?;
    let reflected = ops.inspect_class(&class_key)?;

    let Some(field) = reflected
        .fields
        .into_iter()
        .find(|field| field.name == field_name)
    else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NoSuchFieldException".to_string(),
        });
    };

    let field_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Field",
        &class_key,
        &field.name,
        &field.descriptor,
        field.is_public,
        field.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(field_ref))))
}

pub(crate) fn native_class_get_declared_constructor(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 1))?;

    let Some(constructor) = lookup_reflected_constructor(reflected, &parameter_descriptor, false)
    else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let constructor_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Constructor",
        &class_key,
        &constructor.name,
        &constructor.descriptor,
        constructor.is_public,
        constructor.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(constructor_ref))))
}

pub(crate) fn native_class_get_field(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let field_name = string_value_from_ref(heap, name_ref)?;

    let Some((declaring_class, field)) =
        lookup_public_reflected_field(ops, &class_key, &field_name)?
    else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NoSuchFieldException".to_string(),
        });
    };

    let field_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Field",
        &declaring_class,
        &field.name,
        &field.descriptor,
        field.is_public,
        field.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(field_ref))))
}

pub(crate) fn native_class_get_constructor(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 1))?;

    let Some(constructor) = lookup_reflected_constructor(reflected, &parameter_descriptor, true)
    else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let constructor_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Constructor",
        &class_key,
        &constructor.name,
        &constructor.descriptor,
        constructor.is_public,
        constructor.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(constructor_ref))))
}

pub(crate) fn native_class_new_instance(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let constructors = reflected_constructors(reflected, false);
    let Some(constructor) = constructors
        .iter()
        .find(|constructor| constructor.descriptor == "()V")
        .cloned()
    else {
        return Err(VmError::JavaException {
            class_name: "java/lang/InstantiationException".to_string(),
        });
    };
    if !constructor.is_public {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let instance_ref = ops.allocate_instance(heap, output, &class_key)?;
    match ops.invoke(
        heap,
        output,
        &class_key,
        "<init>",
        &constructor.descriptor,
        vec![Slot::Reference(Some(instance_ref))],
    ) {
        Ok(_) => Ok(Some(Slot::Reference(Some(instance_ref)))),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_class_get_declared_methods(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let method_refs = reflected
        .methods
        .into_iter()
        .filter(|method| method.name != "<init>" && method.name != "<clinit>")
        .map(|method| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Method",
                &class_key,
                &method.name,
                &method.descriptor,
                method.is_public,
                method.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Method;", &method_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_declared_constructors(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let constructor_refs = reflected_constructors(reflected, false)
        .into_iter()
        .map(|constructor| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Constructor",
                &class_key,
                &constructor.name,
                &constructor.descriptor,
                constructor.is_public,
                constructor.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref =
        allocate_reference_array(heap, "[Ljava/lang/reflect/Constructor;", &constructor_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_methods(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let method_refs = collect_public_reflected_methods(ops, &class_key)?
        .into_iter()
        .map(|(declaring_class, method)| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Method",
                &declaring_class,
                &method.name,
                &method.descriptor,
                method.is_public,
                method.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Method;", &method_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_constructors(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let constructor_refs = reflected_constructors(reflected, true)
        .into_iter()
        .map(|constructor| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Constructor",
                &class_key,
                &constructor.name,
                &constructor.descriptor,
                constructor.is_public,
                constructor.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref =
        allocate_reference_array(heap, "[Ljava/lang/reflect/Constructor;", &constructor_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_declared_fields(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let field_refs = reflected
        .fields
        .into_iter()
        .map(|field| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Field",
                &class_key,
                &field.name,
                &field.descriptor,
                field.is_public,
                field.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Field;", &field_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_fields(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let field_refs = collect_public_reflected_fields(ops, &class_key)?
        .into_iter()
        .map(|(declaring_class, field)| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Field",
                &declaring_class,
                &field.name,
                &field.descriptor,
                field.is_public,
                field.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Field;", &field_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_reflect_method_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_name_slot(heap, method_ref)?))
}

pub(crate) fn native_reflect_constructor_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let constructor_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_declaring_class_name_slot(
        heap,
        constructor_ref,
    )?))
}

pub(crate) fn native_reflect_method_get_return_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let method = reflected_method_handle(heap, method_ref)?;
    Ok(Some(descriptor_class_slot_from_source(
        heap,
        ops,
        method_return_descriptor(&method.descriptor),
        Some(method.declaring_class_key.as_str()),
    )?))
}

pub(crate) fn native_reflect_field_get_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let field = reflected_field_handle(heap, field_ref)?;
    Ok(Some(descriptor_class_slot_from_source(
        heap,
        ops,
        &field.descriptor,
        Some(field.declaring_class_key.as_str()),
    )?))
}

pub(crate) fn native_reflection_member_get_declaring_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_declaring_class_slot(
        heap, member_ref,
    )?))
}

pub(crate) fn native_reflect_method_get_parameter_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let method = reflected_method_handle(heap, method_ref)?;
    let count = i32::try_from(parse_arg_count(&method.descriptor)).unwrap_or(i32::MAX);
    Ok(Some(Slot::Int(count)))
}

pub(crate) fn native_reflect_executable_get_parameter_types(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    let member = reflected_method_handle(heap, member_ref)?;
    let parameter_descriptors = parse_arg_descriptors(&member.descriptor);
    let mut class_refs = Vec::with_capacity(parameter_descriptors.len());
    for descriptor in parameter_descriptors {
        let Slot::Reference(Some(class_ref)) = descriptor_class_slot_from_source(
            heap,
            ops,
            &descriptor,
            Some(member.declaring_class_key.as_str()),
        )?
        else {
            return Err(VmError::TypeMismatch {
                expected: "class reference",
                got: "other",
            });
        };
        class_refs.push(class_ref);
    }
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/Class;", &class_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_reflect_field_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_name_slot(heap, field_ref)?))
}

pub(crate) fn native_reflection_member_set_accessible(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    let accessible = extract_int_arg(args, 1)? != 0;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_ACCESSIBLE_FIELD,
        Slot::Int(i32::from(accessible)),
    )?;
    Ok(None)
}

pub(crate) fn native_reflect_field_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let field = reflected_field_handle(heap, field_ref)?;

    if !field.is_public && !field.is_accessible {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let field_type = field.descriptor.chars().next().unwrap_or('L');
    let raw_value = if field.is_static {
        ops.ensure_class_initialized(heap, output, &field.declaring_class_key)?;
        ops.read_static_field(&field.declaring_class_key, &field.field_name)?
    } else {
        let Slot::Reference(Some(target_ref)) = target_slot else {
            return Err(VmError::NullPointerException);
        };
        ops.read_instance_field(
            heap,
            target_ref,
            &field.declaring_class_key,
            &field.field_name,
        )?
    };

    Ok(Some(box_reflection_return_value(
        heap,
        field_type,
        Some(raw_value),
    )?))
}

pub(crate) fn native_reflect_field_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let value_slot = extract_slot_arg(args, 2);
    let field = reflected_field_handle(heap, field_ref)?;

    if !field.is_public && !field.is_accessible {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let field_type = field.descriptor.chars().next().unwrap_or('L');
    let value = unbox_reflection_argument(heap, field_type, value_slot)?;

    if field.is_static {
        ops.ensure_class_initialized(heap, output, &field.declaring_class_key)?;
        ops.write_static_field(&field.declaring_class_key, &field.field_name, value)?;
        return Ok(None);
    }

    let Slot::Reference(Some(target_ref)) = target_slot else {
        return Err(VmError::NullPointerException);
    };
    ops.write_instance_field(
        heap,
        target_ref,
        &field.declaring_class_key,
        &field.field_name,
        value,
    )?;
    Ok(None)
}

pub(crate) fn native_reflect_method_invoke(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let invoke_arg_slots = reflection_array_elements(heap, extract_slot_arg(args, 2))?;
    let method = reflected_method_handle(heap, method_ref)?;

    if !method.is_public && !method.is_accessible {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let invoke_args = build_reflection_invoke_args(
        heap,
        target_slot,
        &method.descriptor,
        invoke_arg_slots,
        method.is_static,
    )?;

    ops.ensure_loaded(&method.declaring_class_key)?;
    match ops.invoke(
        heap,
        output,
        &method.declaring_class_key,
        &method.method_name,
        &method.descriptor,
        invoke_args,
    ) {
        Ok(result) => Ok(Some(box_reflection_return_value(
            heap,
            descriptor_return_type(&method.descriptor),
            result,
        )?)),
        Err(VmError::JavaException { .. }) => Err(VmError::JavaException {
            class_name: "java/lang/reflect/InvocationTargetException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_reflect_constructor_new_instance(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let constructor_ref = extract_ref_arg(args, 0)?;
    let invoke_arg_slots = reflection_array_elements(heap, extract_slot_arg(args, 1))?;
    let constructor = reflected_method_handle(heap, constructor_ref)?;

    if !constructor.is_public && !constructor.is_accessible {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let instance_ref = ops.allocate_instance(heap, output, &constructor.declaring_class_key)?;
    let invoke_args = build_reflection_invoke_args(
        heap,
        Slot::Reference(Some(instance_ref)),
        &constructor.descriptor,
        invoke_arg_slots,
        false,
    )?;

    match ops.invoke(
        heap,
        output,
        &constructor.declaring_class_key,
        &constructor.method_name,
        &constructor.descriptor,
        invoke_args,
    ) {
        Ok(_) => Ok(Some(Slot::Reference(Some(instance_ref)))),
        Err(VmError::JavaException { .. }) => Err(VmError::JavaException {
            class_name: "java/lang/reflect/InvocationTargetException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

/// Native: `String.valueOf(int)` — static method, returns string of int.
pub(crate) fn native_string_value_of_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let s = val.to_string();
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `PrintStream.print(String)` — no newline.
pub(crate) fn native_print_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let string_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => {
            write!(out, "null").ok();
            return Ok(None);
        }
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let obj = heap.get(string_ref)?;
    let text = obj.string_value.as_deref().unwrap_or("null");
    write!(out, "{text}").ok();
    Ok(None)
}

/// Native: `PrintStream.print(int)` — no newline.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_print_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Int(v) => *v, "Int");
    write!(out, "{val}").ok();
    Ok(None)
}

// ---------------------------------------------------------------------------
// println overloads (long, float, double, boolean, char, object)
// ---------------------------------------------------------------------------

pub(crate) fn native_println_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Long(v) => *v, "Long");
    writeln!(out, "{val}").ok();
    Ok(None)
}

pub(crate) fn native_println_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Float(v) => *v, "Float");
    writeln!(out, "{val}").ok();
    Ok(None)
}

pub(crate) fn native_println_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Double(v) => *v, "Double");
    writeln!(out, "{val}").ok();
    Ok(None)
}

pub(crate) fn native_println_boolean(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Int(v) => *v != 0, "Int(boolean)");
    writeln!(out, "{val}").ok();
    Ok(None)
}

pub(crate) fn native_println_char(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Int(v) => char::from_u32((*v).cast_unsigned()).unwrap_or('?'), "Int(char)");
    writeln!(out, "{val}").ok();
    Ok(None)
}

#[allow(clippy::cast_possible_wrap)]
/// Convert a heap object to its Java display string.
///
/// Checks `string_value` first (handles String/StringBuilder).
/// For boxed primitives, extracts the stored value from `fields[0]`.
/// Falls back to `class_name@hex_ref` for opaque objects.
fn heap_object_to_string(obj: &duke_gc::HeapObject, obj_ref: u64) -> String {
    if let Some(s) = &obj.string_value {
        return s.clone();
    }
    match obj.class_name.as_str() {
        "java/lang/Integer" => {
            if let Some(Slot::Int(v)) = obj.fields.first() {
                return v.to_string();
            }
        }
        "java/lang/Long" => {
            if let Some(Slot::Long(v)) = obj.fields.first() {
                return v.to_string();
            }
        }
        "java/lang/Double" => {
            if let Some(Slot::Double(v)) = obj.fields.first() {
                return format_java_double(*v);
            }
        }
        "java/lang/Float" => {
            if let Some(Slot::Float(v)) = obj.fields.first() {
                return format_java_float(*v);
            }
        }
        "java/lang/Boolean" => {
            return match obj.fields.first() {
                Some(Slot::Int(v)) if *v != 0 => "true".to_string(),
                _ => "false".to_string(),
            };
        }
        "java/lang/Character" => {
            if let Some(Slot::Int(v)) = obj.fields.first()
                && let Some(c) = char::from_u32((*v).cast_unsigned())
            {
                return c.to_string();
            }
        }
        _ => {}
    }
    format!("{}@{:x}", obj.class_name, obj_ref)
}

pub(crate) fn native_println_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.get(1) {
        Some(Slot::Reference(Some(r))) => {
            let s = heap_object_to_string(heap.get(*r)?, *r);
            writeln!(out, "{s}").ok();
        }
        Some(Slot::Reference(None)) => {
            writeln!(out, "null").ok();
        }
        _ => {
            writeln!(out, "<unknown>").ok();
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// print overloads (long, float, double, boolean, char, object)
// ---------------------------------------------------------------------------

pub(crate) fn native_print_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Long(v) => *v, "Long");
    write!(out, "{val}").ok();
    Ok(None)
}

pub(crate) fn native_print_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Float(v) => *v, "Float");
    write!(out, "{val}").ok();
    Ok(None)
}

pub(crate) fn native_print_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Double(v) => *v, "Double");
    write!(out, "{val}").ok();
    Ok(None)
}

pub(crate) fn native_print_boolean(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Int(v) => *v != 0, "Int(boolean)");
    write!(out, "{val}").ok();
    Ok(None)
}

pub(crate) fn native_print_char(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Int(v) => char::from_u32((*v).cast_unsigned()).unwrap_or('?'), "Int(char)");
    write!(out, "{val}").ok();
    Ok(None)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_print_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.get(1) {
        Some(Slot::Reference(Some(r))) => {
            let s = heap_object_to_string(heap.get(*r)?, *r);
            write!(out, "{s}").ok();
        }
        Some(Slot::Reference(None)) => {
            write!(out, "null").ok();
        }
        _ => {
            write!(out, "<unknown>").ok();
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// System.exit
// ---------------------------------------------------------------------------

pub(crate) fn native_system_exit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let code = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 1,
    };
    Err(VmError::SystemExit { code })
}

static SYSTEM_PROPERTY_OVERRIDES: std::sync::OnceLock<std::sync::Mutex<HashMap<String, String>>> =
    std::sync::OnceLock::new();

fn system_property_overrides() -> &'static std::sync::Mutex<HashMap<String, String>> {
    SYSTEM_PROPERTY_OVERRIDES.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

fn system_property_value(key: &str) -> Option<String> {
    let override_value = system_property_overrides()
        .lock()
        .expect("system property overrides mutex poisoned")
        .get(key)
        .cloned();
    if let Some(value) = override_value {
        return Some(value);
    }
    match key {
        "java.io.tmpdir" => Some(std::env::temp_dir().to_string_lossy().into_owned()),
        "file.separator" => Some(std::path::MAIN_SEPARATOR.to_string()),
        "path.separator" => Some(if cfg!(windows) { ";" } else { ":" }.to_string()),
        "line.separator" => Some(if cfg!(windows) { "\r\n" } else { "\n" }.to_string()),
        "user.dir" => std::env::current_dir()
            .ok()
            .map(|path| path.to_string_lossy().into_owned()),
        "user.home" => std::env::var("USERPROFILE")
            .ok()
            .or_else(|| std::env::var("HOME").ok()),
        "os.name" => Some(
            if cfg!(windows) {
                "Windows"
            } else if cfg!(target_os = "macos") {
                "Mac OS X"
            } else if cfg!(target_os = "linux") {
                "Linux"
            } else {
                std::env::consts::OS
            }
            .to_string(),
        ),
        "os.arch" => Some(std::env::consts::ARCH.to_string()),
        "java.version" => Some("21".to_string()),
        _ => None,
    }
}

pub(crate) fn native_system_get_property(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let key_ref = extract_ref_arg(args, 0)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let result = system_property_value(&key).map_or(Slot::Reference(None), |value| {
        Slot::Reference(Some(heap.allocate_string(value)))
    });
    Ok(Some(result))
}

pub(crate) fn native_system_set_property(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let key_ref = extract_ref_arg(args, 0)?;
    let value_ref = extract_ref_arg(args, 1)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let value = string_value_from_ref(heap, value_ref)?;
    let previous = system_property_value(&key);
    system_property_overrides()
        .lock()
        .expect("system property overrides mutex poisoned")
        .insert(key, value);
    let result = previous.map_or(Slot::Reference(None), |previous| {
        Slot::Reference(Some(heap.allocate_string(previous)))
    });
    Ok(Some(result))
}

pub(crate) fn native_system_get_property_with_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let key_ref = extract_ref_arg(args, 0)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let result = system_property_value(&key).map_or_else(
        || extract_slot_arg(args, 1),
        |value| Slot::Reference(Some(heap.allocate_string(value))),
    );
    Ok(Some(result))
}

/// Native: `System.lineSeparator()String` — returns the platform line separator.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_system_line_separator(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate_string("\n".to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `System.identityHashCode(Object)I` — returns a stable identity hash (heap address low bits).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_system_identity_hash_code(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    #[allow(clippy::cast_possible_truncation)]
    let hash = match args.first() {
        Some(Slot::Reference(Some(r))) => (*r & 0x7FFF_FFFF) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(hash)))
}

fn system_time_to_epoch_millis(now: std::time::SystemTime) -> i64 {
    now.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_system_current_time_millis(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(Slot::Long(system_time_to_epoch_millis(
        std::time::SystemTime::now(),
    ))))
}

static NANO_TIME_ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn monotonic_nano_time_now() -> i64 {
    let origin = NANO_TIME_ORIGIN.get_or_init(std::time::Instant::now);
    i64::try_from(origin.elapsed().as_nanos()).unwrap_or(i64::MAX)
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_system_nano_time(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(Slot::Long(monotonic_nano_time_now())))
}

const THREAD_TARGET_SLOT: usize = 0;
const THREAD_ID_SLOT: usize = 1;

pub(crate) fn native_thread_current_thread(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let thread_ref = heap.allocate("java/lang/Thread".to_string(), 2);
    let thread = heap.get_mut(thread_ref)?;
    thread.fields[THREAD_TARGET_SLOT] = Slot::Reference(None);
    thread.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    Ok(Some(Slot::Reference(Some(thread_ref))))
}

pub(crate) fn native_thread_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let this = heap.get_mut(this_ref)?;
    this.fields[THREAD_TARGET_SLOT] = Slot::Reference(None);
    this.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    Ok(None)
}

pub(crate) fn native_thread_init_runnable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let this = heap.get_mut(this_ref)?;
    this.fields[THREAD_TARGET_SLOT] = target;
    this.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    Ok(None)
}

pub(crate) fn native_thread_start(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    control.request(NativeThreadAction::Start { thread_ref });
    Ok(None)
}

pub(crate) fn native_thread_join(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let thread = heap.get(thread_ref)?;
    let Some(Slot::Int(thread_id)) = thread.fields.get(THREAD_ID_SLOT) else {
        return Ok(None);
    };
    if *thread_id >= 0 {
        control.request(NativeThreadAction::Join {
            thread_id: *thread_id,
        });
    }
    Ok(None)
}

pub(crate) fn native_thread_sleep(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let millis = match args.first() {
        Some(Slot::Long(value)) => *value,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "long",
                got: "other",
            });
        }
    };
    let millis = u64::try_from(millis.max(0)).unwrap_or(0);
    control.request(NativeThreadAction::Sleep(std::time::Duration::from_millis(
        millis,
    )));
    Ok(None)
}

/// Native: `String.substring(int)` — substring from begin to end.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn native_string_substring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;

    let begin = extract_int_arg(args, 1)? as usize;
    let sub = {
        let obj = heap.get(this_ref)?;
        let s = obj.string_value.as_deref().unwrap_or_default();
        if begin > s.len() {
            return Err(VmError::ArrayIndexOutOfBounds {
                index: i32::try_from(begin).unwrap_or(i32::MAX),
                length: s.len(),
            });
        }
        s.chars().skip(begin).collect::<String>()
    };

    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.substring(int, int)` — substring from begin to end (exclusive).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn native_string_substring_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;

    let begin = extract_int_arg(args, 1)? as usize;
    let end = extract_int_arg(args, 2)? as usize;
    let sub = {
        let obj = heap.get(this_ref)?;
        let s = obj.string_value.as_deref().unwrap_or_default();
        if begin > end || end > s.len() {
            return Err(VmError::ArrayIndexOutOfBounds {
                index: i32::try_from(end).unwrap_or(i32::MAX),
                length: s.len(),
            });
        }
        s.chars().skip(begin).take(end - begin).collect::<String>()
    };

    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.indexOf(String)` — find first occurrence of target.
pub(crate) fn native_string_indexof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;

    let target_ref = extract_ref_arg(args, 1)?;
    // Fetch objects from heap in one go to keep borrows short
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let result = s.find(target).map_or(-1, |i| i as i32);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.contains(CharSequence)` — check if string contains target.
pub(crate) fn native_string_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;

    let target_ref = extract_ref_arg(args, 1)?;
    // Fetch objects from heap in one go to keep borrows short
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.contains(target)))))
}

/// Native: `String.isEmpty()` — check if string is empty.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_string_isempty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let s = obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.is_empty()))))
}

// Dispatch string constants used by the callback-based sort chain.
const COMPARE_TO_METHOD: &str = "compareTo";
const COMPARE_TO_OBJECT_DESC: &str = "(Ljava/lang/Object;)I";
const SORT_COMPARATOR_DESC: &str = "(Ljava/util/Comparator;)V";

/// Maps a `std::cmp::Ordering` to the Java `compareTo` convention: -1 / 0 / 1.
///
/// Used by all boxed-type `compareTo` natives to return a consistent,
/// sign-correct value without relying on `Ordering`'s internal discriminant.
#[inline]
const fn ordering_to_int(o: std::cmp::Ordering) -> i32 {
    match o {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Native: `String.compareTo(String)` — delegates to the Object overload.
pub(crate) fn native_string_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_string_compareto_object(args, heap, out, control)
}

/// Native: `String.compareTo(Object)` — lexicographic comparison via Object descriptor.
pub(crate) fn native_string_compareto_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let str_val = |s: &Slot| -> VmResult<String> {
        match s {
            Slot::Reference(Some(r)) => Ok(heap.get(*r)?.string_value.clone().unwrap_or_default()),
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => str_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = str_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.as_str().cmp(b.as_str())))))
}

/// Native: `String.startsWith(String)` — check if string starts with prefix.
pub(crate) fn native_string_startswith(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let prefix_ref = extract_ref_arg(args, 1)?;
    let prefix = heap
        .get(prefix_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.starts_with(&prefix)))))
}

/// Native: `String.endsWith(String)` — check if string ends with suffix.
pub(crate) fn native_string_endswith(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let suffix_ref = extract_ref_arg(args, 1)?;
    let suffix = heap
        .get(suffix_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.ends_with(&suffix)))))
}

/// Native: `String.trim()` — remove leading and trailing whitespace.
pub(crate) fn native_string_trim(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let trimmed = s.trim().to_string();
    let r = heap.allocate_string(trimmed);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.toCharArray()` — convert string to char array.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_string_tochararray(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let chars: Vec<char> = s.chars().collect();
    let arr_ref = heap.allocate("[C".to_string(), chars.len());
    for (i, &c) in chars.iter().enumerate() {
        heap.get_mut(arr_ref)?.fields[i] = Slot::Int(c as i32);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

// ---- Integer natives ----

/// Native: `Integer.parseInt(String)` — parses string to int.
pub(crate) fn native_integer_parseint(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(args, heap, i32::MIN, i32::MAX)?;
    Ok(Some(Slot::Int(val)))
}

/// Native: `Integer.parseInt(String,int)` — parses string to int with radix.
pub(crate) fn native_integer_parseint_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(args, heap, i32::MIN, i32::MAX)?;
    Ok(Some(Slot::Int(val)))
}

/// Native: `Integer.valueOf(int)` — boxes int into Integer object.
pub(crate) fn native_integer_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.valueOf(String)` — parses and boxes int.
pub(crate) fn native_integer_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(args, heap, i32::MIN, i32::MAX)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.valueOf(String,int)` — parses and boxes int with radix.
pub(crate) fn native_integer_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(args, heap, i32::MIN, i32::MAX)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.decode(String)` — parses prefixed string and boxes int.
pub(crate) fn native_integer_decode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_i32_decode_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.intValue()` — unboxes Integer to int.
pub(crate) fn native_integer_intvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

/// Native: `Integer.toString(int)` — static, converts int to String.
pub(crate) fn native_integer_tostring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.toHexString(int)` — unsigned lowercase hex string.
pub(crate) fn native_integer_tohexstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let r = heap.allocate_string(format!("{val:x}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.toOctalString(int)` — unsigned octal string.
pub(crate) fn native_integer_tooctalstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let r = heap.allocate_string(format!("{val:o}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.toBinaryString(int)` — unsigned binary string.
pub(crate) fn native_integer_tobinarystring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let r = heap.allocate_string(format!("{val:b}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.toUnsignedLong(int)` — widen via unsigned 32-bit interpretation.
pub(crate) fn native_integer_tounsignedlong_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    Ok(Some(Slot::Long(i64::from(val))))
}

/// Native: `Integer.compareUnsigned(int,int)` — compares ints as unsigned 32-bit values.
pub(crate) fn native_integer_compareunsigned_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let b = u32::from_ne_bytes(extract_int_arg(args, 1)?.to_ne_bytes());
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Native: `Integer.compareTo(Object)` — compares two boxed Integers.
pub(crate) fn native_integer_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let int_val = |s: &Slot| -> VmResult<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => int_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = int_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

// ---- Integer bit / arithmetic operations ----

/// Native: `Integer.bitCount(int)` — count number of set bits (popcount).
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
pub(crate) fn native_integer_bitcount(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.count_ones() as i32)))
}

/// Native: `Integer.numberOfLeadingZeros(int)` — count leading zero bits.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
pub(crate) fn native_integer_leading_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.leading_zeros() as i32)))
}

/// Native: `Integer.numberOfTrailingZeros(int)` — count trailing zero bits.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
pub(crate) fn native_integer_trailing_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.trailing_zeros() as i32)))
}

/// Native: `Integer.highestOneBit(int)` — return value with only the highest set bit.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_integer_highest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    let result = if v == 0 { 0u32 } else { 1u32 << v.ilog2() };
    Ok(Some(Slot::Int(result as i32)))
}

/// Native: `Integer.lowestOneBit(int)` — return value with only the lowest set bit.
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn native_integer_lowest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(v & v.wrapping_neg())))
}

/// Native: `Integer.reverse(int)` — reverse bit order.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_integer_reverse(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.reverse_bits() as i32)))
}

/// Native: `Integer.reverseBytes(int)` — reverse byte order (swap endianness).
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_integer_reverse_bytes(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.swap_bytes() as i32)))
}

/// Native: `Integer.signum(int)` — returns -1, 0, or 1.
pub(crate) fn native_integer_signum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(v.signum())))
}

/// Native: `Integer.compare(int, int)` — static two-value comparison.
pub(crate) fn native_integer_compare_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Native: `Integer.sum(int, int)` — static addition (functional interface target).
pub(crate) fn native_integer_sum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.wrapping_add(b))))
}

/// Native: `Integer.max(int, int)` — static max (functional interface target).
pub(crate) fn native_integer_max_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.max(b))))
}

/// Native: `Integer.min(int, int)` — static min (functional interface target).
pub(crate) fn native_integer_min_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.min(b))))
}

// ---- Long bit / arithmetic operations ----

/// Native: `Long.bitCount(long)` — count number of set bits.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
pub(crate) fn native_long_bitcount(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Int(v.count_ones() as i32)))
}

/// Native: `Long.numberOfLeadingZeros(long)` — count leading zero bits.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
pub(crate) fn native_long_leading_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Int(v.leading_zeros() as i32)))
}

/// Native: `Long.numberOfTrailingZeros(long)` — count trailing zero bits.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
pub(crate) fn native_long_trailing_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Int(v.trailing_zeros() as i32)))
}

/// Native: `Long.highestOneBit(long)` — return value with only the highest set bit.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_long_highest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    let result = if v == 0 { 0u64 } else { 1u64 << v.ilog2() };
    Ok(Some(Slot::Long(result as i64)))
}

/// Native: `Long.lowestOneBit(long)` — return value with only the lowest set bit.
pub(crate) fn native_long_lowest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Long(v & v.wrapping_neg())))
}

/// Native: `Long.reverse(long)` — reverse bit order.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_long_reverse(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Long(v.reverse_bits() as i64)))
}

/// Native: `Long.reverseBytes(long)` — reverse byte order (swap endianness).
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_long_reverse_bytes(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Long(v.swap_bytes() as i64)))
}

/// Native: `Long.signum(long)` — returns -1, 0, or 1.
pub(crate) fn native_long_signum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Int(v.signum() as i32)))
}

/// Native: `Long.compare(long, long)` — static two-value comparison.
pub(crate) fn native_long_compare_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Native: `Long.sum(long, long)` — static addition (functional interface target).
pub(crate) fn native_long_sum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(a.wrapping_add(b))))
}

// ---- java.util.Objects natives ----

/// Native: `Objects.isNull(Object)Z` — returns 1 if argument is null.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_objects_is_null(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let is_null = matches!(args.first(), Some(Slot::Reference(None)) | None);
    Ok(Some(Slot::Int(i32::from(is_null))))
}

/// Native: `Objects.nonNull(Object)Z` — returns 1 if argument is not null.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_objects_non_null(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let is_null = matches!(args.first(), Some(Slot::Reference(None)) | None);
    Ok(Some(Slot::Int(i32::from(!is_null))))
}

/// Native: `Objects.requireNonNull(Object)Object` — throws NPE if null, else returns arg.
pub(crate) fn native_objects_require_non_null(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(None)) | None => Err(VmError::NullPointerException),
        Some(s) => Ok(Some(*s)),
    }
}

/// Native: `Objects.requireNonNull(Object, String)Object` — throws NPE with message if null.
pub(crate) fn native_objects_require_non_null_msg(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(None)) | None => Err(VmError::NullPointerException),
        Some(s) => Ok(Some(*s)),
    }
}

/// Native: `Objects.equals(Object, Object)Z` — null-safe equality check.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_objects_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = args.first().copied().unwrap_or(Slot::Reference(None));
    let b = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let equal = slots_equal(&a, &b, heap);
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `Objects.toString(Object)` — returns `"null"` if null, else `string_value` or class name.
pub(crate) fn native_objects_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let s = match args.first() {
        Some(Slot::Reference(None)) | None => heap.allocate_string("null".to_string()),
        Some(Slot::Reference(Some(r))) => {
            let obj = heap.get(*r)?;
            let text = obj
                .string_value
                .as_deref()
                .map_or_else(|| format!("{}@{}", obj.class_name, r), str::to_owned);
            let _ = obj;
            heap.allocate_string(text)
        }
        Some(Slot::Int(n)) => heap.allocate_string(n.to_string()),
        Some(Slot::Long(n)) => heap.allocate_string(n.to_string()),
        Some(other) => heap.allocate_string(format!("{other:?}")),
    };
    Ok(Some(Slot::Reference(Some(s))))
}

/// Native: `Objects.hashCode(Object)I` — returns 0 for null, else object identity hash.
#[allow(clippy::cast_possible_truncation, clippy::unnecessary_wraps)]
pub(crate) fn native_objects_hashcode(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let hash = match args.first() {
        Some(Slot::Reference(Some(r))) => (*r & 0x7FFF_FFFF) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(hash)))
}

// ---- Collections utilities ----

/// Native: `Collections.emptyList()List` — returns a new empty `ArrayList`.
pub(crate) fn native_collections_empty_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.emptySet()Set` — returns a new empty `HashSet`.
pub(crate) fn native_collections_empty_set(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.emptyMap()Map` — returns a new empty `HashMap`.
pub(crate) fn native_collections_empty_map(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.unmodifiableList(List)List` — returns the same list (no-copy; single-threaded).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_list(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(args.first().copied().unwrap_or(Slot::Reference(None))))
}

/// Native: `Math.random()D` — returns a pseudo-random double in [0.0, 1.0).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_math_random(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // Use a simple deterministic seed based on stack pointer heuristic
    // For a JVM interpreter we just use a fixed-seed LCG for reproducibility
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(12345);
    let old = SEED.load(Ordering::Relaxed);
    let new = old.wrapping_mul(25_214_903_917).wrapping_add(11) & 0x0000_FFFF_FFFF_FFFF;
    SEED.store(new, Ordering::Relaxed);
    #[allow(clippy::cast_precision_loss)]
    let v = (new as f64) / (1_u64 << 48) as f64;
    Ok(Some(Slot::Double(v)))
}

/// Native: `Arrays.stream(int[])IntStream` — wraps an int array into an `IntStream`.
pub(crate) fn native_arrays_stream_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    // int[] layout: fields = Int elements directly (no length header); arraylength = fields.len()
    let arr_obj = heap.get(arr_ref)?;
    let values: Vec<i32> = arr_obj
        .fields
        .iter()
        .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
        .collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

/// Native: `Arrays.stream(int[], int, int)IntStream` — wraps a subrange as an `IntStream`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_arrays_stream_int_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let from = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let to = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let arr_obj = heap.get(arr_ref)?;
    let values: Vec<i32> = arr_obj
        .fields
        .get(from..to.min(arr_obj.fields.len()))
        .unwrap_or(&[])
        .iter()
        .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
        .collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

/// Native: `Arrays.stream(Object[])Stream` — wraps a reference array as an eager `Stream`.
pub(crate) fn native_arrays_stream_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let elems: Vec<Slot> = heap.get(arr_ref)?.fields.clone();
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for elem in elems {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `Comparator.comparing(Function)Comparator` — creates a comparator by key extractor.
/// Returns a `duke/util/ComparingComparator` with `fields[0]`=fn\_ref.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_comparing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ComparingComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Comparator.comparing compare(Object,Object)I` — compare via key extractor.
pub(crate) fn native_comparing_comparator_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let fn_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Reference(None));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, b],
        )?
        .unwrap_or(Slot::Reference(None));
    // Compare extracted keys via compareTo
    let cmp = match (&ka, &kb) {
        (Slot::Reference(Some(ra)), Slot::Reference(Some(rb))) => {
            let sa = heap
                .get(*ra)
                .ok()
                .and_then(|o| o.string_value.clone())
                .unwrap_or_default();
            let sb = heap
                .get(*rb)
                .ok()
                .and_then(|o| o.string_value.clone())
                .unwrap_or_default();
            sa.cmp(&sb) as i32
        }
        (Slot::Int(a), Slot::Int(b)) => a.cmp(b) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(cmp)))
}

// ---------------------------------------------------------------------------
// Phase 43: Collectors.toSet/toMap, Stream.mapToInt/min/max, IntStream.reduce,
//           Arrays.sort(Object[]), String(char[])/valueOf(char[])
// ---------------------------------------------------------------------------

/// Native: `Collectors.toSet()Collector` — returns a `ToSetCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_set(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("duke/util/ToSetCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.toMap(keyFn, valFn)Collector` — stores both functions in `ToMapCollector`.
pub(crate) fn native_collectors_to_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let key_fn = args.first().copied().unwrap_or(Slot::Reference(None));
    let val_fn = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ToMapCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = key_fn;
    heap.get_mut(r)?.fields[1] = val_fn;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Stream.mapToInt(ToIntFunction)IntStream` — maps each element via `applyAsInt`.
pub(crate) fn native_stream_map_to_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut values = Vec::with_capacity(elems.len());
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(Ljava/lang/Object;)I",
                vec![fn_slot, elem],
            )?
            .unwrap_or(Slot::Int(0));
        match result {
            Slot::Int(n) => values.push(n),
            _ => values.push(0),
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

/// Native: `IntStream.reduce(int, IntBinaryOperator)I` — fold with identity via callback.
pub(crate) fn native_int_stream_reduce_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let identity = match args.get(1) {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    let fn_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(identity)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let values = int_stream_elems(heap, stream_ref);
    let mut acc = identity;
    for v in values {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(II)I",
                vec![fn_slot, Slot::Int(acc), Slot::Int(v)],
            )?
            .unwrap_or(Slot::Int(0));
        acc = match result {
            Slot::Int(n) => n,
            _ => 0,
        };
    }
    Ok(Some(Slot::Int(acc)))
}

/// Native: `IntStream.reduce(IntBinaryOperator)OptionalInt` — fold without identity.
pub(crate) fn native_int_stream_reduce_optional(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        // Return empty OptionalInt
        let r = make_optional_int(heap, None);
        return Ok(Some(Slot::Reference(Some(r))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let values = int_stream_elems(heap, stream_ref);
    if values.is_empty() {
        return Ok(Some(Slot::Reference(Some(make_optional_int(heap, None)))));
    }
    let mut acc = values[0];
    for &v in &values[1..] {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(II)I",
                vec![fn_slot, Slot::Int(acc), Slot::Int(v)],
            )?
            .unwrap_or(Slot::Int(0));
        acc = match result {
            Slot::Int(n) => n,
            _ => 0,
        };
    }
    Ok(Some(Slot::Reference(Some(make_optional_int(
        heap,
        Some(acc),
    )))))
}

/// Native: `Stream.min(Comparator)Optional` — returns minimum element by comparator.
pub(crate) fn native_stream_min_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    stream_min_max_by_comparator(args, heap, out, ops, false)
}

/// Native: `Stream.max(Comparator)Optional` — returns maximum element by comparator.
pub(crate) fn native_stream_max_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    stream_min_max_by_comparator(args, heap, out, ops, true)
}

fn stream_min_max_by_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    want_max: bool,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let cmp_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(cmp_ref)) = cmp_slot else {
        let r = heap.allocate("java/util/Optional".to_string(), 1);
        heap.get_mut(r)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(r))));
    };
    let cmp_class = heap.get(cmp_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let opt_r = heap.allocate("java/util/Optional".to_string(), 1);
    if elems.is_empty() {
        heap.get_mut(opt_r)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(opt_r))));
    }
    let mut best = elems[0];
    for elem in elems.into_iter().skip(1) {
        let cmp_result = ops
            .invoke(
                heap,
                out,
                &cmp_class,
                "compare",
                "(Ljava/lang/Object;Ljava/lang/Object;)I",
                vec![cmp_slot, elem, best],
            )?
            .unwrap_or(Slot::Int(0));
        let cmp_val = match cmp_result {
            Slot::Int(n) => n,
            _ => 0,
        };
        // For min: pick elem if elem < best (cmp_val < 0)
        // For max: pick elem if elem > best (cmp_val > 0)
        if (want_max && cmp_val > 0) || (!want_max && cmp_val < 0) {
            best = elem;
        }
    }
    heap.get_mut(opt_r)?.fields[0] = best;
    Ok(Some(Slot::Reference(Some(opt_r))))
}

/// Native: `Arrays.sort(Object[])V` — natural order sort using `compareTo`.
#[allow(clippy::too_many_lines)]
pub(crate) fn native_arrays_sort_objects(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let len = heap.get(arr_ref)?.fields.len();
    // Insertion sort with compareTo callbacks
    for i in 1..len {
        let mut j = i;
        while j > 0 {
            let a = heap.get(arr_ref)?.fields[j - 1];
            let b = heap.get(arr_ref)?.fields[j];
            let cmp = compare_slots_natural(a, b, heap, out, ops)?;
            if cmp <= 0 {
                break;
            }
            heap.write_field(arr_ref, j - 1, b)?;
            heap.write_field(arr_ref, j, a)?;
            j -= 1;
        }
    }
    Ok(None)
}

fn compare_slots_natural(
    a: Slot,
    b: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> VmResult<i32> {
    match (a, b) {
        (Slot::Reference(Some(ra)), Slot::Reference(Some(_rb))) => {
            let a_class = heap.get(ra)?.class_name.clone();
            let result = ops
                .invoke(
                    heap,
                    out,
                    &a_class,
                    "compareTo",
                    "(Ljava/lang/Object;)I",
                    vec![a, b],
                )?
                .unwrap_or(Slot::Int(0));
            Ok(match result {
                Slot::Int(n) => n,
                _ => 0,
            })
        }
        (Slot::Int(a), Slot::Int(b)) => Ok(a.cmp(&b) as i32),
        _ => Ok(0),
    }
}

/// Native: `String.<init>(char[])V` — constructs a String from a char array.
pub(crate) fn native_string_init_from_chars(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Reference(Some(arr_ref))) = args.get(1).copied() else {
        heap.get_mut(this_ref)?.string_value = Some(String::new());
        return Ok(None);
    };
    let chars: String = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| match s {
            Slot::Int(n) => char::from_u32(u32::try_from(*n).unwrap_or(0)),
            _ => None,
        })
        .collect();
    heap.get_mut(this_ref)?.string_value = Some(chars);
    Ok(None)
}

/// Native: `String.valueOf(char[])String` — creates String from char array.
pub(crate) fn native_string_value_of_char_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => {
            return Ok(Some(Slot::Reference(Some(
                heap.allocate_string(String::new()),
            ))));
        }
    };
    let chars: String = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| match s {
            Slot::Int(n) => char::from_u32(u32::try_from(*n).unwrap_or(0)),
            _ => None,
        })
        .collect();
    Ok(Some(Slot::Reference(Some(heap.allocate_string(chars)))))
}

/// Native: `Collections.singletonList(Object)List` — returns a one-element `ArrayList`.
pub(crate) fn native_collections_singleton_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let element = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    native_arraylist_add(&[Slot::Reference(Some(r)), element], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.reverse(List)V` — reverses an `ArrayList` in-place.
pub(crate) fn native_collections_reverse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(None),
    };
    // Elements are at fields[1..=size]; reverse that slice.
    let fields = &mut heap.get_mut(list_ref)?.fields;
    fields[1..=size].reverse();
    Ok(None)
}

/// Native: `Collections.frequency(Collection, Object)I` — count occurrences of element.
pub(crate) fn native_collections_frequency(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let coll_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let size = match heap.get(coll_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let fields = heap.get(coll_ref)?.fields[1..=size].to_vec();
    let count = fields
        .iter()
        .filter(|s| slots_equal(s, &target, heap))
        .count();
    Ok(Some(Slot::Int(i32::try_from(count).unwrap_or(i32::MAX))))
}

// ---- String.valueOf overloads ----

/// Native: `String.valueOf(long)` — converts long to String.
pub(crate) fn native_string_value_of_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(double)` — converts double to String.
pub(crate) fn native_string_value_of_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_double_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(float)` — converts float to String.
pub(crate) fn native_string_value_of_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_float_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(boolean)` — converts boolean to String.
pub(crate) fn native_string_value_of_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v != 0,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int(boolean)",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(if val { "true" } else { "false" }.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(char)` — converts char to String.
pub(crate) fn native_string_value_of_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('?'),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int(char)",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(Object)` — converts Object to String.
pub(crate) fn native_string_value_of_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let s = heap_object_to_string(heap.get(*r)?, *r);
            let r = heap.allocate_string(s);
            Ok(Some(Slot::Reference(Some(r))))
        }
        Some(Slot::Reference(None)) => {
            let r = heap.allocate_string("null".to_string());
            Ok(Some(Slot::Reference(Some(r))))
        }
        _ => Err(VmError::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

// ---- String.concat ----

/// Native: `String.concat(String)` — concatenates two strings.
pub(crate) fn native_string_concat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s1 = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let other_ref = extract_ref_arg(args, 1)?;
    let s2 = heap
        .get(other_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let r = heap.allocate_string(format!("{s1}{s2}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Formats a single boxed slot value using the given format specifier.
/// Apply width/alignment/flags to an already-formatted value string.
fn apply_format_width(s: String, width: usize, left_align: bool, zero_pad: bool) -> String {
    if s.len() >= width {
        return s;
    }
    let pad = width - s.len();
    if left_align {
        format!("{s}{}", " ".repeat(pad))
    } else if zero_pad {
        // zero-pad: insert zeros after optional sign
        if s.starts_with('-') || s.starts_with('+') {
            let (sign, rest) = s.split_at(1);
            format!("{sign}{}{rest}", "0".repeat(pad))
        } else {
            format!("{}{s}", "0".repeat(pad))
        }
    } else {
        format!("{}{s}", " ".repeat(pad))
    }
}

#[allow(clippy::too_many_lines)]
fn format_arg(
    spec: char,
    flags: &str,
    width: Option<usize>,
    precision: Option<usize>,
    slot: &Slot,
    heap: &duke_gc::Heap,
) -> VmResult<String> {
    let left_align = flags.contains('-');
    let force_sign = flags.contains('+');
    let zero_pad = flags.contains('0') && !left_align;

    let raw = match slot {
        Slot::Reference(None) => {
            if spec == 'b' {
                "false".to_string()
            } else {
                "null".to_string()
            }
        }
        Slot::Reference(Some(r)) => {
            let obj = heap.get(*r)?;
            match spec {
                's' => heap_object_to_string(obj, *r),
                'b' => {
                    // true if non-null Boolean true, else depends
                    if obj.class_name == "java/lang/Boolean" {
                        match obj.fields.first() {
                            Some(Slot::Int(n)) => {
                                if *n != 0 { "true" } else { "false" }.to_string()
                            }
                            _ => "true".to_string(),
                        }
                    } else {
                        "true".to_string() // non-null object → true
                    }
                }
                'c' => {
                    let code = match obj.fields.first() {
                        Some(Slot::Int(n)) => *n,
                        _ => 0,
                    };
                    #[allow(clippy::cast_sign_loss)]
                    char::from_u32(code as u32).map_or(String::new(), |c| c.to_string())
                }
                'd' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Int(v)) => i64::from(*v),
                        Some(Slot::Long(v)) => *v,
                        _ => 0,
                    };
                    if force_sign && v >= 0 {
                        format!("+{v}")
                    } else {
                        v.to_string()
                    }
                }
                'o' => match obj.fields.first() {
                    Some(Slot::Int(v)) => format!("{v:o}"),
                    Some(Slot::Long(v)) => format!("{v:o}"),
                    _ => "0".to_string(),
                },
                'f' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Double(v)) => *v,
                        Some(Slot::Float(v)) => f64::from(*v),
                        _ => 0.0,
                    };
                    let s = precision.map_or_else(|| format!("{v:.6}"), |p| format!("{v:.p$}"));
                    if force_sign && v >= 0.0 {
                        format!("+{s}")
                    } else {
                        s
                    }
                }
                'e' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Double(v)) => *v,
                        Some(Slot::Float(v)) => f64::from(*v),
                        _ => 0.0,
                    };
                    let prec = precision.unwrap_or(6);
                    // format in scientific notation matching Java's %e output
                    let s = format_scientific(v, prec, false);
                    if force_sign && v >= 0.0 {
                        format!("+{s}")
                    } else {
                        s
                    }
                }
                'x' => match obj.fields.first() {
                    Some(Slot::Int(v)) => format!("{v:x}"),
                    Some(Slot::Long(v)) => format!("{v:x}"),
                    _ => "0".to_string(),
                },
                'X' => match obj.fields.first() {
                    Some(Slot::Int(v)) => format!("{v:X}"),
                    Some(Slot::Long(v)) => format!("{v:X}"),
                    _ => "0".to_string(),
                },
                _ => String::new(),
            }
        }
        Slot::Int(n) => match spec {
            'd' => {
                if force_sign && *n >= 0 {
                    format!("+{n}")
                } else {
                    n.to_string()
                }
            }
            'b' => "true".to_string(),
            'c' =>
            {
                #[allow(clippy::cast_sign_loss)]
                char::from_u32(*n as u32).map_or(String::new(), |c| c.to_string())
            }
            'o' => format!("{n:o}"),
            'x' => format!("{n:x}"),
            'X' => format!("{n:X}"),
            _ => n.to_string(),
        },
        Slot::Long(n) => match spec {
            'd' => {
                if force_sign && *n >= 0 {
                    format!("+{n}")
                } else {
                    n.to_string()
                }
            }
            'o' => format!("{n:o}"),
            'x' => format!("{n:x}"),
            'X' => format!("{n:X}"),
            _ => n.to_string(),
        },
        Slot::Double(v) => match spec {
            'f' => {
                let s = precision.map_or_else(|| format!("{v:.6}"), |p| format!("{v:.p$}"));
                if force_sign && *v >= 0.0 {
                    format!("+{s}")
                } else {
                    s
                }
            }
            'e' => {
                let prec = precision.unwrap_or(6);
                let s = format_scientific(*v, prec, false);
                if force_sign && *v >= 0.0 {
                    format!("+{s}")
                } else {
                    s
                }
            }
            _ => format!("{v}"),
        },
        _ => String::new(),
    };

    Ok(match width {
        None => raw,
        Some(w) => apply_format_width(raw, w, left_align, zero_pad),
    })
}

/// Format a float in Java-style scientific notation `1.234568e+05`.
fn format_scientific(v: f64, prec: usize, upper: bool) -> String {
    if v == 0.0 {
        let zeros = "0".repeat(prec);
        let e = if upper { 'E' } else { 'e' };
        return format!("0.{zeros}{e}+00");
    }
    #[allow(clippy::cast_possible_truncation)]
    let exp = v.abs().log10().floor() as i32;
    let mantissa = v / 10_f64.powi(exp);
    let s = format!("{mantissa:.prec$}");
    let e_char = if upper { 'E' } else { 'e' };
    if exp >= 0 {
        format!("{s}{e_char}+{exp:02}")
    } else {
        format!("{s}{e_char}-{:02}", exp.unsigned_abs())
    }
}

/// Native: `String.format(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;`
pub(crate) fn native_string_format(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fmt_ref = extract_ref_arg(args, 0)?;
    let fmt = heap.get(fmt_ref)?.string_value.clone().unwrap_or_default();

    let arr_len = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.fields.len(),
        _ => 0,
    };

    let mut result = String::new();
    let mut arg_idx = 0usize;
    let mut chars = fmt.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '%' {
            result.push(ch);
            continue;
        }

        // Parse flags: -, +, 0
        let mut flags = String::new();
        while let Some(&f) = chars.peek() {
            if matches!(f, '-' | '+' | '0' | ' ' | '#') {
                flags.push(f);
                chars.next();
            } else {
                break;
            }
        }

        // Parse optional width
        let mut width_str = String::new();
        while let Some(&d) = chars.peek() {
            if d.is_ascii_digit() {
                width_str.push(d);
                chars.next();
            } else {
                break;
            }
        }
        let width: Option<usize> = if width_str.is_empty() {
            None
        } else {
            width_str.parse().ok()
        };

        // Parse optional precision: .N
        let precision: Option<usize> = if chars.peek() == Some(&'.') {
            chars.next();
            let mut prec_str = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    prec_str.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            prec_str.parse().ok()
        } else {
            None
        };

        let Some(spec) = chars.next() else {
            break;
        };

        match spec {
            '%' => result.push('%'),
            'n' => result.push('\n'),
            's' | 'd' | 'f' | 'x' | 'X' | 'b' | 'c' | 'o' | 'e' | 'E' => {
                let slot = if arg_idx < arr_len {
                    match args.get(1) {
                        Some(Slot::Reference(Some(r))) => heap
                            .get(*r)?
                            .fields
                            .get(arg_idx)
                            .copied()
                            .unwrap_or(Slot::Reference(None)),
                        _ => Slot::Reference(None),
                    }
                } else {
                    Slot::Reference(None)
                };
                arg_idx += 1;
                let formatted = format_arg(spec, &flags, width, precision, &slot, heap)?;
                result.push_str(&formatted);
            }
            _ => {
                result.push('%');
                result.push(spec);
            }
        }
    }

    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

// ---- Extended String natives ----

/// Native: `String.toUpperCase()` — returns a new uppercase String.
pub(crate) fn native_string_touppercase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.to_uppercase());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.toLowerCase()` — returns a new lowercase String.
pub(crate) fn native_string_tolowercase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.to_lowercase());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.replace(char, char)` — replaces all occurrences of old char with new char.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_string_replace_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let old_char = char::from_u32(extract_int_arg(args, 1)?.cast_unsigned()).unwrap_or('?');
    let new_char = char::from_u32(extract_int_arg(args, 2)?.cast_unsigned()).unwrap_or('?');
    let result = s.replace(old_char, &new_char.to_string());
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.replace(CharSequence, CharSequence)` — replaces all occurrences of target with replacement.
pub(crate) fn native_string_replace_charsequence(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let target_ref = extract_ref_arg(args, 1)?;
    let target = heap
        .get(target_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let replacement_ref = extract_ref_arg(args, 2)?;
    let replacement = heap
        .get(replacement_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let result = s.replace(&*target, &replacement);
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.split(String)` — splits string by delimiter, returns String array.
pub(crate) fn native_string_split(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let delim_ref = extract_ref_arg(args, 1)?;
    let delim = heap
        .get(delim_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    // Use regex split (Java's String.split uses regex); remove trailing empty strings
    // to match Java's default split behaviour.
    // Special case: split("") splits into individual chars (Java 21 semantics — no leading "").
    let parts: Vec<String> = if delim.is_empty() {
        s.chars().map(|c| c.to_string()).collect()
    } else {
        regex::Regex::new(&delim).map_or_else(
            |_| s.split(delim.as_str()).map(str::to_string).collect(),
            |re| {
                let mut v: Vec<String> = re.split(&s).map(str::to_string).collect();
                while v.last().is_some_and(String::is_empty) {
                    v.pop();
                }
                v
            },
        )
    };
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), parts.len());
    for (i, part) in parts.iter().enumerate() {
        let str_ref = heap.allocate_string(part.clone());
        heap.get_mut(arr_ref)?.fields[i] = Slot::Reference(Some(str_ref));
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `String.hashCode()` — Java's hash algorithm: `s[0]*31^(n-1) + s[1]*31^(n-2) + ... + s[n-1]`.
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn native_string_hashcode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let mut h: i32 = 0;
    for ch in s.chars() {
        h = h.wrapping_mul(31).wrapping_add(ch as i32);
    }
    Ok(Some(Slot::Int(h)))
}

/// Native: `String.toString()` — identity, returns `this`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_string_tostring(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(args.first().copied().unwrap_or(Slot::Reference(None))))
}

// ---- Math natives ----

/// Native: `Math.max(int, int)` — returns the larger value.
pub(crate) fn native_math_max_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.max(b))))
}

/// Native: `Math.min(int, int)` — returns the smaller value.
pub(crate) fn native_math_min_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.min(b))))
}

/// Native: `Math.abs(int)` — returns absolute value.
pub(crate) fn native_math_abs_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(a.wrapping_abs())))
}

/// Native: `Math.floorMod(int, int)` — remainder with the divisor's sign.
pub(crate) fn native_math_floor_mod_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    if b == 0 {
        return Err(VmError::DivisionByZero);
    }
    let remainder = a.wrapping_rem(b);
    let floor_mod = if remainder != 0 && (remainder < 0) != (b < 0) {
        remainder.wrapping_add(b)
    } else {
        remainder
    };
    Ok(Some(Slot::Int(floor_mod)))
}

// ---- Extended Math natives ----

/// Native: `Math.sqrt(double)` — returns square root.
pub(crate) fn native_math_sqrt(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.sqrt())))
}

/// Native: `Math.pow(double, double)` — returns a raised to the power b.
pub(crate) fn native_math_pow(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    let b = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(a.powf(b))))
}

/// Native: `Math.floor(double)` — returns floor value.
pub(crate) fn native_math_floor(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.floor())))
}

/// Native: `Math.ceil(double)` — returns ceiling value.
pub(crate) fn native_math_ceil(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.ceil())))
}

/// Native: `Math.round(double)` — returns closest long.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_math_round_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Long(a.round() as i64)))
}

/// Native: `Math.abs(long)` — returns absolute value.
pub(crate) fn native_math_abs_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Long(a.wrapping_abs())))
}

/// Native: `Math.abs(double)` — returns absolute value.
pub(crate) fn native_math_abs_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.abs())))
}

/// Native: `Math.max(long, long)` — returns the larger value.
pub(crate) fn native_math_max_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(a.max(b))))
}

/// Native: `Math.min(long, long)` — returns the smaller value.
pub(crate) fn native_math_min_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(a.min(b))))
}

/// Native: `Math.max(double, double)` — returns the larger value.
pub(crate) fn native_math_max_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    let b = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(a.max(b))))
}

/// Native: `Math.min(double, double)` — returns the smaller value.
pub(crate) fn native_math_min_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    let b = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(a.min(b))))
}

// ---- Extended Math trig / transcendental natives ----

/// Native: `Math.sin(double)` — sine (argument in radians).
pub(crate) fn native_math_sin(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.sin())))
}

/// Native: `Math.cos(double)` — cosine (argument in radians).
pub(crate) fn native_math_cos(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.cos())))
}

/// Native: `Math.tan(double)` — tangent (argument in radians).
pub(crate) fn native_math_tan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.tan())))
}

/// Native: `Math.asin(double)` — arc sine, result in [-π/2, π/2].
pub(crate) fn native_math_asin(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.asin())))
}

/// Native: `Math.acos(double)` — arc cosine, result in [0, π].
pub(crate) fn native_math_acos(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.acos())))
}

/// Native: `Math.atan(double)` — arc tangent, result in [-π/2, π/2].
pub(crate) fn native_math_atan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.atan())))
}

/// Native: `Math.atan2(double, double)` — angle of vector (y, x) in [-π, π].
pub(crate) fn native_math_atan2(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let y = extract_double_arg(args, 0)?;
    let x = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(y.atan2(x))))
}

/// Native: `Math.log(double)` — natural logarithm.
pub(crate) fn native_math_log(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.ln())))
}

/// Native: `Math.log10(double)` — base-10 logarithm.
pub(crate) fn native_math_log10(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.log10())))
}

/// Native: `Math.exp(double)` — Euler's number raised to the given power.
pub(crate) fn native_math_exp(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.exp())))
}

/// Native: `Math.signum(double)` — sign of a: -1.0, 0.0, or 1.0.
pub(crate) fn native_math_signum_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.signum())))
}

/// Native: `Math.signum(float)` — sign of a as float: -1.0, 0.0, or 1.0.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_math_signum_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_float_arg(args, 0)?;
    Ok(Some(Slot::Float(a.signum())))
}

/// Native: `Math.toRadians(double)` — converts degrees to radians.
pub(crate) fn native_math_to_radians(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.to_radians())))
}

/// Native: `Math.toDegrees(double)` — converts radians to degrees.
pub(crate) fn native_math_to_degrees(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.to_degrees())))
}

/// Native: `Math.cbrt(double)` — cube root.
pub(crate) fn native_math_cbrt(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.cbrt())))
}

/// Native: `Math.hypot(double, double)` — sqrt(x²+y²) without overflow.
pub(crate) fn native_math_hypot(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let x = extract_double_arg(args, 0)?;
    let y = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(x.hypot(y))))
}

/// Native: `Math.floorDiv(int, int)` — largest int ≤ quotient.
pub(crate) fn native_math_floor_div_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    if b == 0 {
        return Err(VmError::DivisionByZero);
    }
    Ok(Some(Slot::Int(
        a.div_euclid(b) - i32::from(a.wrapping_rem(b) != 0 && (a < 0) != (b < 0)),
    )))
}

/// Native: `Math.round(float)` — rounds float to nearest int.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_math_round_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_float_arg(args, 0)?;
    Ok(Some(Slot::Int(a.round() as i32)))
}

// ---- System.arraycopy native ----

/// Native: `System.arraycopy(Object src, int srcPos, Object dst, int dstPos, int length)`.
/// Copies `length` elements from `src` starting at `srcPos` into `dst` starting at `dstPos`.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_system_arraycopy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let src_pos = extract_int_arg(args, 1)?;
    let dst_ref = match args.get(2) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let dst_pos = extract_int_arg(args, 3)?;
    let length = extract_int_arg(args, 4)?;
    if length < 0 || src_pos < 0 || dst_pos < 0 {
        return Err(VmError::NegativeArraySize {
            size: length.min(src_pos).min(dst_pos),
        });
    }
    let src_pos = src_pos as usize;
    let dst_pos = dst_pos as usize;
    let length = length as usize;
    // Copy elements one by one to support src == dst (overlapping ranges handled via clone).
    let src_len = heap.get(src_ref)?.fields.len();
    if src_pos + length > src_len {
        return Err(VmError::ArrayIndexOutOfBounds {
            index: i32::try_from(src_pos + length - 1).unwrap_or(i32::MAX),
            length: src_len,
        });
    }
    let src_elems: Vec<Slot> = heap.get(src_ref)?.fields[src_pos..src_pos + length].to_vec();
    let dst_len = heap.get(dst_ref)?.fields.len();
    if dst_pos + length > dst_len {
        return Err(VmError::ArrayIndexOutOfBounds {
            index: i32::try_from(dst_pos + length - 1).unwrap_or(i32::MAX),
            length: dst_len,
        });
    }
    let dst_fields = &mut heap.get_mut(dst_ref)?.fields;
    for (i, slot) in src_elems.into_iter().enumerate() {
        dst_fields[dst_pos + i] = slot;
    }
    Ok(None)
}

// ---- HashMap / Map$Entry iteration natives ----

/// Native: `Map$Entry.getKey()Object` — returns the key field.
pub(crate) fn native_map_entry_get_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    Ok(Some(key))
}

/// Native: `Map$Entry.getValue()Object` — returns the value field.
pub(crate) fn native_map_entry_get_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    Ok(Some(val))
}

/// Native: `HashMap.keySet()` — returns a new `HashSet` containing all keys.
pub(crate) fn native_hashmap_key_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(this_ref)?.fields.clone();
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    let mut i = 1usize;
    while i < fields.len() {
        let key = fields[i];
        native_hashset_add(&[Slot::Reference(Some(set_ref)), key], heap, out, control)?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}

/// Native: `HashMap.values()` — returns a new `ArrayList` containing all values.
/// `HashMap` fields: `[size, key0, val0, key1, val1, ...]`; values are at even indices 2, 4, 6, ...
pub(crate) fn native_hashmap_values(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(this_ref)?.fields.clone();
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    // Pairs start at index 1; values are at indices 2, 4, 6, ...
    let mut i = 2usize;
    while i < fields.len() {
        let val = fields[i];
        native_arraylist_add(&[Slot::Reference(Some(list_ref)), val], heap, out, control)?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}

/// Native: `HashMap.entrySet()` — returns a new `HashSet` of `java/util/Map$Entry` objects.
pub(crate) fn native_hashmap_entry_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(this_ref)?.fields.clone();
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    let mut i = 1usize;
    while i + 1 < fields.len() {
        let key = fields[i];
        let val = fields[i + 1];
        let entry_ref = heap.allocate("java/util/Map$Entry".to_string(), 2);
        {
            let entry_obj = heap.get_mut(entry_ref)?;
            entry_obj.fields[0] = key;
            entry_obj.fields[1] = val;
        }
        native_hashset_add(
            &[
                Slot::Reference(Some(set_ref)),
                Slot::Reference(Some(entry_ref)),
            ],
            heap,
            out,
            control,
        )?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}

// ---- Long class natives ----

/// Native: `Long.parseLong(String)` — parses string to long.
pub(crate) fn native_long_parselong(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_i64_from_string_arg(args, heap)?;
    Ok(Some(Slot::Long(val)))
}

/// Native: `Long.parseLong(String,int)` — parses string to long with radix.
pub(crate) fn native_long_parselong_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_i64_from_string_and_radix_args(args, heap)?;
    Ok(Some(Slot::Long(val)))
}

/// Native: `Long.valueOf(long)` — boxes long into Long object.
pub(crate) fn native_long_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.valueOf(String)` — parses and boxes long.
pub(crate) fn native_long_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_i64_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.valueOf(String,int)` — parses and boxes long with radix.
pub(crate) fn native_long_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_i64_from_string_and_radix_args(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.decode(String)` — parses prefixed string and boxes long.
pub(crate) fn native_long_decode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_i64_decode_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.longValue()` — unboxes Long to long.
pub(crate) fn native_long_longvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

/// Native: `Long.toString(long)` — static, converts long to String.
pub(crate) fn native_long_tostring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.toHexString(long)` — unsigned lowercase hex string.
pub(crate) fn native_long_tohexstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(format!("{val:x}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.toOctalString(long)` — unsigned octal string.
pub(crate) fn native_long_tooctalstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(format!("{val:o}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.toBinaryString(long)` — unsigned binary string.
pub(crate) fn native_long_tobinarystring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(format!("{val:b}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.compareUnsigned(long,long)` — compares longs as unsigned 64-bit values.
pub(crate) fn native_long_compareunsigned_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = u64::from_ne_bytes(extract_long_arg(args, 0)?.to_ne_bytes());
    let b = u64::from_ne_bytes(extract_long_arg(args, 1)?.to_ne_bytes());
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Native: `Long.compareTo(Object)` — compares two boxed Longs.
pub(crate) fn native_long_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let long_val = |s: &Slot| -> VmResult<i64> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Long(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => long_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = long_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

// ---- Double class natives ----

/// Native: `Double.parseDouble(String)` — parses string to double.
pub(crate) fn native_double_parsedouble(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f64 = s.trim().parse().map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Double(val)))
}

/// Native: `Double.valueOf(double)` — boxes double into Double object.
pub(crate) fn native_double_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_double_arg(args, 0)?;
    let r = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Double(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Double.doubleValue()` — unboxes Double to double.
pub(crate) fn native_double_doublevalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

// ---- Float class native ----

pub(crate) fn native_float_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_float_arg(args, 0)?;
    let r = heap.allocate("java/lang/Float".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Float(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_float_floatvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

pub(crate) fn native_float_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let float_val = |s: &Slot| -> VmResult<f32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Float(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => float_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = float_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.total_cmp(&b)))))
}

/// Native: `Float.parseFloat(String)` — parses string to float.
pub(crate) fn native_float_parsefloat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f32 = s.trim().parse().map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Float(val)))
}

// ---- Boolean class native ----

pub(crate) fn native_boolean_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Boolean".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(i32::from(val != 0));
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_boolean_booleanvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

pub(crate) fn native_boolean_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let bool_val = |s: &Slot| -> VmResult<bool> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n != 0),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => bool_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = bool_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Native: `Boolean.parseBoolean(String)` — case-insensitive "true" → 1, else 0.
pub(crate) fn native_boolean_parseboolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let s = heap.get(*r)?.string_value.clone().unwrap_or_default();
            let val = s.eq_ignore_ascii_case("true");
            Ok(Some(Slot::Int(i32::from(val))))
        }
        Some(Slot::Reference(None)) => Ok(Some(Slot::Int(0))),
        _ => Err(VmError::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

fn parse_bounded_i32_from_string_arg(
    args: &[Slot],
    heap: &duke_gc::Heap,
    min: i32,
    max: i32,
) -> VmResult<i32> {
    let s = extract_string_arg_value(args, 0, heap)?;
    parse_bounded_i32_with_radix(&s, 10, min, max)
}

fn parse_bounded_i32_from_string_and_radix_args(
    args: &[Slot],
    heap: &duke_gc::Heap,
    min: i32,
    max: i32,
) -> VmResult<i32> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let radix = extract_parse_radix_arg(args, 1)?;
    parse_bounded_i32_with_radix(&s, radix, min, max)
}

fn parse_bounded_i32_with_radix(s: &str, radix: u32, min: i32, max: i32) -> VmResult<i32> {
    let val = i32::from_str_radix(s.trim(), radix).map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    if val < min || val > max {
        return Err(VmError::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    Ok(val)
}

fn parse_i64_from_string_arg(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<i64> {
    let s = extract_string_arg_value(args, 0, heap)?;
    parse_i64_with_radix(&s, 10)
}

fn parse_i64_from_string_and_radix_args(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<i64> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let radix = extract_parse_radix_arg(args, 1)?;
    parse_i64_with_radix(&s, radix)
}

fn parse_i64_with_radix(s: &str, radix: u32) -> VmResult<i64> {
    i64::from_str_radix(s.trim(), radix).map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })
}

fn parse_i32_decode_from_string_arg(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<i32> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let val = parse_i128_decode(&s)?;
    if val < i128::from(i32::MIN) || val > i128::from(i32::MAX) {
        return Err(VmError::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    i32::try_from(val).map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })
}

fn parse_i64_decode_from_string_arg(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<i64> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let val = parse_i128_decode(&s)?;
    if val < i128::from(i64::MIN) || val > i128::from(i64::MAX) {
        return Err(VmError::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    i64::try_from(val).map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })
}

fn parse_i128_decode(s: &str) -> VmResult<i128> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(VmError::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }

    let (negative, sign_len) = match trimmed.as_bytes().first() {
        Some(b'+') => (false, 1),
        Some(b'-') => (true, 1),
        _ => (false, 0),
    };

    let rest = &trimmed[sign_len..];
    let (radix, prefix_len) = if rest.starts_with("0x") || rest.starts_with("0X") {
        (16, 2)
    } else if rest.starts_with('#') {
        (16, 1)
    } else if rest.starts_with('0') && rest.len() > 1 {
        (8, 1)
    } else {
        (10, 0)
    };

    let digits = &rest[prefix_len..];
    if digits.is_empty() || digits.starts_with('+') || digits.starts_with('-') {
        return Err(VmError::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }

    let magnitude = i128::from_str_radix(digits, radix).map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(if negative { -magnitude } else { magnitude })
}

fn extract_string_arg_value(args: &[Slot], index: usize, heap: &duke_gc::Heap) -> VmResult<String> {
    let str_ref = extract_ref_arg(args, index)?;
    Ok(heap.get(str_ref)?.string_value.clone().unwrap_or_default())
}

fn extract_parse_radix_arg(args: &[Slot], index: usize) -> VmResult<u32> {
    let radix = extract_int_arg(args, index)?;
    if !(2..=36).contains(&radix) {
        return Err(VmError::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    Ok(u32::try_from(radix).unwrap_or(0))
}

pub(crate) fn native_byte_parsebyte(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i8::MIN), i32::from(i8::MAX))?;
    Ok(Some(Slot::Int(val)))
}

pub(crate) fn native_byte_parsebyte_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i8::MIN),
        i32::from(i8::MAX),
    )?;
    Ok(Some(Slot::Int(val)))
}

pub(crate) fn native_byte_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_byte_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i8::MIN), i32::from(i8::MAX))?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_byte_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i8::MIN),
        i32::from(i8::MAX),
    )?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_byte_bytevalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

pub(crate) fn native_byte_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let byte_val = |s: &Slot| -> VmResult<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => byte_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = byte_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

pub(crate) fn native_short_parseshort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i16::MIN), i32::from(i16::MAX))?;
    Ok(Some(Slot::Int(val)))
}

pub(crate) fn native_short_parseshort_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i16::MIN),
        i32::from(i16::MAX),
    )?;
    Ok(Some(Slot::Int(val)))
}

pub(crate) fn native_short_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_short_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i16::MIN), i32::from(i16::MAX))?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_short_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i16::MIN),
        i32::from(i16::MAX),
    )?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_short_shortvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

pub(crate) fn native_short_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let short_val = |s: &Slot| -> VmResult<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => short_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = short_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

pub(crate) fn native_char_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let char_val = |s: &Slot| -> VmResult<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => char_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = char_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Execute a `StringConcatFactory` recipe: walk the recipe string, replacing
/// `\u{1}` placeholders with stringified dynamic args from the operand stack.
fn execute_string_concat_recipe(
    recipe: &str,
    dynamic_args: &[Slot],
    arg_types: &[char],
    constants: &[String],
    heap: &mut duke_gc::Heap,
) -> VmResult<Slot> {
    let mut result = String::new();
    let mut dyn_idx = 0;
    let mut const_idx = 0;

    for ch in recipe.chars() {
        match ch {
            '\u{1}' => {
                if dyn_idx < dynamic_args.len() {
                    let type_hint = arg_types.get(dyn_idx).copied().unwrap_or('I');
                    stringify_slot(&dynamic_args[dyn_idx], type_hint, heap, &mut result)?;
                    dyn_idx += 1;
                }
            }
            '\u{2}' => {
                if const_idx < constants.len() {
                    result.push_str(&constants[const_idx]);
                    const_idx += 1;
                }
            }
            other => result.push(other),
        }
    }

    let r = heap.allocate_string(result);
    Ok(Slot::Reference(Some(r)))
}

/// Convert a Slot to its string representation (like Java's String.valueOf).
/// `type_hint` is the JVM type descriptor char: 'Z' for boolean, 'I' for int, etc.
fn stringify_slot(
    slot: &Slot,
    type_hint: char,
    heap: &duke_gc::Heap,
    out: &mut String,
) -> VmResult<()> {
    match slot {
        Slot::Int(v) => {
            if type_hint == 'Z' {
                // JVM boolean: 0 = false, nonzero = true.
                out.push_str(if *v != 0 { "true" } else { "false" });
            } else if type_hint == 'C' {
                // JVM char: render as the Unicode character.
                if let Some(ch) = char::from_u32((*v).cast_unsigned()) {
                    out.push(ch);
                } else {
                    out.push('?');
                }
            } else {
                out.push_str(&v.to_string());
            }
        }
        Slot::Long(v) => out.push_str(&v.to_string()),
        Slot::Float(v) => out.push_str(&format_java_float(*v)),
        Slot::Double(v) => out.push_str(&format_java_double(*v)),
        Slot::Reference(None) => out.push_str("null"),
        Slot::Reference(Some(r)) => {
            out.push_str(&heap_object_to_string(heap.get(*r)?, *r));
        }
        Slot::ReturnAddress(v) => out.push_str(&v.to_string()),
    }
    Ok(())
}

/// Format a float like Java's Float.toString.
fn format_java_float(v: f32) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    let s = format!("{v}");
    if s.contains('.') { s } else { format!("{v}.0") }
}

/// Format a double like Java's Double.toString.
fn format_java_double(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    let s = format!("{v}");
    if s.contains('.') { s } else { format!("{v}.0") }
}

/// Execute a decoded JVM instruction stream.
///
/// # Parameters
/// - `instructions`: output of `duke_bytecode::decode`
/// - `cp`: constant pool from the parsed `ClassFile`
/// - `args`: initial local variable values (method arguments)
/// - `max_stack`: `Code.max_stack` from the class file
/// - `max_locals`: `Code.max_locals` from the class file
///
/// # Returns
/// `Ok(Some(slot))` for value-returning methods, `Ok(None)` for `void`.
///
/// # Errors
/// Returns [`VmError`] on execution faults (division by zero, stack overflow,
/// unimplemented instruction, etc.).
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
pub fn execute(
    instructions: &[(usize, Instruction)],
    cp: &[Option<CpEntry>],
    args: Vec<Slot>,
    max_stack: u16,
    max_locals: u16,
) -> VmResult<Option<Slot>> {
    // Build PC → instruction-index map for O(1) branch resolution.
    let pc_to_idx: HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(i, &(pc, _))| (pc, i))
        .collect();

    let mut frame = Frame::new(usize::from(max_stack), usize::from(max_locals), args)?;
    let mut idx: usize = 0;
    // Local heap for array objects allocated during single-method execution.
    let mut local_heap: Vec<(String, Vec<Slot>, Option<String>)> = Vec::new();
    let mut string_intern: HashMap<usize, u64> = HashMap::new();

    loop {
        let Some((pc, instr)) = instructions.get(idx) else {
            return Err(VmError::FellOffEnd);
        };
        let pc = *pc;

        // Jump to a PC-relative branch target (offset relative to current `pc`).
        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64).wrapping_add(i64::from($offset)) as usize;
                idx = *pc_to_idx
                    .get(&target)
                    .ok_or(VmError::InvalidBranchTarget { pc: target })?;
                continue;
            }};
        }

        match instr {
            // ----------------------------------------------------------------
            // Constants
            // ----------------------------------------------------------------
            Instruction::Nop => {}
            Instruction::AconstNull => frame.push(Slot::Reference(None))?,
            Instruction::IconstM1 => frame.push(Slot::Int(-1))?,
            Instruction::Iconst0 => frame.push(Slot::Int(0))?,
            Instruction::Iconst1 => frame.push(Slot::Int(1))?,
            Instruction::Iconst2 => frame.push(Slot::Int(2))?,
            Instruction::Iconst3 => frame.push(Slot::Int(3))?,
            Instruction::Iconst4 => frame.push(Slot::Int(4))?,
            Instruction::Iconst5 => frame.push(Slot::Int(5))?,
            Instruction::Lconst0 => frame.push(Slot::Long(0))?,
            Instruction::Lconst1 => frame.push(Slot::Long(1))?,
            Instruction::Fconst0 => frame.push(Slot::Float(0.0))?,
            Instruction::Fconst1 => frame.push(Slot::Float(1.0))?,
            Instruction::Fconst2 => frame.push(Slot::Float(2.0))?,
            Instruction::Dconst0 => frame.push(Slot::Double(0.0))?,
            Instruction::Dconst1 => frame.push(Slot::Double(1.0))?,
            Instruction::Bipush(v) => frame.push(Slot::Int(i32::from(*v)))?,
            Instruction::Sipush(v) => frame.push(Slot::Int(i32::from(*v)))?,

            // ----------------------------------------------------------------
            // Constant pool load
            // ----------------------------------------------------------------
            Instruction::Ldc(raw_idx) => {
                let cp_idx = usize::from(*raw_idx);
                if let Some(CpEntry::String { string_index }) =
                    cp.get(cp_idx).and_then(|e| e.as_ref())
                {
                    let si = string_index.0 as usize;
                    let r = if let Some(&cached) = string_intern.get(&cp_idx) {
                        cached
                    } else {
                        let s = match cp.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        let r = local_heap.len() as u64;
                        local_heap.push(("java/lang/String".to_string(), Vec::new(), Some(s)));
                        string_intern.insert(cp_idx, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    ldc_push(&mut frame, cp, cp_idx)?;
                }
            }
            Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
                let idx_val = usize::from(cp_idx.0);
                if let Some(CpEntry::String { string_index }) =
                    cp.get(idx_val).and_then(|e| e.as_ref())
                {
                    let si = string_index.0 as usize;
                    let r = if let Some(&cached) = string_intern.get(&idx_val) {
                        cached
                    } else {
                        let s = match cp.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        let r = local_heap.len() as u64;
                        local_heap.push(("java/lang/String".to_string(), Vec::new(), Some(s)));
                        string_intern.insert(idx_val, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    ldc_push(&mut frame, cp, idx_val)?;
                }
            }

            // ----------------------------------------------------------------
            // Loads
            // ----------------------------------------------------------------
            Instruction::Iload(i)
            | Instruction::Lload(i)
            | Instruction::Fload(i)
            | Instruction::Dload(i)
            | Instruction::Aload(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }
            Instruction::Iload0
            | Instruction::Lload0
            | Instruction::Fload0
            | Instruction::Dload0
            | Instruction::Aload0 => {
                let s = frame.load_local(0)?;
                frame.push(s)?;
            }
            Instruction::Iload1
            | Instruction::Lload1
            | Instruction::Fload1
            | Instruction::Dload1
            | Instruction::Aload1 => {
                let s = frame.load_local(1)?;
                frame.push(s)?;
            }
            Instruction::Iload2
            | Instruction::Lload2
            | Instruction::Fload2
            | Instruction::Dload2
            | Instruction::Aload2 => {
                let s = frame.load_local(2)?;
                frame.push(s)?;
            }
            Instruction::Iload3
            | Instruction::Lload3
            | Instruction::Fload3
            | Instruction::Dload3
            | Instruction::Aload3 => {
                let s = frame.load_local(3)?;
                frame.push(s)?;
            }
            Instruction::IloadW(i)
            | Instruction::LloadW(i)
            | Instruction::FloadW(i)
            | Instruction::DloadW(i)
            | Instruction::AloadW(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }

            // ----------------------------------------------------------------
            // Stores
            // ----------------------------------------------------------------
            Instruction::Istore(i)
            | Instruction::Lstore(i)
            | Instruction::Fstore(i)
            | Instruction::Dstore(i)
            | Instruction::Astore(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }
            Instruction::Istore0
            | Instruction::Lstore0
            | Instruction::Fstore0
            | Instruction::Dstore0
            | Instruction::Astore0 => {
                let v = frame.pop()?;
                frame.store_local(0, v)?;
            }
            Instruction::Istore1
            | Instruction::Lstore1
            | Instruction::Fstore1
            | Instruction::Dstore1
            | Instruction::Astore1 => {
                let v = frame.pop()?;
                frame.store_local(1, v)?;
            }
            Instruction::Istore2
            | Instruction::Lstore2
            | Instruction::Fstore2
            | Instruction::Dstore2
            | Instruction::Astore2 => {
                let v = frame.pop()?;
                frame.store_local(2, v)?;
            }
            Instruction::Istore3
            | Instruction::Lstore3
            | Instruction::Fstore3
            | Instruction::Dstore3
            | Instruction::Astore3 => {
                let v = frame.pop()?;
                frame.store_local(3, v)?;
            }
            Instruction::IstoreW(i)
            | Instruction::LstoreW(i)
            | Instruction::FstoreW(i)
            | Instruction::DstoreW(i)
            | Instruction::AstoreW(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }

            // ----------------------------------------------------------------
            // Stack manipulation
            // ----------------------------------------------------------------
            Instruction::Pop => {
                frame.pop()?;
            }
            Instruction::Pop2 => {
                frame.pop()?;
                frame.pop()?;
            }
            Instruction::Dup => {
                let v = frame.pop()?;
                frame.push(v)?;
                frame.push(v)?;
            }
            Instruction::DupX1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::DupX2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                let v4 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v4)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Swap => {
                let a = frame.pop()?;
                let b = frame.pop()?;
                frame.push(a)?;
                frame.push(b)?;
            }

            // ----------------------------------------------------------------
            // Integer arithmetic
            // ----------------------------------------------------------------
            Instruction::Iadd => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_add(b)))?;
            }
            Instruction::Isub => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_sub(b)))?;
            }
            Instruction::Imul => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_mul(b)))?;
            }
            Instruction::Idiv => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_div(b)))?;
            }
            Instruction::Irem => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_rem(b)))?;
            }
            Instruction::Ineg => {
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_neg()))?;
            }
            Instruction::Ishl => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shl((s & 0x1F) as u32)))?;
            }
            Instruction::Ishr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shr((s & 0x1F) as u32)))?;
            }
            Instruction::Iushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(((a as u32) >> (s as u32 & 0x1F)) as i32))?;
            }
            Instruction::Iand => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a & b))?;
            }
            Instruction::Ior => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a | b))?;
            }
            Instruction::Ixor => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a ^ b))?;
            }
            Instruction::Iinc { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }
            Instruction::IincW { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }

            // ----------------------------------------------------------------
            // Long arithmetic
            // ----------------------------------------------------------------
            Instruction::Ladd => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_add(b)))?;
            }
            Instruction::Lsub => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_sub(b)))?;
            }
            Instruction::Lmul => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_mul(b)))?;
            }
            Instruction::Ldiv => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_div(b)))?;
            }
            Instruction::Lrem => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_rem(b)))?;
            }
            Instruction::Lneg => {
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_neg()))?;
            }
            Instruction::Lshl => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shl((s & 0x3F) as u32)))?;
            }
            Instruction::Lshr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shr((s & 0x3F) as u32)))?;
            }
            Instruction::Lushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(((a as u64) >> (s as u32 & 0x3F)) as i64))?;
            }
            Instruction::Land => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a & b))?;
            }
            Instruction::Lor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a | b))?;
            }
            Instruction::Lxor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a ^ b))?;
            }
            Instruction::Lcmp => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                let r = match a.cmp(&b) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Float arithmetic
            // ----------------------------------------------------------------
            Instruction::Fadd => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a + b))?;
            }
            Instruction::Fsub => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a - b))?;
            }
            Instruction::Fmul => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a * b))?;
            }
            Instruction::Fdiv => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a / b))?;
            }
            Instruction::Frem => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a % b))?;
            }
            Instruction::Fneg => {
                let a = frame.pop_float()?;
                frame.push(Slot::Float(-a))?;
            }
            Instruction::Fcmpl | Instruction::Fcmpg => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                // Java semantics: exact bit equality for 0 check; NaN handled separately.
                #[allow(clippy::float_cmp)]
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Fcmpg) {
                    1 // NaN result: Fcmpg pushes 1, Fcmpl pushes -1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Double arithmetic
            // ----------------------------------------------------------------
            Instruction::Dadd => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a + b))?;
            }
            Instruction::Dsub => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a - b))?;
            }
            Instruction::Dmul => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a * b))?;
            }
            Instruction::Ddiv => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a / b))?;
            }
            Instruction::Drem => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a % b))?;
            }
            Instruction::Dneg => {
                let a = frame.pop_double()?;
                frame.push(Slot::Double(-a))?;
            }
            Instruction::Dcmpl | Instruction::Dcmpg => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                // Java semantics: exact bit equality for 0 check; NaN handled separately.
                #[allow(clippy::float_cmp)]
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Dcmpg) {
                    1 // NaN result: Dcmpg pushes 1, Dcmpl pushes -1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Type conversions
            // ----------------------------------------------------------------
            Instruction::I2l => {
                let v = frame.pop_int()?;
                frame.push(Slot::Long(i64::from(v)))?;
            }
            Instruction::I2f => {
                let v = frame.pop_int()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2d => {
                let v = frame.pop_int()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::L2i => {
                let v = frame.pop_long()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::L2f => {
                let v = frame.pop_long()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::L2d => {
                let v = frame.pop_long()?;
                frame.push(Slot::Double(v as f64))?;
            }
            Instruction::F2i => {
                let v = frame.pop_float()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::F2l => {
                let v = frame.pop_float()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::F2d => {
                let v = frame.pop_float()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::D2i => {
                let v = frame.pop_double()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::D2l => {
                let v = frame.pop_double()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::D2f => {
                let v = frame.pop_double()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2b => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i8)))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as u16)))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i16)))?;
            }

            // ----------------------------------------------------------------
            // Returns
            // ----------------------------------------------------------------
            Instruction::Return => return Ok(None),
            Instruction::Ireturn => return Ok(Some(Slot::Int(frame.pop_int()?))),
            Instruction::Lreturn => return Ok(Some(Slot::Long(frame.pop_long()?))),
            Instruction::Freturn => return Ok(Some(Slot::Float(frame.pop_float()?))),
            Instruction::Dreturn => return Ok(Some(Slot::Double(frame.pop_double()?))),

            // ----------------------------------------------------------------
            // Branches
            // ----------------------------------------------------------------
            Instruction::Goto(offset) => jump!(*offset),
            Instruction::GotoW(offset) => jump!(*offset),

            Instruction::Ifeq(offset) => {
                if frame.pop_int()? == 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifne(offset) => {
                if frame.pop_int()? != 0 {
                    jump!(*offset);
                }
            }
            Instruction::Iflt(offset) => {
                if frame.pop_int()? < 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifge(offset) => {
                if frame.pop_int()? >= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifgt(offset) => {
                if frame.pop_int()? > 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifle(offset) => {
                if frame.pop_int()? <= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifnull(offset) => {
                if matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::Ifnonnull(offset) => {
                if !matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpeq(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpne(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a != b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmplt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a < b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpge(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a >= b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpgt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a > b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmple(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a <= b {
                    jump!(*offset);
                }
            }
            // Reference comparisons
            Instruction::IfAcmpeq(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfAcmpne(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a != b {
                    jump!(*offset);
                }
            }

            // ----------------------------------------------------------------
            // Array allocation
            // ----------------------------------------------------------------
            Instruction::Newarray(array_type) => {
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let class_name = match array_type {
                    ArrayType::Boolean => "[Z",
                    ArrayType::Char => "[C",
                    ArrayType::Float => "[F",
                    ArrayType::Double => "[D",
                    ArrayType::Byte => "[B",
                    ArrayType::Short => "[S",
                    ArrayType::Int => "[I",
                    ArrayType::Long => "[J",
                };
                let init_slot = match array_type {
                    ArrayType::Long => Slot::Long(0),
                    ArrayType::Float => Slot::Float(0.0),
                    ArrayType::Double => Slot::Double(0.0),
                    _ => Slot::Int(0),
                };
                let r = local_heap.len() as u64;
                local_heap.push((
                    class_name.to_string(),
                    vec![init_slot; count as usize],
                    None,
                ));
                frame.push(Slot::Reference(Some(r)))?;
            }
            Instruction::Anewarray(cp_idx) => {
                let element_type = resolve_class_name(cp, usize::from(cp_idx.0))?;
                let array_type = format!("[L{element_type};");
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let r = local_heap.len() as u64;
                local_heap.push((
                    array_type,
                    vec![Slot::Reference(None); count as usize],
                    None,
                ));
                frame.push(Slot::Reference(Some(r)))?;
            }
            Instruction::Arraylength => {
                let r = frame.pop_ref()?;
                let len = local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1
                    .len();
                frame.push(Slot::Int(len as i32))?;
            }

            // ---- Int array ----
            Instruction::Iaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_int()?;
                frame.push(Slot::Int(v))?;
            }
            Instruction::Iastore => {
                let val = frame.pop_int()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Long array ----
            Instruction::Laload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_long()?;
                frame.push(Slot::Long(v))?;
            }
            Instruction::Lastore => {
                let val = frame.pop_long()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Long(val);
            }

            // ---- Float array ----
            Instruction::Faload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_float()?;
                frame.push(Slot::Float(v))?;
            }
            Instruction::Fastore => {
                let val = frame.pop_float()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Float(val);
            }

            // ---- Double array ----
            Instruction::Daload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_double()?;
                frame.push(Slot::Double(v))?;
            }
            Instruction::Dastore => {
                let val = frame.pop_double()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Double(val);
            }

            // ---- Reference array ----
            Instruction::Aaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize];
                frame.push(v)?;
            }
            Instruction::Aastore => {
                let val = frame.pop()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = val;
            }

            // ---- Byte/boolean array (stored as Int, truncated to i8) ----
            Instruction::Baload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as i8);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = i32::from(frame.pop_int()? as i8);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Char array (stored as Int, masked to u16) ----
            Instruction::Caload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as u16);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = i32::from(frame.pop_int()? as u16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Short array (stored as Int, truncated to i16) ----
            Instruction::Saload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as i16);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = i32::from(frame.pop_int()? as i16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Switch ----
            Instruction::Tableswitch {
                default,
                low,
                high,
                offsets,
            } => {
                let key = frame.pop_int()?;
                let offset = if key >= *low && key <= *high {
                    offsets[(key - low) as usize]
                } else {
                    *default
                };
                jump!(offset);
            }
            Instruction::Lookupswitch { default, pairs } => {
                let key = frame.pop_int()?;
                let offset = pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or(*default, |(_, off)| *off);
                jump!(offset);
            }

            // ---- checkcast / instanceof ----
            Instruction::Checkcast(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(slot)?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = resolve_class_name(cp, usize::from(cp_idx.0))?;
                        let actual = local_heap
                            .get(*r as usize)
                            .ok_or(VmError::InvalidRef { address: *r })?
                            .0
                            .clone();
                        if actual == target {
                            frame.push(slot)?;
                        } else {
                            return Err(VmError::ClassCastException {
                                from: actual,
                                to: target,
                            });
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }
            Instruction::Instanceof(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(Slot::Int(0))?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = resolve_class_name(cp, usize::from(cp_idx.0))?;
                        let actual = local_heap
                            .get(*r as usize)
                            .ok_or(VmError::InvalidRef { address: *r })?
                            .0
                            .clone();
                        if actual == target {
                            frame.push(Slot::Int(1))?;
                        } else {
                            frame.push(Slot::Int(0))?;
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }

            // ---- athrow ----
            Instruction::Athrow => {
                let r = frame.pop_ref()?;
                let class_name = local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .0
                    .clone();
                return Err(VmError::JavaException { class_name });
            }

            // ---- monitor (no-op, single-threaded) ----
            Instruction::Monitorenter | Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }

            other => {
                return Err(VmError::Unimplemented {
                    mnemonic: other.mnemonic(),
                });
            }
        }

        idx += 1;
    }
}

/// Ensure a class is initialized. Runs `<clinit>` if present and not yet run.
///
/// Must be called before first active use of a class (new, getstatic, putstatic, invokestatic).
/// `triggered_by` names the class that caused this init (empty string for the entry-point class).
#[allow(clippy::used_underscore_binding, clippy::cast_possible_truncation)]
fn ensure_initialized(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    _triggered_by: &str,
) -> VmResult<()> {
    if registry.is_initialized(class_name) {
        return Ok(());
    }
    // Mark as initialized BEFORE running clinit to prevent infinite recursion.
    registry.mark_initialized(class_name);
    initialize_primitive_wrapper_type_field(registry, heap, class_name)?;

    // Check if the class has a <clinit> method.
    let has_clinit = registry.get(class_name).is_ok_and(|ctx| {
        ctx.methods
            .iter()
            .any(|m| m.name == "<clinit>" && m.descriptor == "()V")
    });

    if has_clinit {
        let class_loader = registry.class_loader(class_name).cloned();
        let init_loader = class_loader.as_deref().map_or(loader, |v| v);
        // Run <clinit> by calling it through execute_class.
        #[cfg(feature = "telemetry")]
        #[allow(clippy::used_underscore_binding)]
        let _clinit_start = std::time::Instant::now();
        execute_class(
            registry,
            init_loader,
            heap,
            stdout,
            class_name,
            "<clinit>",
            "()V",
            &[],
        )?;
        #[cfg(feature = "telemetry")]
        registry.telemetry.class_init_dag.record(
            class_name,
            _triggered_by,
            _clinit_start.elapsed().as_nanos() as u64,
        );
    }
    Ok(())
}

fn primitive_wrapper_type_descriptor(class_name: &str) -> Option<&'static str> {
    match class_name {
        "java/lang/Boolean" => Some("Z"),
        "java/lang/Byte" => Some("B"),
        "java/lang/Character" => Some("C"),
        "java/lang/Double" => Some("D"),
        "java/lang/Float" => Some("F"),
        "java/lang/Integer" => Some("I"),
        "java/lang/Long" => Some("J"),
        "java/lang/Short" => Some("S"),
        "java/lang/Void" => Some("V"),
        _ => None,
    }
}

fn initialize_primitive_wrapper_type_field(
    registry: &mut ClassRegistry,
    heap: &mut duke_gc::Heap,
    class_name: &str,
) -> VmResult<()> {
    let Some(descriptor) = primitive_wrapper_type_descriptor(class_name) else {
        return Ok(());
    };
    let type_slot = {
        let ctx = registry.get(class_name)?;
        static_field_idx(ctx, "TYPE")?
    };
    if matches!(
        registry.get(class_name)?.static_fields.get(type_slot),
        Some(Slot::Reference(Some(_)))
    ) {
        return Ok(());
    }
    let class_ref = allocate_class_object(heap, descriptor)?;
    registry.get_mut(class_name)?.static_fields[type_slot] = Slot::Reference(Some(class_ref));
    Ok(())
}

/// Reusable pool of frame backing buffers.
///
/// Eliminates per-call `Vec<Slot>` allocation for locals and operand stack.
/// On a recursive workload the pool reaches steady state after the first
/// call-depth calls; all subsequent frames are pool hits with zero allocation.
///
/// The pool is private to a single `execute_class` invocation.
struct FramePool {
    free: Vec<(Vec<Slot>, Vec<Slot>)>,
}

impl FramePool {
    const fn new() -> Self {
        Self { free: Vec::new() }
    }

    /// Acquire a `(locals_buf, stack_buf)` pair.
    /// Returns a pooled pair if available, otherwise allocates fresh Vecs.
    fn acquire(&mut self) -> (Vec<Slot>, Vec<Slot>) {
        self.free.pop().unwrap_or_default()
    }

    /// Return buffers to the pool.
    /// `stack` must already be empty (guaranteed by `Frame::into_pool_bufs`).
    /// Caps pool at 256 entries to bound memory usage.
    fn release(&mut self, locals: Vec<Slot>, stack: Vec<Slot>) {
        // Cap at 256 entries — typical max call depth is well under 100; anything
        // beyond this is dead weight. Entries dropped here are freed to the allocator.
        if self.free.len() < 256 {
            self.free.push((locals, stack));
        }
    }
}

struct ExecutionState {
    current_class: String,
    method_idx: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: std::sync::Arc<[(usize, Instruction)]>,
    frame: Frame,
    call_stack: Vec<CallFrame>,
    frame_pool: FramePool,
    dispatch_cache: HashMap<String, HashMap<u16, (String, usize, usize)>>,
    idx: usize,
    string_intern: HashMap<usize, u64>,
    #[cfg(feature = "telemetry")]
    current_method: String,
}

struct InterpreterCallbackOps<'a> {
    registry: &'a mut ClassRegistry,
    loader: &'a dyn ClassLoader,
}

#[allow(clippy::too_many_arguments, clippy::option_option)]
fn callback_invoke_registered_lambda(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    class: &str,
    method: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Option<Slot>>> {
    let Some(lambda_info) = registry.get_lambda(class).cloned() else {
        return Ok(None);
    };
    if method != lambda_info.sam_method || descriptor != lambda_info.sam_desc {
        return Ok(None);
    }

    let this_ref = match args.first().copied() {
        Some(Slot::Reference(Some(reference))) => reference,
        Some(Slot::Reference(None)) | None => return Err(VmError::NullPointerException),
        Some(_) => {
            return Err(VmError::TypeMismatch {
                expected: "reference",
                got: "other",
            });
        }
    };
    let lambda_object = heap.get(this_ref)?;
    let mut impl_args =
        Vec::with_capacity(lambda_info.captured_count + args.len().saturating_sub(1));
    for capture_index in 0..lambda_info.captured_count {
        let slot =
            lambda_object
                .fields
                .get(capture_index)
                .copied()
                .ok_or(VmError::Unimplemented {
                    mnemonic: "lambda capture missing",
                })?;
        impl_args.push(slot);
    }
    impl_args.extend_from_slice(&args[1..]);

    let dispatch_class = match lambda_info.impl_kind {
        6 | 7 => lambda_info.impl_class.clone(),
        5 | 9 => match impl_args.first().copied() {
            Some(Slot::Reference(Some(receiver_ref))) => {
                let receiver_class = heap.get(receiver_ref)?.class_name.clone();
                if has_registered_native_override(
                    registry,
                    &receiver_class,
                    &lambda_info.impl_method,
                    &lambda_info.impl_desc,
                ) {
                    receiver_class
                } else if let Some((dispatch_class, _)) = resolve_method_in_hierarchy(
                    registry,
                    loader,
                    &receiver_class,
                    &lambda_info.impl_method,
                    &lambda_info.impl_desc,
                ) {
                    dispatch_class
                } else {
                    lambda_info.impl_class.clone()
                }
            }
            Some(Slot::Reference(None)) | None => return Err(VmError::NullPointerException),
            Some(_) => {
                return Err(VmError::TypeMismatch {
                    expected: "reference",
                    got: "other",
                });
            }
        },
        _ => {
            return Err(VmError::Unimplemented {
                mnemonic: "unsupported lambda impl kind",
            });
        }
    };

    let impl_args = expand_args_for_desc(&impl_args, &lambda_info.impl_desc);
    Ok(Some(execute_class(
        registry,
        loader,
        heap,
        output,
        &dispatch_class,
        &lambda_info.impl_method,
        &lambda_info.impl_desc,
        &impl_args,
    )?))
}

impl CallbackOps for InterpreterCallbackOps<'_> {
    fn invoke(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
        method: &str,
        descriptor: &str,
        args: Vec<Slot>,
    ) -> VmResult<Option<Slot>> {
        if let Some(result) = callback_invoke_registered_lambda(
            self.registry,
            self.loader,
            heap,
            output,
            class,
            method,
            descriptor,
            &args,
        )? {
            return Ok(result);
        }
        execute_class(
            self.registry,
            self.loader,
            heap,
            output,
            class,
            method,
            descriptor,
            &args,
        )
    }

    fn ensure_loaded(&mut self, class: &str) -> VmResult<()> {
        match self.registry.resolve_loaded_class_key(class) {
            Ok(_) => return Ok(()),
            Err(VmError::ClassNotFound { .. }) => {}
            Err(err) => return Err(err),
        }
        if self.registry.ensure_loaded(class, self.loader)? {
            Ok(())
        } else {
            Err(VmError::ClassNotFound {
                name: class.to_string(),
            })
        }
    }

    fn inspect_class(&mut self, class: &str) -> VmResult<ReflectedClassInfo> {
        inspect_reflected_class(self.registry, self.loader, class)
    }

    fn ensure_class_initialized(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
    ) -> VmResult<()> {
        self.ensure_loaded(class)?;
        ensure_initialized(self.registry, self.loader, heap, output, class, "")
    }

    fn code_source_for_class(&mut self, class: &str) -> VmResult<Option<String>> {
        Ok(self
            .registry
            .code_source_for_class(class)
            .map(ToOwned::to_owned))
    }

    fn ensure_loaded_with_runtime_loader(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: u64,
        class: &str,
    ) -> VmResult<()> {
        if let Some(path) = launched_class_loader_archive_path(self.registry, heap, loader_ref)? {
            if self
                .registry
                .ensure_loaded_with_provenance(class, &path, Some(loader_ref))?
            {
                return Ok(());
            }
            return Err(VmError::ClassNotFound {
                name: class.to_string(),
            });
        }
        self.ensure_loaded(class)
    }

    fn instance_field_slot(&mut self, class: &str, field_name: &str) -> VmResult<usize> {
        field_slot_idx(self.registry, class, field_name)
    }

    fn read_instance_field(
        &mut self,
        heap: &duke_gc::Heap,
        object_ref: u64,
        declaring_class: &str,
        field_name: &str,
    ) -> VmResult<Slot> {
        let actual_class = heap.get(object_ref)?.class_name.clone();
        if !is_assignable_from(
            self.registry,
            self.loader,
            &actual_class,
            declaring_class,
            Some(declaring_class),
        ) {
            return Err(VmError::JavaException {
                class_name: "java/lang/IllegalArgumentException".to_string(),
            });
        }
        let slot = field_slot_idx(self.registry, declaring_class, field_name)?;
        Ok(heap.get(object_ref)?.fields[slot])
    }

    fn write_instance_field(
        &mut self,
        heap: &mut duke_gc::Heap,
        object_ref: u64,
        declaring_class: &str,
        field_name: &str,
        value: Slot,
    ) -> VmResult<()> {
        let actual_class = heap.get(object_ref)?.class_name.clone();
        if !is_assignable_from(
            self.registry,
            self.loader,
            &actual_class,
            declaring_class,
            Some(declaring_class),
        ) {
            return Err(VmError::JavaException {
                class_name: "java/lang/IllegalArgumentException".to_string(),
            });
        }
        let slot = field_slot_idx(self.registry, declaring_class, field_name)?;
        heap.write_field(object_ref, slot, value)?;
        Ok(())
    }

    fn read_static_field(&mut self, class: &str, field_name: &str) -> VmResult<Slot> {
        let slot = {
            let ctx = self.registry.get(class)?;
            static_field_idx(ctx, field_name)?
        };
        Ok(self.registry.get(class)?.static_fields[slot])
    }

    fn write_static_field(&mut self, class: &str, field_name: &str, value: Slot) -> VmResult<()> {
        let slot = {
            let ctx = self.registry.get(class)?;
            static_field_idx(ctx, field_name)?
        };
        self.registry.get_mut(class)?.static_fields[slot] = value;
        Ok(())
    }

    fn runtime_loader_for_class(&mut self, class: &str) -> VmResult<Option<u64>> {
        Ok(self.registry.runtime_loader_for_class(class))
    }

    fn class_key_for_loaded_class(&mut self, class: &str) -> VmResult<String> {
        self.registry.resolve_loaded_class_key(class)
    }

    fn class_key_for_runtime_loader(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: u64,
        class: &str,
    ) -> VmResult<String> {
        if let Some(path) = launched_class_loader_archive_path(self.registry, heap, loader_ref)? {
            return Ok(self.registry.class_key_from_provenance(
                class,
                Some(&path),
                Some(loader_ref),
            ));
        }
        self.class_key_for_loaded_class(class)
    }

    fn class_key_from_source(
        &mut self,
        class: &str,
        source_class: Option<&str>,
    ) -> VmResult<String> {
        Ok(self.registry.class_key_from_source(class, source_class))
    }

    fn allocate_instance(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
    ) -> VmResult<u64> {
        allocate_reflection_instance(self.registry, self.loader, heap, output, class)
    }
}

impl ExecutionState {
    fn new(
        registry: &ClassRegistry,
        class_name: &str,
        #[cfg_attr(not(feature = "telemetry"), allow(unused_variables))] method_name: &str,
        entry_idx: usize,
        args: &[Slot],
    ) -> VmResult<Self> {
        let current_class = class_name.to_string();
        let pc_to_idx = {
            let ctx = registry.get(&current_class)?;
            std::sync::Arc::clone(&ctx.methods[entry_idx].pc_to_idx)
        };
        let frame = {
            let ctx = registry.get(&current_class)?;
            Frame::new(
                usize::from(ctx.methods[entry_idx].max_stack),
                usize::from(ctx.methods[entry_idx].max_locals),
                args.to_vec(),
            )?
        };
        let instructions = {
            let ctx = registry.get(&current_class)?;
            std::sync::Arc::clone(&ctx.methods[entry_idx].instructions)
        };

        Ok(Self {
            current_class,
            method_idx: entry_idx,
            pc_to_idx,
            instructions,
            frame,
            call_stack: Vec::new(),
            frame_pool: FramePool::new(),
            dispatch_cache: HashMap::new(),
            idx: 0,
            string_intern: HashMap::new(),
            #[cfg(feature = "telemetry")]
            current_method: method_name.to_string(),
        })
    }
}

#[cfg(feature = "telemetry")]
fn refresh_current_method_name(
    current_method: &mut String,
    registry: &ClassRegistry,
    current_class: &str,
    method_idx: usize,
) {
    *current_method = registry
        .get(current_class)
        .map(|c| {
            c.methods
                .get(method_idx)
                .map(|m| m.name.clone())
                .unwrap_or_default()
        })
        .unwrap_or_default();
}

#[allow(clippy::too_many_arguments)]
fn activate_method_state(
    frame: &mut Frame,
    method_idx: &mut usize,
    pc_to_idx: &mut std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: &mut std::sync::Arc<[(usize, Instruction)]>,
    current_class: &mut String,
    call_stack: &mut Vec<CallFrame>,
    registry: &ClassRegistry,
    callee_class: String,
    callee_idx: usize,
    callee_pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    callee_frame: Frame,
    resume_idx: usize,
    #[cfg(feature = "telemetry")] current_method: &mut String,
) -> VmResult<()> {
    call_stack.push(CallFrame {
        frame: std::mem::replace(frame, callee_frame),
        method_idx: *method_idx,
        pc_to_idx: std::sync::Arc::clone(pc_to_idx),
        resume_idx,
        class_name: current_class.clone(),
    });
    *method_idx = callee_idx;
    *pc_to_idx = callee_pc_to_idx;
    *current_class = callee_class;
    *instructions =
        std::sync::Arc::clone(&registry.get(current_class)?.methods[*method_idx].instructions);
    #[cfg(feature = "telemetry")]
    refresh_current_method_name(current_method, registry, current_class, *method_idx);
    Ok(())
}

enum ExecutionOutcome {
    Returned(Option<Slot>),
    ThreadAction(NativeThreadAction),
    /// The thread has exhausted its instruction quantum and should yield so
    /// other threads can make progress.
    Yield,
}

/// Default number of bytecode instructions a thread may execute before
/// yielding the VM lock, enabling fair interleaving of Java threads.
const DEFAULT_THREAD_QUANTUM: usize = 1024;

fn finish_native_call(
    native_control: &mut NativeControl,
    frame: &mut Frame,
    idx: &mut usize,
    result: Option<Slot>,
) -> VmResult<Option<ExecutionOutcome>> {
    if let Some(val) = result {
        frame.push(val)?;
    }
    *idx += 1;
    if let Some(action) = native_control.take() {
        return Ok(Some(ExecutionOutcome::ThreadAction(action)));
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
fn prepare_execution_state(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<ExecutionState> {
    let class_name = match registry.resolve_loaded_class_key(class_name) {
        Ok(class_key) => class_key,
        Err(VmError::ClassNotFound { .. }) => {
            if !registry.ensure_loaded(class_name, loader)? {
                return Err(VmError::ClassNotFound {
                    name: class_name.to_string(),
                });
            }
            registry.resolve_loaded_class_key(class_name)?
        }
        Err(err) => return Err(err),
    };
    let entry_idx = {
        let ctx = registry.get(&class_name)?;
        ctx.methods
            .iter()
            .position(|m| m.name == method_name && m.descriptor == descriptor)
            .ok_or_else(|| VmError::MethodNotFound {
                name: format!("{class_name}.{method_name}"),
                descriptor: descriptor.to_string(),
            })?
    };

    let class_loader = registry.class_loader(&class_name).cloned();
    let init_loader = class_loader.as_deref().map_or(loader, |v| v);
    ensure_initialized(registry, init_loader, heap, stdout, &class_name, "")?;
    ExecutionState::new(registry, &class_name, method_name, entry_idx, args)
}

#[cfg(feature = "telemetry")]
#[allow(clippy::too_many_lines)]
const fn instr_name(instr: &duke_bytecode::Instruction) -> &'static str {
    use duke_bytecode::Instruction as I;
    match instr {
        I::Nop => "nop",
        I::AconstNull => "aconst_null",
        I::Iconst0 | I::Iconst1 | I::Iconst2 | I::Iconst3 | I::Iconst4 | I::Iconst5 => "iconst_n",
        I::IconstM1 => "iconst_m1",
        I::Lconst0 | I::Lconst1 => "lconst_n",
        I::Fconst0 | I::Fconst1 | I::Fconst2 => "fconst_n",
        I::Dconst0 | I::Dconst1 => "dconst_n",
        I::Bipush(_) => "bipush",
        I::Sipush(_) => "sipush",
        I::Ldc(_) | I::LdcW(_) | I::Ldc2W(_) => "ldc",
        I::Iload(_) | I::Iload0 | I::Iload1 | I::Iload2 | I::Iload3 | I::IloadW(_) => "iload",
        I::Lload(_) | I::Lload0 | I::Lload1 | I::Lload2 | I::Lload3 | I::LloadW(_) => "lload",
        I::Fload(_) | I::Fload0 | I::Fload1 | I::Fload2 | I::Fload3 | I::FloadW(_) => "fload",
        I::Dload(_) | I::Dload0 | I::Dload1 | I::Dload2 | I::Dload3 | I::DloadW(_) => "dload",
        I::Aload(_) | I::Aload0 | I::Aload1 | I::Aload2 | I::Aload3 | I::AloadW(_) => "aload",
        I::Istore(_) | I::Istore0 | I::Istore1 | I::Istore2 | I::Istore3 | I::IstoreW(_) => {
            "istore"
        }
        I::Lstore(_) | I::Lstore0 | I::Lstore1 | I::Lstore2 | I::Lstore3 | I::LstoreW(_) => {
            "lstore"
        }
        I::Fstore(_) | I::Fstore0 | I::Fstore1 | I::Fstore2 | I::Fstore3 | I::FstoreW(_) => {
            "fstore"
        }
        I::Dstore(_) | I::Dstore0 | I::Dstore1 | I::Dstore2 | I::Dstore3 | I::DstoreW(_) => {
            "dstore"
        }
        I::Astore(_) | I::Astore0 | I::Astore1 | I::Astore2 | I::Astore3 | I::AstoreW(_) => {
            "astore"
        }
        I::Iaload => "iaload",
        I::Laload => "laload",
        I::Faload => "faload",
        I::Daload => "daload",
        I::Aaload => "aaload",
        I::Baload => "baload",
        I::Caload => "caload",
        I::Saload => "saload",
        I::Iastore => "iastore",
        I::Lastore => "lastore",
        I::Fastore => "fastore",
        I::Dastore => "dastore",
        I::Aastore => "aastore",
        I::Bastore => "bastore",
        I::Castore => "castore",
        I::Sastore => "sastore",
        I::Pop => "pop",
        I::Pop2 => "pop2",
        I::Dup => "dup",
        I::DupX1 => "dup_x1",
        I::DupX2 => "dup_x2",
        I::Dup2 => "dup2",
        I::Dup2X1 => "dup2_x1",
        I::Dup2X2 => "dup2_x2",
        I::Swap => "swap",
        I::Iadd => "iadd",
        I::Ladd => "ladd",
        I::Fadd => "fadd",
        I::Dadd => "dadd",
        I::Isub => "isub",
        I::Lsub => "lsub",
        I::Fsub => "fsub",
        I::Dsub => "dsub",
        I::Imul => "imul",
        I::Lmul => "lmul",
        I::Fmul => "fmul",
        I::Dmul => "dmul",
        I::Idiv => "idiv",
        I::Ldiv => "ldiv",
        I::Fdiv => "fdiv",
        I::Ddiv => "ddiv",
        I::Irem => "irem",
        I::Lrem => "lrem",
        I::Frem => "frem",
        I::Drem => "drem",
        I::Ineg => "ineg",
        I::Lneg => "lneg",
        I::Fneg => "fneg",
        I::Dneg => "dneg",
        I::Ishl => "ishl",
        I::Lshl => "lshl",
        I::Ishr => "ishr",
        I::Lshr => "lshr",
        I::Iushr => "iushr",
        I::Lushr => "lushr",
        I::Iand => "iand",
        I::Land => "land",
        I::Ior => "ior",
        I::Lor => "lor",
        I::Ixor => "ixor",
        I::Lxor => "lxor",
        I::Iinc { .. } | I::IincW { .. } => "iinc",
        I::I2l => "i2l",
        I::I2f => "i2f",
        I::I2d => "i2d",
        I::L2i => "l2i",
        I::L2f => "l2f",
        I::L2d => "l2d",
        I::F2i => "f2i",
        I::F2l => "f2l",
        I::F2d => "f2d",
        I::D2i => "d2i",
        I::D2l => "d2l",
        I::D2f => "d2f",
        I::I2b => "i2b",
        I::I2c => "i2c",
        I::I2s => "i2s",
        I::Lcmp => "lcmp",
        I::Fcmpl => "fcmpl",
        I::Fcmpg => "fcmpg",
        I::Dcmpl => "dcmpl",
        I::Dcmpg => "dcmpg",
        I::Ifeq(_) | I::Ifne(_) | I::Iflt(_) | I::Ifge(_) | I::Ifgt(_) | I::Ifle(_) => "if_<cond>",
        I::IfIcmpeq(_)
        | I::IfIcmpne(_)
        | I::IfIcmplt(_)
        | I::IfIcmpge(_)
        | I::IfIcmpgt(_)
        | I::IfIcmple(_) => "if_icmp<cond>",
        I::IfAcmpeq(_) | I::IfAcmpne(_) => "if_acmp<cond>",
        I::Goto(_) | I::GotoW(_) => "goto",
        I::Jsr(_) | I::JsrW(_) => "jsr",
        I::Ret(_) | I::RetW(_) => "ret",
        I::Tableswitch { .. } => "tableswitch",
        I::Lookupswitch { .. } => "lookupswitch",
        I::Ireturn => "ireturn",
        I::Lreturn => "lreturn",
        I::Freturn => "freturn",
        I::Dreturn => "dreturn",
        I::Areturn => "areturn",
        I::Return => "return",
        I::Getstatic(_) => "getstatic",
        I::Putstatic(_) => "putstatic",
        I::Getfield(_) => "getfield",
        I::Putfield(_) => "putfield",
        I::Invokevirtual(_) => "invokevirtual",
        I::Invokespecial(_) => "invokespecial",
        I::Invokestatic(_) => "invokestatic",
        I::Invokeinterface { .. } => "invokeinterface",
        I::Invokedynamic(_) => "invokedynamic",
        I::New(_) => "new",
        I::Newarray(_) => "newarray",
        I::Anewarray(_) => "anewarray",
        I::Arraylength => "arraylength",
        I::Athrow => "athrow",
        I::Checkcast(_) => "checkcast",
        I::Instanceof(_) => "instanceof",
        I::Monitorenter => "monitorenter",
        I::Monitorexit => "monitorexit",
        I::Multianewarray { .. } => "multianewarray",
        I::Ifnull(_) | I::Ifnonnull(_) => "ifnull/nonnull",
    }
}

/// Execute a static method by name within a loaded class context.
///
/// Supports `invokestatic` calls between methods in the same class.
///
/// # Errors
/// Returns [`VmError`] on execution faults or if `method_name`/`descriptor`
/// are not found in `ctx`.
///
/// # Panics
/// Panics on internal invariant violations, such as a `Class` CP entry whose
/// name index refers to a non-`Utf8` entry (indicates a malformed class file).
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::manual_let_else,
    clippy::single_match,
    clippy::single_match_else,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]
pub fn execute_class(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>> {
    // Fast path: if a native handler is registered for this class/method/descriptor,
    // dispatch it directly without requiring a ClassContext in the registry.
    // This handles both Simple natives and Callback natives at the top-level call site.
    match lookup_registered_native_kind(registry, class_name, method_name, descriptor) {
        Some(HandlerKind::Simple(h)) => {
            let mut native_control = NativeControl::default();
            let result = h(args, heap, stdout, &mut native_control)?;
            if native_control.take().is_some() {
                return Err(VmError::Unimplemented {
                    mnemonic: "thread action requires execute_class_to_completion",
                });
            }
            return Ok(result);
        }
        Some(HandlerKind::Callback(h)) => {
            let mut native_control = NativeControl::default();
            let result = {
                let mut callback_ops = InterpreterCallbackOps { registry, loader };
                h(args, heap, stdout, &mut native_control, &mut callback_ops)?
            };
            if native_control.take().is_some() {
                return Err(VmError::Unimplemented {
                    mnemonic: "thread action requires execute_class_to_completion",
                });
            }
            return Ok(result);
        }
        None => {}
    }

    let mut state = prepare_execution_state(
        registry,
        loader,
        heap,
        stdout,
        class_name,
        method_name,
        descriptor,
        args,
    )?;
    match run_execution(&mut state, registry, loader, heap, stdout, true, None)? {
        ExecutionOutcome::Returned(result) => Ok(result),
        ExecutionOutcome::ThreadAction(_) | ExecutionOutcome::Yield => {
            Err(VmError::Unimplemented {
                mnemonic: "thread action requires execute_class_to_completion",
            })
        }
    }
}

#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::manual_let_else,
    clippy::single_match,
    clippy::single_match_else,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]
fn run_execution(
    state: &mut ExecutionState,
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    gc_allowed: bool,
    quantum: Option<usize>,
) -> VmResult<ExecutionOutcome> {
    let ExecutionState {
        current_class,
        method_idx,
        pc_to_idx,
        instructions,
        frame,
        call_stack,
        frame_pool,
        dispatch_cache,
        idx,
        string_intern,
        #[cfg(feature = "telemetry")]
        current_method,
    } = state;

    let mut remaining = quantum.unwrap_or(usize::MAX);

    loop {
        if remaining == 0 {
            return Ok(ExecutionOutcome::Yield);
        }
        remaining = remaining.saturating_sub(1);

        let (pc, instr) = {
            let Some(&(pc, ref instr)) = instructions.get(*idx) else {
                return Err(VmError::FellOffEnd);
            };
            (pc, instr.clone())
        };

        if std::env::var_os("DUKE_TRACE_EXEC").is_some() {
            let method_name = registry
                .get(current_class)
                .ok()
                .and_then(|ctx| ctx.methods.get(*method_idx))
                .map_or("<unknown>", |method| method.name.as_str());
            eprintln!(
                "duke: trace {}::{}@{} {} stack={}",
                current_class,
                method_name,
                pc,
                instr.mnemonic(),
                frame.stack_len()
            );
        }

        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64).wrapping_add(i64::from($offset)) as usize;
                *idx = *pc_to_idx
                    .get(&target)
                    .ok_or(VmError::InvalidBranchTarget { pc: target })?;
                continue;
            }};
        }

        // Helper macro for return instructions: pop call stack or return to Rust.
        macro_rules! do_return {
            ($val:expr) => {{
                match call_stack.pop() {
                    None => return Ok(ExecutionOutcome::Returned($val)),
                    Some(caller) => {
                        let ret_val = $val;
                        // Recycle callee frame buffers before overwriting `frame`.
                        let old = std::mem::replace(frame, caller.frame);
                        let (l, s) = old.into_pool_bufs();
                        frame_pool.release(l, s);
                        *method_idx = caller.method_idx;
                        *pc_to_idx = caller.pc_to_idx;
                        *idx = caller.resume_idx;
                        *current_class = caller.class_name;
                        *instructions = std::sync::Arc::clone(
                            &registry.get(&current_class)?.methods[*method_idx].instructions,
                        );
                        #[cfg(feature = "telemetry")]
                        {
                            *current_method = registry
                                .get(&current_class)
                                .map(|c| {
                                    c.methods
                                        .get(*method_idx)
                                        .map(|m| m.name.clone())
                                        .unwrap_or_default()
                                })
                                .unwrap_or_default();
                        }
                        if let Some(v) = ret_val {
                            frame.push(v)?;
                        }
                        continue;
                    }
                }
            }};
        }

        macro_rules! propagate_java_exception {
            ($exc_class_name:expr, $exception_ref:expr, $throw_pc:expr) => {{
                let exc_class_name = $exc_class_name;
                let exception_ref = $exception_ref;

                #[cfg(feature = "telemetry")]
                let _telem_exc_event_idx = registry.telemetry.exception_flow.record_throw(
                    &exc_class_name,
                    &current_class,
                    &current_method,
                    $throw_pc,
                );

                // Clone the exception table to release the borrow on registry,
                // so find_exception_handler can use &mut registry for hierarchy checks.
                // ⚡ Bolt: `ExceptionEntry` now derives `Clone`, avoiding manual field-by-field mapping.
                let exc_table = registry.get(&current_class)?.methods[*method_idx]
                    .exception_table
                    .clone();
                let handler = find_exception_handler(
                    &exc_table,
                    $throw_pc,
                    &exc_class_name,
                    current_class,
                    registry,
                    loader,
                );
                if let Some(handler_pc) = handler {
                    #[cfg(feature = "telemetry")]
                    registry.telemetry.exception_flow.record_catch(
                        _telem_exc_event_idx,
                        &current_class,
                        &current_method,
                        handler_pc as usize,
                    );
                    frame.clear_stack();
                    frame.push(Slot::Reference(Some(exception_ref)))?;
                    *idx = *pc_to_idx.get(&(handler_pc as usize)).ok_or(
                        VmError::InvalidBranchTarget {
                            pc: handler_pc as usize,
                        },
                    )?;
                    continue;
                }

                loop {
                    match call_stack.pop() {
                        None => {
                            return Err(VmError::JavaException {
                                class_name: exc_class_name,
                            });
                        }
                        Some(caller) => {
                            // Recycle callee frame buffers before overwriting `frame` —
                            // mirrors the do_return! pattern to avoid a pool leak.
                            let old = std::mem::replace(frame, caller.frame);
                            let (l, s) = old.into_pool_bufs();
                            frame_pool.release(l, s);
                            *method_idx = caller.method_idx;
                            *pc_to_idx = caller.pc_to_idx;
                            *current_class = caller.class_name;
                            *instructions = std::sync::Arc::clone(
                                &registry.get(&current_class)?.methods[*method_idx].instructions,
                            );
                            #[cfg(feature = "telemetry")]
                            {
                                *current_method = registry
                                    .get(&current_class)
                                    .map(|c| {
                                        c.methods
                                            .get(*method_idx)
                                            .map(|m| m.name.clone())
                                            .unwrap_or_default()
                                    })
                                    .unwrap_or_default();
                            }

                            let (caller_exc_table, caller_pc) = {
                                let ctx = registry.get(&current_class)?;
                                let cpc = if caller.resume_idx > 0 {
                                    ctx.methods[*method_idx].instructions[caller.resume_idx - 1].0
                                } else {
                                    0
                                };
                                // ⚡ Bolt: `ExceptionEntry` now derives `Clone`, avoiding manual field-by-field mapping.
                                let tbl = ctx.methods[*method_idx]
                                    .exception_table
                                    .clone();
                                (tbl, cpc)
                            };
                            let handler = find_exception_handler(
                                &caller_exc_table,
                                caller_pc,
                                &exc_class_name,
                                current_class,
                                registry,
                                loader,
                            );

                            if let Some(handler_pc) = handler {
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.exception_flow.record_catch(
                                    _telem_exc_event_idx,
                                    &current_class,
                                    &current_method,
                                    handler_pc as usize,
                                );
                                frame.clear_stack();
                                frame.push(Slot::Reference(Some(exception_ref)))?;
                                *idx = *pc_to_idx.get(&(handler_pc as usize)).ok_or(
                                    VmError::InvalidBranchTarget {
                                        pc: handler_pc as usize,
                                    },
                                )?;
                                break;
                            }
                        }
                    }
                }
                continue;
            }};
        }

        // Telemetry: capture opcode name and start time before dispatch.
        // Arms that use `continue` (branches, invokes) will skip the post-match
        // recording for that iteration — timing is approximate for those opcodes.
        #[cfg(feature = "telemetry")]
        #[allow(clippy::used_underscore_binding)]
        let (_telem_name, _telem_pc, _telem_start) = {
            let name = instr_name(&instr);
            let pc_val = pc;
            (name, pc_val, std::time::Instant::now())
        };

        match &instr {
            // ---- invokestatic ----
            Instruction::Invokestatic(cp_idx) => {
                if let Some(&(ref cached_cls, cached_idx, cached_ac)) = dispatch_cache
                    .get(current_class.as_str())
                    .and_then(|m| m.get(&cp_idx.0))
                {
                    // Fast path: cache hit — skip CP walk and method search.
                    let (callee_pc_to_idx, callee_frame) = {
                        let ctx = registry.get(cached_cls)?;
                        let max_locals = usize::from(ctx.methods[cached_idx].max_locals);
                        let max_stack = usize::from(ctx.methods[cached_idx].max_stack);
                        let pci = std::sync::Arc::clone(&ctx.methods[cached_idx].pc_to_idx);
                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                        locals_buf.resize(max_locals, Slot::Int(0));
                        if cached_ac > max_locals {
                            return Err(VmError::LocalOutOfBounds {
                                index: cached_ac,
                                max_locals,
                            });
                        }
                        for i in (0..cached_ac).rev() {
                            locals_buf[i] = frame.pop()?;
                        }
                        let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                        (pci, f)
                    };
                    let callee_class = cached_cls.clone();
                    let callee_idx = cached_idx;
                    activate_method_state(
                        frame,
                        method_idx,
                        pc_to_idx,
                        instructions,
                        current_class,
                        call_stack,
                        registry,
                        callee_class,
                        callee_idx,
                        callee_pc_to_idx,
                        callee_frame,
                        *idx + 1,
                        #[cfg(feature = "telemetry")]
                        current_method,
                    )?;
                    *idx = 0;
                    continue;
                }
                // Slow path: full CP resolution + method search.
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let class_was_loaded = registry.ensure_loaded_from(
                    &callee_class,
                    Some(current_class.as_str()),
                    loader,
                )?;
                let callee_class_key =
                    registry.class_key_from_source(&callee_class, Some(current_class.as_str()));
                if class_was_loaded {
                    ensure_initialized(
                        registry,
                        loader,
                        heap,
                        stdout,
                        &callee_class_key,
                        current_class,
                    )?;
                }
                let callee_idx = if has_registered_native_override(
                    registry,
                    &callee_class_key,
                    &callee_name,
                    &callee_desc,
                ) {
                    None
                } else {
                    let ctx = registry.get(&callee_class_key)?;
                    ctx.methods
                        .iter()
                        .position(|m| m.name == callee_name && m.descriptor == callee_desc)
                };
                match callee_idx {
                    Some(callee_idx) => {
                        let arg_count = parse_arg_count(&callee_desc);
                        dispatch_cache
                            .entry(current_class.clone())
                            .or_default()
                            .insert(cp_idx.0, (callee_class_key.clone(), callee_idx, arg_count));
                        let (callee_pc_to_idx, callee_frame) = {
                            let ctx = registry.get(&callee_class_key)?;
                            let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                            let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                            let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                            let (mut locals_buf, stack_buf) = frame_pool.acquire();
                            locals_buf.resize(max_locals, Slot::Int(0));
                            if arg_count > max_locals {
                                return Err(VmError::LocalOutOfBounds {
                                    index: arg_count,
                                    max_locals,
                                });
                            }
                            for i in (0..arg_count).rev() {
                                locals_buf[i] = frame.pop()?;
                            }
                            let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                            (pci, f)
                        };
                        activate_method_state(
                            frame,
                            method_idx,
                            pc_to_idx,
                            instructions,
                            current_class,
                            call_stack,
                            registry,
                            callee_class_key,
                            callee_idx,
                            callee_pc_to_idx,
                            callee_frame,
                            *idx + 1,
                            #[cfg(feature = "telemetry")]
                            current_method,
                        )?;
                        *idx = 0;
                        continue;
                    }
                    None => {
                        // Check native registry before erroring.
                        let handler_kind = lookup_registered_native_kind(
                            registry,
                            &callee_class_key,
                            &callee_name,
                            &callee_desc,
                        );
                        match handler_kind {
                            Some(HandlerKind::Simple(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut native_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }

                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = NativeControl::default();
                                let result =
                                    handler(&native_args, heap, stdout, &mut native_control);
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.native_boundary.record_call(
                                    registry.internal_name_for_class(&callee_class_key),
                                    &callee_name,
                                    _native_start.elapsed().as_nanos() as u64,
                                    result.is_err(),
                                );
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(outcome) =
                                    finish_native_call(&mut native_control, frame, idx, result)?
                                {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            Some(HandlerKind::Callback(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut native_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = NativeControl::default();
                                let result = {
                                    let mut callback_ops =
                                        InterpreterCallbackOps { registry, loader };
                                    handler(
                                        &native_args,
                                        heap,
                                        stdout,
                                        &mut native_control,
                                        &mut callback_ops,
                                    )
                                };
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.native_boundary.record_call(
                                    registry.internal_name_for_class(&callee_class_key),
                                    &callee_name,
                                    _native_start.elapsed().as_nanos() as u64,
                                    result.is_err(),
                                );
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(outcome) =
                                    finish_native_call(&mut native_control, frame, idx, result)?
                                {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            None => {
                                return Err(VmError::MethodNotFound {
                                    name: format!(
                                        "{}.{callee_name}",
                                        registry.internal_name_for_class(&callee_class_key)
                                    ),
                                    descriptor: callee_desc,
                                });
                            }
                        }
                    }
                }
            }

            // ---- returns ----
            Instruction::Return => do_return!(None),
            Instruction::Ireturn => {
                let v = frame.pop_int()?;
                do_return!(Some(Slot::Int(v)));
            }
            Instruction::Lreturn => {
                let v = frame.pop_long()?;
                do_return!(Some(Slot::Long(v)));
            }
            Instruction::Freturn => {
                let v = frame.pop_float()?;
                do_return!(Some(Slot::Float(v)));
            }
            Instruction::Dreturn => {
                let v = frame.pop_double()?;
                do_return!(Some(Slot::Double(v)));
            }

            // ---- all other instructions: same as execute() ----
            Instruction::Nop => {}
            Instruction::AconstNull => frame.push(Slot::Reference(None))?,
            Instruction::IconstM1 => frame.push(Slot::Int(-1))?,
            Instruction::Iconst0 => frame.push(Slot::Int(0))?,
            Instruction::Iconst1 => frame.push(Slot::Int(1))?,
            Instruction::Iconst2 => frame.push(Slot::Int(2))?,
            Instruction::Iconst3 => frame.push(Slot::Int(3))?,
            Instruction::Iconst4 => frame.push(Slot::Int(4))?,
            Instruction::Iconst5 => frame.push(Slot::Int(5))?,
            Instruction::Lconst0 => frame.push(Slot::Long(0))?,
            Instruction::Lconst1 => frame.push(Slot::Long(1))?,
            Instruction::Fconst0 => frame.push(Slot::Float(0.0))?,
            Instruction::Fconst1 => frame.push(Slot::Float(1.0))?,
            Instruction::Fconst2 => frame.push(Slot::Float(2.0))?,
            Instruction::Dconst0 => frame.push(Slot::Double(0.0))?,
            Instruction::Dconst1 => frame.push(Slot::Double(1.0))?,
            Instruction::Bipush(v) => frame.push(Slot::Int(i32::from(*v)))?,
            Instruction::Sipush(v) => frame.push(Slot::Int(i32::from(*v)))?,
            Instruction::Ldc(raw_idx) => {
                let cp_idx = usize::from(*raw_idx);
                let string_info = {
                    let ctx = registry.get(current_class)?;
                    if let Some(CpEntry::String { string_index }) =
                        ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
                    {
                        let si = string_index.0 as usize;
                        let s = match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        Some(s)
                    } else {
                        None
                    }
                };
                if let Some(s) = string_info {
                    let r = if let Some(&cached) = string_intern.get(&cp_idx) {
                        cached
                    } else {
                        let r = heap.allocate_string(s);
                        string_intern.insert(cp_idx, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    // Check for Class constant
                    let class_info = {
                        let ctx = registry.get(current_class)?;
                        if let Some(CpEntry::Class { name_index }) =
                            ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
                        {
                            match ctx
                                .constant_pool
                                .get(name_index.0 as usize)
                                .and_then(|e| e.as_ref())
                            {
                                Some(CpEntry::Utf8(s)) => Some(s.clone()),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    };
                    if let Some(class_name) = class_info {
                        // Intern class literals using offset key to avoid collision with String interning
                        let intern_key = cp_idx + 100_000;
                        let r = if let Some(&cached) = string_intern.get(&intern_key) {
                            cached
                        } else {
                            let r = heap.allocate("java/lang/Class".to_string(), 0);
                            heap.get_mut(r)?.string_value = Some(class_name);
                            string_intern.insert(intern_key, r);
                            r
                        };
                        frame.push(Slot::Reference(Some(r)))?;
                    } else {
                        let ctx = registry.get(current_class)?;
                        ldc_push(frame, &ctx.constant_pool, cp_idx)?;
                    }
                }
            }
            Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
                let idx_val = usize::from(cp_idx.0);
                let string_info = {
                    let ctx = registry.get(current_class)?;
                    if let Some(CpEntry::String { string_index }) =
                        ctx.constant_pool.get(idx_val).and_then(|e| e.as_ref())
                    {
                        let si = string_index.0 as usize;
                        let s = match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        Some(s)
                    } else {
                        None
                    }
                };
                if let Some(s) = string_info {
                    let r = if let Some(&cached) = string_intern.get(&idx_val) {
                        cached
                    } else {
                        let r = heap.allocate_string(s);
                        string_intern.insert(idx_val, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    // Check for Class constant
                    let class_info = {
                        let ctx = registry.get(current_class)?;
                        if let Some(CpEntry::Class { name_index }) =
                            ctx.constant_pool.get(idx_val).and_then(|e| e.as_ref())
                        {
                            match ctx
                                .constant_pool
                                .get(name_index.0 as usize)
                                .and_then(|e| e.as_ref())
                            {
                                Some(CpEntry::Utf8(s)) => Some(s.clone()),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    };
                    if let Some(class_name) = class_info {
                        // Intern class literals using offset key to avoid collision with String interning
                        let intern_key = idx_val + 100_000;
                        let r = if let Some(&cached) = string_intern.get(&intern_key) {
                            cached
                        } else {
                            let r = heap.allocate("java/lang/Class".to_string(), 0);
                            heap.get_mut(r)?.string_value = Some(class_name);
                            string_intern.insert(intern_key, r);
                            r
                        };
                        frame.push(Slot::Reference(Some(r)))?;
                    } else {
                        let ctx = registry.get(current_class)?;
                        ldc_push(frame, &ctx.constant_pool, idx_val)?;
                    }
                }
            }
            Instruction::Iload(i)
            | Instruction::Lload(i)
            | Instruction::Fload(i)
            | Instruction::Dload(i)
            | Instruction::Aload(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }
            Instruction::Iload0
            | Instruction::Lload0
            | Instruction::Fload0
            | Instruction::Dload0
            | Instruction::Aload0 => {
                let s = frame.load_local(0)?;
                frame.push(s)?;
            }
            Instruction::Iload1
            | Instruction::Lload1
            | Instruction::Fload1
            | Instruction::Dload1
            | Instruction::Aload1 => {
                let s = frame.load_local(1)?;
                frame.push(s)?;
            }
            Instruction::Iload2
            | Instruction::Lload2
            | Instruction::Fload2
            | Instruction::Dload2
            | Instruction::Aload2 => {
                let s = frame.load_local(2)?;
                frame.push(s)?;
            }
            Instruction::Iload3
            | Instruction::Lload3
            | Instruction::Fload3
            | Instruction::Dload3
            | Instruction::Aload3 => {
                let s = frame.load_local(3)?;
                frame.push(s)?;
            }
            Instruction::IloadW(i)
            | Instruction::LloadW(i)
            | Instruction::FloadW(i)
            | Instruction::DloadW(i)
            | Instruction::AloadW(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }
            Instruction::Istore(i)
            | Instruction::Lstore(i)
            | Instruction::Fstore(i)
            | Instruction::Dstore(i)
            | Instruction::Astore(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }
            Instruction::Istore0
            | Instruction::Lstore0
            | Instruction::Fstore0
            | Instruction::Dstore0
            | Instruction::Astore0 => {
                let v = frame.pop()?;
                frame.store_local(0, v)?;
            }
            Instruction::Istore1
            | Instruction::Lstore1
            | Instruction::Fstore1
            | Instruction::Dstore1
            | Instruction::Astore1 => {
                let v = frame.pop()?;
                frame.store_local(1, v)?;
            }
            Instruction::Istore2
            | Instruction::Lstore2
            | Instruction::Fstore2
            | Instruction::Dstore2
            | Instruction::Astore2 => {
                let v = frame.pop()?;
                frame.store_local(2, v)?;
            }
            Instruction::Istore3
            | Instruction::Lstore3
            | Instruction::Fstore3
            | Instruction::Dstore3
            | Instruction::Astore3 => {
                let v = frame.pop()?;
                frame.store_local(3, v)?;
            }
            Instruction::IstoreW(i)
            | Instruction::LstoreW(i)
            | Instruction::FstoreW(i)
            | Instruction::DstoreW(i)
            | Instruction::AstoreW(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }
            Instruction::Pop => {
                frame.pop()?;
            }
            Instruction::Pop2 => {
                frame.pop()?;
                frame.pop()?;
            }
            Instruction::Dup => {
                let v = frame.pop()?;
                frame.push(v)?;
                frame.push(v)?;
            }
            Instruction::DupX1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::DupX2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                let v4 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v4)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Swap => {
                let a = frame.pop()?;
                let b = frame.pop()?;
                frame.push(a)?;
                frame.push(b)?;
            }
            Instruction::Iadd => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_add(b)))?;
            }
            Instruction::Isub => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_sub(b)))?;
            }
            Instruction::Imul => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_mul(b)))?;
            }
            Instruction::Idiv => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_div(b)))?;
            }
            Instruction::Irem => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_rem(b)))?;
            }
            Instruction::Ineg => {
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_neg()))?;
            }
            Instruction::Ishl => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shl((s & 0x1F) as u32)))?;
            }
            Instruction::Ishr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shr((s & 0x1F) as u32)))?;
            }
            Instruction::Iushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(((a as u32) >> (s as u32 & 0x1F)) as i32))?;
            }
            Instruction::Iand => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a & b))?;
            }
            Instruction::Ior => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a | b))?;
            }
            Instruction::Ixor => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a ^ b))?;
            }
            Instruction::Iinc { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }
            Instruction::IincW { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }
            Instruction::Ladd => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_add(b)))?;
            }
            Instruction::Lsub => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_sub(b)))?;
            }
            Instruction::Lmul => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_mul(b)))?;
            }
            Instruction::Ldiv => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_div(b)))?;
            }
            Instruction::Lrem => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_rem(b)))?;
            }
            Instruction::Lneg => {
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_neg()))?;
            }
            Instruction::Lshl => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shl((s & 0x3F) as u32)))?;
            }
            Instruction::Lshr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shr((s & 0x3F) as u32)))?;
            }
            Instruction::Lushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(((a as u64) >> (s as u32 & 0x3F)) as i64))?;
            }
            Instruction::Land => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a & b))?;
            }
            Instruction::Lor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a | b))?;
            }
            Instruction::Lxor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a ^ b))?;
            }
            Instruction::Lcmp => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                let r = match a.cmp(&b) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                frame.push(Slot::Int(r))?;
            }
            Instruction::Fadd => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a + b))?;
            }
            Instruction::Fsub => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a - b))?;
            }
            Instruction::Fmul => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a * b))?;
            }
            Instruction::Fdiv => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a / b))?;
            }
            Instruction::Frem => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a % b))?;
            }
            Instruction::Fneg => {
                let a = frame.pop_float()?;
                frame.push(Slot::Float(-a))?;
            }
            Instruction::Fcmpl | Instruction::Fcmpg => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Fcmpg) {
                    1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }
            Instruction::Dadd => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a + b))?;
            }
            Instruction::Dsub => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a - b))?;
            }
            Instruction::Dmul => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a * b))?;
            }
            Instruction::Ddiv => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a / b))?;
            }
            Instruction::Drem => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a % b))?;
            }
            Instruction::Dneg => {
                let a = frame.pop_double()?;
                frame.push(Slot::Double(-a))?;
            }
            Instruction::Dcmpl | Instruction::Dcmpg => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Dcmpg) {
                    1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }
            Instruction::I2l => {
                let v = frame.pop_int()?;
                frame.push(Slot::Long(i64::from(v)))?;
            }
            Instruction::I2f => {
                let v = frame.pop_int()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2d => {
                let v = frame.pop_int()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::L2i => {
                let v = frame.pop_long()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::L2f => {
                let v = frame.pop_long()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::L2d => {
                let v = frame.pop_long()?;
                frame.push(Slot::Double(v as f64))?;
            }
            Instruction::F2i => {
                let v = frame.pop_float()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::F2l => {
                let v = frame.pop_float()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::F2d => {
                let v = frame.pop_float()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::D2i => {
                let v = frame.pop_double()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::D2l => {
                let v = frame.pop_double()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::D2f => {
                let v = frame.pop_double()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2b => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i8)))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as u16)))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i16)))?;
            }
            Instruction::Goto(offset) => jump!(*offset),
            Instruction::GotoW(offset) => jump!(*offset),
            Instruction::Ifeq(offset) => {
                if frame.pop_int()? == 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifne(offset) => {
                if frame.pop_int()? != 0 {
                    jump!(*offset);
                }
            }
            Instruction::Iflt(offset) => {
                if frame.pop_int()? < 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifge(offset) => {
                if frame.pop_int()? >= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifgt(offset) => {
                if frame.pop_int()? > 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifle(offset) => {
                if frame.pop_int()? <= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifnull(offset) => {
                if matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::Ifnonnull(offset) => {
                if !matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpeq(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpne(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a != b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmplt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a < b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpge(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a >= b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpgt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a > b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmple(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a <= b {
                    jump!(*offset);
                }
            }
            Instruction::IfAcmpeq(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfAcmpne(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a != b {
                    jump!(*offset);
                }
            }

            // ---- Object allocation ----
            Instruction::New(cp_idx) => {
                let target_class = {
                    let ctx = registry.get(current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let target_class_key =
                    registry.class_key_from_source(&target_class, Some(current_class.as_str()));
                registry.ensure_loaded_from(&target_class, Some(current_class.as_str()), loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &target_class_key,
                    current_class,
                )?;
                // Walk the super chain to sum all instance field counts
                // (e.g. Enum has 2 fields inherited by every enum subclass).
                let field_count = total_instance_field_count(registry, &target_class_key);
                #[cfg(feature = "telemetry")]
                registry.telemetry.object_lineage.record(
                    current_class,
                    current_method,
                    pc,
                    &target_class_key,
                );
                let r = heap.allocate(target_class_key.clone(), field_count);
                // Set reference/long/float/double fields to their JVM-spec defaults.
                // heap.allocate initialises everything to Int(0), which is wrong
                // for reference-typed fields (should be Reference(None)).
                init_object_fields(registry, heap, r, &target_class_key);
                frame.push(Slot::Reference(Some(r)))?;
                if gc_allowed && heap.should_gc() {
                    let roots = gather_roots(frame, call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(frame, call_stack, registry, heap);
                }
            }

            // ---- Field access ----
            Instruction::Getfield(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let target_class_key =
                    registry.class_key_from_source(&target_class, Some(current_class.as_str()));
                let r = frame.pop_ref()?;
                registry.ensure_loaded_from(&target_class, Some(current_class.as_str()), loader)?;
                let fidx = field_slot_idx(registry, &target_class_key, &field_name)?;
                let val = heap.get(r)?.fields[fidx];
                frame.push(val)?;
            }
            Instruction::Putfield(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let target_class_key =
                    registry.class_key_from_source(&target_class, Some(current_class.as_str()));
                let val = frame.pop()?;
                let r = frame.pop_ref()?;
                registry.ensure_loaded_from(&target_class, Some(current_class.as_str()), loader)?;
                let fidx = field_slot_idx(registry, &target_class_key, &field_name)?;
                heap.write_field(r, fidx, val)?;
            }
            Instruction::Getstatic(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let target_class_key =
                    registry.class_key_from_source(&target_class, Some(current_class.as_str()));
                registry.ensure_loaded_from(&target_class, Some(current_class.as_str()), loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &target_class_key,
                    current_class,
                )?;
                let sidx = static_field_idx(registry.get(&target_class_key)?, &field_name)?;
                let val = registry.get(&target_class_key)?.static_fields[sidx];
                frame.push(val)?;
            }
            Instruction::Putstatic(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let target_class_key =
                    registry.class_key_from_source(&target_class, Some(current_class.as_str()));
                let val = frame.pop()?;
                registry.ensure_loaded_from(&target_class, Some(current_class.as_str()), loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &target_class_key,
                    current_class,
                )?;
                let sidx = static_field_idx(registry.get(&target_class_key)?, &field_name)?;
                registry.get_mut(&target_class_key)?.static_fields[sidx] = val;
            }

            // ---- Instance method dispatch ----
            //
            // invokespecial and invokevirtual: resolve class+name+descriptor from
            // the Methodref.  Dispatch cross-class via registry; unloadable
            // classes (e.g. java/lang/Object) fall back to no-op.
            Instruction::Invokespecial(cp_idx) | Instruction::Invokevirtual(cp_idx) => {
                // Fast path: cache hit for invokespecial (static dispatch — safe to cache).
                if matches!(instr, Instruction::Invokespecial(_))
                    && let Some(&(ref cached_cls, cached_idx, cached_ac)) = dispatch_cache
                        .get(current_class.as_str())
                        .and_then(|m| m.get(&cp_idx.0))
                {
                    let (callee_pc_to_idx, callee_frame) = {
                        let ctx = registry.get(cached_cls)?;
                        let max_locals = usize::from(ctx.methods[cached_idx].max_locals);
                        let max_stack = usize::from(ctx.methods[cached_idx].max_stack);
                        let pci = std::sync::Arc::clone(&ctx.methods[cached_idx].pc_to_idx);
                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                        locals_buf.resize(max_locals, Slot::Int(0));
                        if cached_ac + 1 > max_locals {
                            return Err(VmError::LocalOutOfBounds {
                                index: cached_ac + 1,
                                max_locals,
                            });
                        }
                        for i in (1..=cached_ac).rev() {
                            locals_buf[i] = frame.pop()?;
                        }
                        locals_buf[0] = frame.pop()?; // `this`
                        let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                        (pci, f)
                    };
                    let dispatch_class = cached_cls.clone();
                    let callee_idx = cached_idx;
                    activate_method_state(
                        frame,
                        method_idx,
                        pc_to_idx,
                        instructions,
                        current_class,
                        call_stack,
                        registry,
                        dispatch_class,
                        callee_idx,
                        callee_pc_to_idx,
                        callee_frame,
                        *idx + 1,
                        #[cfg(feature = "telemetry")]
                        current_method,
                    )?;
                    *idx = 0;
                    continue;
                }
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let callee_class_key =
                    registry.class_key_from_source(&callee_class, Some(current_class.as_str()));
                // <clinit> (static initialiser) is not supported yet — skip silently.
                if callee_name == "<clinit>" {
                    *idx += 1;
                    continue;
                }
                // Attempt to load the target class; soft-fail for unloadable.
                let class_was_loaded = registry.ensure_loaded_from(
                    &callee_class,
                    Some(current_class.as_str()),
                    loader,
                )?;
                let virtual_start: Option<String> =
                    if matches!(instr, Instruction::Invokevirtual(_)) {
                        let arg_count = parse_arg_count(&callee_desc);
                        let stack_len = frame.stack_len();
                        if stack_len > arg_count {
                            let this_pos = stack_len - arg_count - 1;
                            if let Ok(Slot::Reference(Some(r))) = frame.peek_at(this_pos) {
                                heap.get(r).ok().map(|o| o.class_name.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                let resolved = if let Some(runtime_class) = virtual_start.as_ref() {
                    match resolve_method_in_hierarchy_lookup(
                        registry,
                        loader,
                        runtime_class,
                        &callee_name,
                        &callee_desc,
                    ) {
                        MethodHierarchyLookup::Bytecode(class_name, method_idx) => {
                            MethodHierarchyLookup::Bytecode(class_name, method_idx)
                        }
                        MethodHierarchyLookup::NativeOverride => {
                            MethodHierarchyLookup::NativeOverride
                        }
                        MethodHierarchyLookup::Missing if class_was_loaded => {
                            resolve_method_in_hierarchy_lookup(
                                registry,
                                loader,
                                &callee_class_key,
                                &callee_name,
                                &callee_desc,
                            )
                        }
                        MethodHierarchyLookup::Missing => MethodHierarchyLookup::Missing,
                    }
                } else if class_was_loaded {
                    resolve_method_in_hierarchy_lookup(
                        registry,
                        loader,
                        &callee_class_key,
                        &callee_name,
                        &callee_desc,
                    )
                } else {
                    MethodHierarchyLookup::Missing
                };
                let allow_lambda_dispatch = matches!(resolved, MethodHierarchyLookup::Missing);
                let (dispatch_class, callee_idx) = match resolved {
                    MethodHierarchyLookup::Bytecode(cls, i) => (cls, i),
                    MethodHierarchyLookup::NativeOverride | MethodHierarchyLookup::Missing => {
                        // Check lambda dispatch before native fallback.
                        if allow_lambda_dispatch {
                            let arg_count = parse_arg_count(&callee_desc);
                            let stack_len = frame.stack_len();
                            if stack_len > arg_count
                                && let Some(ref actual_class) = virtual_start
                                && let Some(lambda_info) =
                                    registry.get_lambda(actual_class).cloned()
                                && callee_name == lambda_info.sam_method
                            {
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut sam_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    sam_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?;
                                let this_ref = match &this_slot {
                                    Slot::Reference(Some(r)) => *r,
                                    _ => return Err(VmError::NullPointerException),
                                };

                                let obj = heap.get(this_ref)?;
                                let mut impl_args: Vec<Slot> = Vec::new();
                                for i in 0..lambda_info.captured_count {
                                    impl_args.push(obj.fields[i]);
                                }
                                impl_args.extend(sam_args.iter().copied());

                                let _ = registry.ensure_loaded_from(
                                    &lambda_info.impl_class,
                                    Some(current_class.as_str()),
                                    loader,
                                );
                                let impl_class_key = registry.class_key_from_source(
                                    &lambda_info.impl_class,
                                    Some(current_class.as_str()),
                                );

                                let resolved = resolve_method_in_hierarchy(
                                    registry,
                                    loader,
                                    &impl_class_key,
                                    &lambda_info.impl_method,
                                    &lambda_info.impl_desc,
                                );
                                if let Some((dispatch_class, impl_idx)) = resolved {
                                    let (callee_pc_to_idx, callee_frame) = {
                                        let ctx = registry.get(&dispatch_class)?;
                                        let max_locals =
                                            usize::from(ctx.methods[impl_idx].max_locals);
                                        let max_stack =
                                            usize::from(ctx.methods[impl_idx].max_stack);
                                        let pci =
                                            std::sync::Arc::clone(&ctx.methods[impl_idx].pc_to_idx);
                                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                        locals_buf.resize(max_locals, Slot::Int(0));
                                        let expanded_impl_args = expand_args_for_desc(
                                            &impl_args,
                                            &lambda_info.impl_desc,
                                        );
                                        for (i, slot) in expanded_impl_args.into_iter().enumerate()
                                        {
                                            if i < max_locals {
                                                locals_buf[i] = slot;
                                            }
                                        }
                                        let f =
                                            Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                                        (pci, f)
                                    };
                                    activate_method_state(
                                        frame,
                                        method_idx,
                                        pc_to_idx,
                                        instructions,
                                        current_class,
                                        call_stack,
                                        registry,
                                        dispatch_class,
                                        impl_idx,
                                        callee_pc_to_idx,
                                        callee_frame,
                                        *idx + 1,
                                        #[cfg(feature = "telemetry")]
                                        current_method,
                                    )?;
                                    *idx = 0;
                                    continue;
                                }
                            }
                        }
                        // Check native registry, walking the super chain.
                        let native_handler_kind = {
                            let mut found = None;

                            // Walk from runtime class first (invokevirtual virtual dispatch).
                            if let Some(ref runtime_class) = virtual_start {
                                let mut sc: Option<String> = Some(runtime_class.clone());
                                while let Some(ref s) = sc {
                                    if let Some(h) = lookup_registered_native_kind(
                                        registry,
                                        s,
                                        &callee_name,
                                        &callee_desc,
                                    ) {
                                        found = Some(h);
                                        break;
                                    }
                                    sc = registry.get(s).ok().and_then(|c| c.super_class.clone());
                                }
                            }

                            // Fall back to declared (callee_class) super chain.
                            if found.is_none() {
                                found = lookup_registered_native_kind(
                                    registry,
                                    &callee_class_key,
                                    &callee_name,
                                    &callee_desc,
                                );
                            }
                            if found.is_none() {
                                // Walk super chain for native lookup (e.g. Enum.ordinal
                                // called via SimpleEnum$Color.ordinal).
                                let start = if callee_class_key.starts_with('[') {
                                    // Arrays inherit from Object.
                                    Some("java/lang/Object".to_string())
                                } else {
                                    registry
                                        .get(&callee_class_key)
                                        .ok()
                                        .and_then(|c| c.super_class.clone())
                                };
                                let mut sc = start;
                                while let Some(ref s) = sc {
                                    if let Some(h) = lookup_registered_native_kind(
                                        registry,
                                        s,
                                        &callee_name,
                                        &callee_desc,
                                    ) {
                                        found = Some(h);
                                        break;
                                    }
                                    sc = registry.get(s).ok().and_then(|c| c.super_class.clone());
                                }
                            }
                            found
                        };
                        match native_handler_kind {
                            Some(HandlerKind::Simple(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut native_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }

                                let this_slot = frame.pop()?; // pop `this`
                                native_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = NativeControl::default();
                                let result =
                                    handler(&native_args, heap, stdout, &mut native_control);
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        registry.internal_name_for_class(&callee_class_key),
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    if matches!(instr, Instruction::Invokevirtual(_)) {
                                        // Native methods do not perform a bytecode hierarchy
                                        // walk — hierarchy_walk is always false here.
                                        registry.telemetry.dispatch_resolution.record(
                                            current_class,
                                            cp_idx.0,
                                            registry.internal_name_for_class(&callee_class_key),
                                            false,
                                        );
                                    }
                                }
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(outcome) =
                                    finish_native_call(&mut native_control, frame, idx, result)?
                                {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            Some(HandlerKind::Callback(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut native_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?; // pop `this`
                                native_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = NativeControl::default();
                                let result = {
                                    let mut callback_ops =
                                        InterpreterCallbackOps { registry, loader };
                                    handler(
                                        &native_args,
                                        heap,
                                        stdout,
                                        &mut native_control,
                                        &mut callback_ops,
                                    )
                                };
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        registry.internal_name_for_class(&callee_class_key),
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    if matches!(instr, Instruction::Invokevirtual(_)) {
                                        registry.telemetry.dispatch_resolution.record(
                                            current_class,
                                            cp_idx.0,
                                            registry.internal_name_for_class(&callee_class_key),
                                            false,
                                        );
                                    }
                                }
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(outcome) =
                                    finish_native_call(&mut native_control, frame, idx, result)?
                                {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            None => {
                                if !class_was_loaded {
                                    // Unloadable target — preserve legacy soft-fail behavior.
                                    let arg_count = parse_arg_count(&callee_desc);
                                    for _ in 0..arg_count {
                                        frame.pop()?;
                                    }
                                    frame.pop()?; // pop `this`
                                    *idx += 1;
                                    continue;
                                }
                                return Err(VmError::MethodNotFound {
                                    name: format!(
                                        "{}.{callee_name}",
                                        registry.internal_name_for_class(
                                            virtual_start
                                                .as_deref()
                                                .unwrap_or(callee_class_key.as_str()),
                                        )
                                    ),
                                    descriptor: callee_desc,
                                });
                            }
                        }
                    }
                };
                let arg_count = parse_arg_count(&callee_desc);
                // Populate dispatch cache for invokespecial (static dispatch — result is stable).
                if matches!(instr, Instruction::Invokespecial(_)) {
                    dispatch_cache
                        .entry(current_class.clone())
                        .or_default()
                        .insert(cp_idx.0, (dispatch_class.clone(), callee_idx, arg_count));
                }
                #[cfg(feature = "telemetry")]
                if matches!(instr, Instruction::Invokevirtual(_)) {
                    registry.telemetry.dispatch_resolution.record(
                        current_class,
                        cp_idx.0,
                        &dispatch_class,
                        dispatch_class != callee_class_key,
                    );
                }
                let (callee_pc_to_idx, callee_frame) = {
                    let ctx = registry.get(&dispatch_class)?;
                    let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                    let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                    let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(max_locals, Slot::Int(0));
                    if arg_count + 1 > max_locals {
                        return Err(VmError::LocalOutOfBounds {
                            index: arg_count + 1,
                            max_locals,
                        });
                    }
                    // Pop args into locals[1..=arg_count] in reverse (stack top = last arg).
                    for i in (1..=arg_count).rev() {
                        locals_buf[i] = frame.pop()?;
                    }
                    locals_buf[0] = frame.pop()?; // `this`
                    let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                    (pci, f)
                };
                activate_method_state(
                    frame,
                    method_idx,
                    pc_to_idx,
                    instructions,
                    current_class,
                    call_stack,
                    registry,
                    dispatch_class,
                    callee_idx,
                    callee_pc_to_idx,
                    callee_frame,
                    *idx + 1,
                    #[cfg(feature = "telemetry")]
                    current_method,
                )?;
                *idx = 0;
                continue;
            }

            // ---- Reference return ----
            Instruction::Areturn => {
                let v = frame.pop()?;
                do_return!(Some(v));
            }

            // ----------------------------------------------------------------
            // Array allocation
            // ----------------------------------------------------------------
            Instruction::Newarray(array_type) => {
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let class_name = match array_type {
                    ArrayType::Boolean => "[Z",
                    ArrayType::Char => "[C",
                    ArrayType::Float => "[F",
                    ArrayType::Double => "[D",
                    ArrayType::Byte => "[B",
                    ArrayType::Short => "[S",
                    ArrayType::Int => "[I",
                    ArrayType::Long => "[J",
                };
                let r = heap.allocate(class_name.to_string(), count as usize);
                // Fix element types for non-int primitive arrays.
                match array_type {
                    ArrayType::Long => {
                        let obj = heap.get_mut(r)?;
                        for slot in &mut obj.fields {
                            *slot = Slot::Long(0);
                        }
                    }
                    ArrayType::Float => {
                        let obj = heap.get_mut(r)?;
                        for slot in &mut obj.fields {
                            *slot = Slot::Float(0.0);
                        }
                    }
                    ArrayType::Double => {
                        let obj = heap.get_mut(r)?;
                        for slot in &mut obj.fields {
                            *slot = Slot::Double(0.0);
                        }
                    }
                    _ => {} // Int/Boolean/Byte/Char/Short default to Slot::Int(0)
                }
                frame.push(Slot::Reference(Some(r)))?;
                if gc_allowed && heap.should_gc() {
                    let roots = gather_roots(frame, call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(frame, call_stack, registry, heap);
                }
            }
            Instruction::Anewarray(cp_idx) => {
                let element_type = {
                    let ctx = registry.get(current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let array_type = format!("[L{element_type};");
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let r = heap.allocate(array_type, count as usize);
                // Fix elements to Reference(None).
                let obj = heap.get_mut(r)?;
                for slot in &mut obj.fields {
                    *slot = Slot::Reference(None);
                }
                frame.push(Slot::Reference(Some(r)))?;
                if gc_allowed && heap.should_gc() {
                    let roots = gather_roots(frame, call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(frame, call_stack, registry, heap);
                }
            }
            Instruction::Arraylength => {
                let r = frame.pop_ref()?;
                let len = heap.get(r)?.fields.len();
                frame.push(Slot::Int(len as i32))?;
            }

            // ---- Int array ----
            Instruction::Iaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_int()?
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Iastore => {
                let val = frame.pop_int()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Long array ----
            Instruction::Laload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_long()?
                };
                frame.push(Slot::Long(v))?;
            }
            Instruction::Lastore => {
                let val = frame.pop_long()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Long(val))?;
            }

            // ---- Float array ----
            Instruction::Faload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_float()?
                };
                frame.push(Slot::Float(v))?;
            }
            Instruction::Fastore => {
                let val = frame.pop_float()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Float(val))?;
            }

            // ---- Double array ----
            Instruction::Daload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_double()?
                };
                frame.push(Slot::Double(v))?;
            }
            Instruction::Dastore => {
                let val = frame.pop_double()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Double(val))?;
            }

            // ---- Reference array ----
            Instruction::Aaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize]
                };
                frame.push(v)?;
            }
            Instruction::Aastore => {
                let val = frame.pop()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, val)?;
            }

            // ---- Byte/boolean array (stored as Int, truncated to i8) ----
            Instruction::Baload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    i32::from(fields[idx_val as usize].as_int()? as i8)
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = i32::from(frame.pop_int()? as i8);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Char array (stored as Int, masked to u16) ----
            Instruction::Caload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    i32::from(fields[idx_val as usize].as_int()? as u16)
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = i32::from(frame.pop_int()? as u16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Short array (stored as Int, truncated to i16) ----
            Instruction::Saload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    i32::from(fields[idx_val as usize].as_int()? as i16)
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = i32::from(frame.pop_int()? as i16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Switch ----
            Instruction::Tableswitch {
                default,
                low,
                high,
                offsets,
            } => {
                let key = frame.pop_int()?;
                let offset = if key >= *low && key <= *high {
                    offsets[(key - low) as usize]
                } else {
                    *default
                };
                jump!(offset);
            }
            Instruction::Lookupswitch { default, pairs } => {
                let key = frame.pop_int()?;
                let offset = pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or(*default, |(_, off)| *off);
                jump!(offset);
            }

            // ---- checkcast / instanceof ----
            Instruction::Checkcast(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(slot)?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = {
                            let ctx = registry.get(current_class)?;
                            resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                        };
                        let actual = heap.get(*r)?.class_name.clone();
                        if is_assignable_from(
                            registry,
                            loader,
                            &actual,
                            &target,
                            Some(current_class.as_str()),
                        ) {
                            frame.push(slot)?;
                        } else {
                            return Err(VmError::ClassCastException {
                                from: actual,
                                to: target,
                            });
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }
            Instruction::Instanceof(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(Slot::Int(0))?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = {
                            let ctx = registry.get(current_class)?;
                            resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                        };
                        let actual = heap.get(*r)?.class_name.clone();
                        let result = i32::from(is_assignable_from(
                            registry,
                            loader,
                            &actual,
                            &target,
                            Some(current_class.as_str()),
                        ));
                        frame.push(Slot::Int(result))?;
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }

            // ---- athrow with exception table dispatch ----
            Instruction::Athrow => {
                let exception_ref = frame.pop_ref()?;
                let exc_class_name = heap.get(exception_ref)?.class_name.clone();
                propagate_java_exception!(exc_class_name, exception_ref, pc);
            }

            // ----------------------------------------------------------------
            // invokedynamic — resolve bootstrap method, dispatch based on
            // the bootstrap class (StringConcatFactory, LambdaMetafactory).
            // ----------------------------------------------------------------
            Instruction::Invokedynamic(cp_idx) => {
                let cp_idx_val = usize::from(cp_idx.0);

                // 1. Resolve InvokeDynamic CP entry.
                let (bsm_idx, call_name, call_desc) = {
                    let ctx = registry.get(current_class)?;
                    let cp = &ctx.constant_pool;
                    match cp.get(cp_idx_val).and_then(|e| e.as_ref()) {
                        Some(CpEntry::InvokeDynamic {
                            bootstrap_method_attr_index,
                            name_and_type_index,
                        }) => {
                            let (name, desc) =
                                resolve_name_and_type(cp, name_and_type_index.0 as usize)?;
                            (*bootstrap_method_attr_index as usize, name, desc)
                        }
                        _ => return Err(VmError::InvalidCpIndex { index: cp_idx_val }),
                    }
                };

                // 2. Look up the bootstrap method entry.
                let (bsm_class, bsm_args) = {
                    let ctx = registry.get(current_class)?;
                    let bsm_entry = ctx
                        .bootstrap_methods
                        .get(bsm_idx)
                        .ok_or(VmError::InvalidCpIndex { index: bsm_idx })?;
                    let (_kind, class, _name, _desc) =
                        resolve_method_handle(&ctx.constant_pool, bsm_entry.method_ref.0 as usize)?;
                    let args: Vec<duke_classfile::types::CpIndex> = bsm_entry.arguments.clone();
                    (class, args)
                };

                let _ = &call_name; // suppress unused warning for now

                // 3. Dispatch based on bootstrap method class.
                if bsm_class == "java/lang/invoke/StringConcatFactory" {
                    // --- StringConcatFactory.makeConcatWithConstants ---
                    let arg_count = parse_arg_count(&call_desc);
                    let arg_types = parse_arg_types(&call_desc);
                    // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                    let mut dynamic_args = vec![Slot::Int(0); arg_count];

                    for i in (0..arg_count).rev() {
                        dynamic_args[i] = frame.pop()?;
                    }

                    // Resolve recipe (first bootstrap arg) and constants (remaining).
                    let (recipe, constants) = {
                        let ctx = registry.get(current_class)?;
                        let cp = &ctx.constant_pool;
                        let recipe = if bsm_args.is_empty() {
                            String::new()
                        } else {
                            resolve_cp_string(cp, bsm_args[0].0 as usize)?
                        };
                        let mut consts = Vec::new();
                        for arg_idx in bsm_args.iter().skip(1) {
                            if let Ok(s) = resolve_cp_string(cp, arg_idx.0 as usize) {
                                consts.push(s);
                            }
                        }
                        (recipe, consts)
                    };

                    let result = execute_string_concat_recipe(
                        &recipe,
                        &dynamic_args,
                        &arg_types,
                        &constants,
                        heap,
                    )?;
                    frame.push(result)?;
                } else if bsm_class == "java/lang/invoke/LambdaMetafactory" {
                    // --- LambdaMetafactory.metafactory ---
                    // Bootstrap args: [MethodType erased, MethodHandle impl, MethodType specialized]

                    let (impl_kind, impl_class, impl_method, impl_desc) = {
                        let ctx = registry.get(current_class)?;
                        let cp = &ctx.constant_pool;
                        if bsm_args.len() < 3 {
                            return Err(VmError::Unimplemented {
                                mnemonic: "LambdaMetafactory requires 3 bootstrap args",
                            });
                        }
                        resolve_method_handle(cp, bsm_args[1].0 as usize)?
                    };

                    let sam_method = call_name.clone();

                    // Resolve erased SAM descriptor from bootstrap arg 0.
                    let sam_desc = {
                        let ctx = registry.get(current_class)?;
                        let cp = &ctx.constant_pool;
                        match cp.get(bsm_args[0].0 as usize).and_then(|e| e.as_ref()) {
                            Some(CpEntry::MethodType { descriptor_index }) => {
                                match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                                    Some(CpEntry::Utf8(s)) => s.clone(),
                                    _ => {
                                        return Err(VmError::InvalidCpIndex {
                                            index: descriptor_index.0 as usize,
                                        });
                                    }
                                }
                            }
                            _ => {
                                return Err(VmError::InvalidCpIndex {
                                    index: bsm_args[0].0 as usize,
                                });
                            }
                        }
                    };

                    // Pop captured variables from the stack.
                    let captured_count = parse_arg_count(&call_desc);
                    // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                    let mut captured_args = vec![Slot::Int(0); captured_count];

                    for i in (0..captured_count).rev() {
                        captured_args[i] = frame.pop()?;
                    }

                    let lambda_info = LambdaInfo {
                        impl_class: impl_class.clone(),
                        impl_method: impl_method.clone(),
                        impl_desc: impl_desc.clone(),
                        impl_kind,
                        sam_method,
                        sam_desc,
                        captured_count,
                    };
                    let lambda_class = registry.register_lambda(lambda_info);

                    let r = heap.allocate(lambda_class, captured_count);
                    for (i, slot) in captured_args.into_iter().enumerate() {
                        heap.get_mut(r)?.fields[i] = slot;
                    }

                    let _ = registry.ensure_loaded_from(
                        &impl_class,
                        Some(current_class.as_str()),
                        loader,
                    );

                    frame.push(Slot::Reference(Some(r)))?;
                    if gc_allowed && heap.should_gc() {
                        let roots = gather_roots(frame, call_stack, registry);
                        heap.collect(&roots);
                        patch_forwarded_slots(frame, call_stack, registry, heap);
                    }
                } else {
                    // Unknown bootstrap method — pop args and push null.
                    let arg_count = parse_arg_count(&call_desc);
                    for _ in 0..arg_count {
                        frame.pop()?;
                    }
                    if !call_desc.ends_with(")V") {
                        frame.push(Slot::Reference(None))?;
                    }
                }
            }

            // ----------------------------------------------------------------
            // invokeinterface — like invokevirtual but resolves from
            // InterfaceMethodref and dispatches on the actual object class.
            // ----------------------------------------------------------------
            Instruction::Invokeinterface {
                index: cp_idx,
                count: _,
            } => {
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let callee_class_key =
                    registry.class_key_from_source(&callee_class, Some(current_class.as_str()));
                if callee_name == "<clinit>" {
                    *idx += 1;
                    continue;
                }
                let arg_count = parse_arg_count(&callee_desc);
                // Peek at `this` (sits below the args) to determine the actual
                // runtime class without consuming the stack yet.  Each dispatch
                // path (native / lambda / bytecode) pops what it needs itself.
                let stack_len = frame.stack_len();
                let actual_class = if stack_len > arg_count {
                    let this_pos = stack_len - arg_count - 1;
                    if let Ok(Slot::Reference(Some(r))) = frame.peek_at(this_pos) {
                        heap.get(r).ok().map(|o| o.class_name.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
                .unwrap_or_else(|| callee_class.clone());

                // Try to find the method on the actual class (walking hierarchy).
                let resolved = match resolve_method_in_hierarchy_lookup(
                    registry,
                    loader,
                    &actual_class,
                    &callee_name,
                    &callee_desc,
                ) {
                    MethodHierarchyLookup::Bytecode(class_name, method_idx) => {
                        MethodHierarchyLookup::Bytecode(class_name, method_idx)
                    }
                    MethodHierarchyLookup::NativeOverride => MethodHierarchyLookup::NativeOverride,
                    MethodHierarchyLookup::Missing => resolve_method_in_hierarchy_lookup(
                        registry,
                        loader,
                        &callee_class_key,
                        &callee_name,
                        &callee_desc,
                    ),
                };
                let allow_lambda_dispatch = matches!(resolved, MethodHierarchyLookup::Missing);

                let (dispatch_class, callee_idx) =
                    if let MethodHierarchyLookup::Bytecode(dispatch_class, callee_idx) = resolved {
                        (dispatch_class, callee_idx)
                    } else {
                        // Check native registry — try actual class then interface class.
                        let native_kind = lookup_registered_native_kind(
                            registry,
                            &actual_class,
                            &callee_name,
                            &callee_desc,
                        )
                        .or_else(|| {
                            lookup_registered_native_kind(
                                registry,
                                &callee_class_key,
                                &callee_name,
                                &callee_desc,
                            )
                        });
                        match native_kind {
                            Some(HandlerKind::Simple(handler)) => {
                                // Native path: collect args + this into a Vec<Slot>
                                // for the handler(&[Slot], ...) signature.
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut callee_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    callee_args[i] = frame.pop()?;
                                }

                                let this_slot = frame.pop()?;
                                callee_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = NativeControl::default();
                                let result =
                                    handler(&callee_args, heap, stdout, &mut native_control);
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        registry.internal_name_for_class(&callee_class_key),
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    // Native interface methods skip the bytecode
                                    // hierarchy walk — hierarchy_walk is always false here.
                                    registry.telemetry.dispatch_resolution.record(
                                        current_class,
                                        cp_idx.0,
                                        &actual_class,
                                        false,
                                    );
                                }
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(outcome) =
                                    finish_native_call(&mut native_control, frame, idx, result)?
                                {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            Some(HandlerKind::Callback(handler)) => {
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut callee_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    callee_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?;
                                callee_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = NativeControl::default();
                                let result = {
                                    let mut callback_ops =
                                        InterpreterCallbackOps { registry, loader };
                                    handler(
                                        &callee_args,
                                        heap,
                                        stdout,
                                        &mut native_control,
                                        &mut callback_ops,
                                    )
                                };
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        registry.internal_name_for_class(&callee_class_key),
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    registry.telemetry.dispatch_resolution.record(
                                        current_class,
                                        cp_idx.0,
                                        &actual_class,
                                        false,
                                    );
                                }
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(outcome) =
                                    finish_native_call(&mut native_control, frame, idx, result)?
                                {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            None => {} // fall through to lambda / no-op
                        }
                        // Check lambda registry for SAM dispatch.
                        if allow_lambda_dispatch
                            && let Some(lambda_info) = registry.get_lambda(&actual_class).cloned()
                            && callee_name == lambda_info.sam_method
                        {
                            // Lambda path: collect args + this into a Vec<Slot>.
                            // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                            let mut callee_args = vec![Slot::Int(0); arg_count];

                            for i in (0..arg_count).rev() {
                                callee_args[i] = frame.pop()?;
                            }
                            let this_slot = frame.pop()?;
                            callee_args.insert(0, this_slot);
                            let this_ref = match &callee_args[0] {
                                Slot::Reference(Some(r)) => *r,
                                _ => return Err(VmError::NullPointerException),
                            };
                            let obj = heap.get(this_ref)?;
                            let mut impl_args: Vec<Slot> = Vec::new();
                            for i in 0..lambda_info.captured_count {
                                impl_args.push(obj.fields[i]);
                            }
                            impl_args.extend(callee_args[1..].iter().copied());

                            let _ = registry.ensure_loaded_from(
                                &lambda_info.impl_class,
                                Some(current_class.as_str()),
                                loader,
                            );
                            let impl_class_key = registry.class_key_from_source(
                                &lambda_info.impl_class,
                                Some(current_class.as_str()),
                            );

                            if lambda_info.impl_kind == 6 {
                                // invokeStatic dispatch
                                let resolved = resolve_method_in_hierarchy(
                                    registry,
                                    loader,
                                    &impl_class_key,
                                    &lambda_info.impl_method,
                                    &lambda_info.impl_desc,
                                );
                                if let Some((dispatch_class, impl_idx)) = resolved {
                                    let (callee_pc_to_idx, callee_frame) = {
                                        let ctx = registry.get(&dispatch_class)?;
                                        let max_locals =
                                            usize::from(ctx.methods[impl_idx].max_locals);
                                        let max_stack =
                                            usize::from(ctx.methods[impl_idx].max_stack);
                                        let pci =
                                            std::sync::Arc::clone(&ctx.methods[impl_idx].pc_to_idx);
                                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                        locals_buf.resize(max_locals, Slot::Int(0));
                                        let expanded_impl_args = expand_args_for_desc(
                                            &impl_args,
                                            &lambda_info.impl_desc,
                                        );
                                        for (i, slot) in expanded_impl_args.into_iter().enumerate()
                                        {
                                            if i < max_locals {
                                                locals_buf[i] = slot;
                                            }
                                        }
                                        let f =
                                            Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                                        (pci, f)
                                    };
                                    activate_method_state(
                                        frame,
                                        method_idx,
                                        pc_to_idx,
                                        instructions,
                                        current_class,
                                        call_stack,
                                        registry,
                                        dispatch_class,
                                        impl_idx,
                                        callee_pc_to_idx,
                                        callee_frame,
                                        *idx + 1,
                                        #[cfg(feature = "telemetry")]
                                        current_method,
                                    )?;
                                    *idx = 0;
                                    continue;
                                }
                            } else if lambda_info.impl_kind == 5 || lambda_info.impl_kind == 9 {
                                // invokeVirtual / invokeInterface dispatch
                                let impl_class_key = registry.class_key_from_source(
                                    &lambda_info.impl_class,
                                    Some(current_class.as_str()),
                                );
                                let resolved = resolve_method_in_hierarchy(
                                    registry,
                                    loader,
                                    &impl_class_key,
                                    &lambda_info.impl_method,
                                    &lambda_info.impl_desc,
                                );
                                if let Some((dispatch_class, impl_idx)) = resolved {
                                    let (callee_pc_to_idx, callee_frame) = {
                                        let ctx = registry.get(&dispatch_class)?;
                                        let max_locals =
                                            usize::from(ctx.methods[impl_idx].max_locals);
                                        let max_stack =
                                            usize::from(ctx.methods[impl_idx].max_stack);
                                        let pci =
                                            std::sync::Arc::clone(&ctx.methods[impl_idx].pc_to_idx);
                                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                        locals_buf.resize(max_locals, Slot::Int(0));
                                        let expanded_impl_args = expand_args_for_desc(
                                            &impl_args,
                                            &lambda_info.impl_desc,
                                        );
                                        for (i, slot) in expanded_impl_args.into_iter().enumerate()
                                        {
                                            if i < max_locals {
                                                locals_buf[i] = slot;
                                            }
                                        }
                                        let f =
                                            Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                                        (pci, f)
                                    };
                                    activate_method_state(
                                        frame,
                                        method_idx,
                                        pc_to_idx,
                                        instructions,
                                        current_class,
                                        call_stack,
                                        registry,
                                        dispatch_class,
                                        impl_idx,
                                        callee_pc_to_idx,
                                        callee_frame,
                                        *idx + 1,
                                        #[cfg(feature = "telemetry")]
                                        current_method,
                                    )?;
                                    *idx = 0;
                                    continue;
                                }
                                // Try native fallback for virtual/interface
                                let lambda_native_kind = lookup_registered_native_kind(
                                    registry,
                                    &impl_class_key,
                                    &lambda_info.impl_method,
                                    &lambda_info.impl_desc,
                                );
                                match lambda_native_kind {
                                    Some(HandlerKind::Simple(handler)) => {
                                        #[cfg(feature = "telemetry")]
                                        let _native_start = std::time::Instant::now();
                                        let mut native_control = NativeControl::default();
                                        let result =
                                            handler(&impl_args, heap, stdout, &mut native_control);
                                        #[cfg(feature = "telemetry")]
                                        registry.telemetry.native_boundary.record_call(
                                            registry.internal_name_for_class(&impl_class_key),
                                            &lambda_info.impl_method,
                                            _native_start.elapsed().as_nanos() as u64,
                                            result.is_err(),
                                        );
                                        let result = match result {
                                            Ok(result) => result,
                                            Err(VmError::JavaException { class_name }) => {
                                                let exception_ref =
                                                    materialize_java_exception_object(
                                                        registry,
                                                        loader,
                                                        heap,
                                                        &class_name,
                                                    )?;
                                                propagate_java_exception!(
                                                    class_name,
                                                    exception_ref,
                                                    pc
                                                );
                                            }
                                            Err(err) => return Err(err),
                                        };
                                        if let Some(outcome) = finish_native_call(
                                            &mut native_control,
                                            frame,
                                            idx,
                                            result,
                                        )? {
                                            return Ok(outcome);
                                        }
                                        continue;
                                    }
                                    Some(HandlerKind::Callback(handler)) => {
                                        #[cfg(feature = "telemetry")]
                                        let _native_start = std::time::Instant::now();
                                        let mut native_control = NativeControl::default();
                                        let result = {
                                            let mut callback_ops =
                                                InterpreterCallbackOps { registry, loader };
                                            handler(
                                                &impl_args,
                                                heap,
                                                stdout,
                                                &mut native_control,
                                                &mut callback_ops,
                                            )
                                        };
                                        #[cfg(feature = "telemetry")]
                                        registry.telemetry.native_boundary.record_call(
                                            registry.internal_name_for_class(&impl_class_key),
                                            &lambda_info.impl_method,
                                            _native_start.elapsed().as_nanos() as u64,
                                            result.is_err(),
                                        );
                                        let result = match result {
                                            Ok(result) => result,
                                            Err(VmError::JavaException { class_name }) => {
                                                let exception_ref =
                                                    materialize_java_exception_object(
                                                        registry,
                                                        loader,
                                                        heap,
                                                        &class_name,
                                                    )?;
                                                propagate_java_exception!(
                                                    class_name,
                                                    exception_ref,
                                                    pc
                                                );
                                            }
                                            Err(err) => return Err(err),
                                        };
                                        if let Some(outcome) = finish_native_call(
                                            &mut native_control,
                                            frame,
                                            idx,
                                            result,
                                        )? {
                                            return Ok(outcome);
                                        }
                                        continue;
                                    }
                                    None => {}
                                }
                            }
                            *idx += 1;
                            continue;
                        }
                        if !registry.contains(&actual_class)
                            && !registry.contains(&callee_class_key)
                        {
                            *idx += 1;
                            continue;
                        }
                        return Err(VmError::MethodNotFound {
                            name: format!("{actual_class}.{callee_name}"),
                            descriptor: callee_desc,
                        });
                    };

                // Bytecode execution path — Pattern B: pop directly into locals_buf.
                #[cfg(feature = "telemetry")]
                registry.telemetry.dispatch_resolution.record(
                    current_class,
                    cp_idx.0,
                    &dispatch_class,
                    dispatch_class != actual_class,
                );
                let (callee_pc_to_idx, callee_frame) = {
                    let ctx = registry.get(&dispatch_class)?;
                    let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                    let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                    let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(max_locals, Slot::Int(0));
                    if arg_count + 1 > max_locals {
                        return Err(VmError::LocalOutOfBounds {
                            index: arg_count + 1,
                            max_locals,
                        });
                    }
                    // Pop method args in reverse (stack top = last arg) into locals[1..=arg_count].
                    for i in (1..=arg_count).rev() {
                        locals_buf[i] = frame.pop()?;
                    }
                    locals_buf[0] = frame.pop()?; // `this`
                    let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                    (pci, f)
                };
                activate_method_state(
                    frame,
                    method_idx,
                    pc_to_idx,
                    instructions,
                    current_class,
                    call_stack,
                    registry,
                    dispatch_class,
                    callee_idx,
                    callee_pc_to_idx,
                    callee_frame,
                    *idx + 1,
                    #[cfg(feature = "telemetry")]
                    current_method,
                )?;
                *idx = 0;
                continue;
            }

            // ----------------------------------------------------------------
            // multianewarray — allocate multi-dimensional arrays.
            // ----------------------------------------------------------------
            Instruction::Multianewarray {
                index: cp_idx,
                dimensions,
            } => {
                let element_type = {
                    let ctx = registry.get(current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };

                // Pop dimension sizes from stack (first popped is rightmost dimension).
                let mut dims: Vec<i32> = Vec::new();
                for _ in 0..*dimensions {
                    dims.push(frame.pop_int()?);
                }
                dims.reverse(); // Now dims[0] is outermost.

                // Check for negative sizes.
                for &d in &dims {
                    if d < 0 {
                        return Err(VmError::NegativeArraySize { size: d });
                    }
                }

                // Recursive allocation helper.
                fn alloc_multi(
                    heap: &mut duke_gc::Heap,
                    dims: &[i32],
                    depth: usize,
                    type_name: &str,
                ) -> VmResult<u64> {
                    let size = dims[depth] as usize;
                    let r = heap.allocate(type_name.to_string(), size);
                    if depth < dims.len() - 1 {
                        // Not the innermost — fill with references to sub-arrays.
                        let inner_type = &type_name[1..]; // Strip one '[' for inner dimension.
                        for i in 0..size {
                            let inner = alloc_multi(heap, dims, depth + 1, inner_type)?;
                            heap.get_mut(r)?.fields[i] = Slot::Reference(Some(inner));
                        }
                    }
                    Ok(r)
                }

                let r = alloc_multi(heap, &dims, 0, &element_type)?;
                frame.push(Slot::Reference(Some(r)))?;
                if gc_allowed && heap.should_gc() {
                    let roots = gather_roots(frame, call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(frame, call_stack, registry, heap);
                }
            }

            // ---- monitor (no-op, single-threaded) ----
            Instruction::Monitorenter | Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }

            other => {
                return Err(VmError::Unimplemented {
                    mnemonic: other.mnemonic(),
                });
            }
        }

        #[cfg(feature = "telemetry")]
        {
            let elapsed = _telem_start.elapsed().as_nanos() as u64;
            registry.telemetry.bytecode_cost.record(
                _telem_name,
                current_class,
                current_method,
                _telem_pc,
                elapsed,
            );
        }

        *idx += 1;
    }
}

struct CompletionVm {
    registry: ClassRegistry,
    heap: duke_gc::Heap,
    output: Vec<u8>,
    live_workers: usize,
}

#[derive(Default)]
struct CompletionRuntime {
    threads: threading::ThreadRuntime,
    handles: HashMap<i32, std::thread::JoinHandle<VmResult<()>>>,
}

fn resolve_thread_entry(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &duke_gc::Heap,
    thread_ref: u64,
) -> VmResult<Option<(String, usize, Vec<Slot>)>> {
    let actual_class = heap.get(thread_ref)?.class_name.clone();
    if actual_class != "java/lang/Thread"
        && let Some((dispatch_class, method_idx)) =
            resolve_method_in_hierarchy(registry, loader, &actual_class, "run", "()V")
    {
        return Ok(Some((
            dispatch_class,
            method_idx,
            vec![Slot::Reference(Some(thread_ref))],
        )));
    }

    let target_ref = match heap.get(thread_ref)?.fields.get(THREAD_TARGET_SLOT) {
        Some(Slot::Reference(Some(target_ref))) => *target_ref,
        _ => return Ok(None),
    };
    let target_class = heap.get(target_ref)?.class_name.clone();
    let (dispatch_class, method_idx) =
        resolve_method_in_hierarchy(registry, loader, &target_class, "run", "()V").ok_or_else(
            || VmError::AbstractMethodError {
                class_name: target_class.clone(),
                method_name: "run".to_string(),
            },
        )?;
    Ok(Some((
        dispatch_class,
        method_idx,
        vec![Slot::Reference(Some(target_ref))],
    )))
}

fn join_java_thread(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    thread_id: i32,
) -> VmResult<()> {
    loop {
        let handle = {
            let mut runtime = runtime.lock().unwrap();
            let is_finished = runtime
                .threads
                .records()
                .iter()
                .find(|record| record.thread_id == thread_id)
                .is_none_or(|record| record.finished);
            if is_finished {
                return Ok(());
            }
            runtime.handles.remove(&thread_id)
        };

        if let Some(handle) = handle {
            return match handle.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            };
        }

        std::thread::yield_now();
    }
}

fn wait_for_all_java_threads(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
) -> VmResult<()> {
    let mut first_error = None;
    loop {
        let handles = {
            let mut runtime = runtime.lock().unwrap();
            if runtime.handles.is_empty() {
                return first_error.unwrap_or(Ok(()));
            }
            runtime
                .handles
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };

        for handle in handles {
            match handle.join() {
                Ok(result) => {
                    if let Err(e) = result {
                        first_error.get_or_insert(Err(e));
                    }
                }
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }
    }
}

fn handle_thread_action(
    action: NativeThreadAction,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> VmResult<()> {
    match action {
        NativeThreadAction::Start { thread_ref } => {
            spawn_java_thread(shared, runtime, loader, thread_ref)
        }
        NativeThreadAction::Sleep(duration) => {
            std::thread::sleep(duration);
            Ok(())
        }
        NativeThreadAction::Join { thread_id } => join_java_thread(runtime, thread_id),
    }
}

fn run_thread_to_completion(
    mut state: ExecutionState,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> VmResult<()> {
    loop {
        let mut shared_guard = shared.lock().unwrap();
        let CompletionVm {
            registry,
            heap,
            output,
            live_workers,
        } = &mut *shared_guard;
        let outcome = run_execution(
            &mut state,
            registry,
            loader.as_ref(),
            heap,
            output,
            *live_workers == 0,
            Some(DEFAULT_THREAD_QUANTUM),
        )?;
        drop(shared_guard);

        match outcome {
            ExecutionOutcome::Returned(_) => return Ok(()),
            ExecutionOutcome::ThreadAction(action) => {
                handle_thread_action(action, shared, runtime, loader)?;
            }
            ExecutionOutcome::Yield => {
                // `std::sync::Mutex` is not fair — the same thread can
                // immediately re-acquire the lock, starving others.  A
                // brief sleep forces the OS scheduler to consider other
                // runnable threads before we loop back.
                std::thread::sleep(std::time::Duration::from_micros(1));
            }
        }
    }
}

fn spawn_java_thread(
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
    thread_ref: u64,
) -> VmResult<()> {
    {
        let mut shared_guard = shared.lock().unwrap();
        let thread = shared_guard.heap.get_mut(thread_ref)?;
        let already_started = matches!(
            thread.fields.get(THREAD_ID_SLOT),
            Some(Slot::Int(thread_id)) if *thread_id >= 0
        );
        drop(shared_guard);
        if already_started {
            return Ok(());
        }
    }

    let thread_id = {
        let mut runtime = runtime.lock().unwrap();
        let thread_id = runtime.threads.allocate_thread_id();
        runtime
            .threads
            .register(threading::ThreadRecord::new(thread_ref, thread_id));
        thread_id
    };

    let entry = {
        let mut shared_guard = shared.lock().unwrap();
        {
            let thread = shared_guard.heap.get_mut(thread_ref)?;
            thread.fields[THREAD_ID_SLOT] = Slot::Int(thread_id);
        }
        let CompletionVm {
            registry,
            heap,
            live_workers,
            ..
        } = &mut *shared_guard;
        let entry = resolve_thread_entry(registry, loader.as_ref(), heap, thread_ref)?;
        if entry.is_some() {
            *live_workers += 1;
        }
        drop(shared_guard);
        entry
    };
    let Some((dispatch_class, method_idx, args)) = entry else {
        let _ = runtime.lock().unwrap().threads.mark_finished(thread_id);
        return Ok(());
    };

    let state = {
        let shared = shared.lock().unwrap();
        ExecutionState::new(&shared.registry, &dispatch_class, "run", method_idx, &args)?
    };

    let shared_clone = std::sync::Arc::clone(shared);
    let runtime_clone = std::sync::Arc::clone(runtime);
    let loader_clone = std::sync::Arc::clone(loader);
    let handle = std::thread::spawn(move || {
        let result = run_thread_to_completion(state, &shared_clone, &runtime_clone, &loader_clone);
        {
            let mut shared = shared_clone.lock().unwrap();
            shared.live_workers = shared.live_workers.saturating_sub(1);
        }
        let _ = runtime_clone
            .lock()
            .unwrap()
            .threads
            .mark_finished_by_java_ref(thread_ref);
        result
    });
    runtime.lock().unwrap().handles.insert(thread_id, handle);
    Ok(())
}

/// Execute a Java entrypoint and keep the VM alive until any spawned worker
/// threads have either finished or been joined.
///
/// # Errors
///
/// Returns `VmError` if class resolution, method dispatch, or bytecode
/// execution fails in any thread.
///
/// # Panics
///
/// Panics if a `Mutex` protecting shared VM state is poisoned by a
/// panicking thread, or if the `Arc` cannot be unwound after all threads
/// have joined.
#[allow(clippy::too_many_arguments)]
pub fn execute_class_to_completion<L>(
    registry: &mut ClassRegistry,
    loader: L,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>>
where
    L: ClassLoader + Send + Sync + 'static,
{
    let loader: std::sync::Arc<dyn ClassLoader + Send + Sync> = std::sync::Arc::new(loader);
    if lookup_registered_native_kind(registry, class_name, method_name, descriptor).is_some() {
        return execute_class(
            registry,
            loader.as_ref(),
            heap,
            stdout,
            class_name,
            method_name,
            descriptor,
            args,
        );
    }

    let mut vm = CompletionVm {
        registry: std::mem::take(registry),
        heap: std::mem::replace(heap, duke_gc::Heap::new()),
        output: Vec::new(),
        live_workers: 0,
    };

    let mut state = match prepare_execution_state(
        &mut vm.registry,
        loader.as_ref(),
        &mut vm.heap,
        &mut vm.output,
        class_name,
        method_name,
        descriptor,
        args,
    ) {
        Ok(state) => state,
        Err(err) => {
            *registry = vm.registry;
            *heap = vm.heap;
            return Err(err);
        }
    };

    let shared = std::sync::Arc::new(std::sync::Mutex::new(vm));
    let runtime = std::sync::Arc::new(std::sync::Mutex::new(CompletionRuntime::default()));

    let run_result: VmResult<Option<Slot>> = loop {
        let mut shared_guard = shared.lock().unwrap();
        let CompletionVm {
            registry,
            heap,
            output,
            live_workers,
        } = &mut *shared_guard;
        let outcome = run_execution(
            &mut state,
            registry,
            loader.as_ref(),
            heap,
            output,
            *live_workers == 0,
            Some(DEFAULT_THREAD_QUANTUM),
        )?;
        drop(shared_guard);

        match outcome {
            ExecutionOutcome::Returned(result) => break Ok(result),
            ExecutionOutcome::ThreadAction(action) => {
                handle_thread_action(action, &shared, &runtime, &loader)?;
            }
            ExecutionOutcome::Yield => {
                // See comment in run_thread_to_completion — brief sleep
                // ensures fair scheduling across Java threads.
                std::thread::sleep(std::time::Duration::from_micros(1));
            }
        }
    };

    let wait_result = wait_for_all_java_threads(&runtime);
    let Ok(shared) = std::sync::Arc::try_unwrap(shared) else {
        panic!("completion runtime released shared VM state")
    };
    let Ok(shared) = shared.into_inner() else {
        panic!("shared VM mutex poisoned")
    };
    let flush_result = stdout
        .write_all(&shared.output)
        .map_err(|_| VmError::Unimplemented {
            mnemonic: "output flush",
        });
    *registry = shared.registry;
    *heap = shared.heap;

    match run_result {
        Ok(result) => {
            wait_result?;
            flush_result?;
            Ok(result)
        }
        Err(err) => {
            let _ = wait_result;
            let _ = flush_result;
            Err(err)
        }
    }
}

/// Saved state of a caller frame suspended during a method call.
struct CallFrame {
    frame: Frame,
    method_idx: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    resume_idx: usize,
    /// Class that was executing when this frame was pushed.
    class_name: String,
}

/// Build a [`ClassContext`] from a parsed [`duke_classfile::ClassFile`].
///
/// Decodes all methods with a Code attribute and extracts field metadata.
/// Methods without Code (abstract, native) are silently skipped.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build_class_context(cf: &duke_classfile::ClassFile) -> ClassContext {
    use duke_bytecode::decode;
    use duke_classfile::access_flags::{FieldAccessFlags, MethodAccessFlags};
    use duke_classfile::types::{AttributeData, CpEntry};

    // Resolve this_class -> class name string.
    let class_name = {
        let entry = cf
            .constant_pool
            .get(cf.this_class.0 as usize)
            .and_then(|e| e.as_ref());
        if let Some(CpEntry::Class { name_index }) = entry {
            match cf
                .constant_pool
                .get(name_index.0 as usize)
                .and_then(|e| e.as_ref())
            {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => String::new(),
            }
        } else {
            String::new()
        }
    };

    let methods = cf
        .methods
        .iter()
        .filter_map(|m| {
            let name = match cf.constant_pool.get(m.name_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let descriptor = match cf.constant_pool.get(m.descriptor_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let code = m.attributes.iter().find_map(|a| {
                if let AttributeData::Code(c) = &a.data {
                    Some(c)
                } else {
                    None
                }
            })?;
            let instructions = decode(&code.code).ok()?;
            let exception_table: Vec<ExceptionEntry> = code
                .exception_table
                .iter()
                .map(|e| {
                    let catch_type = if e.catch_type.0 == 0 {
                        None // catch-all (finally)
                    } else {
                        match cf
                            .constant_pool
                            .get(e.catch_type.0 as usize)
                            .and_then(|x| x.as_ref())
                        {
                            Some(CpEntry::Class { name_index }) => {
                                match cf
                                    .constant_pool
                                    .get(name_index.0 as usize)
                                    .and_then(|x| x.as_ref())
                                {
                                    Some(CpEntry::Utf8(s)) => Some(s.clone()),
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    };
                    ExceptionEntry {
                        start_pc: e.start_pc,
                        end_pc: e.end_pc,
                        handler_pc: e.handler_pc,
                        catch_type,
                    }
                })
                .collect();
            let pc_to_idx_map: std::collections::HashMap<usize, usize> = instructions
                .iter()
                .enumerate()
                .map(|(i, &(pc, _))| (pc, i))
                .collect();
            Some(MethodEntry {
                name,
                descriptor,
                is_public: m.access_flags.contains(MethodAccessFlags::PUBLIC),
                is_static: m.access_flags.contains(MethodAccessFlags::STATIC),
                instructions: instructions.into(),
                max_stack: code.max_stack,
                max_locals: code.max_locals,
                exception_table,
                pc_to_idx: std::sync::Arc::new(pc_to_idx_map),
            })
        })
        .collect();

    let mut fields = Vec::new();
    let mut static_fields = Vec::new();
    let mut instance_count = 0usize;

    for f in &cf.fields {
        let name = match cf.constant_pool.get(f.name_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s.clone(),
            _ => continue,
        };
        let descriptor = match cf.constant_pool.get(f.descriptor_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s.clone(),
            _ => continue,
        };
        let is_static = f.access_flags.contains(FieldAccessFlags::STATIC);
        if is_static {
            static_fields.push(default_slot_for_descriptor(&descriptor));
        } else {
            instance_count += 1;
        }
        fields.push(FieldEntry {
            name,
            descriptor,
            is_static,
        });
    }

    // Resolve super_class: if the index is 0, this is java/lang/Object (no super).
    let super_class = if cf.super_class.0 != 0 {
        resolve_class_name(&cf.constant_pool, cf.super_class.0 as usize).ok()
    } else {
        None
    };

    // Resolve directly-implemented interfaces.
    let interfaces: Vec<String> = cf
        .interfaces
        .iter()
        .filter_map(|idx| resolve_class_name(&cf.constant_pool, idx.0 as usize).ok())
        .collect();

    // Extract BootstrapMethods from class-level attributes.
    let bootstrap_methods = cf
        .attributes
        .iter()
        .find_map(|a| {
            if let AttributeData::BootstrapMethods(entries) = &a.data {
                Some(entries.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    ClassContext {
        class_name,
        super_class,
        interfaces,
        constant_pool: cf.constant_pool.clone(),
        methods,
        fields,
        static_fields,
        instance_field_count: instance_count,
        bootstrap_methods,
    }
}

/// Resolve a CP Class entry to its name string.
fn resolve_class_name(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Class { name_index }) => {
            match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => Err(VmError::InvalidCpIndex {
                    index: name_index.0 as usize,
                }),
            }
        }
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

fn internal_name_to_binary_name(name: &str) -> String {
    match name {
        "B" => "byte".to_string(),
        "C" => "char".to_string(),
        "D" => "double".to_string(),
        "F" => "float".to_string(),
        "I" => "int".to_string(),
        "J" => "long".to_string(),
        "S" => "short".to_string(),
        "Z" => "boolean".to_string(),
        "V" => "void".to_string(),
        _ => name.replace('/', "."),
    }
}

fn binary_name_to_internal_name(name: &str) -> String {
    name.replace('.', "/")
}

fn cp_utf8_string(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Utf8(s)) => Ok(s.clone()),
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

fn reflected_class_info_from_loader(
    loader: &dyn ClassLoader,
    internal_name: &str,
) -> Option<ReflectedClassInfo> {
    use duke_classfile::access_flags::{FieldAccessFlags, MethodAccessFlags};

    let bytes = loader.find_class(internal_name).ok()?;
    let class_file = duke_classfile::parse(&bytes).ok()?;
    let methods = class_file
        .methods
        .iter()
        .filter_map(|method| {
            let name =
                cp_utf8_string(&class_file.constant_pool, method.name_index.0 as usize).ok()?;
            let descriptor = cp_utf8_string(
                &class_file.constant_pool,
                method.descriptor_index.0 as usize,
            )
            .ok()?;
            Some(ReflectedMethodInfo {
                name,
                descriptor,
                is_public: method.access_flags.contains(MethodAccessFlags::PUBLIC),
                is_static: method.access_flags.contains(MethodAccessFlags::STATIC),
            })
        })
        .collect();
    let fields = class_file
        .fields
        .iter()
        .filter_map(|field| {
            let name =
                cp_utf8_string(&class_file.constant_pool, field.name_index.0 as usize).ok()?;
            let descriptor =
                cp_utf8_string(&class_file.constant_pool, field.descriptor_index.0 as usize)
                    .ok()?;
            Some(ReflectedFieldInfo {
                name,
                descriptor,
                is_public: field.access_flags.contains(FieldAccessFlags::PUBLIC),
                is_static: field.access_flags.contains(FieldAccessFlags::STATIC),
            })
        })
        .collect();
    let super_class = if class_file.super_class.0 != 0 {
        resolve_class_name(&class_file.constant_pool, class_file.super_class.0 as usize).ok()
    } else {
        None
    };
    let interfaces = class_file
        .interfaces
        .iter()
        .filter_map(|idx| resolve_class_name(&class_file.constant_pool, idx.0 as usize).ok())
        .collect();
    Some(ReflectedClassInfo {
        internal_name: internal_name.to_string(),
        binary_name: internal_name_to_binary_name(internal_name),
        super_class,
        interfaces,
        methods,
        fields,
    })
}

fn qualify_reflected_class_info_ancestry(
    registry: &ClassRegistry,
    source_class: &str,
    mut info: ReflectedClassInfo,
) -> ReflectedClassInfo {
    info.super_class = info
        .super_class
        .map(|super_class| registry.class_key_from_source(&super_class, Some(source_class)));
    info.interfaces = info
        .interfaces
        .into_iter()
        .map(|interface| registry.class_key_from_source(&interface, Some(source_class)))
        .collect();
    info
}

fn inspect_reflected_class(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    class: &str,
) -> VmResult<ReflectedClassInfo> {
    let internal_name = registry.internal_name_for_class(class).to_string();
    match registry.resolve_loaded_class_key(class) {
        Ok(class_key) => {
            if let Some(class_loader) = registry.class_loader(&class_key)
                && let Some(info) =
                    reflected_class_info_from_loader(class_loader.as_ref(), &internal_name)
            {
                return Ok(qualify_reflected_class_info_ancestry(
                    registry, &class_key, info,
                ));
            }
            if let Some(info) = reflected_class_info_from_loader(loader, &internal_name) {
                return Ok(qualify_reflected_class_info_ancestry(
                    registry, &class_key, info,
                ));
            }
            let ctx = registry.get(&class_key)?;
            let methods = ctx
                .methods
                .iter()
                .map(|method| ReflectedMethodInfo {
                    name: method.name.clone(),
                    descriptor: method.descriptor.clone(),
                    is_public: method.is_public,
                    is_static: method.is_static,
                })
                .collect();
            let fields = ctx
                .fields
                .iter()
                .map(|field| ReflectedFieldInfo {
                    name: field.name.clone(),
                    descriptor: field.descriptor.clone(),
                    is_public: true,
                    is_static: field.is_static,
                })
                .collect();
            return Ok(ReflectedClassInfo {
                internal_name: internal_name.clone(),
                binary_name: internal_name_to_binary_name(&internal_name),
                super_class: ctx.super_class.clone(),
                interfaces: ctx.interfaces.clone(),
                methods,
                fields,
            });
        }
        Err(VmError::ClassNotFound { .. }) => {}
        Err(err) => return Err(err),
    }

    if !registry.contains(class)
        && let Some(info) = reflected_class_info_from_loader(loader, &internal_name)
    {
        return Ok(info);
    }

    if !registry.contains(class) {
        registry.ensure_loaded(&internal_name, loader)?;
    }
    let class_key = registry.resolve_loaded_class_key(class)?;
    let ctx = registry.get(&class_key)?;
    let methods = ctx
        .methods
        .iter()
        .map(|method| ReflectedMethodInfo {
            name: method.name.clone(),
            descriptor: method.descriptor.clone(),
            is_public: method.is_public,
            is_static: method.is_static,
        })
        .collect();
    let fields = ctx
        .fields
        .iter()
        .map(|field| ReflectedFieldInfo {
            name: field.name.clone(),
            descriptor: field.descriptor.clone(),
            is_public: true,
            is_static: field.is_static,
        })
        .collect();

    Ok(ReflectedClassInfo {
        internal_name: internal_name.clone(),
        binary_name: internal_name_to_binary_name(&internal_name),
        super_class: ctx.super_class.clone(),
        interfaces: ctx.interfaces.clone(),
        methods,
        fields,
    })
}

const REFLECTION_MEMBER_DECLARING_CLASS_FIELD: usize = 0;
const REFLECTION_MEMBER_NAME_FIELD: usize = 1;
const REFLECTION_MEMBER_DESCRIPTOR_FIELD: usize = 2;
const REFLECTION_MEMBER_PUBLIC_FIELD: usize = 3;
const REFLECTION_MEMBER_STATIC_FIELD: usize = 4;
const REFLECTION_MEMBER_ACCESSIBLE_FIELD: usize = 5;

fn class_internal_name_from_key(class_key: &str) -> &str {
    class_key
        .split_once('\0')
        .map_or(class_key, |(internal_name, _)| internal_name)
}

fn allocate_class_object(heap: &mut duke_gc::Heap, class_key: &str) -> VmResult<u64> {
    let class_ref = heap.allocate("java/lang/Class".to_string(), 0);
    heap.get_mut(class_ref)?.string_value = Some(class_key.to_string());
    Ok(class_ref)
}

fn class_key_from_ref(heap: &duke_gc::Heap, class_ref: u64) -> VmResult<String> {
    heap.get(class_ref)?
        .string_value
        .clone()
        .ok_or(VmError::InvalidRef { address: class_ref })
}

fn class_internal_name_from_ref(heap: &duke_gc::Heap, class_ref: u64) -> VmResult<String> {
    Ok(class_internal_name_from_key(&class_key_from_ref(heap, class_ref)?).to_string())
}

fn allocate_reference_array(
    heap: &mut duke_gc::Heap,
    array_class_name: &str,
    elements: &[u64],
) -> VmResult<u64> {
    let array_ref = heap.allocate(array_class_name.to_string(), elements.len());
    {
        let array_obj = heap.get_mut(array_ref)?;
        for slot in &mut array_obj.fields {
            *slot = Slot::Reference(None);
        }
    }
    for (idx, element_ref) in elements.iter().enumerate() {
        heap.write_field(array_ref, idx, Slot::Reference(Some(*element_ref)))?;
    }
    Ok(array_ref)
}

fn allocate_reflection_member_object(
    heap: &mut duke_gc::Heap,
    member_class_name: &str,
    declaring_internal_name: &str,
    name: &str,
    descriptor: &str,
    is_public: bool,
    is_static: bool,
) -> VmResult<u64> {
    let member_ref = heap.allocate(member_class_name.to_string(), 6);
    let declaring_class_ref = allocate_class_object(heap, declaring_internal_name)?;
    let name_ref = heap.allocate_string(name.to_string());
    let descriptor_ref = heap.allocate_string(descriptor.to_string());
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_DECLARING_CLASS_FIELD,
        Slot::Reference(Some(declaring_class_ref)),
    )?;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_NAME_FIELD,
        Slot::Reference(Some(name_ref)),
    )?;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_DESCRIPTOR_FIELD,
        Slot::Reference(Some(descriptor_ref)),
    )?;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_PUBLIC_FIELD,
        Slot::Int(i32::from(is_public)),
    )?;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_STATIC_FIELD,
        Slot::Int(i32::from(is_static)),
    )?;
    heap.write_field(member_ref, REFLECTION_MEMBER_ACCESSIBLE_FIELD, Slot::Int(0))?;
    Ok(member_ref)
}

fn reflection_member_name_slot(heap: &duke_gc::Heap, member_ref: u64) -> VmResult<Slot> {
    heap.get(member_ref)?
        .fields
        .get(REFLECTION_MEMBER_NAME_FIELD)
        .copied()
        .ok_or(VmError::InvalidRef {
            address: member_ref,
        })
}

fn reflection_member_declaring_class_slot(heap: &duke_gc::Heap, member_ref: u64) -> VmResult<Slot> {
    heap.get(member_ref)?
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied()
        .ok_or(VmError::InvalidRef {
            address: member_ref,
        })
}

fn reflection_member_declaring_class_name_slot(
    heap: &mut duke_gc::Heap,
    member_ref: u64,
) -> VmResult<Slot> {
    let Slot::Reference(Some(class_ref)) =
        reflection_member_declaring_class_slot(heap, member_ref)?
    else {
        return Err(VmError::InvalidRef {
            address: member_ref,
        });
    };
    let class_key = class_key_from_ref(heap, class_ref)?;
    let binary_name = internal_name_to_binary_name(class_internal_name_from_key(&class_key));
    let name_ref = heap.allocate_string(binary_name);
    Ok(Slot::Reference(Some(name_ref)))
}

fn descriptor_class_key_from_source(
    ops: &mut dyn CallbackOps,
    descriptor: &str,
    source_class: Option<&str>,
) -> VmResult<String> {
    match descriptor.as_bytes().first().copied() {
        Some(b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b'V')
            if descriptor.len() == 1 =>
        {
            Ok(descriptor.to_string())
        }
        Some(b'[') => Ok(descriptor.to_string()),
        Some(b'L') if descriptor.ends_with(';') => {
            ops.class_key_from_source(&descriptor[1..descriptor.len() - 1], source_class)
        }
        _ => Err(VmError::TypeMismatch {
            expected: "type descriptor",
            got: "other",
        }),
    }
}

fn descriptor_class_slot_from_source(
    heap: &mut duke_gc::Heap,
    ops: &mut dyn CallbackOps,
    descriptor: &str,
    source_class: Option<&str>,
) -> VmResult<Slot> {
    let class_key = descriptor_class_key_from_source(ops, descriptor, source_class)?;
    let class_ref = allocate_class_object(heap, &class_key)?;
    Ok(Slot::Reference(Some(class_ref)))
}

fn reflection_array_elements(heap: &duke_gc::Heap, args_slot: Slot) -> VmResult<Vec<Slot>> {
    match args_slot {
        Slot::Reference(None) => Ok(Vec::new()),
        Slot::Reference(Some(array_ref)) => Ok(heap.get(array_ref)?.fields.clone()),
        _ => Err(VmError::TypeMismatch {
            expected: "reference array",
            got: "other",
        }),
    }
}

fn class_descriptor_from_class_ref(heap: &duke_gc::Heap, class_ref: u64) -> VmResult<String> {
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    if internal_name.starts_with('[') || internal_name.len() == 1 {
        Ok(internal_name)
    } else {
        Ok(format!("L{internal_name};"))
    }
}

fn parameter_descriptor_from_class_array(heap: &duke_gc::Heap, slot: Slot) -> VmResult<String> {
    let params = reflection_array_elements(heap, slot)?;
    let mut descriptor = String::from("(");
    for param in params {
        let class_ref = match param {
            Slot::Reference(Some(class_ref)) => class_ref,
            Slot::Reference(None) => return Err(VmError::NullPointerException),
            _ => {
                return Err(VmError::TypeMismatch {
                    expected: "reference array",
                    got: "other",
                });
            }
        };
        descriptor.push_str(&class_descriptor_from_class_ref(heap, class_ref)?);
    }
    descriptor.push(')');
    Ok(descriptor)
}

fn descriptor_parameter_part(descriptor: &str) -> &str {
    descriptor
        .find(')')
        .map_or(descriptor, |idx| &descriptor[..=idx])
}

fn lookup_public_reflected_field(
    ops: &mut dyn CallbackOps,
    class: &str,
    field_name: &str,
) -> VmResult<Option<(String, ReflectedFieldInfo)>> {
    lookup_public_reflected_field_inner(ops, class, field_name, &mut HashSet::new())
}

fn lookup_public_reflected_field_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    field_name: &str,
    visited: &mut HashSet<String>,
) -> VmResult<Option<(String, ReflectedFieldInfo)>> {
    if !visited.insert(class.to_string()) {
        return Ok(None);
    }
    let reflected = ops.inspect_class(class)?;
    if let Some(field) = reflected
        .fields
        .iter()
        .find(|field| field.is_public && field.name == field_name)
        .cloned()
    {
        return Ok(Some((class.to_string(), field)));
    }
    if let Some(super_class) = reflected.super_class.clone()
        && let Some(found) =
            lookup_public_reflected_field_inner(ops, &super_class, field_name, visited)?
    {
        return Ok(Some(found));
    }
    for interface in reflected.interfaces {
        if let Some(found) =
            lookup_public_reflected_field_inner(ops, &interface, field_name, visited)?
        {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

fn lookup_public_reflected_method(
    ops: &mut dyn CallbackOps,
    class: &str,
    method_name: &str,
    parameter_descriptor: &str,
) -> VmResult<Option<(String, ReflectedMethodInfo)>> {
    lookup_public_reflected_method_inner(
        ops,
        class,
        method_name,
        parameter_descriptor,
        &mut HashSet::new(),
    )
}

fn lookup_public_reflected_method_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    method_name: &str,
    parameter_descriptor: &str,
    visited: &mut HashSet<String>,
) -> VmResult<Option<(String, ReflectedMethodInfo)>> {
    if !visited.insert(class.to_string()) {
        return Ok(None);
    }
    let reflected = ops.inspect_class(class)?;
    if let Some(method) = reflected
        .methods
        .iter()
        .find(|method| {
            method.is_public
                && method.name != "<init>"
                && method.name != "<clinit>"
                && method.name == method_name
                && descriptor_parameter_part(&method.descriptor) == parameter_descriptor
        })
        .cloned()
    {
        return Ok(Some((class.to_string(), method)));
    }
    if let Some(super_class) = reflected.super_class.clone()
        && let Some(found) = lookup_public_reflected_method_inner(
            ops,
            &super_class,
            method_name,
            parameter_descriptor,
            visited,
        )?
    {
        return Ok(Some(found));
    }
    for interface in reflected.interfaces {
        if let Some(found) = lookup_public_reflected_method_inner(
            ops,
            &interface,
            method_name,
            parameter_descriptor,
            visited,
        )? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

fn collect_public_reflected_fields(
    ops: &mut dyn CallbackOps,
    class: &str,
) -> VmResult<Vec<(String, ReflectedFieldInfo)>> {
    let mut collected = Vec::new();
    collect_public_reflected_fields_inner(
        ops,
        class,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut collected,
    )?;
    Ok(collected)
}

fn collect_public_reflected_fields_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    visited_classes: &mut HashSet<String>,
    seen_fields: &mut HashSet<String>,
    collected: &mut Vec<(String, ReflectedFieldInfo)>,
) -> VmResult<()> {
    if !visited_classes.insert(class.to_string()) {
        return Ok(());
    }
    let reflected = ops.inspect_class(class)?;
    for field in reflected.fields {
        if !field.is_public {
            continue;
        }
        let key = format!("{class}\0{}\0{}", field.name, field.descriptor);
        if seen_fields.insert(key) {
            collected.push((class.to_string(), field));
        }
    }
    if let Some(super_class) = reflected.super_class {
        collect_public_reflected_fields_inner(
            ops,
            &super_class,
            visited_classes,
            seen_fields,
            collected,
        )?;
    }
    for interface in reflected.interfaces {
        collect_public_reflected_fields_inner(
            ops,
            &interface,
            visited_classes,
            seen_fields,
            collected,
        )?;
    }
    Ok(())
}

fn collect_public_reflected_methods(
    ops: &mut dyn CallbackOps,
    class: &str,
) -> VmResult<Vec<(String, ReflectedMethodInfo)>> {
    let mut collected = Vec::new();
    collect_public_reflected_methods_inner(
        ops,
        class,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut collected,
    )?;
    Ok(collected)
}

fn collect_public_reflected_methods_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    visited_classes: &mut HashSet<String>,
    seen_methods: &mut HashSet<String>,
    collected: &mut Vec<(String, ReflectedMethodInfo)>,
) -> VmResult<()> {
    if !visited_classes.insert(class.to_string()) {
        return Ok(());
    }
    let reflected = ops.inspect_class(class)?;
    for method in reflected.methods {
        if !method.is_public || method.name == "<init>" || method.name == "<clinit>" {
            continue;
        }
        let key = format!(
            "{}\0{}",
            method.name,
            descriptor_parameter_part(&method.descriptor)
        );
        if seen_methods.insert(key) {
            collected.push((class.to_string(), method));
        }
    }
    if let Some(super_class) = reflected.super_class {
        collect_public_reflected_methods_inner(
            ops,
            &super_class,
            visited_classes,
            seen_methods,
            collected,
        )?;
    }
    for interface in reflected.interfaces {
        collect_public_reflected_methods_inner(
            ops,
            &interface,
            visited_classes,
            seen_methods,
            collected,
        )?;
    }
    Ok(())
}

struct ReflectedFieldHandle {
    declaring_class_key: String,
    field_name: String,
    descriptor: String,
    is_public: bool,
    is_static: bool,
    is_accessible: bool,
}

fn reflected_field_handle(heap: &duke_gc::Heap, field_ref: u64) -> VmResult<ReflectedFieldHandle> {
    let field_obj = heap.get(field_ref)?;
    let Some(Slot::Reference(Some(declaring_class_ref))) = field_obj
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied()
    else {
        return Err(VmError::InvalidRef { address: field_ref });
    };
    let Some(Slot::Reference(Some(name_ref))) =
        field_obj.fields.get(REFLECTION_MEMBER_NAME_FIELD).copied()
    else {
        return Err(VmError::InvalidRef { address: field_ref });
    };
    let Some(Slot::Reference(Some(descriptor_ref))) = field_obj
        .fields
        .get(REFLECTION_MEMBER_DESCRIPTOR_FIELD)
        .copied()
    else {
        return Err(VmError::InvalidRef { address: field_ref });
    };
    let is_public = matches!(
        field_obj.fields.get(REFLECTION_MEMBER_PUBLIC_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );
    let is_static = matches!(
        field_obj.fields.get(REFLECTION_MEMBER_STATIC_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );
    let is_accessible = matches!(
        field_obj.fields.get(REFLECTION_MEMBER_ACCESSIBLE_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );

    Ok(ReflectedFieldHandle {
        declaring_class_key: class_key_from_ref(heap, declaring_class_ref)?,
        field_name: heap
            .get(name_ref)?
            .string_value
            .clone()
            .ok_or(VmError::NullPointerException)?,
        descriptor: heap
            .get(descriptor_ref)?
            .string_value
            .clone()
            .ok_or(VmError::NullPointerException)?,
        is_public,
        is_static,
        is_accessible,
    })
}

struct ReflectedMethodHandle {
    declaring_class_key: String,
    method_name: String,
    descriptor: String,
    is_public: bool,
    is_static: bool,
    is_accessible: bool,
}

fn reflected_method_handle(
    heap: &duke_gc::Heap,
    method_ref: u64,
) -> VmResult<ReflectedMethodHandle> {
    let method_obj = heap.get(method_ref)?;
    let Some(Slot::Reference(Some(declaring_class_ref))) = method_obj
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied()
    else {
        return Err(VmError::InvalidRef {
            address: method_ref,
        });
    };
    let Some(Slot::Reference(Some(name_ref))) =
        method_obj.fields.get(REFLECTION_MEMBER_NAME_FIELD).copied()
    else {
        return Err(VmError::InvalidRef {
            address: method_ref,
        });
    };
    let Some(Slot::Reference(Some(descriptor_ref))) = method_obj
        .fields
        .get(REFLECTION_MEMBER_DESCRIPTOR_FIELD)
        .copied()
    else {
        return Err(VmError::InvalidRef {
            address: method_ref,
        });
    };
    let is_public = matches!(
        method_obj.fields.get(REFLECTION_MEMBER_PUBLIC_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );
    let is_static = matches!(
        method_obj.fields.get(REFLECTION_MEMBER_STATIC_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );
    let is_accessible = matches!(
        method_obj.fields.get(REFLECTION_MEMBER_ACCESSIBLE_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );

    Ok(ReflectedMethodHandle {
        declaring_class_key: class_key_from_ref(heap, declaring_class_ref)?,
        method_name: heap
            .get(name_ref)?
            .string_value
            .clone()
            .ok_or(VmError::NullPointerException)?,
        descriptor: heap
            .get(descriptor_ref)?
            .string_value
            .clone()
            .ok_or(VmError::NullPointerException)?,
        is_public,
        is_static,
        is_accessible,
    })
}

fn build_reflection_invoke_args(
    heap: &duke_gc::Heap,
    target_slot: Slot,
    descriptor: &str,
    invoke_arg_slots: Vec<Slot>,
    is_static: bool,
) -> VmResult<Vec<Slot>> {
    let arg_types = parse_arg_types(descriptor);
    if arg_types.len() != invoke_arg_slots.len() {
        return Err(VmError::TypeMismatch {
            expected: "matching reflective argument count",
            got: "different count",
        });
    }

    let mut invoke_args = Vec::with_capacity(arg_types.len() + usize::from(!is_static));
    if !is_static {
        match target_slot {
            Slot::Reference(Some(_)) => invoke_args.push(target_slot),
            _ => return Err(VmError::NullPointerException),
        }
    }
    for (descriptor, arg) in arg_types.iter().copied().zip(invoke_arg_slots) {
        invoke_args.push(unbox_reflection_argument(heap, descriptor, arg)?);
        if matches!(descriptor, 'J' | 'D') {
            invoke_args.push(Slot::Int(0));
        }
    }
    Ok(invoke_args)
}

fn unbox_reflection_argument(heap: &duke_gc::Heap, descriptor: char, arg: Slot) -> VmResult<Slot> {
    match descriptor {
        'L' | '[' => match arg {
            Slot::Reference(_) => Ok(arg),
            _ => Err(VmError::TypeMismatch {
                expected: "reference",
                got: "other",
            }),
        },
        'B' | 'C' | 'I' | 'S' | 'Z' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(VmError::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Int(value)) => Ok(Slot::Int(*value)),
                _ => Err(VmError::TypeMismatch {
                    expected: "boxed int-like primitive",
                    got: "other",
                }),
            }
        }
        'J' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(VmError::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Long(value)) => Ok(Slot::Long(*value)),
                _ => Err(VmError::TypeMismatch {
                    expected: "boxed long",
                    got: "other",
                }),
            }
        }
        'F' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(VmError::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Float(value)) => Ok(Slot::Float(*value)),
                _ => Err(VmError::TypeMismatch {
                    expected: "boxed float",
                    got: "other",
                }),
            }
        }
        'D' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(VmError::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Double(value)) => Ok(Slot::Double(*value)),
                _ => Err(VmError::TypeMismatch {
                    expected: "boxed double",
                    got: "other",
                }),
            }
        }
        _ => Err(VmError::Unimplemented {
            mnemonic: "reflection primitive unboxing",
        }),
    }
}

fn descriptor_return_type(descriptor: &str) -> char {
    descriptor
        .split_once(')')
        .and_then(|(_, ret)| ret.chars().next())
        .unwrap_or('V')
}

fn method_return_descriptor(descriptor: &str) -> &str {
    descriptor.split_once(')').map_or("V", |(_, ret)| ret)
}

fn box_reflection_return_value(
    heap: &mut duke_gc::Heap,
    return_type: char,
    result: Option<Slot>,
) -> VmResult<Slot> {
    match return_type {
        'V' => Ok(Slot::Reference(None)),
        'L' | '[' => Ok(result.unwrap_or(Slot::Reference(None))),
        'B' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "byte result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Byte".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'C' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "char result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Character".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'D' => {
            let Some(Slot::Double(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "double result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Double".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Double(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'F' => {
            let Some(Slot::Float(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "float result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Float".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Float(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'I' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "int result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'J' => {
            let Some(Slot::Long(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "long result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Long".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Long(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'S' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "short result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Short".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'Z' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "boolean result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Boolean".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        _ => Err(VmError::Unimplemented {
            mnemonic: "reflection primitive boxing",
        }),
    }
}

/// Push a constant pool value onto the frame's operand stack.
fn ldc_push(frame: &mut Frame, cp: &[Option<CpEntry>], idx: usize) -> VmResult<()> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Integer(v)) => frame.push(Slot::Int(*v)),
        Some(CpEntry::Float(v)) => frame.push(Slot::Float(*v)),
        Some(CpEntry::Long(v)) => frame.push(Slot::Long(*v)),
        Some(CpEntry::Double(v)) => frame.push(Slot::Double(*v)),
        _ => Err(VmError::InvalidCpIndex { index: idx }),
    }
}

/// Check if `from` is a subtype of `to` (i.e., `from` can be assigned where `to` is expected).
///
/// Walks the class hierarchy from `from` upward through superclasses.
/// Returns `true` if `to` is found in the chain, or if `to` is `"java/lang/Object"`.
/// Return `true` if a reference of type `from` can be used where `to` is expected.
///
/// BFS over the full type graph (superclass + all implemented interfaces at each
/// level), so `String instanceof Comparable` resolves correctly.
fn is_assignable_from(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    from: &str,
    to: &str,
    target_source_class: Option<&str>,
) -> bool {
    let from_key = if registry.contains(from) {
        from.to_string()
    } else {
        registry.class_key_from_source(from, Some(from))
    };
    let to_key = if registry.contains(to) {
        to.to_string()
    } else {
        registry.class_key_from_source(to, target_source_class)
    };
    let from_internal = registry.internal_name_for_class(&from_key);
    let to_internal = registry.internal_name_for_class(&to_key);
    if from_key == to_key || to_internal == "java/lang/Object" {
        return true;
    }
    // Arrays implement Cloneable and Serializable; everything else is Object.
    if from_internal.starts_with('[') {
        return matches!(to_internal, "java/lang/Cloneable" | "java/io/Serializable");
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    queue.push_back(from_key);

    while let Some(current) = queue.pop_front() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if !registry.contains(&current) {
            let _ = registry.ensure_loaded_from(&current, Some(from), loader);
        }
        let (super_class, interfaces) = match registry.get(&current) {
            Ok(ctx) => (ctx.super_class.clone(), ctx.interfaces.clone()),
            Err(_) => continue,
        };
        if let Some(sc) = super_class {
            if sc == to_key {
                return true;
            }
            queue.push_back(sc);
        }
        for iface in interfaces {
            if iface == to_key {
                return true;
            }
            queue.push_back(iface);
        }
    }
    false
}

/// Search a method's exception table for a handler matching the given pc and exception class.
///
/// Uses hierarchy-aware type checking: a `catch(Exception)` will match a thrown
/// `RuntimeException` because `RuntimeException` is a subclass of `Exception`.
fn find_exception_handler(
    exception_table: &[ExceptionEntry],
    pc: usize,
    class_name: &str,
    handler_class: &str,
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
) -> Option<u16> {
    exception_table.iter().find_map(|entry| {
        let in_range = pc >= entry.start_pc as usize && pc < entry.end_pc as usize;
        #[allow(clippy::option_if_let_else)] // match is clearer with &mut registry
        let type_matches = match &entry.catch_type {
            None => true, // catch-all (finally)
            Some(ct) => is_assignable_from(registry, loader, class_name, ct, Some(handler_class)),
        };
        if in_range && type_matches {
            Some(entry.handler_pc)
        } else {
            None
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MethodHierarchyLookup {
    Bytecode(String, usize),
    NativeOverride,
    Missing,
}

fn has_registered_native_override(
    registry: &ClassRegistry,
    class_name: &str,
    method_name: &str,
    method_desc: &str,
) -> bool {
    lookup_registered_native_kind(registry, class_name, method_name, method_desc).is_some()
}

fn lookup_registered_native_kind(
    registry: &ClassRegistry,
    class_name: &str,
    method_name: &str,
    method_desc: &str,
) -> Option<HandlerKind> {
    registry
        .natives()
        .get_kind(class_name, method_name, method_desc)
        .or_else(|| {
            let internal_name = registry.internal_name_for_class(class_name);
            (internal_name != class_name)
                .then(|| {
                    registry
                        .natives()
                        .get_kind(internal_name, method_name, method_desc)
                })
                .flatten()
        })
}

/// Walk the class hierarchy to find a bytecode method by name and descriptor,
/// stopping early if an exact native override owns that slot.
fn resolve_method_in_hierarchy_lookup(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    start_class: &str,
    method_name: &str,
    method_desc: &str,
) -> MethodHierarchyLookup {
    let mut current = if registry.contains(start_class) {
        start_class.to_string()
    } else {
        registry.class_key_from_source(start_class, Some(start_class))
    };
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return MethodHierarchyLookup::Missing; // circular — bail
        }
        if has_registered_native_override(registry, &current, method_name, method_desc) {
            return MethodHierarchyLookup::NativeOverride;
        }
        if !registry.contains(&current) {
            let _ = registry.ensure_loaded_from(&current, Some(start_class), loader);
            current = registry.class_key_from_source(&current, Some(start_class));
        }
        match registry.get(&current) {
            Ok(ctx) => {
                if let Some(idx) = ctx
                    .methods
                    .iter()
                    .position(|m| m.name == method_name && m.descriptor == method_desc)
                {
                    return MethodHierarchyLookup::Bytecode(current, idx);
                }
                match &ctx.super_class {
                    Some(s) => current.clone_from(s),
                    None => return MethodHierarchyLookup::Missing,
                }
            }
            Err(_) => return MethodHierarchyLookup::Missing,
        }
    }
}

/// Walk the class hierarchy to find a method by name and descriptor.
/// Returns `(class_name_where_found, method_index)` or `None`.
fn resolve_method_in_hierarchy(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    start_class: &str,
    method_name: &str,
    method_desc: &str,
) -> Option<(String, usize)> {
    match resolve_method_in_hierarchy_lookup(
        registry,
        loader,
        start_class,
        method_name,
        method_desc,
    ) {
        MethodHierarchyLookup::Bytecode(class_name, method_idx) => Some((class_name, method_idx)),
        MethodHierarchyLookup::NativeOverride | MethodHierarchyLookup::Missing => None,
    }
}

/// Resolve a constant pool Methodref to (`class_name`, `method_name`, `descriptor`).
fn resolve_methodref(cp: &[Option<CpEntry>], idx: usize) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(
            CpEntry::Methodref {
                class_index,
                name_and_type_index,
            }
            | CpEntry::InterfaceMethodref {
                class_index,
                name_and_type_index,
            },
        ) => {
            let class_name = match cp.get(class_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    }
                }
                _ => return Err(VmError::InvalidMethodref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    };
                    Ok((class_name, name, desc))
                }
                _ => Err(VmError::InvalidMethodref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidMethodref { index: idx }),
    }
}

/// Count argument slots in a JVM method descriptor like `(ILjava/lang/String;[I)V`.
fn parse_arg_count(descriptor: &str) -> usize {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut count = 0;
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => count += 1,
            '[' => {
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
                count += 1;
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                count += 1;
            }
            _ => {}
        }
    }
    count
}

/// Resolve a constant pool Fieldref to (`class_name`, `field_name`, descriptor).
fn resolve_fieldref(cp: &[Option<CpEntry>], idx: usize) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Fieldref {
            class_index,
            name_and_type_index,
        }) => {
            let class_name = match cp.get(class_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    }
                }
                _ => return Err(VmError::InvalidFieldref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    Ok((class_name, name, desc))
                }
                _ => Err(VmError::InvalidFieldref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidFieldref { index: idx }),
    }
}

/// Resolve a `MethodHandle` CP entry to (`reference_kind`, `class_name`, `method_name`, descriptor).
fn resolve_method_handle(
    cp: &[Option<CpEntry>],
    cp_idx: usize,
) -> VmResult<(u8, String, String, String)> {
    let (kind, ref_idx) = match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::MethodHandle {
            reference_kind,
            reference_index,
        }) => (*reference_kind, reference_index.0 as usize),
        _ => return Err(VmError::InvalidCpIndex { index: cp_idx }),
    };
    let (class_name, method_name, descriptor) = resolve_methodref(cp, ref_idx)?;
    Ok((kind, class_name, method_name, descriptor))
}

/// Resolve a `NameAndType` CP entry to (name, descriptor).
fn resolve_name_and_type(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<(String, String)> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::NameAndType {
            name_index,
            descriptor_index,
        }) => {
            let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(VmError::InvalidCpIndex {
                        index: name_index.0 as usize,
                    });
                }
            };
            let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(VmError::InvalidCpIndex {
                        index: descriptor_index.0 as usize,
                    });
                }
            };
            Ok((name, desc))
        }
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Resolve a CP String entry to its UTF-8 content. Also handles bare Utf8 entries.
fn resolve_cp_string(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::String { string_index }) => {
            match cp.get(string_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => Err(VmError::InvalidCpIndex {
                    index: string_index.0 as usize,
                }),
            }
        }
        Some(CpEntry::Utf8(s)) => Ok(s.clone()),
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Parse argument type descriptors from a JVM method descriptor like `(IZLjava/lang/String;)V`.
/// Returns a Vec of single-char type codes: 'I', 'Z', 'L' (for object refs), '[' (for arrays), etc.
fn parse_arg_types(descriptor: &str) -> Vec<char> {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut types = Vec::new();
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => types.push(c),
            '[' => {
                // Skip array dimensions and element type.
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
                types.push('[');
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                types.push('L');
            }
            _ => {}
        }
    }
    types
}

/// Expand method arguments to JVM local variable slot layout.
///
/// In the JVM spec, `long` (`J`) and `double` (`D`) parameters each occupy
/// *two* local variable slots. The second slot is a phantom placeholder so
/// that all subsequent parameters are addressed at the correct index.
/// For example, a static `(JJ)J` method uses `lload_0` / `lload_2`; without
/// expansion, Duke would pack both longs at indices 0 and 1, causing
/// `lload_2` to see `Slot::Int(0)`.
///
/// If the descriptor contains no wide types this is a zero-copy clone.
fn expand_args_for_desc(args: &[Slot], descriptor: &str) -> Vec<Slot> {
    let param_types = parse_arg_types(descriptor);
    let wide_count = param_types
        .iter()
        .filter(|&&c| c == 'J' || c == 'D')
        .count();
    if wide_count == 0 || args.is_empty() {
        return args.to_vec();
    }
    let mut result = Vec::with_capacity(args.len() + wide_count);
    for (slot, &type_char) in args.iter().zip(param_types.iter()) {
        result.push(*slot);
        if type_char == 'J' || type_char == 'D' {
            result.push(Slot::Int(0));
        }
    }
    // Preserve any trailing args that exceed the descriptor param count (captures, etc.)
    if args.len() > param_types.len() {
        result.extend_from_slice(&args[param_types.len()..]);
    }
    result
}

fn parse_arg_descriptors(descriptor: &str) -> Vec<String> {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut descriptors = Vec::new();
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => descriptors.push(c.to_string()),
            '[' => {
                let mut desc = String::from("[");
                while chars.peek() == Some(&'[') {
                    desc.push(chars.next().unwrap_or('['));
                }
                if chars.peek() == Some(&'L') {
                    desc.push(chars.next().unwrap_or('L'));
                    for c2 in chars.by_ref() {
                        desc.push(c2);
                        if c2 == ';' {
                            break;
                        }
                    }
                } else if let Some(elem) = chars.next() {
                    desc.push(elem);
                }
                descriptors.push(desc);
            }
            'L' => {
                let mut desc = String::from("L");
                for c2 in chars.by_ref() {
                    desc.push(c2);
                    if c2 == ';' {
                        break;
                    }
                }
                descriptors.push(desc);
            }
            _ => {}
        }
    }
    descriptors
}

/// Index of a named instance field within ctx.fields (non-static only).
/// Return the correct default [`Slot`] for a field with the given JVM descriptor.
///
/// Per JVMS §2.3/2.4: numeric types default to 0, reference/array types to null.
#[inline]
fn default_slot_for_descriptor(desc: &str) -> Slot {
    match desc.chars().next() {
        Some('J') => Slot::Long(0),
        Some('F') => Slot::Float(0.0),
        Some('D') => Slot::Double(0.0),
        Some('L' | '[') => Slot::Reference(None),
        _ => Slot::Int(0), // I, Z, B, C, S
    }
}

/// reference-typed fields (`L…;` / `[…`), which must be `Reference(None)`.
/// Sum a class's instance fields across its full superclass chain.
fn total_instance_field_count(registry: &ClassRegistry, class_name: &str) -> usize {
    let mut count = registry
        .get(class_name)
        .map(|c| c.instance_field_count)
        .unwrap_or(0);
    let mut sc = registry
        .get(class_name)
        .ok()
        .and_then(|c| c.super_class.clone());
    while let Some(ref s) = sc {
        match registry.get(s) {
            Ok(sctx) => {
                count += sctx.instance_field_count;
                sc = sctx.super_class.clone();
            }
            Err(_) => break,
        }
    }
    count
}

/// Allocate a heap exception object for a native-thrown Java exception.
///
/// Native handlers currently surface Java exceptions as class names. Catch
/// blocks need an object reference on the operand stack, so we materialize a
/// minimal heap object of that class before routing through exception-table
/// dispatch.
fn materialize_java_exception_object(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    class_name: &str,
) -> VmResult<u64> {
    registry.ensure_loaded(class_name, loader)?;
    let exc_ref = heap.allocate(
        class_name.to_string(),
        total_instance_field_count(registry, class_name),
    );
    init_object_fields(registry, heap, exc_ref, class_name);
    Ok(exc_ref)
}

fn allocate_reflection_instance(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    class: &str,
) -> VmResult<u64> {
    if !registry.contains(class) {
        registry.ensure_loaded(class, loader)?;
    }
    let class_key = registry.resolve_loaded_class_key(class)?;
    ensure_initialized(registry, loader, heap, output, &class_key, &class_key)?;
    let object_ref = heap.allocate(
        class_key.clone(),
        total_instance_field_count(registry, &class_key),
    );
    init_object_fields(registry, heap, object_ref, &class_key);
    Ok(object_ref)
}

/// After allocating an object on the heap, initialize each field slot to the
/// JVM-spec default for its descriptor.
///
/// `heap.allocate` sets all slots to `Slot::Int(0)`, which is wrong for
/// reference-typed fields (`L...;` / `[...]`), which must be `Reference(None)`.
/// This function walks the full class hierarchy (Object-first) and writes the
/// correct default into every slot that differs from `Int(0)`.
fn init_object_fields(
    registry: &ClassRegistry,
    heap: &mut duke_gc::Heap,
    obj_ref: u64,
    class_name: &str,
) {
    // Collect hierarchy: class_name → … → root
    let mut chain: Vec<String> = Vec::new();
    let mut cur = Some(class_name.to_string());
    while let Some(cls) = cur {
        if let Ok(ctx) = registry.get(&cls) {
            let sc = ctx.super_class.clone();
            chain.push(cls);
            cur = sc;
        } else {
            break;
        }
    }
    chain.reverse(); // Object-first

    let mut slot_idx = 0usize;
    for cls in &chain {
        if let Ok(ctx) = registry.get(cls) {
            for field in ctx.fields.iter().filter(|f| !f.is_static) {
                let default = default_slot_for_descriptor(&field.descriptor);
                // Only write non-Int-zero defaults (avoids an unnecessary mut borrow).
                if !matches!(default, Slot::Int(0))
                    && let Ok(obj) = heap.get_mut(obj_ref)
                    && slot_idx < obj.fields.len()
                {
                    obj.fields[slot_idx] = default;
                }
                slot_idx += 1;
            }
        }
    }
}

/// Compute the absolute slot index of a named instance field within a heap
/// object whose class is `target_class` (or any subclass of it).
///
/// JVM `Fieldref` entries name the access class (often a subclass), not
/// necessarily the declaring class. This function walks the full hierarchy
/// from the root (Object) down to `target_class`, searching each class for
/// the field and accumulating the running slot offset as it goes.
///
/// Layout: root fields occupy the lowest-numbered slots; each subclass
/// appends its fields immediately after its superclass's fields.
fn field_slot_idx(registry: &ClassRegistry, target_class: &str, name: &str) -> VmResult<usize> {
    let mut current = target_class;

    while let Ok(ctx) = registry.get(current) {
        if let Some(local_idx) = ctx
            .fields
            .iter()
            .filter(|f| !f.is_static)
            .position(|f| f.name == name)
        {
            let super_fields = ctx
                .super_class
                .as_deref()
                .map_or(0, |sc| total_instance_field_count(registry, sc));
            return Ok(super_fields + local_idx);
        }

        if let Some(ref sc) = ctx.super_class {
            current = sc;
        } else {
            break;
        }
    }

    Err(VmError::InvalidFieldref { index: 0 })
}

/// Index of a named static field within `ctx.static_fields`.
fn static_field_idx(ctx: &ClassContext, name: &str) -> VmResult<usize> {
    ctx.fields
        .iter()
        .filter(|f| f.is_static)
        .position(|f| f.name == name)
        .ok_or(VmError::InvalidFieldref { index: 0 })
}

// ---------------------------------------------------------------------------
// StringBuilder natives
// ---------------------------------------------------------------------------

/// Native: `StringBuilder.<init>()V` — initialise empty buffer.
pub(crate) fn native_sb_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    obj.string_value = Some(String::new());
    Ok(None)
}

/// Native: `StringBuilder.<init>(Ljava/lang/String;)V` — init with string.
pub(crate) fn native_sb_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let init_str = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone().unwrap_or_default(),
        _ => String::new(),
    };
    let obj = heap.get_mut(this_ref)?;
    obj.string_value = Some(init_str);
    Ok(None)
}

/// Native: `StringBuilder.append(Ljava/lang/String;)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let append_str = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone().unwrap_or_default(),
        _ => "null".to_string(),
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&append_str);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(I)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_int_arg(args, 1)?;
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(J)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_long_arg(args, 1)?;
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(D)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_double_arg(args, 1)?;
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(F)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_float_arg(args, 1)?;
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(Z)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v != 0,
        _ => false,
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(if val { "true" } else { "false" });
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(C)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = match args.get(1) {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('\0'),
        _ => '\0',
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push(val);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.toString()Ljava/lang/String;`
pub(crate) fn native_sb_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let content = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(content);
    Ok(Some(Slot::Reference(Some(r))))
}

// ---- StringBuilder extended operations ----

/// Native: `StringBuilder.insert(int, String)StringBuilder` — inserts string at index.
pub(crate) fn native_sb_insert_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let offset = extract_int_arg(args, 1)?;
    let s = match args.get(2) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone().unwrap_or_default(),
        Some(Slot::Reference(None)) | None => "null".to_string(),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "String",
                got: "other",
            });
        }
    };
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    let byte_idx = offset
        .try_into()
        .ok()
        .and_then(|i: usize| buf.char_indices().nth(i).map(|(b, _)| b))
        .unwrap_or(buf.len());
    buf.insert_str(byte_idx, &s);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.insert(int, char)StringBuilder` — inserts char at index.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_sb_insert_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let offset = extract_int_arg(args, 1)?;
    let ch = char::from_u32(extract_int_arg(args, 2)? as u32).unwrap_or('\0');
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    let byte_idx = offset
        .try_into()
        .ok()
        .and_then(|i: usize| buf.char_indices().nth(i).map(|(b, _)| b))
        .unwrap_or(buf.len());
    buf.insert(byte_idx, ch);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.delete(int, int)StringBuilder` — removes chars in [start, end).
pub(crate) fn native_sb_delete(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let start = extract_int_arg(args, 1)?;
    let end = extract_int_arg(args, 2)?;
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    let start_byte = usize::try_from(start)
        .ok()
        .and_then(|i| buf.char_indices().nth(i).map(|(b, _)| b))
        .unwrap_or(buf.len());
    let end_byte = usize::try_from(end)
        .ok()
        .and_then(|i| buf.char_indices().nth(i).map(|(b, _)| b))
        .unwrap_or(buf.len());
    buf.drain(start_byte..end_byte);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.deleteCharAt(int)StringBuilder` — removes single char at index.
pub(crate) fn native_sb_delete_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let index = extract_int_arg(args, 1)?;
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    if let Some(i) = usize::try_from(index)
        .ok()
        .and_then(|i| buf.char_indices().nth(i).map(|(b, _)| b))
    {
        buf.remove(i);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.reverse()StringBuilder` — reverses the character sequence.
pub(crate) fn native_sb_reverse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    *buf = buf.chars().rev().collect();
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.charAt(int)C` — returns char at given index.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_sb_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let index = extract_int_arg(args, 1)?;
    let ch = heap
        .get(this_ref)?
        .string_value
        .as_deref()
        .and_then(|s| usize::try_from(index).ok().and_then(|i| s.chars().nth(i)))
        .unwrap_or('\0');
    Ok(Some(Slot::Int(ch as i32)))
}

/// Native: `StringBuilder.setLength(int)V` — truncates or pads with null chars.
pub(crate) fn native_sb_set_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let new_len = usize::try_from(extract_int_arg(args, 1)?).unwrap_or(0);
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    let char_count = buf.chars().count();
    if new_len <= char_count {
        *buf = buf.chars().take(new_len).collect();
    } else {
        buf.extend(std::iter::repeat_n('\0', new_len - char_count));
    }
    Ok(None)
}

/// Native: `StringBuilder.length()I`
pub(crate) fn native_sb_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let len = heap
        .get(this_ref)?
        .string_value
        .as_ref()
        .map_or(0, String::len);
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}

// Character natives
// ---------------------------------------------------------------------------

/// Helper: extract a `char` from a `Slot::Int` argument.
fn slot_to_char(slot: &Slot) -> VmResult<char> {
    match slot {
        Slot::Int(v) => Ok(char::from_u32((*v).cast_unsigned()).unwrap_or('\0')),
        _ => Err(VmError::TypeMismatch {
            expected: "Int (char)",
            got: "other",
        }),
    }
}

/// Native: `Character.isDigit(C)Z`
pub(crate) fn native_char_is_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_ascii_digit()))))
}

/// Native: `Character.isLetter(C)Z`
pub(crate) fn native_char_is_letter(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphabetic()))))
}

/// Native: `Character.isWhitespace(C)Z`
pub(crate) fn native_char_is_whitespace(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_whitespace()))))
}

/// Native: `Character.isUpperCase(C)Z`
pub(crate) fn native_char_is_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_uppercase()))))
}

/// Native: `Character.isLowerCase(C)Z`
pub(crate) fn native_char_is_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_lowercase()))))
}

/// Native: `Character.toUpperCase(C)C`
pub(crate) fn native_char_to_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    let upper = ch.to_uppercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(upper as i32)))
}

/// Native: `Character.toLowerCase(C)C`
pub(crate) fn native_char_to_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    let lower = ch.to_lowercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(lower as i32)))
}

/// Native: `Character.isLetterOrDigit(C)Z`
pub(crate) fn native_char_is_letter_or_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphanumeric()))))
}

/// Native: `Character.valueOf(C)Ljava/lang/Character;` — box a char.
pub(crate) fn native_char_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int (char)",
                got: "other",
            });
        }
    };
    let r = heap.allocate("java/lang/Character".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Character.charValue()C` — unbox Character to char.
pub(crate) fn native_char_charvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

// ---------------------------------------------------------------------------
// ArrayList natives
// ---------------------------------------------------------------------------

/// Native: `ArrayList.<init>()V` — initializes with size=0.
pub(crate) fn native_arraylist_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ArrayList.<init>(Collection)V` — copies elements from another `ArrayList`/collection.
pub(crate) fn native_arraylist_init_from_collection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Reference(Some(src_ref))) = args.get(1).copied() else {
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
        return Ok(None);
    };
    // Copy size and elements from source collection (ArrayList layout: fields[0]=size, fields[1..]=elems)
    let src_fields = heap.get(src_ref)?.fields.clone();
    let src_size = match src_fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(src_size);
    for elem in src_fields
        .into_iter()
        .skip(1)
        .take(usize::try_from(src_size).unwrap_or(0))
    {
        heap.get_mut(this_ref)?.fields.push(elem);
    }
    Ok(None)
}

/// Native: `ArrayList.add(Object)Z` — appends element, returns true.
pub(crate) fn native_arraylist_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException), // shouldn't happen; init sets fields[0]=Int(0)
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1))) // boolean true
}

/// Native: `ArrayList.get(I)Object` — returns element at index.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)? as usize;
    let obj = heap.get(this_ref)?;
    obj.fields.get(idx + 1).map_or_else(
        || {
            Err(VmError::JavaException {
                class_name: "java/lang/ArrayIndexOutOfBoundsException".to_string(),
            })
        },
        |slot| Ok(Some(*slot)),
    )
}

/// Native: `ArrayList.size()I`
pub(crate) fn native_arraylist_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(Slot::Int(sz)) => Ok(Some(Slot::Int(*sz))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `ArrayList.iterator()Iterator` — creates an `ArrayListIterator`.
pub(crate) fn native_arraylist_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_ref = heap.allocate("duke/util/ArrayListIterator".to_string(), 2);
    {
        let iter_obj = heap.get_mut(iter_ref)?;
        iter_obj.fields[0] = Slot::Reference(Some(this_ref));
        iter_obj.fields[1] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
}

fn collection_elements_from_ref(heap: &duke_gc::Heap, collection_ref: u64) -> VmResult<Vec<Slot>> {
    let collection = heap.get(collection_ref)?;
    match collection.class_name.as_str() {
        "java/util/ArrayList" | "java/util/HashSet" => {
            let size = match collection.fields.first() {
                Some(Slot::Int(size)) if *size >= 0 => usize::try_from(*size).unwrap_or(0),
                _ => 0,
            };
            Ok(collection
                .fields
                .iter()
                .skip(1)
                .take(size)
                .copied()
                .collect())
        }
        _ => Err(VmError::TypeMismatch {
            expected: "java/util/Collection",
            got: "other",
        }),
    }
}

fn allocate_reference_array_from_slots(
    heap: &mut duke_gc::Heap,
    array_class_name: &str,
    elements: &[Slot],
) -> VmResult<u64> {
    let array_ref = heap.allocate(array_class_name.to_string(), elements.len());
    let array = heap.get_mut(array_ref)?;
    for slot in &mut array.fields {
        *slot = Slot::Reference(None);
    }
    for (idx, element) in elements.iter().enumerate() {
        array.fields[idx] = *element;
    }
    Ok(array_ref)
}

/// Native: `Collection.toArray()` — copies Duke-backed collection elements into `Object[]`.
pub(crate) fn native_collection_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elements = collection_elements_from_ref(heap, this_ref)?;
    let array_ref = allocate_reference_array_from_slots(heap, "[Ljava/lang/Object;", &elements)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

/// Native: `Collection.toArray(Object[])` — preserves the requested array type.
pub(crate) fn native_collection_to_array_with_seed_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let seed_array_ref = extract_ref_arg(args, 1)?;
    let elements = collection_elements_from_ref(heap, this_ref)?;
    let seed_class_name = heap.get(seed_array_ref)?.class_name.clone();
    let seed_len = heap.get(seed_array_ref)?.fields.len();

    if seed_len < elements.len() {
        let new_array_ref = allocate_reference_array_from_slots(heap, &seed_class_name, &elements)?;
        return Ok(Some(Slot::Reference(Some(new_array_ref))));
    }

    let seed_array = heap.get_mut(seed_array_ref)?;
    for slot in &mut seed_array.fields {
        *slot = Slot::Reference(None);
    }
    for (idx, element) in elements.iter().enumerate() {
        seed_array.fields[idx] = *element;
    }
    Ok(Some(Slot::Reference(Some(seed_array_ref))))
}

/// Native: `ArrayList.sort(Comparator)V` — sorts in-place using insertion sort,
/// calling `compareTo` on each element pair via the interpreter callback.
///
/// Only null Comparator (natural ordering via `compareTo`) is supported.
pub(crate) fn array_list_sort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // args[0] = ArrayList ref, args[1] = Comparator (null = natural ordering)
    let list_ref = extract_ref_arg(args, 0)?;

    // args[1] = optional Comparator ref (null = natural ordering via compareTo)
    let comparator = args.get(1).copied();

    // Fix 2: guard against a negative size stored in fields[0].
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(VmError::NegativeArraySize { size: *n }),
        _ => return Ok(None),
    };

    if size <= 1 {
        return Ok(None);
    }

    // Collect element refs (fields[1..=size]).
    let mut elems: Vec<u64> = (1..=size)
        .filter_map(|i| match heap.get(list_ref).ok()?.fields.get(i) {
            Some(Slot::Reference(Some(r))) => Some(*r),
            _ => None,
        })
        .collect();

    // Fix 3: malformed list → InvalidRef, not silent Ok(None).
    if elems.len() != size {
        return Err(VmError::InvalidRef { address: list_ref });
    }

    // Insertion sort — O(n²), correct, easy to verify.
    for i in 1..elems.len() {
        let mut key = elems[i];
        let mut j = i;
        while j > 0 {
            let receiver = elems[j - 1];
            let cmp = if let Some(Slot::Reference(Some(comp_ref))) = comparator {
                let comp_class = heap.get(comp_ref)?.class_name.clone();
                ops.invoke(
                    heap,
                    output,
                    &comp_class,
                    "compare",
                    "(Ljava/lang/Object;Ljava/lang/Object;)I",
                    vec![
                        Slot::Reference(Some(comp_ref)),
                        Slot::Reference(Some(receiver)),
                        Slot::Reference(Some(key)),
                    ],
                )?
            } else {
                let class_name = heap.get(receiver)?.class_name.clone();
                ops.invoke(
                    heap,
                    output,
                    &class_name,
                    COMPARE_TO_METHOD,
                    COMPARE_TO_OBJECT_DESC,
                    vec![Slot::Reference(Some(receiver)), Slot::Reference(Some(key))],
                )?
            };

            // Patch stale young-gen refs if a minor GC fired during the callback.
            // Guarded by `has_pending_forwards` so the common (no-GC) path pays
            // only one bool check instead of O(n) HashMap probes.
            if heap.has_pending_forwards() {
                for elem in &mut elems {
                    let mut slot = Slot::Reference(Some(*elem));
                    heap.apply_forward(&mut slot);
                    if let Slot::Reference(Some(r)) = slot {
                        *elem = r;
                    }
                }
                // Re-read key after forwarding patch (it lives outside elems
                // during the innermost loop iteration).
                let mut key_slot = Slot::Reference(Some(key));
                heap.apply_forward(&mut key_slot);
                if let Slot::Reference(Some(r)) = key_slot {
                    key = r;
                }
            }

            // Fix 4: explicit error on non-Int compareTo return.
            match cmp {
                Some(Slot::Int(n)) if n <= 0 => break,
                Some(Slot::Int(_)) => {} // n > 0, keep shifting
                _ => {
                    return Err(VmError::TypeMismatch {
                        expected: "Int",
                        got: "other",
                    });
                }
            }
            elems[j] = elems[j - 1];
            j -= 1;
        }
        elems[j] = key;
    }

    // Write sorted elements back using the write barrier.
    for (i, &r) in elems.iter().enumerate() {
        heap.write_field(list_ref, i + 1, Slot::Reference(Some(r)))?;
    }

    Ok(None)
}

// ---------------------------------------------------------------------------
// Collections natives
// ---------------------------------------------------------------------------

/// Native: `Collections.sort(List)V` — delegates to the list's sort(null) method.
///
/// `Collections.sort(list)` is compiled by javac as
/// `invokestatic java/util/Collections.sort:(Ljava/util/List;)V`.
/// We forward to the runtime class's `sort(Comparator=null)`, which for an
/// `ArrayList` performs the insertion-sort-with-compareTo callback.
pub(crate) fn native_collections_sort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // args[0] = List ref
    let list_ref = extract_ref_arg(args, 0)?;
    // Dispatch on the actual runtime class so any List implementation works.
    let class_name = heap.get(list_ref)?.class_name.clone();
    ops.invoke(
        heap,
        output,
        &class_name,
        "sort",
        SORT_COMPARATOR_DESC,
        vec![Slot::Reference(Some(list_ref)), Slot::Reference(None)],
    )?;
    Ok(None)
}

// ---------------------------------------------------------------------------
// ArrayListIterator natives
// ---------------------------------------------------------------------------

/// Native: `ArrayListIterator.<init>` — no-op; fields set directly by `native_arraylist_iterator`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_arraylist_iter_init(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `ArrayListIterator.hasNext()Z`
pub(crate) fn native_arraylist_iter_hasnext(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_obj = heap.get(this_ref)?;
    let list_ref = match iter_obj.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Int(0))),
    };
    let cursor = match iter_obj.fields.get(1) {
        Some(Slot::Int(i)) => *i,
        _ => 0,
    };
    let list_size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(cursor < list_size))))
}

/// Native: `ArrayListIterator.next()Object` — returns element at cursor, advances cursor.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (list_ref, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let lr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let c = match iter_obj.fields.get(1) {
            Some(Slot::Int(i)) => *i,
            _ => 0,
        };
        (lr, c)
    };
    let element = {
        let list_obj = heap.get(list_ref)?;
        match list_obj.fields.get(cursor as usize + 1) {
            Some(slot) => *slot,
            None => {
                return Err(VmError::JavaException {
                    class_name: "java/util/NoSuchElementException".to_string(),
                });
            }
        }
    };
    heap.get_mut(this_ref)?.fields[1] = Slot::Int(cursor + 1);
    Ok(Some(element))
}

// ---- ArrayList extended methods ----

/// Native: `ArrayList.remove(I)Object` — removes element at index, returns it.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_remove_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)?;
    if idx < 0 {
        return Err(VmError::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let idx = idx as usize;
    let len = heap.get(this_ref)?.fields.len();
    // fields[0]=size, elements start at 1; idx is 0-based element index → field index = idx+1
    if idx + 1 >= len {
        return Err(VmError::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let removed = heap.get(this_ref)?.fields[idx + 1];
    let obj = heap.get_mut(this_ref)?;
    obj.fields.remove(idx + 1);
    if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
        *sz -= 1;
    }
    Ok(Some(removed))
}

/// Native: `ArrayList.remove(Object)Z` — removes first occurrence, returns true if found.
pub(crate) fn native_arraylist_remove_obj(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();
    let found = fields
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, slot)| slots_equal(slot, &target, heap))
        .map(|(i, _)| i);
    if let Some(field_idx) = found {
        let obj = heap.get_mut(this_ref)?;
        obj.fields.remove(field_idx);
        if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
            *sz -= 1;
        }
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `ArrayList.contains(Object)Z` — returns 1 if element is present.
pub(crate) fn native_arraylist_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();
    let found = fields
        .iter()
        .skip(1)
        .any(|slot| slots_equal(slot, &target, heap));
    Ok(Some(Slot::Int(i32::from(found))))
}

/// Native: `ArrayList.clear()V` — removes all elements.
pub(crate) fn native_arraylist_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields.truncate(1);
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ArrayList.isEmpty()Z` — returns 1 if size is 0.
pub(crate) fn native_arraylist_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let is_empty = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz == 0,
        _ => true,
    };
    Ok(Some(Slot::Int(i32::from(is_empty))))
}

/// Native: `ArrayList.set(I,Object)Object` — replaces element at index, returns old value.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)?;
    let value = extract_slot_arg(args, 2);
    if idx < 0 {
        return Err(VmError::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let field_idx = idx as usize + 1;
    let old = *heap
        .get(this_ref)?
        .fields
        .get(field_idx)
        .ok_or_else(|| VmError::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        })?;
    heap.get_mut(this_ref)?.fields[field_idx] = value;
    Ok(Some(old))
}

/// Native: `ArrayList.indexOf(Object)I` — returns first index of element, or -1.
pub(crate) fn native_arraylist_index_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();
    let idx = fields
        .iter()
        .skip(1)
        .position(|slot| slots_equal(slot, &target, heap))
        .map_or(-1, |i| i32::try_from(i).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(idx)))
}

/// Native: `ArrayList.add(I,Object)V` — inserts element at index, shifting others right.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_add_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)?;
    let element = extract_slot_arg(args, 2);
    if idx < 0 {
        return Err(VmError::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let field_idx = idx as usize + 1;
    let obj = heap.get_mut(this_ref)?;
    let len = obj.fields.len();
    if field_idx > len {
        return Err(VmError::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    obj.fields.insert(field_idx, element);
    if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
        *sz += 1;
    }
    Ok(None)
}

// ---- HashMap extended methods ----

/// Native: `HashMap.putIfAbsent(K,V)Object` — inserts only if key is absent; returns existing or null.
pub(crate) fn native_hashmap_put_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let val = extract_slot_arg(args, 2);
    let fields = heap.get(this_ref)?.fields.clone();
    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        // Key already present — return existing value.
        return Ok(Some(fields[i + 1]));
    }
    // Key absent — insert and return null.
    let obj = heap.get_mut(this_ref)?;
    if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
        *sz += 1;
    }
    obj.fields.push(key);
    obj.fields.push(val);
    Ok(Some(Slot::Reference(None)))
}

/// Native: `HashMap.clear()V` — removes all entries.
pub(crate) fn native_hashmap_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields.truncate(1);
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `HashMap.containsValue(Object)Z` — returns 1 if any entry has this value.
pub(crate) fn native_hashmap_contains_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();
    // Values are at even indices: 2, 4, 6, ...
    let mut i = 2usize;
    while i < fields.len() {
        if slots_equal(&fields[i], &target, heap) {
            return Ok(Some(Slot::Int(1)));
        }
        i += 2;
    }
    Ok(Some(Slot::Int(0)))
}

// ---- Double.isNaN ----

/// Native: `Double.isNaN(D)Z` — returns 1 if value is NaN.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_isnan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Double(v)) => Ok(Some(Slot::Int(i32::from(v.is_nan())))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `Double.compareTo(Object)` — compares two boxed Doubles.
pub(crate) fn native_double_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let double_val = |s: &Slot| -> VmResult<f64> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Double(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => double_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = double_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    // Use total_cmp: implements Java's total order where NaN > +∞ > … > -∞.
    Ok(Some(Slot::Int(ordering_to_int(a.total_cmp(&b)))))
}

// ---- Arrays natives ----

/// Native: `Arrays.fill(int[], int)` — fills all elements with val.
pub(crate) fn native_arrays_fill_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let val = match args.get(1) {
        Some(Slot::Int(v)) => Slot::Int(*v),
        _ => Slot::Int(0),
    };
    let obj = heap.get_mut(arr_ref)?;
    for slot in &mut obj.fields {
        *slot = val;
    }
    Ok(None)
}

/// Native: `Arrays.fill(Object[], Object)` — fills all elements with val.
pub(crate) fn native_arrays_fill_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let val = extract_slot_arg(args, 1);
    let obj = heap.get_mut(arr_ref)?;
    for slot in &mut obj.fields {
        *slot = val;
    }
    Ok(None)
}

/// Native: `Arrays.copyOf(int[], int)` — copies to new int[] of given length.
pub(crate) fn native_arrays_copyof_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let new_len = match args.get(1) {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(VmError::NegativeArraySize { size: *n }),
        _ => 0,
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[I".to_string(), new_len);
    let dst = heap.get_mut(dst_ref)?;
    for i in 0..new_len {
        dst.fields[i] = src_fields.get(i).copied().unwrap_or(Slot::Int(0));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.copyOf(Object[], int)` — copies to new Object[] of given length.
pub(crate) fn native_arrays_copyof_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let new_len = match args.get(1) {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(VmError::NegativeArraySize { size: *n }),
        _ => 0,
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[Ljava/lang/Object;".to_string(), new_len);
    let dst = heap.get_mut(dst_ref)?;
    for i in 0..new_len {
        dst.fields[i] = src_fields.get(i).copied().unwrap_or(Slot::Reference(None));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.sort(int[])` — sorts fields in place.
pub(crate) fn native_arrays_sort_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(arr_ref)?;
    obj.fields.sort_by(|a, b| match (a, b) {
        (Slot::Int(x), Slot::Int(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    });
    Ok(None)
}

// ---------------------------------------------------------------------------
// Phase 44: Arrays.copyOfRange, List.subList, Comparator.reversed,
//           Collections.binarySearch, String.intern, ArrayList.removeIf
// ---------------------------------------------------------------------------

/// Native: `Arrays.copyOfRange(int[], int, int)int[]` — slice of int array, zero-padded.
pub(crate) fn native_arrays_copy_of_range_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let from = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let to = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let new_len = to.saturating_sub(from);
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[I".to_string(), new_len);
    for i in 0..new_len {
        heap.get_mut(dst_ref)?.fields[i] =
            src_fields.get(from + i).copied().unwrap_or(Slot::Int(0));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.copyOfRange(Object[], int, int)Object[]` — slice of reference array.
pub(crate) fn native_arrays_copy_of_range_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let from = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let to = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let new_len = to.saturating_sub(from);
    let (src_class, src_fields) = {
        let obj = heap.get(src_ref)?;
        (obj.class_name.clone(), obj.fields.clone())
    };
    let dst_ref = heap.allocate(src_class, new_len);
    for i in 0..new_len {
        heap.get_mut(dst_ref)?.fields[i] = src_fields
            .get(from + i)
            .copied()
            .unwrap_or(Slot::Reference(None));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `ArrayList.subList(int, int)List` — returns a new `ArrayList` with the sub-range.
pub(crate) fn native_arraylist_sub_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let from = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let to = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let new_len = to.saturating_sub(from);
    // ArrayList layout: fields[0]=size, fields[1..]=elements
    let src_elems: Vec<Slot> = {
        let fields = &heap.get(this_ref)?.fields;
        fields
            .iter()
            .skip(1 + from)
            .take(new_len)
            .copied()
            .collect()
    };
    let sub_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    heap.get_mut(sub_ref)?.fields[0] = Slot::Int(i32::try_from(new_len).unwrap_or(0));
    for elem in src_elems {
        heap.get_mut(sub_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(sub_ref))))
}

/// Native: `ArrayList.removeIf(Predicate)Z` — removes all elements where predicate returns true.
pub(crate) fn native_arraylist_remove_if(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let mut kept = Vec::with_capacity(elems.len());
    let mut removed = false;
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![fn_slot, elem],
            )?
            .unwrap_or(Slot::Int(0));
        match result {
            Slot::Int(1) => {
                removed = true;
            } // predicate true → remove
            _ => kept.push(elem),
        }
    }
    let new_size = i32::try_from(kept.len()).unwrap_or(0);
    heap.get_mut(this_ref)?.fields.truncate(1);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    for elem in kept {
        heap.get_mut(this_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Int(i32::from(removed))))
}

/// Native: `Comparator.reversed()Comparator` — wraps comparator to invert ordering.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_reversed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let delegate = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ReversedComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = delegate;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ReversedComparator.compare(a, b)I` — inverts delegate comparison.
pub(crate) fn native_reversed_comparator_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let delegate = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(del_ref)) = delegate else {
        return Ok(Some(Slot::Int(0)));
    };
    let del_class = heap.get(del_ref)?.class_name.clone();
    let result = ops
        .invoke(
            heap,
            out,
            &del_class,
            "compare",
            "(Ljava/lang/Object;Ljava/lang/Object;)I",
            vec![delegate, a, b],
        )?
        .unwrap_or(Slot::Int(0));
    let cmp = match result {
        Slot::Int(n) => n,
        _ => 0,
    };
    Ok(Some(Slot::Int(-cmp)))
}

/// Native: `Collections.binarySearch(List, T)I` — binary search on sorted `ArrayList`.
pub(crate) fn native_collections_binary_search(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(list_ref)?.fields[1..=size].to_vec();
    let mut lo: i64 = 0;
    let mut hi: i64 = i64::try_from(elems.len()).unwrap_or(0) - 1;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2; // avoid overflow via i64 midpoint
        let mid_elem = elems[usize::try_from(mid).unwrap_or(0)];
        let cmp = compare_slots_natural(mid_elem, key, heap, out, ops)?;
        match cmp.cmp(&0) {
            std::cmp::Ordering::Equal => {
                return Ok(Some(Slot::Int(i32::try_from(mid).unwrap_or(0))));
            }
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid - 1,
        }
    }
    // Return -(insertion point) - 1
    let insertion = i32::try_from(lo).unwrap_or(0);
    Ok(Some(Slot::Int(-(insertion + 1))))
}

/// Native: `String.intern()String` — returns canonical string (identity for our heap strings).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_string_intern(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // In our interpreter, string equality is by value already; intern = identity.
    Ok(Some(args.first().copied().unwrap_or(Slot::Reference(None))))
}

// ---- Arrays.asList ----

/// Native: `Arrays.asList(Object[])List` — wraps a reference array as an `ArrayList`.
pub(crate) fn native_arrays_as_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let arr_len = heap.get(arr_ref)?.fields.len();
    // Create a new ArrayList (1 field slot for size counter) and populate it.
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    for i in 0..arr_len {
        let elem = heap
            .get(arr_ref)?
            .fields
            .get(i)
            .copied()
            .unwrap_or(Slot::Reference(None));
        native_arraylist_add(&[Slot::Reference(Some(list_ref)), elem], heap, out, control)?;
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}

// ---- String extended operations (Java 11+) ----

/// Native: `String.strip()String` — removes leading and trailing Unicode whitespace.
pub(crate) fn native_string_strip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.trim().to_owned());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.stripLeading()String` — removes leading Unicode whitespace.
pub(crate) fn native_string_strip_leading(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.trim_start().to_owned());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.stripTrailing()String` — removes trailing Unicode whitespace.
pub(crate) fn native_string_strip_trailing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.trim_end().to_owned());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.isBlank()Z` — true if empty or all whitespace.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_string_is_blank(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let blank = heap.get(this_ref).map_or(true, |o| {
        o.string_value
            .as_deref()
            .is_none_or(|s| s.chars().all(char::is_whitespace))
    });
    Ok(Some(Slot::Int(i32::from(blank))))
}

/// Native: `String.repeat(int)String` — repeats this string n times.
pub(crate) fn native_string_repeat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let n = usize::try_from(extract_int_arg(args, 1)?.max(0)).unwrap_or(0);
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.repeat(n));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.formatted(Object[])String` — instance alias for `String.format(this, args)`.
pub(crate) fn native_string_formatted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // args[0] = this (the format string), args[1] = Object[] varargs
    let fmt_ref = extract_ref_arg(args, 0)?;
    let arr_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    native_string_format(
        &[Slot::Reference(Some(fmt_ref)), arr_slot],
        heap,
        out,
        control,
    )
}

/// Native: `String.join(CharSequence, CharSequence[])String` — joins array elements with delimiter.
pub(crate) fn native_string_join(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let delim_ref = extract_ref_arg(args, 0)?;
    let delim = heap
        .get(delim_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    // args[1] can be an Object[] array (varargs) or a single Iterable (ArrayList)
    let parts: Vec<String> = match args.get(1) {
        Some(Slot::Reference(Some(arr_ref))) => {
            let obj = heap.get(*arr_ref)?;
            if obj.class_name.starts_with('[') {
                // It's an array — fields are the elements.
                let len = obj.fields.len();
                let mut result = Vec::with_capacity(len);
                let slots: Vec<Slot> = obj.fields.clone();
                let _ = obj;
                for slot in slots {
                    let s = match slot {
                        Slot::Reference(Some(r)) => heap
                            .get(r)?
                            .string_value
                            .clone()
                            .unwrap_or_else(|| "null".to_string()),
                        Slot::Reference(None) => "null".to_string(),
                        Slot::Int(n) => n.to_string(),
                        Slot::Long(n) => n.to_string(),
                        other => format!("{other:?}"),
                    };
                    result.push(s);
                }
                result
            } else {
                // ArrayList or similar — fields[0]=size, fields[1..]=elements
                let size_val = match obj.fields.first() {
                    Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                    _ => 0,
                };
                let elems: Vec<Slot> =
                    obj.fields[1..=size_val.min(obj.fields.len().saturating_sub(1))].to_vec();
                let _ = obj;
                let mut result = Vec::with_capacity(size_val);
                for slot in elems {
                    let s = match slot {
                        Slot::Reference(Some(r)) => heap
                            .get(r)?
                            .string_value
                            .clone()
                            .unwrap_or_else(|| "null".to_string()),
                        Slot::Reference(None) => "null".to_string(),
                        Slot::Int(n) => n.to_string(),
                        Slot::Long(n) => n.to_string(),
                        other => format!("{other:?}"),
                    };
                    result.push(s);
                }
                result
            }
        }
        _ => Vec::new(),
    };
    let joined = parts.join(&delim);
    let r = heap.allocate_string(joined);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.indexOf(int)I` — finds first occurrence of char (as Unicode code point).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn native_string_index_of_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let ch = char::from_u32(extract_int_arg(args, 1)? as u32).unwrap_or('\0');
    let idx = heap
        .get(this_ref)?
        .string_value
        .as_deref()
        .and_then(|s| s.char_indices().find(|(_, c)| *c == ch).map(|(i, _)| i))
        .and_then(|byte_pos| {
            heap.get(this_ref).ok().and_then(|o| {
                o.string_value
                    .as_deref()
                    .map(|s| s[..byte_pos].chars().count())
            })
        });
    let result = idx.and_then(|i| i32::try_from(i).ok()).unwrap_or(-1);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.lastIndexOf(String)I` — finds last occurrence of substring.
pub(crate) fn native_string_last_index_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let sub_ref = extract_ref_arg(args, 1)?;
    let this_str = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let sub_str = heap.get(sub_ref)?.string_value.clone().unwrap_or_default();
    let result = this_str
        .rfind(sub_str.as_str())
        .and_then(|byte_pos| i32::try_from(this_str[..byte_pos].chars().count()).ok())
        .unwrap_or(-1);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.codePointAt(I)I` — returns the Unicode code point at the given index.
pub(crate) fn native_string_code_point_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = usize::try_from(extract_int_arg(args, 1)?).unwrap_or(0);
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let cp = s
        .chars()
        .nth(idx)
        .map_or(0_i32, |c| i32::try_from(u32::from(c)).unwrap_or(0));
    Ok(Some(Slot::Int(cp)))
}

/// Native: `String.lines()Stream` — splits on newlines, wraps in Stream.
pub(crate) fn native_string_lines(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let n = lines.len();
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), n + 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(i32::try_from(n).unwrap_or(0));
    for (i, line) in lines.iter().enumerate() {
        let s_ref = heap.allocate_string((*line).to_string());
        heap.get_mut(stream_ref)?.fields[i + 1] = Slot::Reference(Some(s_ref));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

// ---------------------------------------------------------------------------
// java.util.Random — 48-bit LCG (same multiplier/addend as Java's java.util.Random)
// fields[0] = Long(seed as i64); all bit-level casts are intentional.
// ---------------------------------------------------------------------------

#[allow(clippy::unreadable_literal)]
const RANDOM_MULTIPLIER: u64 = 25_214_903_917; // 0x5DEECE66D — Java's LCG multiplier
const RANDOM_ADDEND: u64 = 0xB;
const RANDOM_MASK: u64 = (1u64 << 48) - 1;

/// Advance the LCG and return `bits` high bits of the new state.
#[allow(clippy::cast_possible_truncation)]
const fn random_next(seed: u64, bits: u32) -> (u64, i32) {
    let new_seed = seed
        .wrapping_mul(RANDOM_MULTIPLIER)
        .wrapping_add(RANDOM_ADDEND)
        & RANDOM_MASK;
    let value = (new_seed >> (48 - bits)) as i32; // intentional truncation to bit pattern
    (new_seed, value)
}

/// Native: `Random.<init>()V` — seed from current time.
#[allow(clippy::unnecessary_wraps, clippy::cast_possible_wrap)]
pub(crate) fn native_random_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let nanos = u64::from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos()),
    );
    let initial = (nanos ^ RANDOM_MULTIPLIER) & RANDOM_MASK;
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(initial as i64); // safe: mask ensures < 2^48
    Ok(None)
}

/// Native: `Random.<init>(J)V` — seed with explicit long value.
#[allow(
    clippy::unnecessary_wraps,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
pub(crate) fn native_random_init_seed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let seed = match args.get(1).copied() {
        Some(Slot::Long(v)) => v as u64,
        Some(Slot::Int(v)) => v as u64,
        _ => 0,
    };
    let initial = (seed ^ RANDOM_MULTIPLIER) & RANDOM_MASK;
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(initial as i64); // safe: < 2^48
    Ok(None)
}

/// Retrieve and advance seed from `fields[0]`, returning new seed and `bits` high bits.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
fn random_step(heap: &mut duke_gc::Heap, this_ref: u64, bits: u32) -> VmResult<(u64, i32)> {
    let old_seed = match heap.get(this_ref)?.fields.first().copied() {
        Some(Slot::Long(v)) => v as u64,
        _ => 0,
    };
    let (new_seed, value) = random_next(old_seed, bits);
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(new_seed as i64); // safe: < 2^48
    Ok((new_seed, value))
}

/// Native: `Random.nextInt()I` — full-range random int.
pub(crate) fn native_random_next_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, v) = random_step(heap, this_ref, 32)?;
    Ok(Some(Slot::Int(v)))
}

/// Native: `Random.nextInt(I)I` — bounded random int [0, bound).
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]
pub(crate) fn native_random_next_int_bound(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let bound = extract_int_arg(args, 1)?;
    if bound <= 0 {
        return Err(duke_runtime::VmError::JavaException {
            class_name: "java/lang/IllegalArgumentException".to_string(),
        });
    }
    let bound_u = bound as u32;
    // Java rejection-sampling to avoid modulo bias
    loop {
        let (_, bits) = random_step(heap, this_ref, 31)?;
        let bits_u = bits as u32; // bits from next(31) are always non-negative
        let val = bits_u % bound_u;
        if bits_u.wrapping_sub(val).wrapping_add(bound_u - 1) < u32::MAX {
            return Ok(Some(Slot::Int(val as i32))); // val < bound <= i32::MAX
        }
    }
}

/// Native: `Random.nextLong()J` — 64-bit random long (two 32-bit calls).
pub(crate) fn native_random_next_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, hi) = random_step(heap, this_ref, 32)?;
    let (_, lo) = random_step(heap, this_ref, 32)?;
    let v = (i64::from(hi) << 32) + i64::from(lo);
    Ok(Some(Slot::Long(v)))
}

/// Native: `Random.nextDouble()D` — uniform [0.0, 1.0).
#[allow(clippy::cast_precision_loss)]
pub(crate) fn native_random_next_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, hi) = random_step(heap, this_ref, 26)?;
    let (_, lo) = random_step(heap, this_ref, 27)?;
    // Java spec: ((long)(next(26)) << 27) + next(27)) / (double)(1L << 53)
    let combined = (i64::from(hi) << 27) + i64::from(lo);
    let v = combined as f64 / (1u64 << 53) as f64;
    Ok(Some(Slot::Double(v)))
}

/// Native: `Random.nextFloat()F` — uniform [0.0, 1.0) as float.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn native_random_next_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, bits) = random_step(heap, this_ref, 24)?;
    // next(24) is non-negative, safe to cast to f32
    let v = bits as f32 / (1u32 << 24) as f32;
    Ok(Some(Slot::Float(v)))
}

/// Native: `Random.nextBoolean()Z` — random boolean.
pub(crate) fn native_random_next_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, v) = random_step(heap, this_ref, 1)?;
    Ok(Some(Slot::Int(v)))
}

// ---------------------------------------------------------------------------
// java.util.regex.Pattern / Matcher
// Pattern: string_value = regex string.
// Matcher: fields[0]=Pattern ref, fields[1]=input ref, fields[2]=pos,
//          fields[3]=match_start (-1=no match), fields[4]=match_end;
//          string_value = last matched text.
// ---------------------------------------------------------------------------

/// Helper: compile a regex from a pattern string.
/// Returns `Err` with `JavaException` on bad pattern.
fn compile_java_regex(pattern: &str) -> VmResult<regex::Regex> {
    regex::Regex::new(pattern).map_err(|e| duke_runtime::VmError::JavaException {
        class_name: format!("java/util/regex/PatternSyntaxException: {e}"),
    })
}

/// Native: `Pattern.compile(String)Pattern` — static factory.
pub(crate) fn native_pattern_compile(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let pat_str_ref = extract_ref_arg(args, 0)?;
    let pattern_str = heap
        .get(pat_str_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    // Validate the regex eagerly so we fail here not at match time.
    compile_java_regex(&pattern_str)?;
    let pat_ref = heap.allocate("java/util/regex/Pattern".to_string(), 0);
    heap.get_mut(pat_ref)?.string_value = Some(pattern_str);
    Ok(Some(Slot::Reference(Some(pat_ref))))
}

/// Native: `Pattern.matcher(CharSequence)Matcher` — creates a Matcher.
pub(crate) fn native_pattern_matcher(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let pat_ref = extract_ref_arg(args, 0)?;
    let input_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    // fields: [0]=pattern_ref, [1]=input_ref, [2]=pos, [3]=match_start, [4]=match_end
    let m_ref = heap.allocate("java/util/regex/Matcher".to_string(), 5);
    heap.get_mut(m_ref)?.fields[0] = Slot::Reference(Some(pat_ref));
    heap.get_mut(m_ref)?.fields[1] = input_slot;
    heap.get_mut(m_ref)?.fields[2] = Slot::Int(0);
    heap.get_mut(m_ref)?.fields[3] = Slot::Int(-1);
    heap.get_mut(m_ref)?.fields[4] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(m_ref))))
}

/// Native: `Pattern.matches(String,CharSequence)Z` — static full-string match.
pub(crate) fn native_pattern_matches_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let pat_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re
        .find(&input)
        .is_some_and(|m| m.start() == 0 && m.end() == input.len());
    Ok(Some(Slot::Int(i32::from(result))))
}

/// Native: `Matcher.find()Z` — finds next match; advances position.
pub(crate) fn native_matcher_find(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.first().copied() else {
        return Ok(Some(Slot::Int(0)));
    };
    let input_slot = fields.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(input_ref)) = input_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let pos = match fields.get(2).copied() {
        Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        _ => 0,
    };
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    if let Some(m) = re.find_at(&input, pos) {
        let start = i32::try_from(m.start()).unwrap_or(0);
        let end = i32::try_from(m.end()).unwrap_or(0);
        let matched = m.as_str().to_string();
        heap.get_mut(m_ref)?.fields[2] = Slot::Int(end); // advance past match
        heap.get_mut(m_ref)?.fields[3] = Slot::Int(start);
        heap.get_mut(m_ref)?.fields[4] = Slot::Int(end);
        heap.get_mut(m_ref)?.string_value = Some(matched);
        Ok(Some(Slot::Int(1)))
    } else {
        heap.get_mut(m_ref)?.fields[3] = Slot::Int(-1);
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `Matcher.matches()Z` — full-string match (resets position).
pub(crate) fn native_matcher_matches(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.first().copied() else {
        return Ok(Some(Slot::Int(0)));
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(1).copied() else {
        return Ok(Some(Slot::Int(0)));
    };
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re
        .find(&input)
        .is_some_and(|m| m.start() == 0 && m.end() == input.len());
    if result {
        let end = i32::try_from(input.len()).unwrap_or(0);
        heap.get_mut(m_ref)?.fields[3] = Slot::Int(0);
        heap.get_mut(m_ref)?.fields[4] = Slot::Int(end);
        heap.get_mut(m_ref)?.string_value = Some(input);
    }
    Ok(Some(Slot::Int(i32::from(result))))
}

/// Native: `Matcher.group()String` — returns text of last match.
pub(crate) fn native_matcher_group(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let matched = heap.get(m_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(matched);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Matcher.start()I` — start index of last match.
pub(crate) fn native_matcher_start(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let start = match heap.get(m_ref)?.fields.get(3).copied() {
        Some(Slot::Int(n)) => n,
        _ => -1,
    };
    Ok(Some(Slot::Int(start)))
}

/// Native: `Matcher.end()I` — exclusive end index of last match.
pub(crate) fn native_matcher_end(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let end = match heap.get(m_ref)?.fields.get(4).copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    Ok(Some(Slot::Int(end)))
}

/// Native: `Matcher.replaceAll(String)String` — replace all matches.
pub(crate) fn native_matcher_replace_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let repl_ref = extract_ref_arg(args, 1)?;
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.first().copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(1).copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let repl = heap.get(repl_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re.replace_all(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Matcher.replaceFirst(String)String` — replace first match.
pub(crate) fn native_matcher_replace_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let repl_ref = extract_ref_arg(args, 1)?;
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.first().copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(1).copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let repl = heap.get(repl_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re.replace(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.matches(String)Z` — full-string regex match.
pub(crate) fn native_string_matches_regex(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let pat_ref = extract_ref_arg(args, 1)?;
    let input = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let matched = re
        .find(&input)
        .is_some_and(|m| m.start() == 0 && m.end() == input.len());
    Ok(Some(Slot::Int(i32::from(matched))))
}

/// Native: `String.replaceAll(String,String)String` — regex replace all.
pub(crate) fn native_string_replace_all_regex(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let pat_ref = extract_ref_arg(args, 1)?;
    let repl_ref = extract_ref_arg(args, 2)?;
    let input = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let repl = heap.get(repl_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re.replace_all(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.replaceFirst(String,String)String` — regex replace first.
pub(crate) fn native_string_replace_first_regex(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let pat_ref = extract_ref_arg(args, 1)?;
    let repl_ref = extract_ref_arg(args, 2)?;
    let input = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let repl = heap.get(repl_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re.replace(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Optional extensions (callback-based)
// ---------------------------------------------------------------------------

/// Native: `Optional.map(Function)Optional` — maps value if present.
pub(crate) fn native_optional_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let result_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if matches!(value, Slot::Reference(None)) {
        // empty — propagate empty
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    }
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mapped = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, value],
    )?;
    heap.get_mut(result_ref)?.fields[0] = mapped.unwrap_or(Slot::Reference(None));
    Ok(Some(Slot::Reference(Some(result_ref))))
}

/// Native: `Optional.filter(Predicate)Optional` — keeps value only if predicate passes.
pub(crate) fn native_optional_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let result_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if matches!(value, Slot::Reference(None)) {
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    }
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        heap.get_mut(result_ref)?.fields[0] = value;
        return Ok(Some(Slot::Reference(Some(result_ref))));
    };
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let test_result = ops.invoke(
        heap,
        out,
        &pred_class,
        "test",
        "(Ljava/lang/Object;)Z",
        vec![pred_slot, value],
    )?;
    let passes = matches!(test_result, Some(Slot::Int(n)) if n != 0);
    let stored = if passes { value } else { Slot::Reference(None) };
    heap.get_mut(result_ref)?.fields[0] = stored;
    Ok(Some(Slot::Reference(Some(result_ref))))
}

/// Native: `Optional.flatMap(Function)Optional` — maps value to Optional if present, flattens.
pub(crate) fn native_optional_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    if matches!(value, Slot::Reference(None)) {
        let empty_ref = heap.allocate("java/util/Optional".to_string(), 1);
        heap.get_mut(empty_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(empty_ref))));
    }
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        let empty_ref = heap.allocate("java/util/Optional".to_string(), 1);
        heap.get_mut(empty_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(empty_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    // The function returns an Optional — return it directly (flat, not wrapped again)
    let result = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, value],
    )?;
    Ok(Some(result.unwrap_or(Slot::Reference(None))))
}

/// Native: `Optional.ifPresent(Consumer)V` — invokes consumer if value is present.
pub(crate) fn native_optional_if_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    if let (Slot::Reference(Some(_)), Slot::Reference(Some(consumer_ref))) = (value, consumer_slot)
    {
        let consumer_class = heap.get(consumer_ref)?.class_name.clone();
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, value],
        )?;
    }
    Ok(None)
}

/// Native: `Optional.orElseGet(Supplier)Object` — calls supplier if empty.
pub(crate) fn native_optional_or_else_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let supplier_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    if !matches!(value, Slot::Reference(None)) {
        return Ok(Some(value));
    }
    let Slot::Reference(Some(supplier_ref)) = supplier_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let supplier_class = heap.get(supplier_ref)?.class_name.clone();
    let result = ops.invoke(
        heap,
        out,
        &supplier_class,
        "get",
        "()Ljava/lang/Object;",
        vec![supplier_slot],
    )?;
    Ok(Some(result.unwrap_or(Slot::Reference(None))))
}

// ---------------------------------------------------------------------------
// HashMap extensions: compute, merge
// ---------------------------------------------------------------------------

/// Helper: find key index in `HashMap` fields (`fields[0]`=size, `fields[1,3,5..]`=keys, `fields[2,4,6..]`=vals).
fn hashmap_find_key(fields: &[Slot], key: Slot, heap: &duke_gc::Heap) -> Option<usize> {
    let size = match fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return None,
    };
    for i in 0..size {
        let key_slot = fields.get(1 + i * 2)?;
        if slots_equal(key_slot, &key, heap) {
            return Some(1 + i * 2);
        }
    }
    None
}

/// Native: `HashMap.compute(K, BiFunction)V` — compute new value from old (possibly null).
pub(crate) fn native_hashmap_compute(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let fn_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    // Find old value
    let fields = heap.get(this_ref)?.fields.clone();
    let old_value = hashmap_find_key(&fields, key, heap)
        .and_then(|ki| fields.get(ki + 1).copied())
        .unwrap_or(Slot::Reference(None));
    // Call BiFunction.apply(key, oldValue)
    let new_value = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, key, old_value],
    )?;
    let new_val = new_value.unwrap_or(Slot::Reference(None));
    // Update or insert
    let fields2 = heap.get(this_ref)?.fields.clone();
    if let Some(ki) = hashmap_find_key(&fields2, key, heap) {
        heap.get_mut(this_ref)?.fields[ki + 1] = new_val;
    } else {
        let size = match heap.get(this_ref)?.fields.first().copied() {
            Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
            _ => 0,
        };
        heap.get_mut(this_ref)?.fields.push(key);
        heap.get_mut(this_ref)?.fields.push(new_val);
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    }
    Ok(Some(new_val))
}

/// Native: `HashMap.merge(K, V, BiFunction)V` — put V if absent, else merge with `BiFunction`.
pub(crate) fn native_hashmap_merge(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let new_val_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let fn_slot = args.get(3).copied().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    let old_ki = hashmap_find_key(&fields, key, heap);
    if let Some(ki) = old_ki {
        let old_value = fields.get(ki + 1).copied().unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(old_value));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let merged = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, old_value, new_val_slot],
        )?;
        let merged_val = merged.unwrap_or(Slot::Reference(None));
        heap.get_mut(this_ref)?.fields[ki + 1] = merged_val;
        Ok(Some(merged_val))
    } else {
        // Key absent — insert new value
        let size = match heap.get(this_ref)?.fields.first().copied() {
            Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
            _ => 0,
        };
        heap.get_mut(this_ref)?.fields.push(key);
        heap.get_mut(this_ref)?.fields.push(new_val_slot);
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
        Ok(Some(new_val_slot))
    }
}

// ---------------------------------------------------------------------------
// java.lang.StringBuffer — mutable string, delegates to StringBuilder internals
// (string_value field used as buffer, same as StringBuilder)
// ---------------------------------------------------------------------------

/// Native: `StringBuffer.<init>()V` — empty buffer.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stringbuffer_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.string_value = Some(String::new());
    Ok(None)
}

/// Native: `StringBuffer.<init>(Ljava/lang/String;)V` — init with string.
pub(crate) fn native_stringbuffer_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = match args.get(1).copied() {
        Some(Slot::Reference(Some(r))) => heap.get(r)?.string_value.clone().unwrap_or_default(),
        _ => String::new(),
    };
    heap.get_mut(this_ref)?.string_value = Some(s);
    Ok(None)
}

/// Native: `StringBuffer.append(...)StringBuffer` — append any type; returns `this`.
pub(crate) fn native_stringbuffer_append(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let frag = match args.get(1).copied().unwrap_or(Slot::Reference(None)) {
        Slot::Reference(Some(r)) => heap
            .get(r)?
            .string_value
            .clone()
            .unwrap_or_else(|| "null".to_string()),
        Slot::Reference(None) => "null".to_string(),
        Slot::Int(n) => n.to_string(),
        Slot::Long(n) => n.to_string(),
        Slot::Double(d) => format!("{d}"),
        Slot::Float(f) => format!("{f}"),
        Slot::ReturnAddress(_) => String::new(),
    };
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    buf.push_str(&frag);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuffer.toString()String`.
pub(crate) fn native_stringbuffer_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `StringBuffer.length()I`.
pub(crate) fn native_stringbuffer_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let len = heap
        .get(this_ref)?
        .string_value
        .as_deref()
        .unwrap_or("")
        .len();
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}

// ---------------------------------------------------------------------------
// java.util.StringJoiner
// fields[0] = Reference(delimiter), fields[1] = Reference(prefix), fields[2] = Reference(suffix)
// fields[3] = Reference(emptyValue), fields[4..] = added elements
// instance_field_count = 4 (slots 0-3 pre-allocated)
// ---------------------------------------------------------------------------

/// Native: `StringJoiner.<init>(CharSequence)V` — delimiter only.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stringjoiner_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let delim_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let empty_ref = heap.allocate_string(String::new());
    let prefix_ref = heap.allocate_string(String::new());
    let suffix_ref = heap.allocate_string(String::new());
    heap.get_mut(this_ref)?.fields[0] = delim_slot;
    heap.get_mut(this_ref)?.fields[1] = Slot::Reference(Some(prefix_ref));
    heap.get_mut(this_ref)?.fields[2] = Slot::Reference(Some(suffix_ref));
    heap.get_mut(this_ref)?.fields[3] = Slot::Reference(Some(empty_ref));
    Ok(None)
}

/// Native: `StringJoiner.<init>(CharSequence,CharSequence,CharSequence)V` — delim + prefix + suffix.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stringjoiner_init_prefix_suffix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let delim_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let prefix_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let suffix_slot = args.get(3).copied().unwrap_or(Slot::Reference(None));
    let empty_ref = heap.allocate_string(String::new());
    heap.get_mut(this_ref)?.fields[0] = delim_slot;
    heap.get_mut(this_ref)?.fields[1] = prefix_slot;
    heap.get_mut(this_ref)?.fields[2] = suffix_slot;
    heap.get_mut(this_ref)?.fields[3] = Slot::Reference(Some(empty_ref));
    Ok(None)
}

/// Native: `StringJoiner.add(CharSequence)StringJoiner` — append element.
pub(crate) fn native_stringjoiner_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    heap.get_mut(this_ref)?.fields.push(elem);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringJoiner.setEmptyValue(CharSequence)StringJoiner`.
pub(crate) fn native_stringjoiner_set_empty_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let empty_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    heap.get_mut(this_ref)?.fields[3] = empty_slot;
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Helper: read a string from a slot (returns "" for null).
fn slot_to_string(slot: Slot, heap: &duke_gc::Heap) -> String {
    match slot {
        Slot::Reference(Some(r)) => heap
            .get(r)
            .ok()
            .and_then(|o| o.string_value.clone())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Native: `StringJoiner.toString()String` — builds the joined result.
pub(crate) fn native_stringjoiner_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(this_ref)?.fields.clone();
    // elements start at index 4
    let elems: Vec<Slot> = fields.get(4..).map(<[Slot]>::to_vec).unwrap_or_default();
    if elems.is_empty() {
        let empty_slot = fields.get(3).copied().unwrap_or(Slot::Reference(None));
        let prefix = slot_to_string(
            fields.get(1).copied().unwrap_or(Slot::Reference(None)),
            heap,
        );
        let suffix = slot_to_string(
            fields.get(2).copied().unwrap_or(Slot::Reference(None)),
            heap,
        );
        let empty = slot_to_string(empty_slot, heap);
        // If prefix+suffix are both empty, return emptyValue; otherwise prefix+suffix
        let result = if prefix.is_empty() && suffix.is_empty() {
            empty
        } else {
            format!("{prefix}{suffix}")
        };
        let r = heap.allocate_string(result);
        return Ok(Some(Slot::Reference(Some(r))));
    }
    let delim = slot_to_string(
        fields.first().copied().unwrap_or(Slot::Reference(None)),
        heap,
    );
    let prefix = slot_to_string(
        fields.get(1).copied().unwrap_or(Slot::Reference(None)),
        heap,
    );
    let suffix = slot_to_string(
        fields.get(2).copied().unwrap_or(Slot::Reference(None)),
        heap,
    );
    let parts: Vec<String> = elems.iter().map(|s| slot_to_string(*s, heap)).collect();
    let result = format!("{}{}{}", prefix, parts.join(&delim), suffix);
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `StringJoiner.length()I` — length of the `toString()` result.
pub(crate) fn native_stringjoiner_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // Reuse toString and measure
    let result = native_stringjoiner_tostring(args, heap, out, control)?;
    let len = match result {
        Some(Slot::Reference(Some(r))) => heap.get(r)?.string_value.as_deref().unwrap_or("").len(),
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}

// ---------------------------------------------------------------------------
// HashMap natives
// ---------------------------------------------------------------------------

fn uses_first_field_value_equality(class_name: &str) -> bool {
    matches!(
        class_name,
        "java/lang/Boolean"
            | "java/lang/Byte"
            | "java/lang/Character"
            | "java/lang/Double"
            | "java/lang/Float"
            | "java/lang/Integer"
            | "java/lang/Long"
            | "java/lang/Short"
    )
}

/// Semantic equality for `HashMap` keys: reference identity by default, with value
/// semantics for strings, class mirrors, and boxed primitive wrappers.
fn slots_equal(a: &Slot, b: &Slot, heap: &duke_gc::Heap) -> bool {
    match (a, b) {
        (Slot::Reference(None), Slot::Reference(None)) => true,
        (Slot::Reference(Some(ra)), Slot::Reference(Some(rb))) => {
            if ra == rb {
                return true;
            }
            let Ok(oa) = heap.get(*ra) else { return false };
            let Ok(ob) = heap.get(*rb) else { return false };
            if oa.class_name != ob.class_name {
                return false;
            }
            match oa.class_name.as_str() {
                "java/lang/String" | "java/lang/Class" => oa.string_value == ob.string_value,
                class_name if uses_first_field_value_equality(class_name) => {
                    oa.fields.first() == ob.fields.first()
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// Native: `HashMap.<init>()V` — initialises size counter at fields\[0\] to 0.
fn find_hashmap_entry_index(fields: &[Slot], key: &Slot, heap: &duke_gc::Heap) -> Option<usize> {
    (1..fields.len())
        .step_by(2)
        .find(|&i| i + 1 < fields.len() && slots_equal(&fields[i], key, heap))
}

pub(crate) fn native_hashmap_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.is_empty() {
        obj.fields.push(Slot::Int(0));
    } else {
        obj.fields[0] = Slot::Int(0);
    }
    Ok(None)
}

/// Native: `HashMap.put(Object, Object)Object` — inserts or updates a key-value pair.
/// Returns the old value if the key was already present, or null if it is new.
pub(crate) fn native_hashmap_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let val = extract_slot_arg(args, 2);
    // Clone fields to release the immutable borrow before mutating.
    let fields = heap.get(this_ref)?.fields.clone();

    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let old = fields[i + 1];
        heap.get_mut(this_ref)?.fields[i + 1] = val;
        return Ok(Some(old));
    }

    // New key — append pair and bump size.
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException),
    }
    obj.fields.push(key);
    obj.fields.push(val);
    Ok(Some(Slot::Reference(None)))
}

/// Native: `HashMap.get(Object)Object` — returns value for key, or null if absent.
pub(crate) fn native_hashmap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();

    Ok(Some(
        find_hashmap_entry_index(&fields, &key, heap)
            .map_or(Slot::Reference(None), |i| fields[i + 1]),
    ))
}

/// Native: `HashMap.containsKey(Object)Z` — returns 1 if key present, 0 otherwise.
pub(crate) fn native_hashmap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();

    if find_hashmap_entry_index(&fields, &key, heap).is_some() {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashMap.size()I` — returns entry count from fields\[0\].
pub(crate) fn native_hashmap_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Ok(Some(Slot::Int(*n))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `HashMap.remove(Object)Object` — removes a key-value pair, returns old value or null.
/// Uses swap-remove (swaps target pair with last pair) for O(1) deletion.
pub(crate) fn native_hashmap_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();

    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let old_val = fields[i + 1];
        let obj = heap.get_mut(this_ref)?;
        let last_val_idx = obj.fields.len() - 1;
        let last_key_idx = obj.fields.len() - 2;
        obj.fields.swap(i + 1, last_val_idx);
        obj.fields.swap(i, last_key_idx);
        obj.fields.truncate(obj.fields.len() - 2);
        match obj.fields.first_mut() {
            Some(Slot::Int(sz)) => *sz -= 1,
            _ => return Err(VmError::NullPointerException),
        }
        Ok(Some(old_val))
    } else {
        Ok(Some(Slot::Reference(None)))
    }
}

/// Native: `HashMap.isEmpty()Z` — returns 1 if size == 0, else 0.
pub(crate) fn native_hashmap_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) => Ok(Some(Slot::Int(1))),
        Some(Slot::Int(_)) => Ok(Some(Slot::Int(0))),
        _ => Ok(Some(Slot::Int(1))),
    }
}

/// Native: `HashMap.getOrDefault(Object, Object)Object` — returns value for key, or default if absent.
pub(crate) fn native_hashmap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let default = extract_slot_arg(args, 2);
    let fields = heap.get(this_ref)?.fields.clone();

    Ok(Some(
        find_hashmap_entry_index(&fields, &key, heap).map_or(default, |i| fields[i + 1]),
    ))
}

// ---------------------------------------------------------------------------
// HashSet natives
// ---------------------------------------------------------------------------

pub(crate) fn native_set_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;

    match args.first().copied().unwrap_or(Slot::Reference(None)) {
        Slot::Reference(Some(array_ref)) => {
            let elements = heap.get(array_ref)?.fields.clone();
            for element in elements {
                native_hashset_add(
                    &[Slot::Reference(Some(set_ref)), element],
                    heap,
                    out,
                    control,
                )?;
            }
        }
        Slot::Reference(None) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    }

    Ok(Some(Slot::Reference(Some(set_ref))))
}

fn find_hashset_entry_index(
    fields: &[Slot],
    element: &Slot,
    heap: &duke_gc::Heap,
) -> Option<usize> {
    fields
        .iter()
        .skip(1)
        .position(|field| slots_equal(field, element, heap))
        .map(|idx| idx + 1)
}

pub(crate) fn native_hashset_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.is_empty() {
        obj.fields.push(Slot::Int(0));
    } else {
        obj.fields[0] = Slot::Int(0);
    }
    Ok(None)
}

pub(crate) fn native_hashset_init_from_collection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let mut this_ref = extract_ref_arg(args, 0)?;
    let collection_ref = extract_ref_arg(args, 1)?;
    native_hashset_init(&[Slot::Reference(Some(this_ref))], heap, output, control)?;

    let collection_class = heap.get(collection_ref)?.class_name.clone();
    let elements: Vec<Slot> = if collection_class == "java/util/HashSet" {
        heap.get(collection_ref)?
            .fields
            .iter()
            .skip(1)
            .copied()
            .collect()
    } else {
        let array_slot = ops.invoke(
            heap,
            output,
            &collection_class,
            "toArray",
            "()[Ljava/lang/Object;",
            vec![Slot::Reference(Some(collection_ref))],
        )?;
        let Some(Slot::Reference(Some(array_ref))) = array_slot else {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        };
        patch_forwarded_ref_if_needed(heap, &mut this_ref);
        heap.get(array_ref)?.fields.clone()
    };

    for element in elements {
        native_hashset_add(
            &[Slot::Reference(Some(this_ref)), element],
            heap,
            output,
            control,
        )?;
    }
    Ok(None)
}

/// Native: `HashSet.add(Object)Z` — adds element if not already present.
/// Returns 1 if added, 0 if element was already in the set.
pub(crate) fn native_hashset_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();
    // fields[0] = size, fields[1..] = elements
    if find_hashset_entry_index(&fields, &element, heap).is_some() {
        return Ok(Some(Slot::Int(0))); // duplicate
    }

    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException),
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1)))
}

/// Native: `HashSet.contains(Object)Z` — returns 1 if element is present, 0 otherwise.
pub(crate) fn native_hashset_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();

    if find_hashset_entry_index(&fields, &element, heap).is_some() {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashSet.remove(Object)Z` — removes element if present, returns 1 if removed, 0 if absent.
/// Uses swap-remove (swaps target with last element) for O(1) deletion.
pub(crate) fn native_hashset_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();

    if let Some(i) = find_hashset_entry_index(&fields, &element, heap) {
        let obj = heap.get_mut(this_ref)?;
        let last_idx = obj.fields.len() - 1;
        obj.fields.swap(i, last_idx);
        obj.fields.truncate(obj.fields.len() - 1);
        match obj.fields.first_mut() {
            Some(Slot::Int(sz)) => *sz -= 1,
            _ => return Err(VmError::NullPointerException),
        }
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashSet.size()I` — returns element count from fields\[0\].
pub(crate) fn native_hashset_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Ok(Some(Slot::Int(*n))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `HashSet.isEmpty()Z` — returns 1 if size == 0, else 0.
pub(crate) fn native_hashset_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) => Ok(Some(Slot::Int(1))),
        Some(Slot::Int(_)) => Ok(Some(Slot::Int(0))),
        _ => Ok(Some(Slot::Int(1))),
    }
}

/// Native: `HashSet.iterator()Iterator` — creates a `HashSetIterator`.
pub(crate) fn native_hashset_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_ref = heap.allocate("duke/util/HashSetIterator".to_string(), 2);
    {
        let iter_obj = heap.get_mut(iter_ref)?;
        iter_obj.fields[0] = Slot::Reference(Some(this_ref));
        iter_obj.fields[1] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
}

/// Native: `HashSetIterator.<init>` — no-op; fields are set by `native_hashset_iterator`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_hashset_iter_init(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `HashSetIterator.hasNext()Z`
pub(crate) fn native_hashset_iter_hasnext(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_obj = heap.get(this_ref)?;
    let set_ref = match iter_obj.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Int(0))),
    };
    let cursor = match iter_obj.fields.get(1) {
        Some(Slot::Int(i)) => *i,
        _ => 0,
    };
    let set_size = match heap.get(set_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(cursor < set_size))))
}

/// Native: `HashSetIterator.next()Object` — returns element at cursor, advances cursor.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_hashset_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (set_ref, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let sr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let c = match iter_obj.fields.get(1) {
            Some(Slot::Int(i)) => *i,
            _ => 0,
        };
        (sr, c)
    };
    let element = {
        let set_obj = heap.get(set_ref)?;
        match set_obj.fields.get(cursor as usize + 1) {
            Some(slot) => *slot,
            None => {
                return Err(VmError::JavaException {
                    class_name: "java/util/NoSuchElementException".to_string(),
                });
            }
        }
    };
    heap.get_mut(this_ref)?.fields[1] = Slot::Int(cursor + 1);
    Ok(Some(element))
}

/// Native: `HashSet.stream()Stream` — wraps elements into a `duke/util/Stream`.
pub(crate) fn native_hashset_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(i32::try_from(size).unwrap_or(0));
    for elem in elems {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `LinkedList.stream()Stream` — wraps elements into a `duke/util/Stream`.
pub(crate) fn native_linked_list_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_stream(args, heap, out, control)
}

/// Native: `Collections.nCopies(int, Object)List` — returns a list of N copies of an element.
pub(crate) fn native_collections_n_copies(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let n = usize::try_from(extract_int_arg(args, 0)?.max(0)).unwrap_or(0);
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    heap.get_mut(list_ref)?.fields[0] = Slot::Int(i32::try_from(n).unwrap_or(0));
    for _ in 0..n {
        heap.get_mut(list_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}

/// Native: `String.chars()IntStream` — returns char code points as an `IntStream`.
pub(crate) fn native_string_chars(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let values: Vec<i32> = s.chars().map(|c| c as i32).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

const PROCESS_ID_FIELD: usize = 0;
const PROCESS_STDIN_FIELD: usize = 1;
const PROCESS_STDOUT_FIELD: usize = 2;
const PROCESS_STDERR_FIELD: usize = 3;

fn string_array_from_slot(slot: Slot, heap: &duke_gc::Heap) -> VmResult<Vec<String>> {
    let Slot::Reference(Some(array_ref)) = slot else {
        return Err(VmError::NullPointerException);
    };
    let elements = heap.get(array_ref)?.fields.clone();
    elements
        .into_iter()
        .map(|element| match element {
            Slot::Reference(Some(string_ref)) => string_value_from_ref(heap, string_ref),
            _ => Err(VmError::NullPointerException),
        })
        .collect()
}

fn optional_file_path_from_slot(
    slot: Slot,
    heap: &duke_gc::Heap,
) -> VmResult<Option<std::path::PathBuf>> {
    match slot {
        Slot::Reference(Some(file_ref)) => Ok(Some(file_path_from_ref(file_ref, heap)?)),
        Slot::Reference(None) => Ok(None),
        _ => Err(VmError::NullPointerException),
    }
}

fn allocate_process_impl(
    heap: &mut duke_gc::Heap,
    ids: duke_gc::SpawnedProcessIds,
) -> VmResult<Option<Slot>> {
    let process_ref = heap.allocate("java/lang/ProcessImpl".to_string(), 4);
    let process_obj = heap.get_mut(process_ref)?;
    process_obj.fields[PROCESS_ID_FIELD] = Slot::Int(ids.process_id);
    process_obj.fields[PROCESS_STDIN_FIELD] = Slot::Int(ids.stdin_id);
    process_obj.fields[PROCESS_STDOUT_FIELD] = Slot::Int(ids.stdout_id);
    process_obj.fields[PROCESS_STDERR_FIELD] = Slot::Int(ids.stderr_id);
    Ok(Some(Slot::Reference(Some(process_ref))))
}

fn spawn_process_impl(
    heap: &mut duke_gc::Heap,
    command: &[String],
    cwd: Option<&std::path::Path>,
) -> VmResult<Option<Slot>> {
    let ids = heap.spawn_host_process(command, cwd)?;
    allocate_process_impl(heap, ids)
}

fn process_field_id_from_this(
    args: &[Slot],
    heap: &duke_gc::Heap,
    field_idx: usize,
) -> VmResult<i32> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.get(field_idx) {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(VmError::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

fn allocate_process_stream(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    handle_id: i32,
) -> VmResult<Option<Slot>> {
    let stream_ref = heap.allocate(class_name.to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(handle_id);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

pub(crate) fn native_process_builder_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let command_slot = extract_slot_arg(args, 1);
    let builder_obj = heap.get_mut(this_ref)?;
    if builder_obj.fields.len() < 2 {
        return Err(VmError::InvalidRef { address: this_ref });
    }
    builder_obj.fields[0] = command_slot;
    builder_obj.fields[1] = Slot::Reference(None);
    Ok(None)
}

pub(crate) fn native_process_builder_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let directory_slot = extract_slot_arg(args, 1);
    let builder_obj = heap.get_mut(this_ref)?;
    if builder_obj.fields.len() < 2 {
        return Err(VmError::InvalidRef { address: this_ref });
    }
    builder_obj.fields[1] = directory_slot;
    Ok(Some(Slot::Reference(Some(this_ref))))
}

pub(crate) fn native_process_builder_start(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let builder_obj = heap.get(this_ref)?;
    let command_slot = builder_obj
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let directory_slot = builder_obj
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let command = string_array_from_slot(command_slot, heap)?;
    let cwd = optional_file_path_from_slot(directory_slot, heap)?;
    spawn_process_impl(heap, &command, cwd.as_deref())
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_runtime_get_runtime(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let runtime_ref = heap.allocate("java/lang/Runtime".to_string(), 0);
    Ok(Some(Slot::Reference(Some(runtime_ref))))
}

pub(crate) fn native_runtime_exec_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(_))) => {}
        _ => return Err(VmError::NullPointerException),
    }
    let command = string_array_from_slot(extract_slot_arg(args, 1), heap)?;
    spawn_process_impl(heap, &command, None)
}

pub(crate) fn native_runtime_exec_array_dir(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(_))) => {}
        _ => return Err(VmError::NullPointerException),
    }
    let command = string_array_from_slot(extract_slot_arg(args, 1), heap)?;
    let cwd = optional_file_path_from_slot(extract_slot_arg(args, 3), heap)?;
    spawn_process_impl(heap, &command, cwd.as_deref())
}

pub(crate) fn native_process_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stdout_id = process_field_id_from_this(args, heap, PROCESS_STDOUT_FIELD)?;
    allocate_process_stream(heap, "duke/process/ProcessInputStream", stdout_id)
}

pub(crate) fn native_process_get_error_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stderr_id = process_field_id_from_this(args, heap, PROCESS_STDERR_FIELD)?;
    allocate_process_stream(heap, "duke/process/ProcessErrorStream", stderr_id)
}

pub(crate) fn native_process_get_output_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stdin_id = process_field_id_from_this(args, heap, PROCESS_STDIN_FIELD)?;
    allocate_process_stream(heap, "duke/process/ProcessOutputStream", stdin_id)
}

pub(crate) fn native_process_wait_for(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    Ok(Some(Slot::Int(heap.wait_host_process(process_id)?)))
}

pub(crate) fn native_process_exit_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    let Some(exit_code) = heap.try_host_process_exit_value(process_id)? else {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalThreadStateException".into(),
        });
    };
    Ok(Some(Slot::Int(exit_code)))
}

pub(crate) fn native_process_destroy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    heap.destroy_host_process(process_id)?;
    Ok(None)
}

// ---------------------------------------------------------------------------
// GC root gathering
// ---------------------------------------------------------------------------

/// Collect all live Slot values from the interpreter's current execution state.
/// The GC uses these as the root set for reachability analysis.
fn gather_roots(
    frame: &duke_runtime::Frame,
    call_stack: &[CallFrame],
    registry: &ClassRegistry,
) -> Vec<duke_runtime::Slot> {
    let mut roots = Vec::new();
    roots.extend(frame.slots());
    for cf in call_stack {
        roots.extend(cf.frame.slots());
    }
    for ctx in registry.all_classes() {
        roots.extend(ctx.static_fields.iter().copied());
    }
    roots
}

/// Apply GC forwarding pointers to all live interpreter slots after a minor
/// collection. Must be called immediately after `heap.collect()` returns so
/// that stale young-gen references are updated to their new locations.
fn patch_forwarded_slots(
    frame: &mut duke_runtime::Frame,
    call_stack: &mut [CallFrame],
    registry: &mut ClassRegistry,
    heap: &duke_gc::Heap,
) {
    for slot in frame.slots_mut() {
        heap.apply_forward(slot);
    }
    for cf in call_stack.iter_mut() {
        for slot in cf.frame.slots_mut() {
            heap.apply_forward(slot);
        }
    }
    for ctx in registry.all_classes_mut() {
        for slot in &mut ctx.static_fields {
            heap.apply_forward(slot);
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 48: Stream.generate/iterate/concat/empty
// ---------------------------------------------------------------------------

/// Native: `Stream.generate(Supplier)Stream` — returns a `duke/util/GeneratorStream` sentinel.
/// Materialised into a real Stream when `.limit(N)` is called.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_generate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let supplier = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/GeneratorStream".to_string(), 1);
    heap.get_mut(r)?.fields[0] = supplier;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Stream.iterate(seed, UnaryOperator)Stream` — returns a `duke/util/IteratorStream`.
/// Materialised into a real Stream when `.limit(N)` is called.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_iterate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let seed = args.first().copied().unwrap_or(Slot::Reference(None));
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/IteratorStream".to_string(), 2);
    heap.get_mut(r)?.fields[0] = seed;
    heap.get_mut(r)?.fields[1] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Stream.concat(Stream, Stream)Stream` — concatenates two eager streams.
pub(crate) fn native_stream_concat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a_ref = extract_ref_arg(args, 0)?;
    let b_ref = extract_ref_arg(args, 1)?;
    let a_size = match heap.get(a_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let b_size = match heap.get(b_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let a_elems: Vec<Slot> = heap.get(a_ref)?.fields[1..=a_size].to_vec();
    let b_elems: Vec<Slot> = heap.get(b_ref)?.fields[1..=b_size].to_vec();
    let total = a_size + b_size;
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(total).unwrap_or(0));
    for elem in a_elems.into_iter().chain(b_elems) {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `Stream.empty()Stream` — returns a zero-element stream.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_empty(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Phase 46: Stream.takeWhile/dropWhile (Java 9)
// ---------------------------------------------------------------------------

/// Native: `Stream.takeWhile(Predicate)Stream` — keeps prefix while predicate holds.
pub(crate) fn native_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept: Vec<Slot> = Vec::new();
    for elem in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(pred_ref)), elem],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(elem);
        } else {
            break;
        }
    }
    let new_size = i32::try_from(kept.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in kept {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

/// Native: `Stream.dropWhile(Predicate)Stream` — drops prefix while predicate holds, keeps rest.
pub(crate) fn native_stream_drop_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut dropping = true;
    let mut kept: Vec<Slot> = Vec::new();
    for elem in elems {
        if dropping {
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![Slot::Reference(Some(pred_ref)), elem],
            )?;
            if matches!(result, Some(Slot::Int(n)) if n != 0) {
                continue;
            }
            dropping = false;
        }
        kept.push(elem);
    }
    let new_size = i32::try_from(kept.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in kept {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

// ---------------------------------------------------------------------------
// Phase 45: ArrayList.forEach, Stream.sorted(Comparator), Arrays.toString,
//           HashMap.replace, Collections.swap/unmodifiableMap,
//           Collectors.partitioningBy, IntStream.sorted
// ---------------------------------------------------------------------------

/// Native: `ArrayList.forEach(Consumer)V` — invokes consumer.accept(elem) for each element.
pub(crate) fn native_arraylist_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(consumer_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return Ok(None);
    };
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(list_ref)?.fields[1..=size].to_vec();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for elem in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![Slot::Reference(Some(consumer_ref)), elem],
        )?;
    }
    Ok(None)
}

/// Native: `Stream.sorted(Comparator)Stream` — sorts stream elements using the given comparator.
pub(crate) fn native_stream_sorted_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // If no comparator provided, fall back to natural-order sort.
    let Slot::Reference(Some(comp_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return native_stream_sorted(args, heap, out, control, ops);
    };
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    // Insertion sort using the provided comparator.
    for i in 1..elems.len() {
        let mut j = i;
        while j > 0 {
            let comp_class = heap.get(comp_ref)?.class_name.clone();
            let cmp = ops.invoke(
                heap,
                out,
                &comp_class,
                "compare",
                "(Ljava/lang/Object;Ljava/lang/Object;)I",
                vec![Slot::Reference(Some(comp_ref)), elems[j - 1], elems[j]],
            )?;
            if matches!(cmp, Some(Slot::Int(n)) if n > 0) {
                elems.swap(j - 1, j);
                j -= 1;
            } else {
                break;
            }
        }
    }
    let new_size = i32::try_from(elems.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in elems {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

/// Native: `Arrays.toString(int[])String` — formats as `[1, 2, 3]`.
pub(crate) fn native_arrays_to_string_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(arr_ref)?.fields.clone();
    let parts: Vec<String> = fields
        .iter()
        .map(|s| match s {
            Slot::Int(n) => n.to_string(),
            _ => "0".to_string(),
        })
        .collect();
    let result = format!("[{}]", parts.join(", "));
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Arrays.toString(Object[])String` — formats as `[a, b, c]`.
pub(crate) fn native_arrays_to_string_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(arr_ref)?.fields.clone();
    let mut parts: Vec<String> = Vec::with_capacity(fields.len());
    for s in &fields {
        let part = match s {
            Slot::Reference(Some(r)) => heap
                .get(*r)
                .ok()
                .and_then(|o| o.string_value.clone())
                .unwrap_or_else(|| "null".to_string()),
            Slot::Reference(None) => "null".to_string(),
            Slot::Int(n) => n.to_string(),
            _ => "?".to_string(),
        };
        parts.push(part);
    }
    let result = format!("[{}]", parts.join(", "));
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `HashMap.replace(Object, Object)Object` — updates value for existing key,
/// returns the old value or null if key was absent.
pub(crate) fn native_hashmap_replace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let new_val = extract_slot_arg(args, 2);
    let fields = heap.get(this_ref)?.fields.clone();
    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let old = fields[i + 1];
        heap.get_mut(this_ref)?.fields[i + 1] = new_val;
        Ok(Some(old))
    } else {
        Ok(Some(Slot::Reference(None)))
    }
}

/// Native: `Collections.swap(List, int, int)V` — swaps elements at indices i and j.
pub(crate) fn native_collections_swap(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let i = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(None),
    };
    let j = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(None),
    };
    // fields[0] = size, elements at fields[1..=size]
    let fi = i + 1;
    let fj = j + 1;
    let fields = heap.get(list_ref)?.fields.clone();
    let len = fields.len();
    if fi < len && fj < len {
        let vi = fields[fi];
        let vj = fields[fj];
        heap.get_mut(list_ref)?.fields[fi] = vj;
        heap.get_mut(list_ref)?.fields[fj] = vi;
    }
    Ok(None)
}

/// Native: `Collections.unmodifiableMap(Map)Map` — identity stub (we have no mutation checks).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_map(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(args.first().copied().unwrap_or(Slot::Reference(None))))
}

/// Native: `Collectors.partitioningBy(Predicate)Collector` — returns a sentinel collector.
pub(crate) fn native_collectors_partitioning_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let pred = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/PartitioningByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = pred;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `IntStream.sorted()IntStream` — returns a new sorted `IntStream`.
pub(crate) fn native_int_stream_sorted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut vals: Vec<i32> = heap.get(stream_ref)?.fields[1..=size]
        .iter()
        .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
        .collect();
    vals.sort_unstable();
    let new_stream = heap.allocate("duke/util/IntStream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(i32::try_from(vals.len()).unwrap_or(0));
    for v in vals {
        heap.get_mut(new_stream)?.fields.push(Slot::Int(v));
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

// ---------------------------------------------------------------------------
// Phase 49: Comparator.thenComparing, Predicate combinators, Function combinators,
//           Stream.mapToLong, Stream.mapToDouble
// ---------------------------------------------------------------------------

/// Native: `Comparator.thenComparing(Comparator)Comparator` — chains two comparators.
/// Stores primary in `fields[0]`, secondary in `fields[1]`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_then_comparing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let primary = args.first().copied().unwrap_or(Slot::Reference(None));
    let secondary = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ThenComparingComparator".to_string(), 2);
    heap.get_mut(r)?.fields[0] = primary;
    heap.get_mut(r)?.fields[1] = secondary;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ThenComparingComparator.compare(O,O)I` — runs primary then secondary.
pub(crate) fn native_then_comparing_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let primary = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let secondary = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    // Invoke primary.compare(a, b)
    let result = invoke_comparator(primary, a, b, heap, out, ops)?;
    if result != 0 {
        return Ok(Some(Slot::Int(result)));
    }
    // Tie-break with secondary
    let result2 = invoke_comparator(secondary, a, b, heap, out, ops)?;
    Ok(Some(Slot::Int(result2)))
}

/// Helper: dispatch `comparator.compare(a, b)` via ops.invoke.
fn invoke_comparator(
    comparator: Slot,
    a: Slot,
    b: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> VmResult<i32> {
    let Slot::Reference(Some(cmp_ref)) = comparator else {
        return Ok(0);
    };
    let cmp_class = heap.get(cmp_ref)?.class_name.clone();
    let res = ops
        .invoke(
            heap,
            out,
            &cmp_class,
            "compare",
            "(Ljava/lang/Object;Ljava/lang/Object;)I",
            vec![comparator, a, b],
        )?
        .unwrap_or(Slot::Int(0));
    Ok(match res {
        Slot::Int(n) => n,
        _ => 0,
    })
}

/// Native: `Predicate.and(Predicate)Predicate` — logical AND of two predicates.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_predicate_and(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let left = args.first().copied().unwrap_or(Slot::Reference(None));
    let right = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/AndPredicate".to_string(), 2);
    heap.get_mut(r)?.fields[0] = left;
    heap.get_mut(r)?.fields[1] = right;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `AndPredicate.test(O)Z` — both predicates must return true.
pub(crate) fn native_and_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let left = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let right = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let la = invoke_predicate_test(left, elem, heap, out, ops)?;
    if !la {
        return Ok(Some(Slot::Int(0)));
    }
    let rb = invoke_predicate_test(right, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(rb))))
}

/// Native: `Predicate.or(Predicate)Predicate` — logical OR of two predicates.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_predicate_or(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let left = args.first().copied().unwrap_or(Slot::Reference(None));
    let right = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/OrPredicate".to_string(), 2);
    heap.get_mut(r)?.fields[0] = left;
    heap.get_mut(r)?.fields[1] = right;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `OrPredicate.test(O)Z` — either predicate returning true is sufficient.
pub(crate) fn native_or_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let left = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let right = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let la = invoke_predicate_test(left, elem, heap, out, ops)?;
    if la {
        return Ok(Some(Slot::Int(1)));
    }
    let rb = invoke_predicate_test(right, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(rb))))
}

/// Native: `Predicate.negate()Predicate` — logical NOT of a predicate.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_predicate_negate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let original = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/NegatedPredicate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = original;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `NegatedPredicate.test(O)Z` — inverts the wrapped predicate.
pub(crate) fn native_negated_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let original = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let result = invoke_predicate_test(original, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(!result))))
}

/// Helper: dispatch `predicate.test(elem)` via ops.invoke, returns bool.
fn invoke_predicate_test(
    predicate: Slot,
    elem: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> VmResult<bool> {
    let Slot::Reference(Some(pred_ref)) = predicate else {
        return Ok(false);
    };
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let res = ops
        .invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![predicate, elem],
        )?
        .unwrap_or(Slot::Int(0));
    Ok(matches!(res, Slot::Int(n) if n != 0))
}

/// Native: `Function.andThen(Function)Function` — `f.andThen(g)` = `g(f(x))`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_function_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let first = args.first().copied().unwrap_or(Slot::Reference(None));
    let second = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/AndThenFunction".to_string(), 2);
    heap.get_mut(r)?.fields[0] = first;
    heap.get_mut(r)?.fields[1] = second;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `AndThenFunction.apply(O)O` — applies first then second.
pub(crate) fn native_and_then_function_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let input = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let first = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let second = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let mid = invoke_function_apply(first, input, heap, out, ops)?;
    invoke_function_apply(second, mid, heap, out, ops).map(Some)
}

/// Native: `Function.compose(Function)Function` — `f.compose(g)` = `f(g(x))`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_function_compose(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let outer = args.first().copied().unwrap_or(Slot::Reference(None));
    let inner = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ComposeFunction".to_string(), 2);
    heap.get_mut(r)?.fields[0] = outer;
    heap.get_mut(r)?.fields[1] = inner;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ComposeFunction.apply(O)O` — applies inner then outer.
pub(crate) fn native_compose_function_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let input = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let outer = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let inner = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let mid = invoke_function_apply(inner, input, heap, out, ops)?;
    invoke_function_apply(outer, mid, heap, out, ops).map(Some)
}

/// Helper: dispatch `function.apply(input)` via ops.invoke.
fn invoke_function_apply(
    function: Slot,
    input: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> VmResult<Slot> {
    let Slot::Reference(Some(fn_ref)) = function else {
        return Ok(Slot::Reference(None));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    Ok(ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![function, input],
        )?
        .unwrap_or(Slot::Reference(None)))
}

/// Native: `Stream.mapToLong(ToLongFunction)LongStream` — maps each element via `applyAsLong`.
pub(crate) fn native_stream_map_to_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut values = Vec::with_capacity(elems.len());
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(Ljava/lang/Object;)J",
                vec![fn_slot, elem],
            )?
            .unwrap_or(Slot::Long(0));
        let v = match result {
            Slot::Long(n) => n,
            Slot::Int(n) => i64::from(n),
            _ => 0,
        };
        values.push(v);
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}

/// Allocates a `duke/util/LongStream` with `fields[0]=Int(size), fields[1..n]=Long(value)`.
fn make_long_stream(heap: &mut duke_gc::Heap, values: Vec<i64>) -> u64 {
    let r = heap.allocate("duke/util/LongStream".to_string(), 1);
    if let Ok(obj) = heap.get_mut(r) {
        obj.fields[0] = Slot::Int(i32::try_from(values.len()).unwrap_or(0));
        for v in values {
            obj.fields.push(Slot::Long(v));
        }
    }
    r
}

/// Native: `LongStream.sum()J` — sums all elements.
pub(crate) fn native_long_stream_sum(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let sum: i64 = heap.get(stream_ref)?.fields[1..=size]
        .iter()
        .map(|s| match s {
            Slot::Long(n) => *n,
            Slot::Int(n) => i64::from(*n),
            _ => 0,
        })
        .sum();
    Ok(Some(Slot::Long(sum)))
}

/// Native: `Stream.mapToDouble(ToDoubleFunction)DoubleStream` — maps each element via `applyAsDouble`.
pub(crate) fn native_stream_map_to_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut values = Vec::with_capacity(elems.len());
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(Ljava/lang/Object;)D",
                vec![fn_slot, elem],
            )?
            .unwrap_or(Slot::Double(0.0));
        let v = match result {
            Slot::Double(d) => d,
            Slot::Float(f) => f64::from(f),
            Slot::Int(n) => f64::from(n),
            _ => 0.0,
        };
        values.push(v);
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, values,
    )))))
}

/// Allocates a `duke/util/DoubleStream` with `fields[0]=Int(size), fields[1..n]=Double(value)`.
fn make_double_stream(heap: &mut duke_gc::Heap, values: Vec<f64>) -> u64 {
    let r = heap.allocate("duke/util/DoubleStream".to_string(), 1);
    if let Ok(obj) = heap.get_mut(r) {
        obj.fields[0] = Slot::Int(i32::try_from(values.len()).unwrap_or(0));
        for v in values {
            obj.fields.push(Slot::Double(v));
        }
    }
    r
}

/// Native: `DoubleStream.sum()D` — sums all elements.
pub(crate) fn native_double_stream_sum(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let sum: f64 = heap.get(stream_ref)?.fields[1..=size]
        .iter()
        .map(|s| match s {
            Slot::Double(d) => *d,
            Slot::Float(f) => f64::from(*f),
            Slot::Int(n) => f64::from(*n),
            _ => 0.0,
        })
        .sum();
    Ok(Some(Slot::Double(sum)))
}

// ---------------------------------------------------------------------------
// Phase 51: LongStream full ops, DoubleStream full ops,
//           IntStream.asLongStream/asDoubleStream, Collectors.summingInt/averagingInt
// ---------------------------------------------------------------------------

// ---- Helper extractors ----

/// Extract long elements from a `duke/util/LongStream`.
fn long_stream_elems(heap: &duke_gc::Heap, ref_: u64) -> Vec<i64> {
    let size = match heap.get(ref_).ok().and_then(|o| o.fields.first().copied()) {
        Some(Slot::Int(n)) => usize::try_from(n).unwrap_or(0),
        _ => 0,
    };
    heap.get(ref_)
        .ok()
        .map(|o| {
            o.fields[1..=size]
                .iter()
                .filter_map(|s| {
                    if let Slot::Long(n) = s {
                        Some(*n)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Extract double elements from a `duke/util/DoubleStream`.
fn double_stream_elems(heap: &duke_gc::Heap, ref_: u64) -> Vec<f64> {
    let size = match heap.get(ref_).ok().and_then(|o| o.fields.first().copied()) {
        Some(Slot::Int(n)) => usize::try_from(n).unwrap_or(0),
        _ => 0,
    };
    heap.get(ref_)
        .ok()
        .map(|o| {
            o.fields[1..=size]
                .iter()
                .filter_map(|s| {
                    if let Slot::Double(d) = s {
                        Some(*d)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Allocate an `OptionalLong`: `fields[0]=Long(value)`, `fields[1]=Int(present)`.
fn make_optional_long(heap: &mut duke_gc::Heap, value: Option<i64>) -> u64 {
    let r = heap.allocate("duke/util/OptionalLong".to_string(), 2);
    if let Ok(obj) = heap.get_mut(r) {
        if let Some(v) = value {
            obj.fields[0] = Slot::Long(v);
            obj.fields[1] = Slot::Int(1);
        } else {
            obj.fields[1] = Slot::Int(0);
        }
    }
    r
}

/// Allocate an `OptionalDouble` (for LongStream/DoubleStream average/min/max).
fn make_optional_double_val(heap: &mut duke_gc::Heap, value: Option<f64>) -> u64 {
    let r = heap.allocate("duke/util/OptionalDouble".to_string(), 2);
    if let Ok(obj) = heap.get_mut(r) {
        if let Some(v) = value {
            obj.fields[0] = Slot::Double(v);
            obj.fields[1] = Slot::Int(1);
        } else {
            obj.fields[1] = Slot::Int(0);
        }
    }
    r
}

// ---- LongStream static factories ----

/// Native: `LongStream.of(long[])LongStream` — from a long[] vararg array.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let values: Vec<i64> = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| {
            if let Slot::Long(n) = s {
                Some(*n)
            } else {
                None
            }
        })
        .collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}

/// Native: `LongStream.range(long,long)LongStream` — half-open range [start, end).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let start = match args.first().copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let end = match args.get(1).copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let values: Vec<i64> = (start..end).collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}

/// Native: `LongStream.rangeClosed(long,long)LongStream` — inclusive range [start, end].
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_range_closed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let start = match args.first().copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let end = match args.get(1).copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let values: Vec<i64> = (start..=end).collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}

// ---- LongStream terminal ops ----

/// Native: `LongStream.count()J`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match heap.get(r)?.fields.first() {
        Some(Slot::Int(n)) => i64::from(*n),
        _ => 0,
    };
    Ok(Some(Slot::Long(n)))
}

/// Native: `LongStream.min()OptionalLong`
pub(crate) fn native_long_stream_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let opt = make_optional_long(heap, elems.into_iter().min());
    Ok(Some(Slot::Reference(Some(opt))))
}

/// Native: `LongStream.max()OptionalLong`
pub(crate) fn native_long_stream_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let opt = make_optional_long(heap, elems.into_iter().max());
    Ok(Some(Slot::Reference(Some(opt))))
}

/// Native: `LongStream.average()OptionalDouble`
pub(crate) fn native_long_stream_average(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let opt = if elems.is_empty() {
        None
    } else {
        #[allow(clippy::cast_precision_loss)]
        Some(elems.iter().sum::<i64>() as f64 / elems.len() as f64)
    };
    let opt_ref = make_optional_double_val(heap, opt);
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `LongStream.toArray()long[]`
pub(crate) fn native_long_stream_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let arr_ref = heap.allocate("[J".to_string(), elems.len());
    for (i, v) in elems.into_iter().enumerate() {
        heap.get_mut(arr_ref)?.fields[i] = Slot::Long(v);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `LongStream.sorted()LongStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_sorted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let mut elems = long_stream_elems(heap, r);
    elems.sort_unstable();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, elems)))))
}

/// Native: `LongStream.distinct()LongStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_distinct(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let mut seen = std::collections::HashSet::new();
    let elems: Vec<i64> = long_stream_elems(heap, r)
        .into_iter()
        .filter(|v| seen.insert(*v))
        .collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, elems)))))
}

/// Native: `LongStream.reduce(long, LongBinaryOperator)long`
pub(crate) fn native_long_stream_reduce_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let identity = match args.get(1).copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let fn_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Long(identity)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = long_stream_elems(heap, r);
    let mut acc = identity;
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(JJ)J",
                vec![fn_slot, Slot::Long(acc), Slot::Long(v)],
            )?
            .unwrap_or(Slot::Long(0));
        acc = match result {
            Slot::Long(n) => n,
            Slot::Int(n) => i64::from(n),
            _ => acc,
        };
    }
    Ok(Some(Slot::Long(acc)))
}

/// Native: `LongStream.boxed()Stream` — boxes each long into `java/lang/Long`.
pub(crate) fn native_long_stream_boxed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for v in elems {
        let boxed_ref = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(boxed_ref)?.fields[0] = Slot::Long(v);
        heap.get_mut(stream_ref)?
            .fields
            .push(Slot::Reference(Some(boxed_ref)));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

// ---- LongStream intermediate ops ----

/// Native: `LongStream.filter(LongPredicate)LongStream`
pub(crate) fn native_long_stream_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::new();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(J)Z",
            vec![pred_slot, Slot::Long(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, kept)))))
}

/// Native: `LongStream.map(LongUnaryOperator)LongStream`
pub(crate) fn native_long_stream_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::new();
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(J)J",
            vec![fn_slot, Slot::Long(v)],
        )?;
        result.push(match r {
            Some(Slot::Long(n)) => n,
            Some(Slot::Int(n)) => i64::from(n),
            _ => 0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}

/// Native: `LongStream.forEach(LongConsumer)V`
pub(crate) fn native_long_stream_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let consumer_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(None);
    };
    let elems = long_stream_elems(heap, r);
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for v in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(J)V",
            vec![consumer_slot, Slot::Long(v)],
        )?;
    }
    Ok(None)
}

// ---- LongStream.mapToInt / mapToDouble ----

/// Native: `LongStream.mapToInt(LongToIntFunction)IntStream`
pub(crate) fn native_long_stream_map_to_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::new();
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(J)I",
            vec![fn_slot, Slot::Long(v)],
        )?;
        result.push(match r {
            Some(Slot::Int(n)) => n,
            _ => 0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}

// ---- DoubleStream static factories ----

/// Native: `DoubleStream.of(double[])DoubleStream` — from a double[] vararg array.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let values: Vec<f64> = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| {
            if let Slot::Double(d) = s {
                Some(*d)
            } else {
                None
            }
        })
        .collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, values,
    )))))
}

/// Native: `DoubleStream.of(double)DoubleStream` — single-element factory.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_of_single(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = match args.first() {
        Some(Slot::Double(d)) => *d,
        Some(Slot::Float(f)) => f64::from(*f),
        _ => 0.0,
    };
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap,
        vec![v],
    )))))
}

// ---- DoubleStream terminal ops ----

/// Native: `DoubleStream.count()J`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match heap.get(r)?.fields.first() {
        Some(Slot::Int(n)) => i64::from(*n),
        _ => 0,
    };
    Ok(Some(Slot::Long(n)))
}

/// Native: `DoubleStream.min()OptionalDouble`
pub(crate) fn native_double_stream_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let min = elems.iter().copied().reduce(f64::min);
    let opt_ref = make_optional_double_val(heap, min);
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `DoubleStream.max()OptionalDouble`
pub(crate) fn native_double_stream_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let max = elems.iter().copied().reduce(f64::max);
    let opt_ref = make_optional_double_val(heap, max);
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `DoubleStream.average()OptionalDouble`
pub(crate) fn native_double_stream_average(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let opt = if elems.is_empty() {
        None
    } else {
        #[allow(clippy::cast_precision_loss)]
        Some(elems.iter().sum::<f64>() / elems.len() as f64)
    };
    let opt_ref = make_optional_double_val(heap, opt);
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `DoubleStream.toArray()double[]`
pub(crate) fn native_double_stream_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let arr_ref = heap.allocate("[D".to_string(), elems.len());
    for (i, v) in elems.into_iter().enumerate() {
        heap.get_mut(arr_ref)?.fields[i] = Slot::Double(v);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `DoubleStream.sorted()DoubleStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_sorted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let mut elems = double_stream_elems(heap, r);
    elems.sort_by(f64::total_cmp);
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, elems)))))
}

// ---- DoubleStream intermediate ops ----

/// Native: `DoubleStream.filter(DoublePredicate)DoubleStream`
pub(crate) fn native_double_stream_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::new();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(D)Z",
            vec![pred_slot, Slot::Double(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, kept)))))
}

/// Native: `DoubleStream.map(DoubleUnaryOperator)DoubleStream`
pub(crate) fn native_double_stream_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::new();
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(D)D",
            vec![fn_slot, Slot::Double(v)],
        )?;
        result.push(match r {
            Some(Slot::Double(d)) => d,
            Some(Slot::Float(f)) => f64::from(f),
            Some(Slot::Int(n)) => f64::from(n),
            _ => 0.0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, result,
    )))))
}

// ---- IntStream.asLongStream / asDoubleStream ----

/// Native: `IntStream.asLongStream()LongStream` — widens each int to long.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_as_long_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let values: Vec<i64> = int_stream_elems(heap, r)
        .into_iter()
        .map(i64::from)
        .collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}

/// Native: `IntStream.asDoubleStream()DoubleStream` — widens each int to double.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_as_double_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let values: Vec<f64> = int_stream_elems(heap, r)
        .into_iter()
        .map(f64::from)
        .collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, values,
    )))))
}

// ---- OptionalLong ----

/// Native: `OptionalLong.getAsLong()J`
pub(crate) fn native_optional_long_get_as_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    if !present {
        return Err(VmError::MethodNotFound {
            name: "OptionalLong.getAsLong on empty".to_string(),
            descriptor: String::new(),
        });
    }
    Ok(Some(
        heap.get(r)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Long(0)),
    ))
}

/// Native: `OptionalLong.isPresent()Z`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_long_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}

// ---- Collectors.summingInt / averagingInt ----

/// Native: `Collectors.summingInt(ToIntFunction)Collector` — returns a `SummingIntCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_summing_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/SummingIntCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.averagingInt(ToIntFunction)Collector` — returns an `AveragingIntCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_averaging_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/AveragingIntCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Phase 53: IntStream/LongStream terminal ops, Comparator.comparingLong,
//           Optional.or / ifPresentOrElse, Collectors.toUnmodifiable*
// ---------------------------------------------------------------------------

/// Native: `IntStream.findFirst()OptionalInt`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_find_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let opt = make_optional_int(heap, elems.into_iter().next());
    Ok(Some(Slot::Reference(Some(opt))))
}

/// Native: `IntStream.anyMatch(IntPredicate)Z`
pub(crate) fn native_int_stream_any_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(I)Z",
            vec![pred_slot, Slot::Int(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `IntStream.allMatch(IntPredicate)Z`
pub(crate) fn native_int_stream_all_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(I)Z",
            vec![pred_slot, Slot::Int(v)],
        )?;
        if !matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `IntStream.noneMatch(IntPredicate)Z`
pub(crate) fn native_int_stream_none_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(I)Z",
            vec![pred_slot, Slot::Int(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `IntStream.mapToLong(IntToLongFunction)LongStream`
pub(crate) fn native_int_stream_map_to_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::new();
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(I)J",
            vec![fn_slot, Slot::Int(v)],
        )?;
        result.push(match r {
            Some(Slot::Long(n)) => n,
            Some(Slot::Int(n)) => i64::from(n),
            _ => 0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}

/// Native: `LongStream.findFirst()OptionalLong`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_find_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let opt = make_optional_long(heap, elems.into_iter().next());
    Ok(Some(Slot::Reference(Some(opt))))
}

/// Native: `LongStream.anyMatch(LongPredicate)Z`
pub(crate) fn native_long_stream_any_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(J)Z",
            vec![pred_slot, Slot::Long(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `LongStream.allMatch(LongPredicate)Z`
pub(crate) fn native_long_stream_all_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(J)Z",
            vec![pred_slot, Slot::Long(v)],
        )?;
        if !matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `LongStream.noneMatch(LongPredicate)Z`
pub(crate) fn native_long_stream_none_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(J)Z",
            vec![pred_slot, Slot::Long(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `Comparator.comparingLong(ToLongFunction)Comparator` — wraps key extractor.
/// Creates a `duke/util/ComparingLongComparator` with `fields[0] = fn_ref`.
pub(crate) fn native_comparator_comparing_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_ref = extract_ref_arg(args, 0)?;
    let r = heap.allocate("duke/util/ComparingLongComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(Some(fn_ref));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ComparingLongComparator.compare(O,O)I` — calls `fn.applyAsLong(o)` for each element.
pub(crate) fn native_comparing_long_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(Ljava/lang/Object;)J",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Long(0));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(Ljava/lang/Object;)J",
            vec![fn_slot, b],
        )?
        .unwrap_or(Slot::Long(0));
    let result = match (ka, kb) {
        (Slot::Long(la), Slot::Long(lb)) => la.cmp(&lb) as i32,
        (Slot::Int(ia), Slot::Int(ib)) => ia.cmp(&ib) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(result)))
}

// ---------------------------------------------------------------------------
// Phase 58: Comparator.comparingDouble, Map.copyOf/entry/ofEntries,
//           Collections.singletonMap/singletonSet/unmodifiableSet
// ---------------------------------------------------------------------------

/// Creates a `duke/util/ComparingDoubleComparator` with `fields[0] = fn_ref`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_comparing_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_ref = extract_ref_arg(args, 0)?;
    let r = heap.allocate("duke/util/ComparingDoubleComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(Some(fn_ref));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ComparingDoubleComparator.compare(O,O)I` — calls `fn.applyAsDouble(o)` for each.
pub(crate) fn native_comparing_double_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(Ljava/lang/Object;)D",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Double(0.0));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(Ljava/lang/Object;)D",
            vec![fn_slot, b],
        )?
        .unwrap_or(Slot::Double(0.0));
    let result = match (ka, kb) {
        (Slot::Double(da), Slot::Double(db)) => da.total_cmp(&db) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(result)))
}

/// Native: `Map.copyOf(Map)Map` — returns an unmodifiable copy backed by `HashMap`.
pub(crate) fn native_map_copy_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let copy_ref = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(copy_ref))], heap, out, control)?;
    // Iterate source map's interleaved key-val pairs: fields[0]=size, fields[1..]=k,v,k,v,...
    let size = match heap.get(src_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let pairs: Vec<Slot> = heap.get(src_ref)?.fields[1..=size * 2].to_vec();
    let mut i = 0;
    while i + 1 < pairs.len() {
        let k = pairs[i];
        let v = pairs[i + 1];
        native_hashmap_put(&[Slot::Reference(Some(copy_ref)), k, v], heap, out, control)?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(copy_ref))))
}

/// Native: `Map.entry(K,V)Map.Entry` — creates an immutable Map.Entry.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_map_entry_factory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let key = args.first().copied().unwrap_or(Slot::Reference(None));
    let val = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("java/util/Map$Entry".to_string(), 2);
    heap.get_mut(r)?.fields[0] = key;
    heap.get_mut(r)?.fields[1] = val;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Map.ofEntries(Map.Entry[])Map` — builds a `HashMap` from varargs Entry array.
pub(crate) fn native_map_of_entries(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(map_ref))], heap, out, control)?;
    // args[0] is the Object[] array of Map.Entry objects (anewarray layout: fields = elements)
    if let Some(Slot::Reference(Some(arr_ref))) = args.first().copied() {
        let entries: Vec<Slot> = heap.get(arr_ref)?.fields.clone();
        for entry_slot in entries {
            let Slot::Reference(Some(entry_ref)) = entry_slot else {
                continue;
            };
            let key = heap
                .get(entry_ref)?
                .fields
                .first()
                .copied()
                .unwrap_or(Slot::Reference(None));
            let val = heap
                .get(entry_ref)?
                .fields
                .get(1)
                .copied()
                .unwrap_or(Slot::Reference(None));
            native_hashmap_put(
                &[Slot::Reference(Some(map_ref)), key, val],
                heap,
                out,
                control,
            )?;
        }
    }
    Ok(Some(Slot::Reference(Some(map_ref))))
}

/// Native: `Collections.singletonMap(K,V)Map` — returns a single-entry unmodifiable map.
pub(crate) fn native_collections_singleton_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_map_of(args, heap, out, control)
}

/// Native: `Collections.singleton(E)Set` — returns a single-element unmodifiable set.
pub(crate) fn native_collections_singleton_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_set_of_factory(args, heap, out, control)
}

/// Native: `Collections.unmodifiableSet(Set)Set` — returns a view of the set (same backing object).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_set(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // Our sets are already value objects; just return the same reference.
    let set_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    Ok(Some(set_slot))
}

// ---------------------------------------------------------------------------
// Phase 59: Stream.flatMapToInt/Long/Double, Collectors.toMap (3-arg),
//           forEach on HashSet/TreeSet/TreeMap/LinkedList/LinkedHashMap/PriorityQueue
// ---------------------------------------------------------------------------

/// Native: `Stream.flatMapToInt(Function<T,IntStream>)IntStream`
pub(crate) fn native_stream_flat_map_to_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result: Vec<i32> = Vec::new();
    for elem in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, elem],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(int_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}

/// Native: `Stream.flatMapToLong(Function<T,LongStream>)LongStream`
pub(crate) fn native_stream_flat_map_to_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result: Vec<i64> = Vec::new();
    for elem in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, elem],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(long_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}

/// Native: `Stream.flatMapToDouble(Function<T,DoubleStream>)DoubleStream`
pub(crate) fn native_stream_flat_map_to_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result: Vec<f64> = Vec::new();
    for elem in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, elem],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(double_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, result,
    )))))
}

/// Native: `Collectors.toMap(keyFn, valFn, mergeFn)Collector` — stores three functions.
pub(crate) fn native_collectors_to_map_merge(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let key_fn = args.first().copied().unwrap_or(Slot::Reference(None));
    let val_fn = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let merge_fn = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ToMapMergeCollector".to_string(), 3);
    heap.get_mut(r)?.fields[0] = key_fn;
    heap.get_mut(r)?.fields[1] = val_fn;
    heap.get_mut(r)?.fields[2] = merge_fn;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `HashSet.forEach(Consumer)V`
pub(crate) fn native_hashset_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(consumer_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return Ok(None);
    };
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for elem in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![Slot::Reference(Some(consumer_ref)), elem],
        )?;
    }
    Ok(None)
}

/// Native: `TreeSet.forEach(Consumer)V`
pub(crate) fn native_treeset_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // TreeSet uses same layout as HashSet: fields[0]=size, fields[1..size]=elements
    native_hashset_for_each(args, heap, out, control, ops)
}

/// Native: `TreeMap.forEach(BiConsumer)V`
pub(crate) fn native_treemap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // TreeMap uses same layout as HashMap: fields[0]=size, fields[1,2]=k0/v0 ...
    native_hashmap_for_each(args, heap, out, control, ops)
}

/// Native: `LinkedHashMap.forEach(BiConsumer)V`
pub(crate) fn native_linkedhashmap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    native_hashmap_for_each(args, heap, out, control, ops)
}

/// Native: `LinkedList.forEach(Consumer)V`
pub(crate) fn native_linked_list_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    native_arraylist_for_each(args, heap, out, control, ops)
}

/// Native: `PriorityQueue.forEach(Consumer)V`
pub(crate) fn native_priorityqueue_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    native_arraylist_for_each(args, heap, out, control, ops)
}

// ---------------------------------------------------------------------------
// Phase 60: IntStream/LongStream/DoubleStream takeWhile/dropWhile,
//           Integer/Long/Double compare/max/min,
//           TreeMap.keySet/values/getOrDefault, TreeSet.stream
// ---------------------------------------------------------------------------

/// Native: `IntStream.takeWhile(IntPredicate)IntStream` — keeps prefix while predicate holds.
pub(crate) fn native_int_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut kept = Vec::new();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(I)Z",
            vec![pred_slot, Slot::Int(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        } else {
            break;
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, kept)))))
}

/// Native: `IntStream.dropWhile(IntPredicate)IntStream` — drops prefix while predicate holds.
pub(crate) fn native_int_stream_drop_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut dropping = true;
    let mut kept = Vec::new();
    for v in elems {
        if dropping {
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(I)Z",
                vec![pred_slot, Slot::Int(v)],
            )?;
            if !matches!(result, Some(Slot::Int(n)) if n != 0) {
                dropping = false;
                kept.push(v);
            }
        } else {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, kept)))))
}

/// Native: `LongStream.takeWhile(LongPredicate)LongStream` — keeps prefix while predicate holds.
pub(crate) fn native_long_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut kept = Vec::new();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(J)Z",
            vec![pred_slot, Slot::Long(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        } else {
            break;
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, kept)))))
}

/// Native: `LongStream.dropWhile(LongPredicate)LongStream` — drops prefix while predicate holds.
pub(crate) fn native_long_stream_drop_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut dropping = true;
    let mut kept = Vec::new();
    for v in elems {
        if dropping {
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(J)Z",
                vec![pred_slot, Slot::Long(v)],
            )?;
            if !matches!(result, Some(Slot::Int(n)) if n != 0) {
                dropping = false;
                kept.push(v);
            }
        } else {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, kept)))))
}

/// Native: `DoubleStream.takeWhile(DoublePredicate)DoubleStream` — keeps prefix while predicate holds.
pub(crate) fn native_double_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut kept = Vec::new();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(D)Z",
            vec![pred_slot, Slot::Double(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        } else {
            break;
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, kept)))))
}

/// Native: `DoubleStream.dropWhile(DoublePredicate)DoubleStream` — drops prefix while predicate holds.
pub(crate) fn native_double_stream_drop_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut dropping = true;
    let mut kept = Vec::new();
    for v in elems {
        if dropping {
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(D)Z",
                vec![pred_slot, Slot::Double(v)],
            )?;
            if !matches!(result, Some(Slot::Int(n)) if n != 0) {
                dropping = false;
                kept.push(v);
            }
        } else {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, kept)))))
}

/// Native: `Integer.compare(int,int)int` — returns negative/zero/positive.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_integer_compare(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(a.cmp(&b) as i32)))
}

/// Native: `Integer.max(int,int)int`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_integer_max(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(a.max(b))))
}

/// Native: `Integer.min(int,int)int`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_integer_min(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(a.min(b))))
}

/// Native: `Long.compare(long,long)int`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_compare(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(Some(Slot::Int(a.cmp(&b) as i32)))
}

/// Native: `Long.max(long,long)long`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_max(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(Some(Slot::Long(a.max(b))))
}

/// Native: `Long.min(long,long)long`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_min(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(Some(Slot::Long(a.min(b))))
}

/// Native: `Double.compare(double,double)int`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_compare(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    Ok(Some(Slot::Int(a.total_cmp(&b) as i32)))
}

/// Native: `Double.max(double,double)double`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_max(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    Ok(Some(Slot::Double(a.max(b))))
}

/// Native: `Double.min(double,double)double`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_min(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    Ok(Some(Slot::Double(a.min(b))))
}

/// Native: `TreeMap.keySet()Set` — delegates to `HashMap` keySet (same field layout).
pub(crate) fn native_treemap_key_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_hashmap_key_set(args, heap, out, control)
}

/// Native: `TreeMap.values()Collection` — delegates to `HashMap` values (same field layout).
pub(crate) fn native_treemap_values(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_hashmap_values(args, heap, out, control)
}

/// Native: `TreeMap.getOrDefault(Object,Object)Object` — looks up key; returns default if absent.
pub(crate) fn native_treemap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let default_val = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    Ok(Some(
        find_hashmap_entry_index(&fields, &key, heap).map_or(default_val, |i| fields[i + 1]),
    ))
}

/// Native: `TreeSet.stream()Stream` — wraps sorted elements into a `duke/util/Stream`.
pub(crate) fn native_treeset_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // TreeSet layout: fields[0]=size, fields[1..=size]=elements (same as HashSet)
    native_hashset_stream(args, heap, out, control)
}

/// Native: `Optional.or(Supplier<Optional>)Optional` (Java 9) —
/// returns this Optional if present; otherwise invokes supplier and returns its result.
pub(crate) fn native_optional_or(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let supplier_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    // If present (non-null value stored), return self
    if !matches!(value, Slot::Reference(None)) {
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    }
    let Slot::Reference(Some(supplier_ref)) = supplier_slot else {
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    };
    let supplier_class = heap.get(supplier_ref)?.class_name.clone();
    let result = ops.invoke(
        heap,
        out,
        &supplier_class,
        "get",
        "()Ljava/lang/Object;",
        vec![supplier_slot],
    )?;
    Ok(Some(result.unwrap_or(Slot::Reference(None))))
}

/// Native: `Optional.ifPresentOrElse(Consumer, Runnable)V` (Java 9) —
/// if value present invokes consumer, otherwise invokes runnable.
pub(crate) fn native_optional_if_present_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let runnable_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    if matches!(value, Slot::Reference(None)) {
        let Slot::Reference(Some(runnable_ref)) = runnable_slot else {
            return Ok(None);
        };
        let runnable_class = heap.get(runnable_ref)?.class_name.clone();
        ops.invoke(
            heap,
            out,
            &runnable_class,
            "run",
            "()V",
            vec![runnable_slot],
        )?;
    } else {
        let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
            return Ok(None);
        };
        let consumer_class = heap.get(consumer_ref)?.class_name.clone();
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, value],
        )?;
    }
    Ok(None)
}

/// Native: `Collectors.toUnmodifiableList()Collector` (Java 10) —
/// returns the same `ToListCollector` sentinel; our interpreter treats all lists as modifiable.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_unmodifiable_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("duke/util/ToListCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.toUnmodifiableSet()Collector` (Java 10) —
/// returns the same `ToSetCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_unmodifiable_set(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("duke/util/ToSetCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Phase 54: IntStream/LongStream/DoubleStream limit/skip,
//           IntStream/LongStream flatMap, Collectors.mapping
// ---------------------------------------------------------------------------

/// Native: `IntStream.limit(long)IntStream` — truncate to at most n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<i32> = int_stream_elems(heap, r).into_iter().take(n).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, elems)))))
}

/// Native: `IntStream.skip(long)IntStream` — skip first n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_skip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<i32> = int_stream_elems(heap, r).into_iter().skip(n).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, elems)))))
}

/// Native: `IntStream.flatMap(IntFunction<IntStream>)IntStream` — map each int to an `IntStream` and concatenate.
pub(crate) fn native_int_stream_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::new();
    for v in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(I)Ljava/lang/Object;",
            vec![fn_slot, Slot::Int(v)],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            let sub_elems = int_stream_elems(heap, sub_ref);
            result.extend(sub_elems);
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}

/// Native: `LongStream.limit(long)LongStream` — truncate to at most n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<i64> = long_stream_elems(heap, r).into_iter().take(n).collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, elems)))))
}

/// Native: `LongStream.skip(long)LongStream` — skip first n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_skip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<i64> = long_stream_elems(heap, r).into_iter().skip(n).collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, elems)))))
}

/// Native: `LongStream.flatMap(LongFunction<LongStream>)LongStream`
pub(crate) fn native_long_stream_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::new();
    for v in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(J)Ljava/lang/Object;",
            vec![fn_slot, Slot::Long(v)],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            let sub_elems = long_stream_elems(heap, sub_ref);
            result.extend(sub_elems);
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}

/// Native: `DoubleStream.limit(long)DoubleStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<f64> = double_stream_elems(heap, r).into_iter().take(n).collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, elems)))))
}

/// Native: `DoubleStream.skip(long)DoubleStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_skip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<f64> = double_stream_elems(heap, r).into_iter().skip(n).collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, elems)))))
}

/// Native: `Collectors.groupingBy(Function, Collector)Collector` — 2-arg version with downstream.
/// Creates a `duke/util/GroupingBy2Collector` with fields[0]=keyFn, fields[1]=downstream.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_grouping_by_2(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let downstream_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/GroupingBy2Collector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = fn_slot;
    heap.get_mut(r)?.fields[1] = downstream_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.mapping(Function, Collector)Collector` — transforms elements before
/// feeding to a downstream collector.  Creates a `duke/util/MappingCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_mapping(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let mapper_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let downstream_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/MappingCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = mapper_slot;
    heap.get_mut(r)?.fields[1] = downstream_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Phase 55: Complete DoubleStream, LongStream gaps, Collectors.summingLong/averagingDouble
// ---------------------------------------------------------------------------

/// Native: `DoubleStream.forEach(DoubleConsumer)V`
pub(crate) fn native_double_stream_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let consumer_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(None);
    };
    let elems = double_stream_elems(heap, r);
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for v in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(D)V",
            vec![consumer_slot, Slot::Double(v)],
        )?;
    }
    Ok(None)
}

/// Native: `DoubleStream.anyMatch(DoublePredicate)Z`
pub(crate) fn native_double_stream_any_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(D)Z",
            vec![pred_slot, Slot::Double(v)],
        )?;
        if matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `DoubleStream.allMatch(DoublePredicate)Z`
pub(crate) fn native_double_stream_all_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(D)Z",
            vec![pred_slot, Slot::Double(v)],
        )?;
        if !matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `DoubleStream.noneMatch(DoublePredicate)Z`
pub(crate) fn native_double_stream_none_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(D)Z",
            vec![pred_slot, Slot::Double(v)],
        )?;
        if matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `DoubleStream.findFirst()OptionalDouble`
pub(crate) fn native_double_stream_find_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let opt_ref = make_optional_double_val(heap, elems.into_iter().next());
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `DoubleStream.reduce(double, DoubleBinaryOperator)D`
pub(crate) fn native_double_stream_reduce_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let identity = match args.get(1).copied() {
        Some(Slot::Double(d)) => d,
        _ => 0.0,
    };
    let fn_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Double(identity)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = double_stream_elems(heap, r);
    let mut acc = identity;
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(DD)D",
                vec![fn_slot, Slot::Double(acc), Slot::Double(v)],
            )?
            .unwrap_or(Slot::Double(0.0));
        acc = match result {
            Slot::Double(d) => d,
            Slot::Float(f) => f64::from(f),
            Slot::Int(n) => f64::from(n),
            _ => acc,
        };
    }
    Ok(Some(Slot::Double(acc)))
}

/// Native: `DoubleStream.reduce(DoubleBinaryOperator)OptionalDouble`
pub(crate) fn native_double_stream_reduce_optional(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        let opt_ref = make_optional_double_val(heap, None);
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = double_stream_elems(heap, r);
    if elems.is_empty() {
        return Ok(Some(Slot::Reference(Some(make_optional_double_val(
            heap, None,
        )))));
    }
    let mut acc = elems[0];
    for &v in &elems[1..] {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(DD)D",
                vec![fn_slot, Slot::Double(acc), Slot::Double(v)],
            )?
            .unwrap_or(Slot::Double(0.0));
        acc = match result {
            Slot::Double(d) => d,
            Slot::Float(f) => f64::from(f),
            Slot::Int(n) => f64::from(n),
            _ => acc,
        };
    }
    Ok(Some(Slot::Reference(Some(make_optional_double_val(
        heap,
        Some(acc),
    )))))
}

/// Native: `DoubleStream.flatMap(DoubleFunction<DoubleStream>)DoubleStream`
pub(crate) fn native_double_stream_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::new();
    for v in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(D)Ljava/lang/Object;",
            vec![fn_slot, Slot::Double(v)],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(double_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, result,
    )))))
}

/// Native: `DoubleStream.mapToInt(DoubleToIntFunction)IntStream`
pub(crate) fn native_double_stream_map_to_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::new();
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(D)I",
            vec![fn_slot, Slot::Double(v)],
        )?;
        result.push(match r {
            Some(Slot::Int(n)) => n,
            _ => 0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}

/// Native: `DoubleStream.mapToLong(DoubleToLongFunction)LongStream`
pub(crate) fn native_double_stream_map_to_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::new();
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(D)J",
            vec![fn_slot, Slot::Double(v)],
        )?;
        result.push(match r {
            Some(Slot::Long(n)) => n,
            Some(Slot::Int(n)) => i64::from(n),
            _ => 0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}

/// Native: `DoubleStream.distinct()DoubleStream` — removes duplicate values.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_distinct(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<f64> = elems
        .into_iter()
        .filter(|&v| seen.insert(v.to_bits()))
        .collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, deduped,
    )))))
}

/// Native: `DoubleStream.boxed()Stream` — boxes each double into `java/lang/Double`.
pub(crate) fn native_double_stream_boxed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for v in elems {
        let boxed_ref = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed_ref)?.fields[0] = Slot::Double(v);
        heap.get_mut(stream_ref)?
            .fields
            .push(Slot::Reference(Some(boxed_ref)));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `OptionalDouble.isPresent()Z`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_double_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}

/// Native: `LongStream.reduce(LongBinaryOperator)OptionalLong`
pub(crate) fn native_long_stream_reduce_optional(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        let opt_ref = make_optional_long(heap, None);
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = long_stream_elems(heap, r);
    if elems.is_empty() {
        return Ok(Some(Slot::Reference(Some(make_optional_long(heap, None)))));
    }
    let mut acc = elems[0];
    for &v in &elems[1..] {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(JJ)J",
                vec![fn_slot, Slot::Long(acc), Slot::Long(v)],
            )?
            .unwrap_or(Slot::Long(0));
        acc = match result {
            Slot::Long(n) => n,
            Slot::Int(n) => i64::from(n),
            _ => acc,
        };
    }
    Ok(Some(Slot::Reference(Some(make_optional_long(
        heap,
        Some(acc),
    )))))
}

/// Native: `LongStream.mapToDouble(LongToDoubleFunction)DoubleStream`
pub(crate) fn native_long_stream_map_to_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::new();
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(J)D",
            vec![fn_slot, Slot::Long(v)],
        )?;
        result.push(match r {
            Some(Slot::Double(d)) => d,
            Some(Slot::Float(f)) => f64::from(f),
            Some(Slot::Int(n)) => f64::from(n),
            _ => 0.0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, result,
    )))))
}

/// Native: `Collectors.summingLong(ToLongFunction)Collector` — returns a `SummingLongCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_summing_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/SummingLongCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Phase 56: Collectors.minBy/maxBy, summingDouble, averagingLong,
//           toUnmodifiableMap, collectingAndThen
// ---------------------------------------------------------------------------

/// Native: `Collectors.minBy(Comparator)Collector` — returns a `MinByCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_min_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let cmp_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/MinByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = cmp_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.maxBy(Comparator)Collector` — returns a `MaxByCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_max_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let cmp_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/MaxByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = cmp_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.summingDouble(ToDoubleFunction)Collector` — returns a `SummingDoubleCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_summing_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/SummingDoubleCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.averagingLong(ToLongFunction)Collector` — returns an `AveragingLongCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_averaging_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/AveragingLongCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.toUnmodifiableMap(keyFn, valueFn)Collector` — same sentinel as `ToMapCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_unmodifiable_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let key_fn_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let val_fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ToMapCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = key_fn_slot;
    heap.get_mut(r)?.fields[1] = val_fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.collectingAndThen(downstream, finisher)Collector` — returns a
/// `CollectingAndThenCollector` with fields[0]=downstream, fields[1]=finisher.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_collecting_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let downstream_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let finisher_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/CollectingAndThenCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = downstream_slot;
    heap.get_mut(r)?.fields[1] = finisher_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.averagingDouble(ToDoubleFunction)Collector` — returns an `AveragingDoubleCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_averaging_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/AveragingDoubleCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Phase 57: Collectors.reducing, Stream.iterate predicate, Optional.stream,
//           ArrayDeque completion
// ---------------------------------------------------------------------------

/// Native: `Collectors.reducing(BinaryOperator)` — returns a
/// `ReducingNoIdentityCollector` with fields[0]=op.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_reducing_no_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let op_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ReducingNoIdentityCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = op_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.reducing(T, BinaryOperator)` — returns a
/// `ReducingCollector` with fields[0]=identity, fields[1]=op.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_reducing_with_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let identity_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let op_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ReducingCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = identity_slot;
    heap.get_mut(r)?.fields[1] = op_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.reducing(U, Function, BinaryOperator)` — returns a
/// `ReducingMappingCollector` with fields[0]=identity, fields[1]=mapper, fields[2]=op.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_reducing_mapping(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let identity_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let mapper_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let op_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ReducingMappingCollector".to_string(), 3);
    heap.get_mut(r)?.fields[0] = identity_slot;
    heap.get_mut(r)?.fields[1] = mapper_slot;
    heap.get_mut(r)?.fields[2] = op_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Stream.iterate(seed, Predicate, UnaryOperator)Stream` — Java 9 3-arg form.
/// Eagerly materialises elements while predicate returns true, capped at 10,000.
pub(crate) fn native_stream_iterate_predicate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let seed = args.first().copied().unwrap_or(Slot::Reference(None));
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let next_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Err(VmError::NullPointerException);
    };
    let Slot::Reference(Some(next_ref)) = next_slot else {
        return Err(VmError::NullPointerException);
    };
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let next_class = heap.get(next_ref)?.class_name.clone();

    let mut elems: Vec<Slot> = Vec::new();
    let mut current = seed;
    for _ in 0..10_000usize {
        let test = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![pred_slot, current],
        )?;
        match test {
            Some(Slot::Int(1)) => {}
            _ => break,
        }
        elems.push(current);
        current = ops
            .invoke(
                heap,
                out,
                &next_class,
                "apply",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
                vec![next_slot, current],
            )?
            .unwrap_or(Slot::Reference(None));
    }
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    let size = i32::try_from(elems.len()).unwrap_or(i32::MAX);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(size);
    for elem in elems {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `Optional.stream()Stream` — returns a stream of 0 or 1 elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    match value {
        Slot::Reference(None) => {
            heap.get_mut(out_ref)?.fields[0] = Slot::Int(0);
        }
        v => {
            heap.get_mut(out_ref)?.fields[0] = Slot::Int(1);
            heap.get_mut(out_ref)?.fields.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `ArrayDeque.addFirst(Object)V` — inserts element at front.
pub(crate) fn native_arraydeque_add_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraydeque_push(args, heap, out, control)
}

/// Native: `ArrayDeque.addLast(Object)V` — appends element at back.
pub(crate) fn native_arraydeque_add_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(None)
}

/// Native: `ArrayDeque.offerFirst(Object)Z` — inserts at front, returns true.
pub(crate) fn native_arraydeque_offer_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraydeque_push(args, heap, out, control)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `ArrayDeque.offerLast(Object)Z` — appends at back, returns true.
pub(crate) fn native_arraydeque_offer_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `ArrayDeque.peekFirst()Object` — same as peek (front element, null if empty).
pub(crate) fn native_arraydeque_peek_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraydeque_peek(args, heap, out, control)
}

/// Native: `ArrayDeque.peekLast()Object` — returns last element without removing; null if empty.
pub(crate) fn native_arraydeque_peek_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[size]))
}

/// Native: `ArrayDeque.pollFirst()Object` — same as poll (remove front, null if empty).
pub(crate) fn native_arraydeque_poll_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraydeque_poll(args, heap, out, control)
}

/// Native: `ArrayDeque.pollLast()Object` — removes and returns last element; null if empty.
pub(crate) fn native_arraydeque_poll_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    let elem = heap.get_mut(this_ref)?.fields.remove(size);
    let new_size = i32::try_from(size - 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(Some(elem))
}

/// Native: `ArrayDeque.contains(Object)Z` — returns true if element is present.
pub(crate) fn native_arraydeque_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let found = elems.iter().any(|e| slots_equal(e, &target, heap));
    Ok(Some(Slot::Int(i32::from(found))))
}

/// Native: `ArrayDeque.stream()Stream` — returns elements as an eager Stream.
pub(crate) fn native_arraydeque_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(size).unwrap_or(0));
    for elem in elems {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `ArrayDeque.forEach(Consumer)V` — invokes consumer for each element.
pub(crate) fn native_arraydeque_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(cons_ref)) = consumer_slot else {
        return Err(VmError::NullPointerException);
    };
    let cons_class = heap.get(cons_ref)?.class_name.clone();
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    for elem in elems {
        ops.invoke(
            heap,
            out,
            &cons_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, elem],
        )?;
    }
    Ok(None)
}

/// Native: `ArrayDeque.clear()V` — removes all elements.
pub(crate) fn native_arraydeque_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields.truncate(1);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fuzz;
