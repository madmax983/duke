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
    let text = string_value_from_ref(heap, string_ref).unwrap_or_else(|_| "null".to_string());
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

// ─── java/io/InputStreamReader + java/io/BufferedReader (ladder canary) ───────
// Minimal synthetic character-stream stack for reading a classpath resource as
// text, as `LadderApplication.main` does:
//   new BufferedReader(new InputStreamReader(cl.getResourceAsStream(n), UTF_8))
//       .readLine()
// Duke does not model the real java.io Reader hierarchy; these two synthetics
// cover exactly the construct-and-readLine path the fixture exercises.
//
// Documented simplifications:
//   * BufferedReader eagerly drains the entire underlying stream at construction
//     time (real java.io reads lazily / block-buffered). Fine for finite
//     classpath resources.
//   * Decoding honours only the byte-preserving ISO-8859-1/US-ASCII family
//     (1:1 byte→char) vs. everything else as UTF-8 (lossy). The fixture uses
//     UTF-8 ASCII, so both paths are exact here.
//   * `readLine` recognises `\n`, `\r`, and `\r\n` terminators (java.io
//     semantics) and returns null at end-of-input.

/// `InputStreamReader` field layout: `` `[0]` `` underlying `InputStream` ref, `` `[1]` ``
/// charset name String ref (nullable → UTF-8).
const INPUT_STREAM_READER_STREAM_FIELD: usize = 0;
const INPUT_STREAM_READER_CHARSET_FIELD: usize = 1;
/// `BufferedReader` field layout: `` `[0]` `` fully-decoded content String ref, `` `[1]` `` read
/// cursor (char offset) Int.
const BUFFERED_READER_CONTENT_FIELD: usize = 0;
const BUFFERED_READER_CURSOR_FIELD: usize = 1;

/// Drain all remaining bytes from an `InputStream` reference to a byte vector.
///
/// Handles the synthetic `duke/io/ResourceInputStream` (backing byte-array with
/// a read cursor, produced by `getResourceAsStream`) and host-file-backed
/// streams (`fields[0]` = positive fd). Advances the stream to end-of-input,
/// matching real `readAllBytes`/drain semantics.
fn input_stream_drain_all_bytes(stream_ref: u64, heap: &mut duke_gc::Heap) -> Result<Vec<u8>> {
    if heap.get(stream_ref)?.class_name == "duke/io/ResourceInputStream" {
        resource_stream_ensure_open(heap, stream_ref)?;
        let array_ref = resource_stream_array_ref(heap, stream_ref)?;
        let cursor = resource_stream_cursor(heap, stream_ref)?;
        let source = &heap.get(array_ref)?.fields;
        let start = cursor.min(source.len());
        let mut bytes = Vec::with_capacity(source.len() - start);
        for slot in &source[start..] {
            let byte = match slot {
                Slot::Int(value) => u8::try_from(*value & 0xFF).unwrap_or(0),
                _ => 0,
            };
            bytes.push(byte);
        }
        let end = source.len();
        resource_stream_set_cursor(heap, stream_ref, end)?;
        return Ok(bytes);
    }

    // Host-file-backed stream: fields[0] is a positive fd.
    let file_id = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            push_pending_java_exception_message(
                "java/io/IOException",
                "stream is not readable".to_string(),
            );
            return Err(Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
        }
    };
    let mut bytes = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = heap.read_host_file_bytes(file_id, &mut buf)?;
        if n < 0 {
            break;
        }
        let n = usize::try_from(n).unwrap_or(0);
        bytes.extend_from_slice(&buf[..n]);
    }
    Ok(bytes)
}

/// Decode bytes to a Rust `String` honouring the byte-preserving charset family;
/// everything else falls back to UTF-8 (lossy).
fn decode_bytes_with_charset(bytes: &[u8], charset_name: Option<&str>) -> String {
    let byte_preserving = charset_name.is_some_and(|name| {
        let upper = name.to_ascii_uppercase();
        upper.contains("8859") || upper.contains("ASCII") || upper == "LATIN1"
    });
    if byte_preserving {
        bytes.iter().map(|&b| b as char).collect()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

pub(crate) fn native_input_stream_reader_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let stream = extract_slot_arg(args, 1);
    heap.write_field(this_ref, INPUT_STREAM_READER_STREAM_FIELD, stream)?;
    heap.write_field(
        this_ref,
        INPUT_STREAM_READER_CHARSET_FIELD,
        Slot::Reference(None),
    )?;
    Ok(None)
}

pub(crate) fn native_input_stream_reader_init_charset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let stream = extract_slot_arg(args, 1);
    // Resolve the charset name from a Charset object (string_value) if present.
    // Read the name first (immutable borrow) before re-allocating (mutable).
    let charset_name = match extract_slot_arg(args, 2) {
        Slot::Reference(Some(charset_ref)) => heap
            .get(charset_ref)
            .ok()
            .and_then(|obj| obj.string_value.clone()),
        _ => None,
    };
    let charset_slot = charset_name.map_or(Slot::Reference(None), |name| {
        Slot::Reference(Some(heap.allocate_string(name)))
    });
    heap.write_field(this_ref, INPUT_STREAM_READER_STREAM_FIELD, stream)?;
    heap.write_field(this_ref, INPUT_STREAM_READER_CHARSET_FIELD, charset_slot)?;
    Ok(None)
}

pub(crate) fn native_input_stream_reader_init_named(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let stream = extract_slot_arg(args, 1);
    let charset_slot = extract_slot_arg(args, 2);
    heap.write_field(this_ref, INPUT_STREAM_READER_STREAM_FIELD, stream)?;
    heap.write_field(this_ref, INPUT_STREAM_READER_CHARSET_FIELD, charset_slot)?;
    Ok(None)
}

pub(crate) fn native_buffered_reader_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let reader_slot = extract_slot_arg(args, 1);

    // Extract the underlying stream + charset from the wrapped Reader. Only the
    // synthetic InputStreamReader is modeled; other readers yield empty content.
    let (stream_ref, charset_name) = match reader_slot {
        Slot::Reference(Some(reader_ref)) => {
            let reader = heap.get(reader_ref)?;
            if reader.class_name == "java/io/InputStreamReader" {
                let stream = reader.fields.get(INPUT_STREAM_READER_STREAM_FIELD).copied();
                let charset = match reader.fields.get(INPUT_STREAM_READER_CHARSET_FIELD) {
                    Some(Slot::Reference(Some(name_ref))) => {
                        charsequence_chars(heap, *name_ref).ok().flatten()
                    }
                    _ => None,
                };
                let stream_ref = match stream {
                    Some(Slot::Reference(Some(sref))) => Some(sref),
                    _ => None,
                };
                (stream_ref, charset)
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    };

    let content = match stream_ref {
        Some(sref) => {
            let bytes = input_stream_drain_all_bytes(sref, heap)?;
            decode_bytes_with_charset(&bytes, charset_name.as_deref())
        }
        None => String::new(),
    };

    let content_ref = heap.allocate_string(content);
    heap.write_field(
        this_ref,
        BUFFERED_READER_CONTENT_FIELD,
        Slot::Reference(Some(content_ref)),
    )?;
    heap.write_field(this_ref, BUFFERED_READER_CURSOR_FIELD, Slot::Int(0))?;
    Ok(None)
}

pub(crate) fn native_buffered_reader_read_line(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let content = match heap.get(this_ref)?.fields.get(BUFFERED_READER_CONTENT_FIELD) {
        Some(Slot::Reference(Some(content_ref))) => {
            string_value_from_ref(heap, *content_ref).unwrap_or_default()
        }
        _ => String::new(),
    };
    let cursor = match heap.get(this_ref)?.fields.get(BUFFERED_READER_CURSOR_FIELD) {
        Some(Slot::Int(value)) if *value >= 0 => usize::try_from(*value).unwrap_or(usize::MAX),
        _ => 0,
    };

    let chars: Vec<char> = content.chars().collect();
    if cursor >= chars.len() {
        return Ok(Some(Slot::Reference(None)));
    }

    let mut idx = cursor;
    let mut line = String::new();
    let mut terminated = false;
    while idx < chars.len() {
        let ch = chars[idx];
        if ch == '\n' {
            idx += 1;
            terminated = true;
            break;
        }
        if ch == '\r' {
            idx += 1;
            if idx < chars.len() && chars[idx] == '\n' {
                idx += 1;
            }
            terminated = true;
            break;
        }
        line.push(ch);
        idx += 1;
    }
    let _ = terminated;

    heap.write_field(
        this_ref,
        BUFFERED_READER_CURSOR_FIELD,
        Slot::Int(i32::try_from(idx).unwrap_or(i32::MAX)),
    )?;
    let line_ref = heap.allocate_string(line);
    Ok(Some(Slot::Reference(Some(line_ref))))
}

pub(crate) fn native_buffered_reader_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    // Advance the cursor to the end so any post-close read yields null; the
    // underlying stream is left as-is (resource streams are in-memory).
    let content_len = match heap.get(this_ref)?.fields.get(BUFFERED_READER_CONTENT_FIELD) {
        Some(Slot::Reference(Some(content_ref))) => {
            read_string_bytes(heap, *content_ref).map_or(0, |s| s.chars().count())
        }
        _ => 0,
    };
    heap.write_field(
        this_ref,
        BUFFERED_READER_CURSOR_FIELD,
        Slot::Int(i32::try_from(content_len).unwrap_or(i32::MAX)),
    )?;
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
    let text = string_value_from_ref(heap, string_ref).unwrap_or_else(|_| "null".to_string());
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
            let s = heap_object_to_string_ref(heap, *r)?;
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
            let s = heap_object_to_string_ref(heap, *r)?;
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