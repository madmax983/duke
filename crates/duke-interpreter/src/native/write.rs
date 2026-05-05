fn write_to_print_stream_or_output(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    text: &str,
) -> Result<()> {
    let print_stream_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    if let Some(print_stream_ref) = print_stream_slot.as_reference() {
        let target_slot = heap
            .get(print_stream_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        if let Some(target_ref) = target_slot.as_reference()
            && heap.get(target_ref)?.class_name == "java/io/ByteArrayOutputStream"
        {
            let target = heap.get_mut(target_ref)?;
            target.string_value.get_or_insert_with(String::new).push_str(text);
            return Ok(());
        }
    }
    write!(out, "{text}").ok();
    Ok(())
}
fn write_lock_try_acquire(
    state: &mut duke_gc::ReadWriteLockState,
    thread_id: std::thread::ThreadId,
) -> bool {
    if state.writer == Some(thread_id) {
        state.write_hold_count = state.write_hold_count.saturating_add(1);
        return true;
    }
    if state.writer.is_some() || !state.readers.is_empty() {
        return false;
    }
    state.writer = Some(thread_id);
    state.write_hold_count = 1;
    true
}
pub(crate) fn native_write_lock_lock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_view_state(
        heap,
        this_ref,
        duke_gc::ReadWriteLockViewKind::Write,
    )?;
    let thread_id = current_host_thread_id();
    let mut guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if !write_lock_try_acquire(&mut guard, thread_id) {
        request_native_retry(control);
    }
    Ok(None)
}
pub(crate) fn native_write_lock_try_lock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_view_state(
        heap,
        this_ref,
        duke_gc::ReadWriteLockViewKind::Write,
    )?;
    let thread_id = current_host_thread_id();
    let mut guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(Some(Slot::Int(i32::from(write_lock_try_acquire(&mut guard, thread_id)))))
}
pub(crate) fn native_write_lock_unlock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_view_state(
        heap,
        this_ref,
        duke_gc::ReadWriteLockViewKind::Write,
    )?;
    let thread_id = current_host_thread_id();
    let mut guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if guard.writer != Some(thread_id) || guard.write_hold_count <= 0 {
        return Err(illegal_monitor_state_error());
    }
    guard.write_hold_count -= 1;
    if guard.write_hold_count == 0 {
        guard.writer = None;
    }
    drop(guard);
    Ok(None)
}
pub(crate) fn native_write_lock_new_condition(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Err(unsupported_operation_error())
}
fn write_message_digest_buffer(
    heap: &mut duke_gc::Heap,
    digest_ref: u64,
    bytes: &[u8],
) -> Result<()> {
    let buffer_slot = if bytes.is_empty() {
        Slot::Reference(None)
    } else {
        Slot::Reference(Some(alloc_byte_array(heap, bytes)))
    };
    let digest = heap.get_mut(digest_ref)?;
    if digest.fields.len() <= MESSAGE_DIGEST_BUFFER_FIELD {
        digest.fields.resize(MESSAGE_DIGEST_BUFFER_FIELD + 1, Slot::Reference(None));
    }
    digest.fields[MESSAGE_DIGEST_BUFFER_FIELD] = buffer_slot;
    Ok(())
}
fn write_uuid_bits(
    heap: &mut duke_gc::Heap,
    uuid_ref: u64,
    msb: i64,
    lsb: i64,
) -> Result<()> {
    let uuid = heap.get_mut(uuid_ref)?;
    if uuid.fields.len() <= UUID_LSB_FIELD {
        uuid.fields.resize(UUID_LSB_FIELD + 1, Slot::Long(0));
    }
    uuid.fields[UUID_MSB_FIELD] = Slot::Long(msb);
    uuid.fields[UUID_LSB_FIELD] = Slot::Long(lsb);
    Ok(())
}
fn write_iso_8859_1_ascii(
    heap: &mut duke_gc::Heap,
    file_id: i32,
    text: &str,
) -> Result<()> {
    for byte in text.bytes() {
        heap.write_host_file_byte(file_id, i32::from(byte))?;
    }
    Ok(())
}
