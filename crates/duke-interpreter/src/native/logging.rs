pub(crate) fn native_jul_level_int_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let level_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(jul_level_value(heap, level_ref)?)))
}

pub(crate) fn native_jul_level_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let level_ref = extract_ref_arg(args, 0)?;
    let name_ref = heap.allocate_string(jul_level_name(heap, level_ref)?);
    Ok(Some(Slot::Reference(Some(name_ref))))
}

pub(crate) fn native_jul_level_parse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let text = string_value_from_ref(heap, name_ref)?;
    if jul_level_spec_by_name(text.as_str()).is_some() {
        return Ok(Some(jul_level_slot_by_name(heap, &text)?));
    }
    if let Ok(value) = text.parse::<i32>() {
        return Ok(Some(jul_level_slot_by_value(heap, value)?));
    }
    Err(jul_illegal_argument(format!("Bad level: {text}")))
}

pub(crate) fn native_jul_logger_get_logger(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let name = string_value_from_ref(heap, name_ref)?;
    let logger_ref = jul_get_or_create_logger(heap, &name)?;
    Ok(Some(Slot::Reference(Some(logger_ref))))
}

pub(crate) fn native_jul_logger_get_logger_with_bundle(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_jul_logger_get_logger(args, heap, out, control)
}

pub(crate) fn native_jul_logger_get_global(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = jul_get_or_create_logger(heap, "global")?;
    Ok(Some(Slot::Reference(Some(logger_ref))))
}

pub(crate) fn native_jul_logger_get_anonymous_logger(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let parent_slot = jul_root_logger_slot(heap)?;
    Ok(Some(jul_create_logger(
        heap,
        None,
        Slot::Reference(None),
        true,
        parent_slot,
        &[],
    )?))
}

pub(crate) fn native_jul_logger_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    Ok(Some(
        heap.get(logger_ref)?
            .fields
            .get(JUL_LOGGER_NAME_FIELD)
            .copied()
            .unwrap_or(Slot::Reference(None)),
    ))
}

pub(crate) fn native_jul_logger_get_level(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    Ok(Some(
        heap.get(logger_ref)?
            .fields
            .get(JUL_LOGGER_LEVEL_FIELD)
            .copied()
            .unwrap_or(Slot::Reference(None)),
    ))
}

pub(crate) fn native_jul_logger_set_level(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    heap.write_field(logger_ref, JUL_LOGGER_LEVEL_FIELD, extract_slot_arg(args, 1))?;
    Ok(None)
}

pub(crate) fn native_jul_logger_is_loggable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    let level_slot = extract_slot_arg(args, 1);
    Ok(Some(Slot::Int(i32::from(jul_logger_is_loggable(
        heap, logger_ref, level_slot,
    )?))))
}

pub(crate) fn native_jul_logger_log(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    jul_log_message_slot(
        heap,
        out,
        logger_ref,
        extract_slot_arg(args, 1),
        extract_slot_arg(args, 2),
        &[],
        Slot::Reference(None),
    )?;
    Ok(None)
}

pub(crate) fn native_jul_logger_log_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    let param = extract_slot_arg(args, 3);
    jul_log_message_slot(
        heap,
        out,
        logger_ref,
        extract_slot_arg(args, 1),
        extract_slot_arg(args, 2),
        &[param],
        Slot::Reference(None),
    )?;
    Ok(None)
}

pub(crate) fn native_jul_logger_log_object_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    let params = match extract_slot_arg(args, 3) {
        Slot::Reference(Some(array_ref)) => heap.get(array_ref)?.fields.clone(),
        Slot::Reference(None) => Vec::new(),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Object[]",
                got: "other",
            });
        }
    };
    jul_log_message_slot(
        heap,
        out,
        logger_ref,
        extract_slot_arg(args, 1),
        extract_slot_arg(args, 2),
        &params,
        Slot::Reference(None),
    )?;
    Ok(None)
}

pub(crate) fn native_jul_logger_log_throwable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    jul_log_message_slot(
        heap,
        out,
        logger_ref,
        extract_slot_arg(args, 1),
        extract_slot_arg(args, 2),
        &[],
        extract_slot_arg(args, 3),
    )?;
    Ok(None)
}

pub(crate) fn native_jul_logger_severe(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    jul_log_named_level(args, heap, out, "SEVERE")
}

pub(crate) fn native_jul_logger_warning(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    jul_log_named_level(args, heap, out, "WARNING")
}

pub(crate) fn native_jul_logger_info(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    jul_log_named_level(args, heap, out, "INFO")
}

pub(crate) fn native_jul_logger_config(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    jul_log_named_level(args, heap, out, "CONFIG")
}

pub(crate) fn native_jul_logger_fine(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    jul_log_named_level(args, heap, out, "FINE")
}

pub(crate) fn native_jul_logger_finer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    jul_log_named_level(args, heap, out, "FINER")
}

pub(crate) fn native_jul_logger_finest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    jul_log_named_level(args, heap, out, "FINEST")
}

pub(crate) fn native_jul_logger_entering(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    let level_slot = jul_level_slot_by_name(heap, "FINER")?;
    if !jul_logger_is_loggable(heap, logger_ref, level_slot)? {
        return Ok(None);
    }
    let source_class = jul_slot_to_text(heap, extract_slot_arg(args, 1))?;
    let source_method = jul_slot_to_text(heap, extract_slot_arg(args, 2))?;
    jul_log_text(
        heap,
        out,
        logger_ref,
        level_slot,
        &format!("ENTRY {source_class}.{source_method}"),
        &[],
        Slot::Reference(None),
    )?;
    Ok(None)
}

pub(crate) fn native_jul_logger_exiting(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    let level_slot = jul_level_slot_by_name(heap, "FINER")?;
    if !jul_logger_is_loggable(heap, logger_ref, level_slot)? {
        return Ok(None);
    }
    let source_class = jul_slot_to_text(heap, extract_slot_arg(args, 1))?;
    let source_method = jul_slot_to_text(heap, extract_slot_arg(args, 2))?;
    jul_log_text(
        heap,
        out,
        logger_ref,
        level_slot,
        &format!("RETURN {source_class}.{source_method}"),
        &[],
        Slot::Reference(None),
    )?;
    Ok(None)
}

pub(crate) fn native_jul_logger_throwing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    let level_slot = jul_level_slot_by_name(heap, "FINER")?;
    if !jul_logger_is_loggable(heap, logger_ref, level_slot)? {
        return Ok(None);
    }
    let source_class = jul_slot_to_text(heap, extract_slot_arg(args, 1))?;
    let source_method = jul_slot_to_text(heap, extract_slot_arg(args, 2))?;
    jul_log_text(
        heap,
        out,
        logger_ref,
        level_slot,
        &format!("THROW {source_class}.{source_method}"),
        &[],
        extract_slot_arg(args, 3),
    )?;
    Ok(None)
}

pub(crate) fn native_jul_logger_add_handler(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    let handler_slot = match extract_slot_arg(args, 1) {
        Slot::Reference(Some(_)) => extract_slot_arg(args, 1),
        Slot::Reference(None) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Handler",
                got: "other",
            });
        }
    };
    let count = match heap
        .get(logger_ref)?
        .fields
        .get(JUL_LOGGER_HANDLER_COUNT_FIELD)
    {
        Some(Slot::Int(count)) => usize::try_from((*count).max(0)).unwrap_or(0),
        _ => 0,
    };
    jul_push_dynamic_field(heap, logger_ref, handler_slot)?;
    heap.write_field(
        logger_ref,
        JUL_LOGGER_HANDLER_COUNT_FIELD,
        Slot::Int(i32::try_from(count.saturating_add(1)).unwrap_or(i32::MAX)),
    )?;
    Ok(None)
}

pub(crate) fn native_jul_logger_remove_handler(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let fields = heap.get(logger_ref)?.fields.clone();
    let count = match fields.get(JUL_LOGGER_HANDLER_COUNT_FIELD) {
        Some(Slot::Int(count)) => usize::try_from((*count).max(0)).unwrap_or(0),
        _ => 0,
    };
    let mut retained = Vec::new();
    let mut removed = false;
    for idx in 0..count {
        let handler_slot = fields
            .get(JUL_LOGGER_HANDLERS_START + idx)
            .copied()
            .unwrap_or(Slot::Reference(None));
        if !removed && handler_slot == target {
            removed = true;
        } else {
            retained.push(handler_slot);
        }
    }
    heap.get_mut(logger_ref)?
        .fields
        .truncate(JUL_LOGGER_HANDLERS_START);
    for handler_slot in &retained {
        jul_push_dynamic_field(heap, logger_ref, *handler_slot)?;
    }
    heap.write_field(
        logger_ref,
        JUL_LOGGER_HANDLER_COUNT_FIELD,
        Slot::Int(i32::try_from(retained.len()).unwrap_or(i32::MAX)),
    )?;
    Ok(None)
}

pub(crate) fn native_jul_logger_get_handlers(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(logger_ref)?.fields.clone();
    let count = match fields.get(JUL_LOGGER_HANDLER_COUNT_FIELD) {
        Some(Slot::Int(count)) => usize::try_from((*count).max(0)).unwrap_or(0),
        _ => 0,
    };
    let handlers: Vec<Slot> = (0..count)
        .map(|idx| {
            fields
                .get(JUL_LOGGER_HANDLERS_START + idx)
                .copied()
                .unwrap_or(Slot::Reference(None))
        })
        .collect();
    let array_ref = allocate_reference_array_from_slots(
        heap,
        "[Ljava/util/logging/Handler;",
        &handlers,
    )?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_jul_logger_set_use_parent_handlers(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    let enabled = extract_int_arg(args, 1)?;
    heap.write_field(
        logger_ref,
        JUL_LOGGER_USE_PARENT_HANDLERS_FIELD,
        Slot::Int(i32::from(enabled != 0)),
    )?;
    Ok(None)
}

pub(crate) fn native_jul_logger_get_use_parent_handlers(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    Ok(Some(
        heap.get(logger_ref)?
            .fields
            .get(JUL_LOGGER_USE_PARENT_HANDLERS_FIELD)
            .copied()
            .unwrap_or(Slot::Int(0)),
    ))
}

pub(crate) fn native_jul_logger_get_parent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    Ok(Some(
        heap.get(logger_ref)?
            .fields
            .get(JUL_LOGGER_PARENT_FIELD)
            .copied()
            .unwrap_or(Slot::Reference(None)),
    ))
}

pub(crate) fn native_jul_log_manager_get_log_manager(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Reference(Some(jul_manager_ref(heap)?))))
}

pub(crate) fn native_jul_log_manager_get_logger(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let manager_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let name = string_value_from_ref(heap, name_ref)?;
    Ok(Some(
        jul_manager_find_logger_by_name(heap, manager_ref, &name)?
            .map_or(Slot::Reference(None), |logger_ref| {
                Slot::Reference(Some(logger_ref))
            }),
    ))
}

pub(crate) fn native_jul_log_manager_get_logger_names(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let manager_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(manager_ref)?.fields.clone();
    let count = jul_manager_count(heap, manager_ref);
    let names: Vec<Slot> = (0..count)
        .map(|idx| {
            fields
                .get(JUL_MANAGER_LOGGERS_START + idx * 2)
                .copied()
                .unwrap_or(Slot::Reference(None))
        })
        .collect();
    let enumeration_ref = heap.allocate(
        "duke/util/JulLoggerNameEnumeration".to_string(),
        JUL_ENUM_NAMES_START + names.len(),
    );
    heap.write_field(enumeration_ref, JUL_ENUM_INDEX_FIELD, Slot::Int(0))?;
    heap.write_field(
        enumeration_ref,
        JUL_ENUM_COUNT_FIELD,
        Slot::Int(i32::try_from(names.len()).unwrap_or(i32::MAX)),
    )?;
    for (idx, name_slot) in names.iter().copied().enumerate() {
        heap.write_field(enumeration_ref, JUL_ENUM_NAMES_START + idx, name_slot)?;
    }
    Ok(Some(Slot::Reference(Some(enumeration_ref))))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_jul_noop_void(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}

pub(crate) fn native_jul_log_manager_read_configuration_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 1)?;
    let stream_class = heap.get(stream_ref)?.class_name.clone();
    for _ in 0..1_048_576 {
        match ops.invoke(
            heap,
            out,
            &stream_class,
            "read",
            "()I",
            vec![Slot::Reference(Some(stream_ref))],
        ) {
            Ok(Some(Slot::Int(value))) if value < 0 => break,
            Ok(Some(Slot::Int(_))) => {}
            Ok(_) | Err(Error::MethodNotFound { .. }) => break,
            Err(err) => return Err(err),
        }
    }
    Ok(None)
}

pub(crate) fn native_jul_log_manager_add_logger(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let manager_ref = extract_ref_arg(args, 0)?;
    let logger_ref = extract_ref_arg(args, 1)?;
    let Some(name) = jul_logger_name_string(heap, logger_ref)? else {
        return Ok(Some(Slot::Int(0)));
    };
    let inserted = jul_manager_insert_logger(
        heap,
        manager_ref,
        &name,
        Slot::Reference(Some(logger_ref)),
    )?;
    Ok(Some(Slot::Int(i32::from(inserted))))
}

pub(crate) fn native_jul_handler_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let handler_ref = extract_ref_arg(args, 0)?;
    let all_level = jul_level_slot_by_name(heap, "ALL")?;
    heap.write_field(handler_ref, JUL_HANDLER_LEVEL_FIELD, all_level)?;
    heap.write_field(
        handler_ref,
        JUL_HANDLER_FORMATTER_FIELD,
        Slot::Reference(None),
    )?;
    Ok(None)
}

pub(crate) fn native_jul_console_handler_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let handler_ref = extract_ref_arg(args, 0)?;
    let all_level = jul_level_slot_by_name(heap, "ALL")?;
    let formatter = jul_create_simple_formatter(heap);
    heap.write_field(handler_ref, JUL_HANDLER_LEVEL_FIELD, all_level)?;
    heap.write_field(handler_ref, JUL_HANDLER_FORMATTER_FIELD, formatter)?;
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_jul_handler_publish(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}

pub(crate) fn native_jul_console_handler_publish(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let handler_ref = extract_ref_arg(args, 0)?;
    let record_ref = extract_ref_arg(args, 1)?;
    jul_publish_record_to_handler(heap, out, handler_ref, record_ref)?;
    Ok(None)
}

pub(crate) fn native_jul_handler_set_level(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let handler_ref = extract_ref_arg(args, 0)?;
    heap.write_field(handler_ref, JUL_HANDLER_LEVEL_FIELD, extract_slot_arg(args, 1))?;
    Ok(None)
}

pub(crate) fn native_jul_handler_get_level(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let handler_ref = extract_ref_arg(args, 0)?;
    Ok(Some(
        heap.get(handler_ref)?
            .fields
            .get(JUL_HANDLER_LEVEL_FIELD)
            .copied()
            .unwrap_or(Slot::Reference(None)),
    ))
}

pub(crate) fn native_jul_handler_set_formatter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let handler_ref = extract_ref_arg(args, 0)?;
    heap.write_field(
        handler_ref,
        JUL_HANDLER_FORMATTER_FIELD,
        extract_slot_arg(args, 1),
    )?;
    Ok(None)
}

pub(crate) fn native_jul_simple_formatter_format(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let record_ref = extract_ref_arg(args, 1)?;
    let formatted_ref = heap.allocate_string(jul_format_record(heap, record_ref)?);
    Ok(Some(Slot::Reference(Some(formatted_ref))))
}

pub(crate) fn native_jul_log_record_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let record_ref = extract_ref_arg(args, 0)?;
    heap.write_field(record_ref, JUL_LOG_RECORD_LEVEL_FIELD, extract_slot_arg(args, 1))?;
    heap.write_field(
        record_ref,
        JUL_LOG_RECORD_MESSAGE_FIELD,
        extract_slot_arg(args, 2),
    )?;
    heap.write_field(
        record_ref,
        JUL_LOG_RECORD_LOGGER_NAME_FIELD,
        Slot::Reference(None),
    )?;
    heap.write_field(record_ref, JUL_LOG_RECORD_THROWN_FIELD, Slot::Reference(None))?;
    heap.write_field(
        record_ref,
        JUL_LOG_RECORD_MILLIS_FIELD,
        Slot::Long(jul_current_millis()),
    )?;
    heap.write_field(
        record_ref,
        JUL_LOG_RECORD_PARAMETERS_FIELD,
        Slot::Reference(None),
    )?;
    Ok(None)
}

fn jul_log_record_get_field(
    args: &[Slot],
    heap: &duke_gc::Heap,
    field_idx: usize,
) -> Result<Slot> {
    let record_ref = extract_ref_arg(args, 0)?;
    Ok(heap
        .get(record_ref)?
        .fields
        .get(field_idx)
        .copied()
        .unwrap_or(Slot::Reference(None)))
}

fn jul_log_record_set_field(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    field_idx: usize,
    value_idx: usize,
) -> Result<Option<Slot>> {
    let record_ref = extract_ref_arg(args, 0)?;
    heap.write_field(record_ref, field_idx, extract_slot_arg(args, value_idx))?;
    Ok(None)
}

pub(crate) fn native_jul_log_record_get_level(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(jul_log_record_get_field(
        args,
        heap,
        JUL_LOG_RECORD_LEVEL_FIELD,
    )?))
}

pub(crate) fn native_jul_log_record_set_level(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    jul_log_record_set_field(args, heap, JUL_LOG_RECORD_LEVEL_FIELD, 1)
}

pub(crate) fn native_jul_log_record_get_message(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(jul_log_record_get_field(
        args,
        heap,
        JUL_LOG_RECORD_MESSAGE_FIELD,
    )?))
}

pub(crate) fn native_jul_log_record_set_message(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    jul_log_record_set_field(args, heap, JUL_LOG_RECORD_MESSAGE_FIELD, 1)
}

pub(crate) fn native_jul_log_record_get_logger_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(jul_log_record_get_field(
        args,
        heap,
        JUL_LOG_RECORD_LOGGER_NAME_FIELD,
    )?))
}

pub(crate) fn native_jul_log_record_set_logger_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    jul_log_record_set_field(args, heap, JUL_LOG_RECORD_LOGGER_NAME_FIELD, 1)
}

pub(crate) fn native_jul_log_record_get_thrown(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(jul_log_record_get_field(
        args,
        heap,
        JUL_LOG_RECORD_THROWN_FIELD,
    )?))
}

pub(crate) fn native_jul_log_record_set_thrown(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    jul_log_record_set_field(args, heap, JUL_LOG_RECORD_THROWN_FIELD, 1)
}

pub(crate) fn native_jul_log_record_get_millis(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let record_ref = extract_ref_arg(args, 0)?;
    Ok(Some(match heap.get(record_ref)?.fields.get(JUL_LOG_RECORD_MILLIS_FIELD) {
        Some(Slot::Long(value)) => Slot::Long(*value),
        _ => Slot::Long(0),
    }))
}

pub(crate) fn native_jul_log_record_get_parameters(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(jul_log_record_get_field(
        args,
        heap,
        JUL_LOG_RECORD_PARAMETERS_FIELD,
    )?))
}

pub(crate) fn native_jul_log_record_set_parameters(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    jul_log_record_set_field(args, heap, JUL_LOG_RECORD_PARAMETERS_FIELD, 1)
}

pub(crate) fn native_jul_logger_names_has_more_elements(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let enum_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(enum_ref)?.fields;
    let index = match fields.get(JUL_ENUM_INDEX_FIELD) {
        Some(Slot::Int(index)) => *index,
        _ => 0,
    };
    let count = match fields.get(JUL_ENUM_COUNT_FIELD) {
        Some(Slot::Int(count)) => *count,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(index < count))))
}

pub(crate) fn native_jul_logger_names_next_element(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let enum_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(enum_ref)?.fields.clone();
    let index = match fields.get(JUL_ENUM_INDEX_FIELD) {
        Some(Slot::Int(index)) => usize::try_from((*index).max(0)).unwrap_or(0),
        _ => 0,
    };
    let count = match fields.get(JUL_ENUM_COUNT_FIELD) {
        Some(Slot::Int(count)) => usize::try_from((*count).max(0)).unwrap_or(0),
        _ => 0,
    };
    if index >= count {
        return Ok(Some(Slot::Reference(None)));
    }
    heap.write_field(
        enum_ref,
        JUL_ENUM_INDEX_FIELD,
        Slot::Int(i32::try_from(index.saturating_add(1)).unwrap_or(i32::MAX)),
    )?;
    Ok(Some(
        fields
            .get(JUL_ENUM_NAMES_START + index)
            .copied()
            .unwrap_or(Slot::Reference(None)),
    ))
}

const REPLACEMENT_CHAR: char = '\u{fffd}';

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StandardCharset {
    Utf8,
    Utf16,
    Utf16Be,
    Utf16Le,
    UsAscii,
    Iso88591,
}

impl StandardCharset {
    const fn canonical_name(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16 => "UTF-16",
            Self::Utf16Be => "UTF-16BE",
            Self::Utf16Le => "UTF-16LE",
            Self::UsAscii => "US-ASCII",
            Self::Iso88591 => "ISO-8859-1",
        }
    }

    fn from_canonical(name: &str) -> Option<Self> {
        match name {
            "UTF-8" => Some(Self::Utf8),
            "UTF-16" => Some(Self::Utf16),
            "UTF-16BE" => Some(Self::Utf16Be),
            "UTF-16LE" => Some(Self::Utf16Le),
            "US-ASCII" => Some(Self::UsAscii),
            "ISO-8859-1" => Some(Self::Iso88591),
            _ => None,
        }
    }
}

const fn charset_for_name(name: &str) -> Option<StandardCharset> {
    if name.eq_ignore_ascii_case("UTF-8")
        || name.eq_ignore_ascii_case("UTF8")
        || name.eq_ignore_ascii_case("utf8")
    {
        return Some(StandardCharset::Utf8);
    }
    if name.eq_ignore_ascii_case("UTF-16") || name.eq_ignore_ascii_case("UTF16") {
        return Some(StandardCharset::Utf16);
    }
    if name.eq_ignore_ascii_case("UTF-16BE") || name.eq_ignore_ascii_case("UTF16BE") {
        return Some(StandardCharset::Utf16Be);
    }
    if name.eq_ignore_ascii_case("UTF-16LE") || name.eq_ignore_ascii_case("UTF16LE") {
        return Some(StandardCharset::Utf16Le);
    }
    if name.eq_ignore_ascii_case("US-ASCII")
        || name.eq_ignore_ascii_case("ASCII")
        || name.eq_ignore_ascii_case("US_ASCII")
    {
        return Some(StandardCharset::UsAscii);
    }
    if name.eq_ignore_ascii_case("ISO-8859-1")
        || name.eq_ignore_ascii_case("ISO8859-1")
        || name.eq_ignore_ascii_case("ISO8859_1")
        || name.eq_ignore_ascii_case("latin1")
    {
        return Some(StandardCharset::Iso88591);
    }
    None
}

pub(crate) fn allocate_standard_charset(heap: &mut duke_gc::Heap, canonical_name: &str) -> u64 {
    if let Some(existing) = heap.find_string_backed_object("java/nio/charset/Charset", canonical_name)
    {
        return existing;
    }
    let charset_ref = heap.allocate("java/nio/charset/Charset".to_string(), 0);
    if let Ok(obj) = heap.get_mut(charset_ref) {
        obj.string_value = Some(canonical_name.to_string());
    }
    charset_ref
}

fn charset_ref(heap: &mut duke_gc::Heap, charset: StandardCharset) -> u64 {
    allocate_standard_charset(heap, charset.canonical_name())
}

fn unsupported_charset_error(name: &str) -> Error {
    push_pending_java_exception_message(
        "java/nio/charset/UnsupportedCharsetException",
        name.to_string(),
    );
    Error::JavaException {
        class_name: "java/nio/charset/UnsupportedCharsetException".to_string(),
    }
}

fn unsupported_encoding_error(name: &str) -> Error {
    push_pending_java_exception_message("java/io/UnsupportedEncodingException", name.to_string());
    Error::JavaException {
        class_name: "java/io/UnsupportedEncodingException".to_string(),
    }
}

fn charset_from_name_ref(
    heap: &duke_gc::Heap,
    string_ref: u64,
    error_for_unknown: fn(&str) -> Error,
) -> Result<StandardCharset> {
    let name = string_value_from_ref(heap, string_ref)?;
    charset_for_name(&name).ok_or_else(|| error_for_unknown(&name))
}

fn charset_from_arg(
    args: &[Slot],
    idx: usize,
    heap: &duke_gc::Heap,
) -> Result<StandardCharset> {
    let charset_ref = extract_ref_arg(args, idx)?;
    let obj = heap.get(charset_ref)?;
    if obj.class_name != "java/nio/charset/Charset" {
        return Err(Error::ClassCastException {
            from: obj.class_name.clone(),
            to: "java/nio/charset/Charset".to_string(),
        });
    }
    let name = obj.string_value.as_deref().ok_or(Error::TypeMismatch {
        expected: "Charset.string_value",
        got: "None",
    })?;
    StandardCharset::from_canonical(name).ok_or_else(|| unsupported_charset_error(name))
}

fn java_string_hash(value: &str) -> i32 {
    let mut hash = 0_i32;
    for ch in value.chars() {
        hash = hash.wrapping_mul(31).wrapping_add(ch as i32);
    }
    hash
}

fn java_byte_slot(byte: u8) -> Slot {
    Slot::Int(i32::from(i8::from_ne_bytes([byte])))
}

const fn byte_from_slot(slot: Slot) -> Result<u8> {
    match slot {
        Slot::Int(value) => Ok(value.to_be_bytes()[3]),
        _ => Err(Error::TypeMismatch {
            expected: "byte",
            got: "other",
        }),
    }
}

fn allocate_byte_array(heap: &mut duke_gc::Heap, bytes: &[u8]) -> Result<u64> {
    let array_ref = heap.allocate("[B".to_string(), bytes.len());
    for (idx, byte) in bytes.iter().copied().enumerate() {
        heap.write_field(array_ref, idx, java_byte_slot(byte))?;
    }
    Ok(array_ref)
}

fn index_out_of_bounds_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/IndexOutOfBoundsException".to_string(),
    }
}

fn byte_array_window(
    heap: &duke_gc::Heap,
    array_ref: u64,
    offset: i32,
    length: i32,
) -> Result<Vec<u8>> {
    if offset < 0 || length < 0 {
        return Err(index_out_of_bounds_error());
    }
    let start = usize::try_from(offset).map_err(|_| index_out_of_bounds_error())?;
    let count = usize::try_from(length).map_err(|_| index_out_of_bounds_error())?;
    let end = start
        .checked_add(count)
        .ok_or_else(index_out_of_bounds_error)?;
    let fields = &heap.get(array_ref)?.fields;
    if end > fields.len() {
        return Err(index_out_of_bounds_error());
    }
    fields[start..end]
        .iter()
        .copied()
        .map(byte_from_slot)
        .collect()
}

fn full_byte_array(heap: &duke_gc::Heap, array_ref: u64) -> Result<Vec<u8>> {
    let length = i32::try_from(heap.get(array_ref)?.fields.len())
        .map_err(|_| index_out_of_bounds_error())?;
    byte_array_window(heap, array_ref, 0, length)
}

fn encode_string_with_charset(value: &str, charset: StandardCharset) -> Vec<u8> {
    match charset {
        StandardCharset::Utf8 => value.as_bytes().to_vec(),
        StandardCharset::UsAscii => value
            .chars()
            .map(|ch| {
                if ch <= '\u{7f}' {
                    ch as u8
                } else {
                    b'?'
                }
            })
            .collect(),
        StandardCharset::Iso88591 => value
            .chars()
            .map(|ch| {
                if u32::from(ch) <= 0xff {
                    ch as u8
                } else {
                    b'?'
                }
            })
            .collect(),
        StandardCharset::Utf16 => {
            let mut bytes = Vec::with_capacity(2 + value.len().saturating_mul(2));
            bytes.extend_from_slice(&[0xfe, 0xff]);
            for unit in value.encode_utf16() {
                bytes.extend_from_slice(&unit.to_be_bytes());
            }
            bytes
        }
        StandardCharset::Utf16Be => {
            let mut bytes = Vec::with_capacity(value.len().saturating_mul(2));
            for unit in value.encode_utf16() {
                bytes.extend_from_slice(&unit.to_be_bytes());
            }
            bytes
        }
        StandardCharset::Utf16Le => {
            let mut bytes = Vec::with_capacity(value.len().saturating_mul(2));
            for unit in value.encode_utf16() {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }
            bytes
        }
    }
}

#[derive(Clone, Copy)]
enum Utf16Endian {
    Big,
    Little,
}

fn decode_utf16_units(units: Vec<u16>, has_trailing_byte: bool) -> String {
    let mut decoded: String = char::decode_utf16(units)
        .map(|item| item.unwrap_or(REPLACEMENT_CHAR))
        .collect();
    if has_trailing_byte {
        decoded.push(REPLACEMENT_CHAR);
    }
    decoded
}

fn decode_utf16_bytes(bytes: &[u8], endian: Utf16Endian) -> String {
    let mut chunks = bytes.chunks_exact(2);
    let mut units = Vec::with_capacity(bytes.len() / 2);
    for chunk in &mut chunks {
        let pair = [chunk[0], chunk[1]];
        let unit = match endian {
            Utf16Endian::Big => u16::from_be_bytes(pair),
            Utf16Endian::Little => u16::from_le_bytes(pair),
        };
        units.push(unit);
    }
    decode_utf16_units(units, !chunks.remainder().is_empty())
}

fn decode_string_with_charset(bytes: &[u8], charset: StandardCharset) -> String {
    match charset {
        StandardCharset::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        StandardCharset::UsAscii => bytes
            .iter()
            .map(|byte| {
                if *byte <= 0x7f {
                    char::from(*byte)
                } else {
                    REPLACEMENT_CHAR
                }
            })
            .collect(),
        StandardCharset::Iso88591 => bytes.iter().map(|byte| char::from(*byte)).collect(),
        StandardCharset::Utf16 => match (
            bytes.strip_prefix(&[0xfe, 0xff]),
            bytes.strip_prefix(&[0xff, 0xfe]),
        ) {
            (Some(rest), _) => decode_utf16_bytes(rest, Utf16Endian::Big),
            (None, Some(rest)) => decode_utf16_bytes(rest, Utf16Endian::Little),
            (None, None) => decode_utf16_bytes(bytes, Utf16Endian::Big),
        },
        StandardCharset::Utf16Be => decode_utf16_bytes(bytes, Utf16Endian::Big),
        StandardCharset::Utf16Le => decode_utf16_bytes(bytes, Utf16Endian::Little),
    }
}

fn string_bytes_for_arg(
    args: &[Slot],
    heap: &duke_gc::Heap,
    charset: StandardCharset,
) -> Result<Vec<u8>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = heap
        .get(this_ref)?
        .string_value
        .as_deref()
        .unwrap_or_default();
    Ok(encode_string_with_charset(value, charset))
}

fn init_string_from_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    bytes: &[u8],
    charset: StandardCharset,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let decoded = decode_string_with_charset(bytes, charset);
    heap.get_mut(this_ref)?.string_value = Some(decoded);
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
