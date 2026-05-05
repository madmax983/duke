fn file_path_from_ref(
    file_ref: u64,
    heap: &duke_gc::Heap,
) -> Result<std::path::PathBuf> {
    let path_ref = match heap.get(file_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(Error::NullPointerException),
    };
    Ok(std::path::PathBuf::from(string_value_from_ref(heap, path_ref)?))
}
fn file_path_from_this(
    args: &[Slot],
    heap: &duke_gc::Heap,
) -> Result<std::path::PathBuf> {
    let this_ref = extract_ref_arg(args, 0)?;
    file_path_from_ref(this_ref, heap)
}
pub(crate) fn native_file_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path_slot = extract_slot_arg(args, 1);
    let file_obj = heap.get_mut(this_ref)?;
    let Some(path_field) = file_obj.fields.first_mut() else {
        return Err(Error::InvalidRef {
            address: this_ref,
        });
    };
    *path_field = path_slot;
    Ok(None)
}
pub(crate) fn native_file_exists(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.exists()))))
}
pub(crate) fn native_file_is_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.is_file()))))
}
pub(crate) fn native_file_is_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.is_dir()))))
}
fn file_stream_id_from_this(args: &[Slot], heap: &duke_gc::Heap) -> Result<i32> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => {
            Err(Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            })
        }
    }
}
pub(crate) fn native_file_input_stream_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path = path_from_string_slot(args, 1, heap)?;
    let file_id = heap.open_host_input_file(&path)?;
    let stream_obj = heap.get_mut(this_ref)?;
    let Some(fd_field) = stream_obj.fields.first_mut() else {
        return Err(Error::InvalidRef {
            address: this_ref,
        });
    };
    *fd_field = Slot::Int(file_id);
    Ok(None)
}
pub(crate) fn native_file_input_stream_read(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let file_id = file_stream_id_from_this(args, heap)?;
    Ok(Some(Slot::Int(heap.read_host_file_byte(file_id)?)))
}
pub(crate) fn native_file_input_stream_read_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let file_id = extract_io_fd(heap, this_ref)?;
    heap.close_host_file(file_id);
    let stream_obj = heap.get_mut(this_ref)?;
    let Some(fd_field) = stream_obj.fields.first_mut() else {
        return Err(Error::InvalidRef {
            address: this_ref,
        });
    };
    *fd_field = Slot::Int(0);
    Ok(None)
}
pub(crate) fn native_file_output_stream_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path = path_from_string_slot(args, 1, heap)?;
    let file_id = heap.open_host_output_file(&path)?;
    let stream_obj = heap.get_mut(this_ref)?;
    let Some(fd_field) = stream_obj.fields.first_mut() else {
        return Err(Error::InvalidRef {
            address: this_ref,
        });
    };
    *fd_field = Slot::Int(file_id);
    Ok(None)
}
pub(crate) fn native_file_output_stream_write(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let file_id = file_stream_id_from_this(args, heap)?;
    let value = extract_int_arg(args, 1)?;
    heap.write_host_file_byte(file_id, value)?;
    Ok(None)
}
/// Native: `FileOutputStream.writeBytes([BIIZ)V` — writes an array of bytes.
///
/// Performance: avoids a large heap allocation (cloning the entire `fields` vector) by using
/// index-based iteration over the byte array, improving I/O throughput for large files.
pub(crate) fn native_file_output_stream_write_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let file_id = file_stream_id_from_this(args, heap)?;
    let array_ref = extract_ref_arg(args, 1)?;
    let len = heap.get(array_ref)?.fields.len();
    for i in 0..len {
        let byte = heap.get(array_ref)?.fields[i];
        let Slot::Int(value) = byte else {
            return Err(Error::TypeMismatch {
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let file_id = extract_io_fd(heap, this_ref)?;
    heap.close_host_file(file_id);
    let stream_obj = heap.get_mut(this_ref)?;
    let Some(fd_field) = stream_obj.fields.first_mut() else {
        return Err(Error::InvalidRef {
            address: this_ref,
        });
    };
    *fd_field = Slot::Int(0);
    Ok(None)
}
fn file_url_to_path(url: &str) -> Result<std::path::PathBuf> {
    let Some(rest) = url.strip_prefix("file://") else {
        return Err(Error::JavaException {
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
