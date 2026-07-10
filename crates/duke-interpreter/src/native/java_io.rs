pub(crate) fn native_println_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let string_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => {
            writeln!(out, "null").ok();
            return Ok(None);
        }
        _ => {
            return Err(Error::TypeMismatch {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    writeln!(out).ok();
    Ok(None)
}
pub(crate) fn native_charset_for_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let charset = charset_from_name_ref(heap, name_ref, unsupported_charset_error)?;
    Ok(Some(Slot::Reference(Some(charset_ref(heap, charset)))))
}
/// `Charset.defaultCharset()` returns UTF-8.
///
/// Duke intentionally mirrors JDK 18+ / JEP 400's deterministic UTF-8 default
/// instead of inheriting a host-process locale.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_charset_default_charset(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Reference(Some(charset_ref(
        heap,
        StandardCharset::Utf8,
    )))))
}
pub(crate) fn native_charset_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let charset = charset_from_arg(args, 0, heap)?;
    let name_ref = heap.allocate_string(charset.canonical_name().to_string());
    Ok(Some(Slot::Reference(Some(name_ref))))
}
pub(crate) fn native_charset_is_registered(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _charset = charset_from_arg(args, 0, heap)?;
    Ok(Some(Slot::Int(1)))
}
pub(crate) fn native_charset_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_charset = charset_from_arg(args, 0, heap)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    let other_obj = heap.get(other_ref)?;
    if other_obj.class_name != "java/nio/charset/Charset" {
        return Ok(Some(Slot::Int(0)));
    }
    let equal = other_obj
        .string_value
        .as_deref()
        .is_some_and(|name| name == this_charset.canonical_name());
    Ok(Some(Slot::Int(i32::from(equal))))
}
pub(crate) fn native_charset_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let charset = charset_from_arg(args, 0, heap)?;
    Ok(Some(Slot::Int(java_string_hash(charset.canonical_name()))))
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
        return Err(Error::InvalidRef { address: this_ref });
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
        return Err(Error::InvalidRef { address: this_ref });
    };
    *fd_field = Slot::Int(file_id);
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
        return Err(Error::InvalidRef { address: this_ref });
    };
    *fd_field = Slot::Int(file_id);
    Ok(None)
}
pub(crate) fn native_printstream_init_output_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = args.get(1).copied().unwrap_or(Slot::Reference(None));
    set_object_field(heap, this_ref, 0, target)?;
    Ok(None)
}
pub(crate) fn native_byte_array_output_stream_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.string_value = Some(String::new());
    Ok(None)
}
pub(crate) fn native_byte_array_output_stream_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = heap
        .get(this_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let string_ref = heap.allocate_string(text);
    Ok(Some(Slot::Reference(Some(string_ref))))
}
pub(crate) fn native_resource_input_stream_read(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    resource_stream_ensure_open(heap, this_ref)?;
    let array_ref = resource_stream_array_ref(heap, this_ref)?;
    let cursor = resource_stream_cursor(heap, this_ref)?;
    let bytes = &heap.get(array_ref)?.fields;
    if cursor >= bytes.len() {
        return Ok(Some(Slot::Int(-1)));
    }
    let next = match bytes.get(cursor) {
        Some(Slot::Int(value)) => *value,
        _ => 0,
    };
    resource_stream_set_cursor(heap, this_ref, cursor + 1)?;
    Ok(Some(Slot::Int(next)))
}
pub(crate) fn native_resource_input_stream_read_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let array_ref = extract_ref_arg(args, 1)?;
    let len = heap.get(array_ref)?.fields.len();
    let len_i32 = i32::try_from(len).unwrap_or(i32::MAX);
    native_resource_input_stream_read_bytes_slice(
        &[
            Slot::Reference(Some(this_ref)),
            Slot::Reference(Some(array_ref)),
            Slot::Int(0),
            Slot::Int(len_i32),
        ],
        heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
    )
}
pub(crate) fn native_resource_input_stream_read_bytes_slice(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target_ref = extract_ref_arg(args, 1)?;
    let offset = extract_int_arg(args, 2)?;
    let len = extract_int_arg(args, 3)?;
    resource_stream_ensure_open(heap, this_ref)?;

    let target_len = heap.get(target_ref)?.fields.len();
    if offset < 0 || len < 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    let len = usize::try_from(len).unwrap_or(usize::MAX);
    if offset > target_len || len > target_len.saturating_sub(offset) {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    if len == 0 {
        return Ok(Some(Slot::Int(0)));
    }

    let source_ref = resource_stream_array_ref(heap, this_ref)?;
    let cursor = resource_stream_cursor(heap, this_ref)?;
    let source = heap.get(source_ref)?.fields.clone();
    if cursor >= source.len() {
        return Ok(Some(Slot::Int(-1)));
    }
    let available = source.len() - cursor;
    let read_len = available.min(len);
    {
        let target = heap.get_mut(target_ref)?;
        target.fields[offset..offset + read_len].copy_from_slice(&source[cursor..cursor + read_len]);
    }
    resource_stream_set_cursor(heap, this_ref, cursor + read_len)?;
    Ok(Some(Slot::Int(i32::try_from(read_len).unwrap_or(i32::MAX))))
}
pub(crate) fn native_resource_input_stream_available(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    resource_stream_ensure_open(heap, this_ref)?;
    let available = resource_stream_available_bytes(heap, this_ref)?;
    Ok(Some(Slot::Int(i32::try_from(available).unwrap_or(i32::MAX))))
}
pub(crate) fn native_resource_input_stream_skip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let requested = extract_long_arg(args, 1)?;
    resource_stream_ensure_open(heap, this_ref)?;
    if requested <= 0 {
        return Ok(Some(Slot::Long(0)));
    }
    let available = resource_stream_available_bytes(heap, this_ref)?;
    let skipped = available.min(usize::try_from(requested).unwrap_or(usize::MAX));
    let cursor = resource_stream_cursor(heap, this_ref)?;
    resource_stream_set_cursor(heap, this_ref, cursor + skipped)?;
    Ok(Some(Slot::Long(i64::try_from(skipped).unwrap_or(i64::MAX))))
}
pub(crate) fn native_resource_input_stream_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.write_field(this_ref, RESOURCE_STREAM_CLOSED_FIELD, Slot::Int(1))?;
    Ok(None)
}
/// Native: `PrintStream.print(String)` — no newline.
pub(crate) fn native_print_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let string_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => {
            write!(out, "null").ok();
            return Ok(None);
        }
        _ => {
            return Err(Error::TypeMismatch {
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
) -> Result<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Int(v) => *v, "Int");
    write!(out, "{val}").ok();
    Ok(None)
}
pub(crate) fn native_println_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Long(v) => *v, "Long");
    writeln!(out, "{val}").ok();
    Ok(None)
}
pub(crate) fn native_println_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Float(v) => *v, "Float");
    writeln!(out, "{val}").ok();
    Ok(None)
}
pub(crate) fn native_println_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Double(v) => *v, "Double");
    writeln!(out, "{val}").ok();
    Ok(None)
}
pub(crate) fn native_println_boolean(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Int(v) => *v != 0, "Int(boolean)");
    writeln!(out, "{val}").ok();
    Ok(None)
}
pub(crate) fn native_println_char(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Int(v) => char::from_u32((*v).cast_unsigned()).unwrap_or('?'), "Int(char)");
    writeln!(out, "{val}").ok();
    Ok(None)
}
pub(crate) fn native_println_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
pub(crate) fn native_print_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Long(v) => *v, "Long");
    write!(out, "{val}").ok();
    Ok(None)
}
pub(crate) fn native_print_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Float(v) => *v, "Float");
    write!(out, "{val}").ok();
    Ok(None)
}
pub(crate) fn native_print_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Double(v) => *v, "Double");
    write!(out, "{val}").ok();
    Ok(None)
}
pub(crate) fn native_print_boolean(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_print_arg!(args, Slot::Int(v) => *v != 0, "Int(boolean)");
    write!(out, "{val}").ok();
    Ok(None)
}
pub(crate) fn native_print_char(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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