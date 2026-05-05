fn resource_lookup_trace_enabled() -> bool {
    std::env::var("RUST_LOG")
        .is_ok_and(|value| {
            let lower = value.to_ascii_lowercase();
            lower.contains("debug") || lower.contains("trace")
        })
}
fn resource_stream_array_ref(heap: &duke_gc::Heap, stream_ref: u64) -> Result<u64> {
    match heap.get(stream_ref)?.fields.get(RESOURCE_STREAM_BYTES_FIELD) {
        Some(Slot::Reference(Some(array_ref))) => Ok(*array_ref),
        _ => {
            Err(Error::InvalidRef {
                address: stream_ref,
            })
        }
    }
}
fn resource_stream_is_closed(heap: &duke_gc::Heap, stream_ref: u64) -> Result<bool> {
    Ok(
        matches!(
            heap.get(stream_ref) ?.fields.get(RESOURCE_STREAM_CLOSED_FIELD),
            Some(Slot::Int(value)) if * value != 0
        ),
    )
}
fn resource_stream_ensure_open(heap: &duke_gc::Heap, stream_ref: u64) -> Result<()> {
    if resource_stream_is_closed(heap, stream_ref)? {
        push_pending_java_exception_message(
            "java/io/IOException",
            "Stream closed".to_string(),
        );
        return Err(Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        });
    }
    Ok(())
}
fn resource_stream_cursor(heap: &duke_gc::Heap, stream_ref: u64) -> Result<usize> {
    match heap.get(stream_ref)?.fields.get(RESOURCE_STREAM_CURSOR_FIELD) {
        Some(Slot::Int(value)) if *value >= 0 => {
            Ok(usize::try_from(*value).unwrap_or(usize::MAX))
        }
        _ => Ok(0),
    }
}
fn resource_stream_set_cursor(
    heap: &mut duke_gc::Heap,
    stream_ref: u64,
    cursor: usize,
) -> Result<()> {
    heap.write_field(
        stream_ref,
        RESOURCE_STREAM_CURSOR_FIELD,
        Slot::Int(i32::try_from(cursor).unwrap_or(i32::MAX)),
    )
}
fn resource_stream_available_bytes(
    heap: &duke_gc::Heap,
    stream_ref: u64,
) -> Result<usize> {
    let array_ref = resource_stream_array_ref(heap, stream_ref)?;
    let len = heap.get(array_ref)?.fields.len();
    Ok(len.saturating_sub(resource_stream_cursor(heap, stream_ref)?))
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
        target
            .fields[offset..offset + read_len]
            .copy_from_slice(&source[cursor..cursor + read_len]);
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
pub(crate) fn native_resource_enumeration_has_more_elements(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let index = match fields.get(RESOURCE_ENUM_INDEX_FIELD) {
        Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
        _ => 0,
    };
    let count = match fields.get(RESOURCE_ENUM_COUNT_FIELD) {
        Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(index < count))))
}
pub(crate) fn native_resource_enumeration_next_element(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (index, count, slot) = {
        let enumeration = heap.get(this_ref)?;
        let index = match enumeration.fields.get(RESOURCE_ENUM_INDEX_FIELD) {
            Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
            _ => 0,
        };
        let count = match enumeration.fields.get(RESOURCE_ENUM_COUNT_FIELD) {
            Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
            _ => 0,
        };
        let slot = enumeration
            .fields
            .get(RESOURCE_ENUM_VALUES_START + index)
            .copied()
            .unwrap_or(Slot::Reference(None));
        (index, count, slot)
    };
    if index >= count {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    heap.write_field(
        this_ref,
        RESOURCE_ENUM_INDEX_FIELD,
        Slot::Int(i32::try_from(index + 1).unwrap_or(i32::MAX)),
    )?;
    Ok(Some(slot))
}
