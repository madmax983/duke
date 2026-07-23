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
pub(crate) fn native_jul_logger_logp(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // logp(Level, sourceClass, sourceMethod, msg): the source class/method are
    // LogRecord metadata in real JUL; Duke's formatter renders the message, so
    // log it at the given level like `log(Level, String)`. args: 0=logger,
    // 1=level, 2=sourceClass, 3=sourceMethod, 4=msg.
    let logger_ref = extract_ref_arg(args, 0)?;
    jul_log_message_slot(
        heap,
        out,
        logger_ref,
        extract_slot_arg(args, 1),
        extract_slot_arg(args, 4),
        &[],
        Slot::Reference(None),
    )?;
    Ok(None)
}
pub(crate) fn native_jul_logger_logp_throwable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // logp(Level, sourceClass, sourceMethod, msg, Throwable): as `logp` above,
    // carrying the thrown reference through. args: 0=logger, 1=level,
    // 2=sourceClass, 3=sourceMethod, 4=msg, 5=thrown.
    let logger_ref = extract_ref_arg(args, 0)?;
    jul_log_message_slot(
        heap,
        out,
        logger_ref,
        extract_slot_arg(args, 1),
        extract_slot_arg(args, 4),
        &[],
        extract_slot_arg(args, 5),
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
    let retained = {
        let fields = &heap.get(logger_ref)?.fields;
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
        retained
    };
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
    let handlers: Vec<Slot> = {
        let fields = &heap.get(logger_ref)?.fields;
        let count = match fields.get(JUL_LOGGER_HANDLER_COUNT_FIELD) {
            Some(Slot::Int(count)) => usize::try_from((*count).max(0)).unwrap_or(0),
            _ => 0,
        };
        (0..count)
            .map(|idx| {
                fields
                    .get(JUL_LOGGER_HANDLERS_START + idx)
                    .copied()
                    .unwrap_or(Slot::Reference(None))
            })
            .collect()
    };
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
    let names: Vec<Slot> = {
        let count = jul_manager_count(heap, manager_ref);
        let fields = &heap.get(manager_ref)?.fields;
        (0..count)
            .map(|idx| {
                fields
                    .get(JUL_MANAGER_LOGGERS_START + idx * 2)
                    .copied()
                    .unwrap_or(Slot::Reference(None))
            })
            .collect()
    };
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
    let mut stream_ref = extract_ref_arg(args, 1)?;
    let stream_class = heap.get(stream_ref)?.class_name.clone();
    // Pin the stream across the read loop: each `read()` callback may trigger GC,
    // and `stream_ref` is re-dereferenced (re-passed) on the next iteration. The
    // pin keeps it alive and forwarded in place on every collection, so it stays
    // valid across ANY number of GCs.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut stream_ref);
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
    drop(scope);
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