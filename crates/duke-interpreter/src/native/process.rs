fn process_field_id_from_this(
    args: &[Slot],
    heap: &duke_gc::Heap,
    field_idx: usize,
) -> Result<i32> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.get(field_idx) {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => {
            Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            })
        }
    }
}
pub(crate) fn native_process_builder_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let command_slot = extract_slot_arg(args, 1);
    let builder_obj = heap.get_mut(this_ref)?;
    if builder_obj.fields.len() < 2 {
        return Err(Error::InvalidRef {
            address: this_ref,
        });
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let directory_slot = extract_slot_arg(args, 1);
    let builder_obj = heap.get_mut(this_ref)?;
    if builder_obj.fields.len() < 2 {
        return Err(Error::InvalidRef {
            address: this_ref,
        });
    }
    builder_obj.fields[1] = directory_slot;
    Ok(Some(Slot::Reference(Some(this_ref))))
}
pub(crate) fn native_process_builder_start(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
pub(crate) fn native_process_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stdout_id = process_field_id_from_this(args, heap, PROCESS_STDOUT_FIELD)?;
    allocate_process_stream(heap, "duke/process/ProcessInputStream", stdout_id)
}
pub(crate) fn native_process_get_error_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stderr_id = process_field_id_from_this(args, heap, PROCESS_STDERR_FIELD)?;
    allocate_process_stream(heap, "duke/process/ProcessErrorStream", stderr_id)
}
pub(crate) fn native_process_get_output_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stdin_id = process_field_id_from_this(args, heap, PROCESS_STDIN_FIELD)?;
    allocate_process_stream(heap, "duke/process/ProcessOutputStream", stdin_id)
}
pub(crate) fn native_process_wait_for(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    Ok(Some(Slot::Int(heap.wait_host_process(process_id)?)))
}
pub(crate) fn native_process_exit_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    let Some(exit_code) = heap.try_host_process_exit_value(process_id)? else {
        return Err(Error::JavaException {
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
) -> Result<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    heap.destroy_host_process(process_id)?;
    Ok(None)
}
