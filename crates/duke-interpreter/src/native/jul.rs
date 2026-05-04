const JUL_LEVEL_VALUE_FIELD: usize = 0;

const JUL_LOGGER_NAME_FIELD: usize = 0;
const JUL_LOGGER_LEVEL_FIELD: usize = 1;
const JUL_LOGGER_USE_PARENT_HANDLERS_FIELD: usize = 2;
const JUL_LOGGER_PARENT_FIELD: usize = 3;
const JUL_LOGGER_HANDLER_COUNT_FIELD: usize = 4;
const JUL_LOGGER_HANDLERS_START: usize = 5;

const JUL_HANDLER_LEVEL_FIELD: usize = 0;
const JUL_HANDLER_FORMATTER_FIELD: usize = 1;

const JUL_MANAGER_ROOT_LOGGER_FIELD: usize = 0;
const JUL_MANAGER_LOGGER_COUNT_FIELD: usize = 1;
const JUL_MANAGER_LOGGERS_START: usize = 2;

const JUL_LOG_RECORD_LEVEL_FIELD: usize = 0;
const JUL_LOG_RECORD_MESSAGE_FIELD: usize = 1;
const JUL_LOG_RECORD_LOGGER_NAME_FIELD: usize = 2;
const JUL_LOG_RECORD_THROWN_FIELD: usize = 3;
const JUL_LOG_RECORD_MILLIS_FIELD: usize = 4;
const JUL_LOG_RECORD_PARAMETERS_FIELD: usize = 5;

const JUL_ENUM_INDEX_FIELD: usize = 0;
const JUL_ENUM_COUNT_FIELD: usize = 1;
const JUL_ENUM_NAMES_START: usize = 2;

const JUL_LEVELS: [(&str, i32); 9] = [
    ("SEVERE", 1000),
    ("WARNING", 900),
    ("INFO", 800),
    ("CONFIG", 700),
    ("FINE", 500),
    ("FINER", 400),
    ("FINEST", 300),
    ("ALL", i32::MIN),
    ("OFF", i32::MAX),
];

fn jul_level_spec_by_name(name: &str) -> Option<i32> {
    JUL_LEVELS
        .iter()
        .find_map(|(level_name, value)| (*level_name == name).then_some(*value))
}

fn jul_level_name_by_value(value: i32) -> Option<&'static str> {
    JUL_LEVELS
        .iter()
        .find_map(|(level_name, level_value)| (*level_value == value).then_some(*level_name))
}

fn jul_illegal_argument(message: String) -> Error {
    push_pending_java_exception_message("java/lang/IllegalArgumentException", message);
    Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    }
}

fn jul_allocate_level(heap: &mut duke_gc::Heap, name: &str, value: i32) -> Result<u64> {
    let level_ref = heap.allocate("java/util/logging/Level".to_string(), 1);
    heap.write_field(level_ref, JUL_LEVEL_VALUE_FIELD, Slot::Int(value))?;
    heap.get_mut(level_ref)?.string_value = Some(name.to_string());
    Ok(level_ref)
}

fn jul_level_ref_by_name(heap: &duke_gc::Heap, name: &str) -> Option<u64> {
    heap.find_string_backed_object("java/util/logging/Level", name)
}

fn jul_level_ref_by_value(heap: &duke_gc::Heap, value: i32) -> Option<u64> {
    let canonical_name = jul_level_name_by_value(value)?;
    jul_level_ref_by_name(heap, canonical_name)
}

fn jul_level_slot_by_name(heap: &mut duke_gc::Heap, name: &str) -> Result<Slot> {
    if let Some(level_ref) = jul_level_ref_by_name(heap, name) {
        return Ok(Slot::Reference(Some(level_ref)));
    }
    let Some(value) = jul_level_spec_by_name(name) else {
        return Err(jul_illegal_argument(format!("Bad level: {name}")));
    };
    Ok(Slot::Reference(Some(jul_allocate_level(heap, name, value)?)))
}

fn jul_level_slot_by_value(heap: &mut duke_gc::Heap, value: i32) -> Result<Slot> {
    if let Some(level_ref) = jul_level_ref_by_value(heap, value) {
        return Ok(Slot::Reference(Some(level_ref)));
    }
    let name = value.to_string();
    Ok(Slot::Reference(Some(jul_allocate_level(heap, &name, value)?)))
}

fn jul_level_value(heap: &duke_gc::Heap, level_ref: u64) -> Result<i32> {
    match heap.get(level_ref)?.fields.get(JUL_LEVEL_VALUE_FIELD) {
        Some(Slot::Int(value)) => Ok(*value),
        _ => Err(Error::InvalidRef { address: level_ref }),
    }
}

fn jul_level_value_from_slot(heap: &duke_gc::Heap, level_slot: Slot) -> Result<i32> {
    match level_slot {
        Slot::Reference(Some(level_ref)) => jul_level_value(heap, level_ref),
        Slot::Reference(None) => Err(Error::NullPointerException),
        _ => Err(Error::TypeMismatch {
            expected: "Level",
            got: "other",
        }),
    }
}

fn jul_level_name(heap: &duke_gc::Heap, level_ref: u64) -> Result<String> {
    if let Some(name) = heap.get(level_ref)?.string_value.clone() {
        return Ok(name);
    }
    Ok(jul_level_value(heap, level_ref)?.to_string())
}

fn jul_level_name_from_slot(heap: &duke_gc::Heap, level_slot: Slot) -> Result<String> {
    match level_slot {
        Slot::Reference(Some(level_ref)) => jul_level_name(heap, level_ref),
        Slot::Reference(None) => Ok("null".to_string()),
        _ => Ok(jul_level_value_from_slot(heap, level_slot)?.to_string()),
    }
}

fn jul_push_dynamic_field(heap: &mut duke_gc::Heap, obj_ref: u64, value: Slot) -> Result<()> {
    let field_idx = {
        let obj = heap.get_mut(obj_ref)?;
        let field_idx = obj.fields.len();
        obj.fields.push(Slot::Reference(None));
        field_idx
    };
    heap.write_field(obj_ref, field_idx, value)
}

fn jul_create_simple_formatter(heap: &mut duke_gc::Heap) -> Slot {
    Slot::Reference(Some(heap.allocate(
        "java/util/logging/SimpleFormatter".to_string(),
        0,
    )))
}

fn jul_create_console_handler(heap: &mut duke_gc::Heap) -> Result<Slot> {
    let handler_ref = heap.allocate("java/util/logging/ConsoleHandler".to_string(), 2);
    let all_level = jul_level_slot_by_name(heap, "ALL")?;
    let formatter = jul_create_simple_formatter(heap);
    heap.write_field(handler_ref, JUL_HANDLER_LEVEL_FIELD, all_level)?;
    heap.write_field(handler_ref, JUL_HANDLER_FORMATTER_FIELD, formatter)?;
    Ok(Slot::Reference(Some(handler_ref)))
}

fn jul_create_logger(
    heap: &mut duke_gc::Heap,
    name: Option<&str>,
    level_slot: Slot,
    use_parent_handlers: bool,
    parent_slot: Slot,
    handlers: &[Slot],
) -> Result<Slot> {
    let logger_ref = heap.allocate(
        "java/util/logging/Logger".to_string(),
        JUL_LOGGER_HANDLERS_START + handlers.len(),
    );
    let name_slot = name.map_or(Slot::Reference(None), |logger_name| {
        Slot::Reference(Some(heap.allocate_string(logger_name.to_string())))
    });
    heap.write_field(logger_ref, JUL_LOGGER_NAME_FIELD, name_slot)?;
    heap.write_field(logger_ref, JUL_LOGGER_LEVEL_FIELD, level_slot)?;
    heap.write_field(
        logger_ref,
        JUL_LOGGER_USE_PARENT_HANDLERS_FIELD,
        Slot::Int(i32::from(use_parent_handlers)),
    )?;
    heap.write_field(logger_ref, JUL_LOGGER_PARENT_FIELD, parent_slot)?;
    heap.write_field(
        logger_ref,
        JUL_LOGGER_HANDLER_COUNT_FIELD,
        Slot::Int(i32::try_from(handlers.len()).unwrap_or(i32::MAX)),
    )?;
    for (idx, handler) in handlers.iter().copied().enumerate() {
        heap.write_field(logger_ref, JUL_LOGGER_HANDLERS_START + idx, handler)?;
    }
    Ok(Slot::Reference(Some(logger_ref)))
}

fn jul_bootstrap_fallback_manager(heap: &mut duke_gc::Heap) -> Result<u64> {
    let info_level = jul_level_slot_by_name(heap, "INFO")?;
    let root_handler = jul_create_console_handler(heap)?;
    let root_logger = jul_create_logger(
        heap,
        Some(""),
        info_level,
        false,
        Slot::Reference(None),
        &[root_handler],
    )?;
    let global_logger = jul_create_logger(
        heap,
        Some("global"),
        Slot::Reference(None),
        true,
        root_logger,
        &[],
    )?;
    let manager_ref = heap.allocate("java/util/logging/LogManager".to_string(), 2);
    heap.get_mut(manager_ref)?.string_value = Some("singleton".to_string());
    heap.write_field(manager_ref, JUL_MANAGER_ROOT_LOGGER_FIELD, root_logger)?;
    heap.write_field(manager_ref, JUL_MANAGER_LOGGER_COUNT_FIELD, Slot::Int(0))?;
    jul_manager_insert_logger(heap, manager_ref, "", root_logger)?;
    jul_manager_insert_logger(heap, manager_ref, "global", global_logger)?;
    Ok(manager_ref)
}

fn jul_manager_ref(heap: &mut duke_gc::Heap) -> Result<u64> {
    if let Some(manager_ref) =
        heap.find_string_backed_object("java/util/logging/LogManager", "singleton")
    {
        return Ok(manager_ref);
    }
    jul_bootstrap_fallback_manager(heap)
}

fn jul_manager_count(heap: &duke_gc::Heap, manager_ref: u64) -> usize {
    match heap
        .get(manager_ref)
        .ok()
        .and_then(|manager| manager.fields.get(JUL_MANAGER_LOGGER_COUNT_FIELD).copied())
    {
        Some(Slot::Int(count)) => usize::try_from(count.max(0)).unwrap_or(0),
        _ => 0,
    }
}

fn jul_manager_find_logger_by_name(
    heap: &duke_gc::Heap,
    manager_ref: u64,
    name: &str,
) -> Result<Option<u64>> {
    let fields = heap.get(manager_ref)?.fields.clone();
    let count = jul_manager_count(heap, manager_ref);
    for idx in 0..count {
        let name_idx = JUL_MANAGER_LOGGERS_START + idx * 2;
        let logger_idx = name_idx + 1;
        let Some(Slot::Reference(Some(name_ref))) = fields.get(name_idx).copied() else {
            continue;
        };
        if string_value_from_ref(heap, name_ref)? == name
            && let Some(Slot::Reference(Some(logger_ref))) = fields.get(logger_idx).copied()
        {
            return Ok(Some(logger_ref));
        }
    }
    Ok(None)
}

fn jul_manager_insert_logger(
    heap: &mut duke_gc::Heap,
    manager_ref: u64,
    name: &str,
    logger_slot: Slot,
) -> Result<bool> {
    if jul_manager_find_logger_by_name(heap, manager_ref, name)?.is_some() {
        return Ok(false);
    }
    let count = jul_manager_count(heap, manager_ref);
    let name_ref = heap.allocate_string(name.to_string());
    jul_push_dynamic_field(heap, manager_ref, Slot::Reference(Some(name_ref)))?;
    jul_push_dynamic_field(heap, manager_ref, logger_slot)?;
    heap.write_field(
        manager_ref,
        JUL_MANAGER_LOGGER_COUNT_FIELD,
        Slot::Int(i32::try_from(count.saturating_add(1)).unwrap_or(i32::MAX)),
    )?;
    Ok(true)
}

fn jul_root_logger_slot(heap: &mut duke_gc::Heap) -> Result<Slot> {
    let manager_ref = jul_manager_ref(heap)?;
    Ok(heap
        .get(manager_ref)?
        .fields
        .get(JUL_MANAGER_ROOT_LOGGER_FIELD)
        .copied()
        .unwrap_or(Slot::Reference(None)))
}

fn jul_get_or_create_logger(heap: &mut duke_gc::Heap, name: &str) -> Result<u64> {
    let manager_ref = jul_manager_ref(heap)?;
    if let Some(logger_ref) = jul_manager_find_logger_by_name(heap, manager_ref, name)? {
        return Ok(logger_ref);
    }
    let parent_slot = jul_root_logger_slot(heap)?;
    let logger_slot = jul_create_logger(
        heap,
        Some(name),
        Slot::Reference(None),
        true,
        parent_slot,
        &[],
    )?;
    jul_manager_insert_logger(heap, manager_ref, name, logger_slot)?;
    match logger_slot {
        Slot::Reference(Some(logger_ref)) => Ok(logger_ref),
        _ => Err(Error::InvalidRef { address: 0 }),
    }
}

fn jul_logger_name_string(heap: &duke_gc::Heap, logger_ref: u64) -> Result<Option<String>> {
    match heap
        .get(logger_ref)?
        .fields
        .get(JUL_LOGGER_NAME_FIELD)
        .copied()
        .unwrap_or(Slot::Reference(None))
    {
        Slot::Reference(Some(name_ref)) => Ok(Some(string_value_from_ref(heap, name_ref)?)),
        Slot::Reference(None) => Ok(None),
        _ => Err(Error::TypeMismatch {
            expected: "String",
            got: "other",
        }),
    }
}

fn jul_effective_level_value(heap: &duke_gc::Heap, logger_ref: u64) -> Result<i32> {
    let mut current = Some(logger_ref);
    let mut depth = 0usize;
    while let Some(current_ref) = current {
        if depth > 64 {
            return Ok(800);
        }
        let logger = heap.get(current_ref)?;
        let level_slot = logger
            .fields
            .get(JUL_LOGGER_LEVEL_FIELD)
            .copied()
            .unwrap_or(Slot::Reference(None));
        if !matches!(level_slot, Slot::Reference(None)) {
            return jul_level_value_from_slot(heap, level_slot);
        }
        current = match logger
            .fields
            .get(JUL_LOGGER_PARENT_FIELD)
            .copied()
            .unwrap_or(Slot::Reference(None))
        {
            Slot::Reference(Some(parent_ref)) => Some(parent_ref),
            _ => None,
        };
        depth = depth.saturating_add(1);
    }
    Ok(800)
}

const fn jul_is_loggable_for_value(effective: i32, candidate: i32) -> bool {
    effective != i32::MAX && candidate != i32::MAX && candidate >= effective
}

fn jul_logger_is_loggable(heap: &duke_gc::Heap, logger_ref: u64, level_slot: Slot) -> Result<bool> {
    let candidate = jul_level_value_from_slot(heap, level_slot)?;
    let effective = jul_effective_level_value(heap, logger_ref)?;
    Ok(jul_is_loggable_for_value(effective, candidate))
}

fn jul_slot_to_text(heap: &duke_gc::Heap, slot: Slot) -> Result<String> {
    match slot {
        Slot::Reference(Some(obj_ref)) => Ok(heap_object_to_string(heap.get(obj_ref)?, obj_ref)),
        Slot::Reference(None) => Ok("null".to_string()),
        Slot::Int(value) => Ok(value.to_string()),
        Slot::Long(value) => Ok(value.to_string()),
        Slot::Float(value) => Ok(format_java_float(value)),
        Slot::Double(value) => Ok(format_java_double(value)),
        Slot::ReturnAddress(value) => Ok(value.to_string()),
    }
}

fn jul_apply_parameters(heap: &duke_gc::Heap, message: &str, params: &[Slot]) -> Result<String> {
    let mut rendered = message.to_string();
    for (idx, param) in params.iter().copied().enumerate() {
        let token = format!("{{{idx}}}");
        if rendered.contains(&token) {
            let value = jul_slot_to_text(heap, param)?;
            rendered = rendered.replace(&token, &value);
        }
    }
    Ok(rendered)
}

fn jul_current_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

fn jul_allocate_record(
    heap: &mut duke_gc::Heap,
    logger_ref: u64,
    level_slot: Slot,
    message: &str,
    thrown_slot: Slot,
    params: &[Slot],
) -> Result<u64> {
    let record_ref = heap.allocate("java/util/logging/LogRecord".to_string(), 6);
    let message_ref = heap.allocate_string(message.to_string());
    let logger_name_slot = heap
        .get(logger_ref)?
        .fields
        .get(JUL_LOGGER_NAME_FIELD)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let parameters_slot = if params.is_empty() {
        Slot::Reference(None)
    } else {
        let array_ref = allocate_reference_array_from_slots(heap, "[Ljava/lang/Object;", params)?;
        Slot::Reference(Some(array_ref))
    };
    heap.write_field(record_ref, JUL_LOG_RECORD_LEVEL_FIELD, level_slot)?;
    heap.write_field(
        record_ref,
        JUL_LOG_RECORD_MESSAGE_FIELD,
        Slot::Reference(Some(message_ref)),
    )?;
    heap.write_field(record_ref, JUL_LOG_RECORD_LOGGER_NAME_FIELD, logger_name_slot)?;
    heap.write_field(record_ref, JUL_LOG_RECORD_THROWN_FIELD, thrown_slot)?;
    heap.write_field(
        record_ref,
        JUL_LOG_RECORD_MILLIS_FIELD,
        Slot::Long(jul_current_millis()),
    )?;
    heap.write_field(record_ref, JUL_LOG_RECORD_PARAMETERS_FIELD, parameters_slot)?;
    Ok(record_ref)
}

fn jul_record_level_slot(heap: &duke_gc::Heap, record_ref: u64) -> Slot {
    heap.get(record_ref)
        .ok()
        .and_then(|record| record.fields.get(JUL_LOG_RECORD_LEVEL_FIELD).copied())
        .unwrap_or(Slot::Reference(None))
}

fn jul_record_message(heap: &duke_gc::Heap, record_ref: u64) -> Result<String> {
    match heap
        .get(record_ref)?
        .fields
        .get(JUL_LOG_RECORD_MESSAGE_FIELD)
        .copied()
        .unwrap_or(Slot::Reference(None))
    {
        Slot::Reference(Some(message_ref)) => string_value_from_ref(heap, message_ref),
        Slot::Reference(None) => Ok(String::new()),
        _ => Err(Error::TypeMismatch {
            expected: "String",
            got: "other",
        }),
    }
}

fn jul_format_record(heap: &duke_gc::Heap, record_ref: u64) -> Result<String> {
    let level_name = jul_level_name_from_slot(heap, jul_record_level_slot(heap, record_ref))?;
    let message = jul_record_message(heap, record_ref)?;
    Ok(format!("{level_name}: {message}"))
}

fn jul_handler_allows_record(
    heap: &duke_gc::Heap,
    handler_ref: u64,
    record_ref: u64,
) -> Result<bool> {
    let handler_level = heap
        .get(handler_ref)?
        .fields
        .get(JUL_HANDLER_LEVEL_FIELD)
        .copied()
        .unwrap_or(Slot::Reference(None));
    if matches!(handler_level, Slot::Reference(None)) {
        return Ok(true);
    }
    let handler_value = jul_level_value_from_slot(heap, handler_level)?;
    let record_value = jul_level_value_from_slot(heap, jul_record_level_slot(heap, record_ref))?;
    Ok(jul_is_loggable_for_value(handler_value, record_value))
}

fn jul_publish_record_to_handler(
    heap: &duke_gc::Heap,
    out: &mut dyn Write,
    handler_ref: u64,
    record_ref: u64,
) -> Result<()> {
    if !jul_handler_allows_record(heap, handler_ref, record_ref)? {
        return Ok(());
    }
    let line = jul_format_record(heap, record_ref)?;
    writeln!(out, "{line}").ok();
    Ok(())
}

fn jul_publish_record_to_logger(
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    logger_ref: u64,
    record_ref: u64,
    depth: usize,
) -> Result<()> {
    if depth > 64 {
        return Ok(());
    }
    let fields = heap.get(logger_ref)?.fields.clone();
    let handler_count = match fields.get(JUL_LOGGER_HANDLER_COUNT_FIELD) {
        Some(Slot::Int(count)) => usize::try_from((*count).max(0)).unwrap_or(0),
        _ => 0,
    };
    for idx in 0..handler_count {
        if let Some(Slot::Reference(Some(handler_ref))) =
            fields.get(JUL_LOGGER_HANDLERS_START + idx).copied()
        {
            jul_publish_record_to_handler(heap, out, handler_ref, record_ref)?;
        }
    }
    let use_parent_handlers = matches!(
        fields.get(JUL_LOGGER_USE_PARENT_HANDLERS_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );
    if use_parent_handlers
        && let Some(Slot::Reference(Some(parent_ref))) = fields.get(JUL_LOGGER_PARENT_FIELD).copied()
    {
        jul_publish_record_to_logger(heap, out, parent_ref, record_ref, depth + 1)?;
    }
    Ok(())
}

fn jul_log_text(
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    logger_ref: u64,
    level_slot: Slot,
    message: &str,
    params: &[Slot],
    thrown_slot: Slot,
) -> Result<()> {
    let rendered = jul_apply_parameters(heap, message, params)?;
    let record_ref = jul_allocate_record(
        heap,
        logger_ref,
        level_slot,
        &rendered,
        thrown_slot,
        params,
    )?;
    jul_publish_record_to_logger(heap, out, logger_ref, record_ref, 0)
}

fn jul_log_message_slot(
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    logger_ref: u64,
    level_slot: Slot,
    message_slot: Slot,
    params: &[Slot],
    thrown_slot: Slot,
) -> Result<()> {
    if !jul_logger_is_loggable(heap, logger_ref, level_slot)? {
        return Ok(());
    }
    let message = jul_slot_to_text(heap, message_slot)?;
    jul_log_text(
        heap,
        out,
        logger_ref,
        level_slot,
        &message,
        params,
        thrown_slot,
    )
}

fn jul_log_named_level(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    level_name: &str,
) -> Result<Option<Slot>> {
    let logger_ref = extract_ref_arg(args, 0)?;
    let level_slot = jul_level_slot_by_name(heap, level_name)?;
    let message_slot = extract_slot_arg(args, 1);
    jul_log_message_slot(
        heap,
        out,
        logger_ref,
        level_slot,
        message_slot,
        &[],
        Slot::Reference(None),
    )?;
    Ok(None)
}

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
