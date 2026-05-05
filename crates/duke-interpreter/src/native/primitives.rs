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

fn path_from_string_slot(
    args: &[Slot],
    idx: usize,
    heap: &duke_gc::Heap,
) -> Result<std::path::PathBuf> {
    let path_ref = extract_ref_arg(args, idx)?;
    let path = heap
        .get(path_ref)?
        .string_value
        .clone()
        .ok_or(Error::NullPointerException)?;
    Ok(std::path::PathBuf::from(path))
}

fn string_value_from_ref(heap: &duke_gc::Heap, string_ref: u64) -> Result<String> {
    heap.get(string_ref)?
        .string_value
        .clone()
        .ok_or(Error::NullPointerException)
}

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

// ---- List.of / Set.of / Map.of factory methods ----

/// Helper: create an `ArrayList` from a slice of `Slot`s.
fn make_list_from_slots(
    elems: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<u64> {
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
) -> Result<Option<Slot>> {
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
) -> Result<u64> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let val = extract_slot_arg(args, 0);
    if matches!(val, Slot::Reference(None)) {
        return Err(Error::NullPointerException);
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
) -> Result<Option<Slot>> {
    let val = extract_slot_arg(args, 0);
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_first_field_arg(heap, this_ref)?;
    if matches!(val, Slot::Reference(None)) {
        return Err(Error::JavaException {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_first_field_arg(heap, this_ref)?;
    let result = if matches!(val, Slot::Reference(None)) {
        extract_slot_arg(args, 1)
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
) -> Result<Option<Slot>> {
    native_optional_get(args, heap, out, control)
}

// ---- ArrayList / HashMap bulk operations ----

/// Native: `IntStream.range(int,int)IntStream` — half-open range [start, end).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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

/// Native: `IntStream.iterate(seed, UnaryOperator)IntStream` — generates up to 4096 elements.
pub(crate) fn native_int_stream_iterate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    const MAX: usize = 4096;
    let seed = match args.first().copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(
            heap,
            vec![seed],
        )))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut values = Vec::with_capacity(MAX);
    let mut cur = seed;
    for _ in 0..MAX {
        values.push(cur);
        let next = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(I)I",
                vec![fn_slot, Slot::Int(cur)],
            )?
            .unwrap_or(Slot::Int(cur));
        match next {
            Slot::Int(n) => cur = n,
            _ => break,
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

/// Native: `IntStream.count()J`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::with_capacity(elems.len());
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

/// Native: `IntStream.peek(IntConsumer)IntStream` — calls consumer for each element, returns same stream.
pub(crate) fn native_int_stream_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let elems = int_stream_elems(heap, r);
    if let Slot::Reference(Some(fn_ref)) = fn_slot {
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        for &v in &elems {
            ops.invoke(
                heap,
                out,
                &fn_class,
                "accept",
                "(I)V",
                vec![fn_slot, Slot::Int(v)],
            )?;
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, elems)))))
}

/// Native: `IntStream.map(IntUnaryOperator)IntStream`
pub(crate) fn native_int_stream_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
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

/// Native: `OptionalInt.getAsInt()I`
pub(crate) fn native_optional_int_get_as_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}

/// Native: `OptionalInt.orElse(int)I` — returns value if present, else the default.
pub(crate) fn native_optional_int_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    if present {
        Ok(Some(
            heap.get(r)?.fields.first().copied().unwrap_or(Slot::Int(0)),
        ))
    } else {
        Ok(Some(args.get(1).copied().unwrap_or(Slot::Int(0))))
    }
}

/// Native: `OptionalInt.of(int)OptionalInt` — creates a present `OptionalInt`.
pub(crate) fn native_optional_int_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let value = args.first().copied().unwrap_or(Slot::Int(0));
    let r = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    heap.get_mut(r)?.fields[0] = value;
    heap.get_mut(r)?.fields[1] = Slot::Int(1); // present = true
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `OptionalInt.empty()OptionalInt` — creates an empty `OptionalInt`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_int_empty(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    heap.get_mut(r)?.fields[1] = Slot::Int(0); // present = false
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `OptionalLong.orElse(long)J`
pub(crate) fn native_optional_long_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    if present {
        Ok(Some(
            heap.get(r)?
                .fields
                .first()
                .copied()
                .unwrap_or(Slot::Long(0)),
        ))
    } else {
        Ok(Some(args.get(1).copied().unwrap_or(Slot::Long(0))))
    }
}

/// Native: `OptionalDouble.orElse(double)D`
pub(crate) fn native_optional_double_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    if present {
        Ok(Some(
            heap.get(r)?
                .fields
                .first()
                .copied()
                .unwrap_or(Slot::Double(0.0)),
        ))
    } else {
        Ok(Some(args.get(1).copied().unwrap_or(Slot::Double(0.0))))
    }
}

/// Native: `OptionalDouble.getAsDouble()D`
pub(crate) fn native_optional_double_get_as_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let v = match heap.get(r)?.fields.first().copied() {
        Some(Slot::Double(d)) => d,
        _ => 0.0,
    };
    Ok(Some(Slot::Double(v)))
}

// ---- ArrayDeque natives ----
// fields[0]=Int(size), fields[1..size]=elements (front at index 1)

/// Native: `ComparingIntComparator.compare(O,O)I` — calls `fn.applyAsInt(o)` for each element.
pub(crate) fn native_comparing_int_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
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

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_false_boolean(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Int(0)))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_zero_long(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Long(0)))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_void_noop(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

// ---------------------------------------------------------------------------
// println overloads (long, float, double, boolean, char, object)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// print overloads (long, float, double, boolean, char, object)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// System.exit
// ---------------------------------------------------------------------------

/// Native: `Integer.parseInt(String)` — parses string to int.
pub(crate) fn native_integer_parseint(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(args, heap, i32::MIN, i32::MAX)?;
    Ok(Some(Slot::Int(val)))
}

/// Native: `Integer.parseInt(String,int)` — parses string to int with radix.
pub(crate) fn native_integer_parseint_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(args, heap, i32::MIN, i32::MAX)?;
    Ok(Some(Slot::Int(val)))
}

/// Native: `Integer.valueOf(int)` — boxes int into Integer object.
pub(crate) fn native_integer_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(args, heap, i32::MIN, i32::MAX)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.valueOf(String,int)` — parses and boxes int with radix.
pub(crate) fn native_integer_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(args, heap, i32::MIN, i32::MAX)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.decode(String)` — parses prefixed string and boxes int.
pub(crate) fn native_integer_decode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i32_decode_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.intValue()` — unboxes Integer to int.
pub(crate) fn native_integer_intvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    Ok(Some(Slot::Long(i64::from(val))))
}

/// Native: `Integer.compareUnsigned(int,int)` — compares ints as unsigned 32-bit values.
pub(crate) fn native_integer_compareunsigned_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let int_val = |s: &Slot| -> Result<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => int_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = int_val(args.get(1).ok_or(Error::NullPointerException)?)?;
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.swap_bytes() as i32)))
}

/// Native: `Integer.signum(int)` — returns -1, 0, or 1.
pub(crate) fn native_integer_signum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(v.signum())))
}

/// Native: `Integer.compare(int, int)` — static two-value comparison.
pub(crate) fn native_integer_compare_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Long(v.swap_bytes() as i64)))
}

/// Native: `Long.signum(long)` — returns -1, 0, or 1.
pub(crate) fn native_long_signum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Int(v.signum() as i32)))
}

/// Native: `Long.compare(long, long)` — static two-value comparison.
pub(crate) fn native_long_compare_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let is_null = matches!(args.first(), Some(Slot::Reference(None)) | None);
    Ok(Some(Slot::Int(i32::from(!is_null))))
}

/// Native: `Objects.requireNonNull(Object)Object` — throws NPE if null, else returns arg.
pub(crate) fn native_objects_require_non_null(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(None)) | None => Err(Error::NullPointerException),
        Some(s) => Ok(Some(*s)),
    }
}

/// Native: `Objects.requireNonNull(Object, String)Object` — throws NPE with message if null.
pub(crate) fn native_objects_require_non_null_msg(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(None)) | None => Err(Error::NullPointerException),
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
) -> Result<Option<Slot>> {
    let a = extract_slot_arg(args, 0);
    let b = extract_slot_arg(args, 1);
    let equal = slots_equal(&a, &b, heap);
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `Objects.toString(Object)` — returns `"null"` if null, else `string_value` or class name.
pub(crate) fn native_objects_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

/// Native: `Objects.toString(Object, String)String` — returns nullDefault if null, else toString.
pub(crate) fn native_objects_tostring_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(None)) | None => {
            // null → return the default string (args[1])
            let default_ref = match args.get(1) {
                Some(Slot::Reference(Some(r))) => *r,
                _ => heap.allocate_string("null".to_string()),
            };
            Ok(Some(Slot::Reference(Some(default_ref))))
        }
        Some(Slot::Reference(Some(r))) => {
            let obj = heap.get(*r)?;
            let text = obj
                .string_value
                .as_deref()
                .map_or_else(|| format!("{}@{}", obj.class_name, r), str::to_owned);
            let _ = obj;
            let s = heap.allocate_string(text);
            Ok(Some(Slot::Reference(Some(s))))
        }
        Some(Slot::Int(n)) => {
            let s = heap.allocate_string(n.to_string());
            Ok(Some(Slot::Reference(Some(s))))
        }
        Some(other) => {
            let s = heap.allocate_string(format!("{other:?}"));
            Ok(Some(Slot::Reference(Some(s))))
        }
    }
}

/// Native: `Objects.hashCode(Object)I` — returns 0 for null, else object identity hash.
#[allow(clippy::cast_possible_truncation, clippy::unnecessary_wraps)]
pub(crate) fn native_objects_hashcode(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let hash = match args.first() {
        Some(Slot::Reference(Some(r))) => (*r & 0x7FFF_FFFF) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(hash)))
}

// ---- Collections utilities ----

/// Native: `IntStream.reduce(int, IntBinaryOperator)I` — fold with identity via callback.
pub(crate) fn native_int_stream_reduce_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let identity = match args.get(1) {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    let fn_slot = extract_slot_arg(args, 2);
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
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
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

/// Native: `Long.parseLong(String)` — parses string to long.
pub(crate) fn native_long_parselong(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_from_string_arg(args, heap)?;
    Ok(Some(Slot::Long(val)))
}

/// Native: `Long.parseLong(String,int)` — parses string to long with radix.
pub(crate) fn native_long_parselong_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_from_string_and_radix_args(args, heap)?;
    Ok(Some(Slot::Long(val)))
}

/// Native: `Long.valueOf(long)` — boxes long into Long object.
pub(crate) fn native_long_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.valueOf(String)` — parses and boxes long.
pub(crate) fn native_long_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.valueOf(String,int)` — parses and boxes long with radix.
pub(crate) fn native_long_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_from_string_and_radix_args(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.decode(String)` — parses prefixed string and boxes long.
pub(crate) fn native_long_decode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

/// Native: `Long.intValue()I` — returns the long value narrowed to int.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_intvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = match heap.get(this_ref)?.fields.first() {
        #[allow(clippy::cast_possible_truncation)] // enum ordinals fit i32
        Some(Slot::Long(v)) => *v as i32,
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(val)))
}

/// Native: `Long.toString(long)` — static, converts long to String.
pub(crate) fn native_long_tostring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(Error::TypeMismatch {
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
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(Error::TypeMismatch {
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
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(Error::TypeMismatch {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let long_val = |s: &Slot| -> Result<i64> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Long(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => long_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = long_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

// ---- Double class natives ----

/// Native: `Double.parseDouble(String)` — parses string to double.
pub(crate) fn native_double_parsedouble(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f64 = s.trim().parse().map_err(|_| Error::JavaException {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let val = extract_float_arg(args, 0)?;
    let r = heap.allocate("java/lang/Float".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Float(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_float_floatvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

pub(crate) fn native_float_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let float_val = |s: &Slot| -> Result<f32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Float(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => float_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = float_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.total_cmp(&b)))))
}

/// Native: `Float.parseFloat(String)` — parses string to float.
pub(crate) fn native_float_parsefloat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f32 = s.trim().parse().map_err(|_| Error::JavaException {
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
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Boolean".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(i32::from(val != 0));
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_boolean_booleanvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

pub(crate) fn native_boolean_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bool_val = |s: &Slot| -> Result<bool> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n != 0),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => bool_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = bool_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Native: `Boolean.parseBoolean(String)` — case-insensitive "true" → 1, else 0.
pub(crate) fn native_boolean_parseboolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let s = heap.get(*r)?.string_value.clone().unwrap_or_default();
            let val = s.eq_ignore_ascii_case("true");
            Ok(Some(Slot::Int(i32::from(val))))
        }
        Some(Slot::Reference(None)) => Ok(Some(Slot::Int(0))),
        _ => Err(Error::TypeMismatch {
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
) -> Result<i32> {
    let s = extract_string_arg_value(args, 0, heap)?;
    parse_bounded_i32_with_radix(&s, 10, min, max)
}

fn parse_bounded_i32_from_string_and_radix_args(
    args: &[Slot],
    heap: &duke_gc::Heap,
    min: i32,
    max: i32,
) -> Result<i32> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let radix = extract_parse_radix_arg(args, 1)?;
    parse_bounded_i32_with_radix(&s, radix, min, max)
}

fn parse_bounded_i32_with_radix(s: &str, radix: u32, min: i32, max: i32) -> Result<i32> {
    let val = i32::from_str_radix(s.trim(), radix).map_err(|_| Error::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    if val < min || val > max {
        return Err(Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    Ok(val)
}

fn parse_i64_from_string_arg(args: &[Slot], heap: &duke_gc::Heap) -> Result<i64> {
    let s = extract_string_arg_value(args, 0, heap)?;
    parse_i64_with_radix(&s, 10)
}

fn parse_i64_from_string_and_radix_args(args: &[Slot], heap: &duke_gc::Heap) -> Result<i64> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let radix = extract_parse_radix_arg(args, 1)?;
    parse_i64_with_radix(&s, radix)
}

fn parse_i64_with_radix(s: &str, radix: u32) -> Result<i64> {
    i64::from_str_radix(s.trim(), radix).map_err(|_| Error::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })
}

fn parse_i32_decode_from_string_arg(args: &[Slot], heap: &duke_gc::Heap) -> Result<i32> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let val = parse_i128_decode(&s)?;
    if val < i128::from(i32::MIN) || val > i128::from(i32::MAX) {
        return Err(Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    i32::try_from(val).map_err(|_| Error::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })
}

fn parse_i64_decode_from_string_arg(args: &[Slot], heap: &duke_gc::Heap) -> Result<i64> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let val = parse_i128_decode(&s)?;
    if val < i128::from(i64::MIN) || val > i128::from(i64::MAX) {
        return Err(Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    i64::try_from(val).map_err(|_| Error::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })
}

fn parse_i128_decode(s: &str) -> Result<i128> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(Error::JavaException {
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
        return Err(Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }

    let magnitude = i128::from_str_radix(digits, radix).map_err(|_| Error::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(if negative { -magnitude } else { magnitude })
}

fn extract_string_arg_value(args: &[Slot], index: usize, heap: &duke_gc::Heap) -> Result<String> {
    let str_ref = extract_ref_arg(args, index)?;
    Ok(heap.get(str_ref)?.string_value.clone().unwrap_or_default())
}

fn extract_parse_radix_arg(args: &[Slot], index: usize) -> Result<u32> {
    let radix = extract_int_arg(args, index)?;
    if !(2..=36).contains(&radix) {
        return Err(Error::JavaException {
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
) -> Result<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i8::MIN), i32::from(i8::MAX))?;
    Ok(Some(Slot::Int(val)))
}

pub(crate) fn native_byte_parsebyte_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_byte_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i8::MIN), i32::from(i8::MAX))?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_byte_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i8::MIN),
        i32::from(i8::MAX),
    )?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_byte_bytevalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

pub(crate) fn native_byte_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let byte_val = |s: &Slot| -> Result<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => byte_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = byte_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

pub(crate) fn native_short_parseshort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i16::MIN), i32::from(i16::MAX))?;
    Ok(Some(Slot::Int(val)))
}

pub(crate) fn native_short_parseshort_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_short_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i16::MIN), i32::from(i16::MAX))?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_short_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i16::MIN),
        i32::from(i16::MAX),
    )?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_short_shortvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

pub(crate) fn native_short_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let short_val = |s: &Slot| -> Result<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => short_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = short_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

pub(crate) fn native_char_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let char_val = |s: &Slot| -> Result<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => char_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = char_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// `Character.digit(char, int)int` — numeric value of char in given radix, or -1.
#[allow(clippy::unnecessary_wraps)] // signature must match NativeHandler
pub(crate) fn native_char_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = match args.first() {
        Some(Slot::Int(n)) => (*n).cast_unsigned(),
        _ => return Ok(Some(Slot::Int(-1))),
    };
    let radix = match args.get(1) {
        Some(Slot::Int(n)) => (*n).cast_unsigned(),
        _ => 10,
    };
    let result = char::from_u32(ch)
        .and_then(|c| c.to_digit(radix))
        .map_or(-1, u32::cast_signed);
    Ok(Some(Slot::Int(result)))
}

/// Execute a `StringConcatFactory` recipe: walk the recipe string, replacing
/// `\u{1}` placeholders with stringified dynamic args from the operand stack.
fn execute_string_concat_recipe(
    recipe: &str,
    dynamic_args: &[Slot],
    arg_types: &[char],
    constants: &[String],
    heap: &mut duke_gc::Heap,
) -> Result<Slot> {
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
) -> Result<()> {
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
/// Returns [`Error`] on execution faults (division by zero, stack overflow,
/// unimplemented instruction, etc.).
///
/// # Examples
///
/// The core bytecode interpretation loop.
///
/// **Why it exists:** This is the execution engine of the JVM. It reads decoded instructions,
/// manipulates the operand stack and local variables, and manages control flow (jumps, branches).
///
/// # Arguments
///
/// * `instructions` - A slice of `(pc, Instruction)` pairs representing the method's bytecode.
/// * `cp` - The constant pool for the current class.
/// * `args` - The initial local variables (arguments to the method).
/// * `max_stack` - The maximum depth of the operand stack.
/// * `max_locals` - The size of the local variable array.
///
/// # Returns
///
/// * `Ok(Some(Slot))` - If the method completes via `ireturn`, `areturn`, etc., yielding a value.
/// * `Ok(None)` - If the method completes via `return` (void).
/// * `Err(Error)` - If an exception is thrown or a fatal error occurs.
///
/// # Panics
///
/// Contains internal `debug_assert!` checks that will panic in debug mode if the JVM
/// state becomes invalid (e.g., stack underflow).
///
/// # Examples
///
/// The core bytecode interpretation loop.
///
/// **Why it exists:** This is the execution engine of the JVM. It reads decoded instructions,
/// manipulates the operand stack and local variables, and manages control flow (jumps, branches).
///
/// # Arguments
///
/// * `instructions` - A slice of `(pc, Instruction)` pairs representing the method's bytecode.
/// * `cp` - The constant pool for the current class.
/// * `args` - The initial local variables (arguments to the method).
/// * `max_stack` - The maximum depth of the operand stack.
/// * `max_locals` - The size of the local variable array.
///
/// # Returns
///
/// * `Ok(Some(Slot))` - If the method completes via `ireturn`, `areturn`, etc., yielding a value.
/// * `Ok(None)` - If the method completes via `return` (void).
/// * `Err(Error)` - If an exception is thrown or a fatal error occurs.
///
/// # Panics
///
/// Contains internal `debug_assert!` checks that will panic in debug mode if the JVM
/// state becomes invalid (e.g., stack underflow).
///
/// # Examples
///
/// The core bytecode interpretation loop.
///
/// **Why it exists:** This is the execution engine of the JVM. It reads decoded instructions,
/// manipulates the operand stack and local variables, and manages control flow (jumps, branches).
///
/// # Arguments
///
/// * `instructions` - A slice of `(pc, Instruction)` pairs representing the method's bytecode.
/// * `cp` - The constant pool for the current class.
/// * `args` - The initial local variables (arguments to the method).
/// * `max_stack` - The maximum depth of the operand stack.
/// * `max_locals` - The size of the local variable array.
///
/// # Returns
///
/// * `Ok(Some(Slot))` - If the method completes via `ireturn`, `areturn`, etc., yielding a value.
/// * `Ok(None)` - If the method completes via `return` (void).
/// * `Err(Error)` - If an exception is thrown or a fatal error occurs.
///
/// # Panics
///
/// Contains internal `debug_assert!` checks that will panic in debug mode if the JVM
/// state becomes invalid (e.g., stack underflow).
///
/// # Examples
///
/// The core bytecode interpretation loop.
///
/// **Why it exists:** This is the execution engine of the JVM. It reads decoded instructions,
/// manipulates the operand stack and local variables, and manages control flow (jumps, branches).
///
/// # Arguments
///
/// * `instructions` - A slice of `(pc, Instruction)` pairs representing the method's bytecode.
/// * `cp` - The constant pool for the current class.
/// * `args` - The initial local variables (arguments to the method).
/// * `max_stack` - The maximum depth of the operand stack.
/// * `max_locals` - The size of the local variable array.
///
/// # Returns
///
/// * `Ok(Some(Slot))` - If the method completes via `ireturn`, `areturn`, etc., yielding a value.
/// * `Ok(None)` - If the method completes via `return` (void).
/// * `Err(Error)` - If an exception is thrown or a fatal error occurs.
///
/// # Examples
///
/// Executes a sequence of instructions (bytecode) independently.
///
/// This is used heavily internally by the `MethodHandle` resolution and native implementation
/// logic to run standalone bytecodes (e.g., dynamically generated stubs).
///
/// ```
/// use duke_bytecode::Instruction;
/// use duke_runtime::Slot;
/// use duke_interpreter::execute;
///
/// let instructions = vec![
///     (0, Instruction::Iconst5),
///     (1, Instruction::Ireturn),
/// ];
/// let cp = vec![];
/// let result = execute(&instructions, &cp, vec![], 1, 0).unwrap();
/// assert_eq!(result, Some(Slot::Int(5)));
/// ```
///
/// # Errors
///
/// Returns a `Error` if bytecode invariants are broken or if an exception is raised
/// internally.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]
pub fn execute(
    instructions: &[(usize, Instruction)],
    cp: &[Option<CpEntry>],
    args: Vec<Slot>,
    max_stack: u16,
    max_locals: u16,
) -> Result<Option<Slot>> {
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
    let mut string_intern: HashMap<(u8, String), u64> = HashMap::new();

    loop {
        let Some((pc, instr)) = instructions.get(idx) else {
            return Err(Error::FellOffEnd);
        };
        let pc = *pc;

        // Jump to a PC-relative branch target (offset relative to current `pc`).
        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64).wrapping_add(i64::from($offset)) as usize;
                idx = *pc_to_idx
                    .get(&target)
                    .ok_or(Error::InvalidBranchTarget { pc: target })?;
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
                    let s = match cp.get(si).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(Error::InvalidCpIndex { index: si }),
                    };
                    let intern_key = (0, s);
                    let r = if let Some(&cached) = string_intern.get(&intern_key) {
                        cached
                    } else {
                        let r = local_heap.len() as u64;
                        local_heap.push((
                            "java/lang/String".to_string(),
                            Vec::new(),
                            Some(intern_key.1.clone()),
                        ));
                        string_intern.insert(intern_key, r);
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
                    let s = match cp.get(si).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(Error::InvalidCpIndex { index: si }),
                    };
                    let intern_key = (0, s);
                    let r = if let Some(&cached) = string_intern.get(&intern_key) {
                        cached
                    } else {
                        let r = local_heap.len() as u64;
                        local_heap.push((
                            "java/lang/String".to_string(),
                            Vec::new(),
                            Some(intern_key.1.clone()),
                        ));
                        string_intern.insert(intern_key, r);
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
                let value = frame.pop()?;
                if !matches!(value, Slot::Long(_) | Slot::Double(_)) {
                    frame.pop()?;
                }
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
                    return Err(Error::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_div(b)))?;
            }
            Instruction::Irem => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(Error::DivisionByZero);
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
                    return Err(Error::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_div(b)))?;
            }
            Instruction::Lrem => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(Error::DivisionByZero);
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
                    return Err(Error::NegativeArraySize { size: count });
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
                    return Err(Error::NegativeArraySize { size: count });
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
                    .ok_or(Error::InvalidRef { address: r })?
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                            .ok_or(Error::InvalidRef { address: *r })?
                            .0
                            .clone();
                        if actual == target {
                            frame.push(slot)?;
                        } else {
                            return Err(Error::ClassCastException {
                                from: actual,
                                to: target,
                            });
                        }
                    }
                    _ => {
                        return Err(Error::TypeMismatch {
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
                            .ok_or(Error::InvalidRef { address: *r })?
                            .0
                            .clone();
                        if actual == target {
                            frame.push(Slot::Int(1))?;
                        } else {
                            frame.push(Slot::Int(0))?;
                        }
                    }
                    _ => {
                        return Err(Error::TypeMismatch {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .0
                    .clone();
                return Err(Error::JavaException { class_name });
            }

            // ---- monitor (no-op, single-threaded) ----
            Instruction::Monitorenter | Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }

            other => {
                return Err(Error::Unimplemented {
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
) -> Result<()> {
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
) -> Result<()> {
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

/// Pre-resolved method dispatch entry — cached on first resolution to eliminate
/// repeated [`ClassRegistry`] `HashMap` lookups on hot call sites.
///
/// Stored in the static dispatch cache (`dispatch_cache`) and the virtual
/// dispatch cache (`vtable_cache`).  All `Arc` fields are cheap to clone
/// (reference-count bump only).
///
/// The caches are keyed as follows:
/// - `dispatch_cache`: `(caller_class, cp_idx)` for `invokestatic`/`invokespecial`
/// - `vtable_cache`: `(caller_class, cp_idx, receiver_runtime_class)` for `invokevirtual`
struct CachedDispatch {
    class_name: String,
    method_idx: usize,
    arg_count: usize,
    /// JVM primitive type chars for each parameter ('I', 'J', 'D', 'F', 'Z', 'B', 'C', 'S', 'L', '[').
    /// Used to assign wide types (J/D) to the correct local variable slots.
    param_types: Vec<char>,
    max_locals: usize,
    max_stack: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: std::sync::Arc<[(usize, Instruction)]>,
}

struct ExecutionState {
    current_class: String,
    method_idx: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: std::sync::Arc<[(usize, Instruction)]>,
    frame: Frame,
    call_stack: Vec<CallFrame>,
    frame_pool: FramePool,
    /// Static dispatch cache for `invokestatic` and `invokespecial`.
    /// Key: (`caller_class`, `cp_idx`) → pre-resolved method data.
    dispatch_cache: HashMap<String, HashMap<u16, CachedDispatch>>,
    /// Polymorphic inline cache for `invokevirtual`.
    /// Key: (`caller_class`, `cp_idx`, `receiver_runtime_class`) → pre-resolved method data.
    vtable_cache: HashMap<String, HashMap<u16, HashMap<String, CachedDispatch>>>,
    idx: usize,
    string_intern: HashMap<(u8, String), u64>,
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
) -> Result<Option<Option<Slot>>> {
    let Some(lambda_info) = registry.get_lambda(class).cloned() else {
        return Ok(None);
    };
    if method != lambda_info.sam_method || descriptor != lambda_info.sam_desc {
        return Ok(None);
    }

    let this_ref = match args.first().copied() {
        Some(Slot::Reference(Some(reference))) => reference,
        Some(Slot::Reference(None)) | None => return Err(Error::NullPointerException),
        Some(_) => {
            return Err(Error::TypeMismatch {
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
                .ok_or(Error::Unimplemented {
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
            Some(Slot::Reference(None)) | None => return Err(Error::NullPointerException),
            Some(_) => {
                return Err(Error::TypeMismatch {
                    expected: "reference",
                    got: "other",
                });
            }
        },
        _ => {
            return Err(Error::Unimplemented {
                mnemonic: "unsupported lambda impl kind",
            });
        }
    };

    let impl_args = adapt_args_for_impl_desc(&impl_args, &lambda_info.impl_desc, heap);
    let result = execute_class(
        registry,
        loader,
        heap,
        output,
        &dispatch_class,
        &lambda_info.impl_method,
        &lambda_info.impl_desc,
        &impl_args,
    )?;
    let result = autobox_if_needed(result, &lambda_info.impl_desc, &lambda_info.sam_desc, heap)?;
    Ok(Some(result))
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
    ) -> Result<Option<Slot>> {
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

    fn ensure_loaded(&mut self, class: &str) -> Result<()> {
        match self.registry.resolve_loaded_class_key(class) {
            Ok(_) => return Ok(()),
            Err(Error::ClassNotFound { .. }) => {}
            Err(err) => return Err(err),
        }
        if self.registry.ensure_loaded(class, self.loader)? {
            Ok(())
        } else {
            Err(Error::ClassNotFound {
                name: class.to_string(),
            })
        }
    }

    fn ensure_parent_loaded(&mut self, class: &str) -> Result<Option<String>> {
        if self.registry.contains(class) {
            return Ok(Some(class.to_string()));
        }
        if self.registry.ensure_loaded(class, self.loader)? && self.registry.contains(class) {
            Ok(Some(class.to_string()))
        } else {
            Ok(None)
        }
    }

    fn inspect_class(&mut self, class: &str) -> Result<ReflectedClassInfo> {
        inspect_reflected_class(self.registry, self.loader, class)
    }

    fn ensure_class_initialized(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
    ) -> Result<()> {
        self.ensure_loaded(class)?;
        ensure_initialized(self.registry, self.loader, heap, output, class, "")
    }

    fn code_source_for_class(&mut self, class: &str) -> Result<Option<String>> {
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
    ) -> Result<()> {
        let paths = runtime_loader_paths(self.registry, heap, loader_ref)?;
        for path in paths {
            if self
                .registry
                .ensure_loaded_with_provenance(class, &path, Some(loader_ref))?
            {
                return Ok(());
            }
        }
        self.ensure_loaded(class)
    }

    fn instance_field_slot(&mut self, class: &str, field_name: &str) -> Result<usize> {
        field_slot_idx(self.registry, class, field_name)
    }

    fn read_instance_field(
        &mut self,
        heap: &duke_gc::Heap,
        object_ref: u64,
        declaring_class: &str,
        field_name: &str,
    ) -> Result<Slot> {
        let actual_class = heap.get(object_ref)?.class_name.clone();
        if !is_assignable_from(
            self.registry,
            self.loader,
            &actual_class,
            declaring_class,
            Some(declaring_class),
        ) {
            return Err(Error::JavaException {
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
    ) -> Result<()> {
        let actual_class = heap.get(object_ref)?.class_name.clone();
        if !is_assignable_from(
            self.registry,
            self.loader,
            &actual_class,
            declaring_class,
            Some(declaring_class),
        ) {
            return Err(Error::JavaException {
                class_name: "java/lang/IllegalArgumentException".to_string(),
            });
        }
        let slot = field_slot_idx(self.registry, declaring_class, field_name)?;
        heap.write_field(object_ref, slot, value)?;
        Ok(())
    }

    fn read_static_field(&mut self, class: &str, field_name: &str) -> Result<Slot> {
        let slot = {
            let ctx = self.registry.get(class)?;
            static_field_idx(ctx, field_name)?
        };
        Ok(self.registry.get(class)?.static_fields[slot])
    }

    fn write_static_field(&mut self, class: &str, field_name: &str, value: Slot) -> Result<()> {
        let slot = {
            let ctx = self.registry.get(class)?;
            static_field_idx(ctx, field_name)?
        };
        self.registry.get_mut(class)?.static_fields[slot] = value;
        Ok(())
    }

    fn runtime_loader_for_class(&mut self, class: &str) -> Result<Option<u64>> {
        Ok(self.registry.runtime_loader_for_class(class))
    }

    fn service_configuration_files(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: Option<u64>,
        service_binary_name: &str,
    ) -> Result<Vec<Vec<u8>>> {
        if let Some(loader_ref) = loader_ref {
            let paths = runtime_loader_paths(self.registry, heap, loader_ref)?;
            return self
                .registry
                .service_configuration_files_for_paths(&paths, service_binary_name);
        }
        self.loader
            .service_configuration_files(service_binary_name)
            .map_err(|_| Error::JavaException {
                class_name: "java/util/ServiceConfigurationError".to_string(),
            })
    }

    fn find_resource_entry(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: Option<u64>,
        name: &str,
    ) -> Result<Option<duke_loader::LocatedResource>> {
        if let Some(loader_ref) = loader_ref {
            let paths = runtime_loader_paths(self.registry, heap, loader_ref)?;
            return self.registry.find_resource_entry_for_paths(&paths, name);
        }
        match self.loader.find_resource_entry(name) {
            Ok(resource) => Ok(Some(resource)),
            Err(duke_loader::Error::NotFound { .. }) => Ok(None),
            Err(_) => Err(Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            }),
        }
    }

    fn find_resource_entries(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: Option<u64>,
        name: &str,
    ) -> Result<Vec<duke_loader::LocatedResource>> {
        if let Some(loader_ref) = loader_ref {
            let paths = runtime_loader_paths(self.registry, heap, loader_ref)?;
            return self.registry.find_resource_entries_for_paths(&paths, name);
        }
        self.loader.find_resource_entries(name).map_err(|_| Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        })
    }

    fn class_key_for_loaded_class(&mut self, class: &str) -> Result<String> {
        self.registry.resolve_loaded_class_key(class)
    }

    fn class_key_for_runtime_loader(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: u64,
        class: &str,
    ) -> Result<String> {
        let paths = runtime_loader_paths(self.registry, heap, loader_ref)?;
        if let Some(path) = paths.first() {
            return Ok(self
                .registry
                .class_key_from_provenance(class, Some(path), Some(loader_ref)));
        }
        self.class_key_for_loaded_class(class)
    }

    fn class_key_from_source(
        &mut self,
        class: &str,
        source_class: Option<&str>,
    ) -> Result<String> {
        Ok(self.registry.class_key_from_source(class, source_class))
    }

    fn allocate_instance(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
    ) -> Result<u64> {
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
    ) -> Result<Self> {
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
            call_stack: Vec::with_capacity(32),
            frame_pool: FramePool::new(),
            dispatch_cache: HashMap::new(),
            vtable_cache: HashMap::new(),
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

/// Swap interpreter state to begin executing a callee method.
///
/// All method data is passed pre-resolved so this function performs **zero**
/// [`ClassRegistry`] lookups — eliminating the registry `HashMap` access from every
/// method-dispatch hot path.
/// Maximum Java call stack depth before raising `StackOverflowError`.
const MAX_CALL_DEPTH: usize = 500;

#[allow(clippy::too_many_arguments)]
fn activate_method_state(
    frame: &mut Frame,
    method_idx: &mut usize,
    pc_to_idx: &mut std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: &mut std::sync::Arc<[(usize, Instruction)]>,
    current_class: &mut String,
    call_stack: &mut Vec<CallFrame>,
    callee_class: String,
    callee_idx: usize,
    callee_pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    callee_frame: Frame,
    callee_instructions: std::sync::Arc<[(usize, Instruction)]>,
    resume_idx: usize,
    #[cfg(feature = "telemetry")] registry: &ClassRegistry,
    #[cfg(feature = "telemetry")] current_method: &mut String,
) -> Result<()> {
    if call_stack.len() >= MAX_CALL_DEPTH {
        return Err(duke_runtime::Error::JavaException {
            class_name: "java/lang/StackOverflowError".to_string(),
        });
    }
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
    *instructions = callee_instructions;
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
    retry_args: &[Slot],
) -> Result<Option<ExecutionOutcome>> {
    let action = native_control.take();
    if matches!(action, Some(NativeThreadAction::Retry)) {
        for slot in retry_args {
            frame.push(*slot)?;
        }
        return Ok(Some(ExecutionOutcome::ThreadAction(
            NativeThreadAction::Retry,
        )));
    }
    if let Some(val) = result {
        frame.push(val)?;
    }
    *idx += 1;
    if let Some(action) = action {
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
) -> Result<ExecutionState> {
    let class_name = match registry.resolve_loaded_class_key(class_name) {
        Ok(class_key) => class_key,
        Err(Error::ClassNotFound { .. }) => {
            if !registry.ensure_loaded(class_name, loader)? {
                return Err(Error::ClassNotFound {
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
            .ok_or_else(|| Error::MethodNotFound {
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
/// Returns [`Error`] on execution faults or if `method_name`/`descriptor`
/// are not found in `ctx`.
///
/// # Panics
/// Panics on internal invariant violations, such as a `Class` CP entry whose
/// name index refers to a non-`Utf8` entry (indicates a malformed class file).
///
/// # Examples
///
/// ```
/// use std::io::sink;
/// use duke_runtime::Slot;
/// use duke_gc::Heap;
/// use duke_loader::DirectoryLoader;
/// use duke_interpreter::{execute_class, ClassRegistry};
///
/// let mut registry = ClassRegistry::new();
/// let loader = DirectoryLoader::new("my_classes");
/// let mut heap = Heap::new();
/// let mut out = sink();
///
/// // We simulate execution by calling execute_class.
/// // It fails here with a missing class error, but demonstrates the setup.
/// let res = execute_class(&mut registry, &loader, &mut heap, &mut out, "java/lang/Object", "hashCode", "()I", &[]);
/// assert!(res.is_err());
/// ```
/// Starts execution of a specific Java method within a given class.
///
/// **Why it exists:** This function resolves the requested class and method, builds the
/// initial execution frame, and begins interpreting instructions. It bridges the gap
/// between the user requesting a class to run and the `execute` loop.
///
/// # Arguments
/// * `registry` - The class registry for resolving types.
/// * `loader` - The class loader for fetching dependencies.
/// * `heap` - The garbage collector heap.
/// * `stdout` - The standard output stream.
/// * `class_name` - The internal name of the class (e.g., `"java/lang/String"`).
/// * `method_name` - The name of the method to execute (e.g., `"main"`).
/// * `descriptor` - The method descriptor (e.g., `"([Ljava/lang/String;)V"`).
/// * `args` - The arguments to pass to the method.
///
/// # Returns
///
/// * `Ok(Some(Slot))` - If the method completes and returns a value.
/// * `Ok(None)` - If the method is `void` and completes.
/// * `Err(Error)` - If the method throws an unhandled exception or encounters a fatal VM error.
///
/// # Examples
///
/// ```no_run
/// # use duke_interpreter::{execute_class, ClassRegistry};
/// # use duke_loader::DirectoryLoader;
/// # use duke_gc::Heap;
/// # let mut registry = ClassRegistry::new();
/// # let loader = DirectoryLoader::new(".");
/// # let mut heap = Heap::new();
/// # let mut stdout = Vec::new();
/// // Execute `public static void main(String[] args)`
/// let result = execute_class(
///     &mut registry,
///     &loader,
///     &mut heap,
///     &mut stdout,
///     "com/example/App",
///     "main",
///     "([Ljava/lang/String;)V",
///     &[]
/// );
/// ```
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
) -> Result<Option<Slot>> {
    // Fast path: if a native handler is registered for this class/method/descriptor,
    // dispatch it directly without requiring a ClassContext in the registry.
    // This handles both Simple natives and Callback natives at the top-level call site.
    match lookup_registered_native_kind(registry, class_name, method_name, descriptor) {
        Some(HandlerKind::Simple(h)) => {
            let mut native_control = NativeControl::default();
            let result = h(args, heap, stdout, &mut native_control)?;
            if native_control.take().is_some() {
                return Err(Error::Unimplemented {
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
                return Err(Error::Unimplemented {
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
    match execution::run_execution(&mut state, registry, loader, heap, stdout, true, None)? {
        ExecutionOutcome::Returned(result) => Ok(result),
        ExecutionOutcome::ThreadAction(_) | ExecutionOutcome::Yield => {
            Err(Error::Unimplemented {
                mnemonic: "thread action requires execute_class_to_completion",
            })
        }
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
    handles: HashMap<i32, std::thread::JoinHandle<Result<()>>>,
    next_executor_worker_id: i32,
    executor_handles: HashMap<i32, std::thread::JoinHandle<Result<()>>>,
}

fn resolve_thread_entry(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &duke_gc::Heap,
    thread_ref: u64,
) -> Result<Option<(String, usize, Vec<Slot>)>> {
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
            || Error::AbstractMethodError {
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
) -> Result<()> {
    loop {
        let handle = {
            let mut runtime = runtime.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let is_finished = runtime
                .threads
                .records()
                .iter()
                .find(|record| record.thread_id == thread_id)
                .is_none_or(|record| record.finished);
            if is_finished {
                return Ok(());
            }

            // A thread attempting to join its own handle will panic.
            let is_self_join = runtime.handles.get(&thread_id).is_some_and(|h| h.thread().id() == std::thread::current().id());
            if is_self_join {
                // Java semantics dictate that a thread joining itself blocks forever.
                // Instead of panicking or returning immediately, we park the thread.
                drop(runtime);
                loop {
                    std::thread::park();
                }
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
) -> Result<()> {
    let mut first_error = None;
    loop {
        let handles = {
            let mut runtime = runtime.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if runtime.handles.is_empty() && runtime.executor_handles.is_empty() {
                return first_error.unwrap_or(Ok(()));
            }
            let mut handles =
                Vec::with_capacity(runtime.handles.len() + runtime.executor_handles.len());
            for (_, handle) in runtime.handles.drain() {
                handles.push(handle);
            }
            for (_, handle) in runtime.executor_handles.drain() {
                handles.push(handle);
            }
            drop(runtime);
            handles
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

struct ExecutorInvocation {
    state: ExecutionState,
    impl_desc: Option<String>,
    sam_desc: Option<String>,
}

const fn executor_task_signature(kind: duke_gc::ExecutorTaskKind) -> (&'static str, &'static str) {
    match kind {
        duke_gc::ExecutorTaskKind::Runnable => ("run", "()V"),
        duke_gc::ExecutorTaskKind::Callable => ("call", "()Ljava/lang/Object;"),
    }
}

fn prepare_lambda_executor_invocation(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    task_ref: u64,
    method: &str,
    descriptor: &str,
) -> Result<Option<ExecutorInvocation>> {
    let task_class = heap.get(task_ref)?.class_name.clone();
    let Some(lambda_info) = registry.get_lambda(&task_class).cloned() else {
        return Ok(None);
    };
    if method != lambda_info.sam_method || descriptor != lambda_info.sam_desc {
        return Ok(None);
    }

    let lambda_object = heap.get(task_ref)?;
    let mut impl_args = Vec::with_capacity(lambda_info.captured_count);
    for capture_index in 0..lambda_info.captured_count {
        impl_args.push(
            lambda_object
                .fields
                .get(capture_index)
                .copied()
                .ok_or(Error::Unimplemented {
                    mnemonic: "lambda capture missing",
                })?,
        );
    }

    let _ = registry.ensure_loaded_from(&lambda_info.impl_class, Some(task_class.as_str()), loader);
    let impl_class_key =
        registry.class_key_from_source(&lambda_info.impl_class, Some(task_class.as_str()));
    let dispatch_class = match lambda_info.impl_kind {
        6 | 7 => impl_class_key,
        5 | 9 => match impl_args.first().copied() {
            Some(Slot::Reference(Some(receiver_ref))) => {
                let receiver_class = heap.get(receiver_ref)?.class_name.clone();
                if let Some((dispatch_class, _)) = resolve_method_in_hierarchy(
                    registry,
                    loader,
                    &receiver_class,
                    &lambda_info.impl_method,
                    &lambda_info.impl_desc,
                ) {
                    dispatch_class
                } else {
                    impl_class_key
                }
            }
            Some(Slot::Reference(None)) | None => return Err(Error::NullPointerException),
            Some(_) => {
                return Err(Error::TypeMismatch {
                    expected: "reference",
                    got: "other",
                });
            }
        },
        _ => {
            return Err(Error::Unimplemented {
                mnemonic: "executor lambda impl kind",
            });
        }
    };
    ensure_initialized(
        registry,
        loader,
        heap,
        output,
        &dispatch_class,
        task_class.as_str(),
    )?;
    let (_, method_idx) = resolve_method_in_hierarchy(
        registry,
        loader,
        &dispatch_class,
        &lambda_info.impl_method,
        &lambda_info.impl_desc,
    )
    .ok_or_else(|| Error::MethodNotFound {
        name: format!("{}.{}", dispatch_class, lambda_info.impl_method),
        descriptor: lambda_info.impl_desc.clone(),
    })?;
    let impl_args = adapt_args_for_impl_desc(&impl_args, &lambda_info.impl_desc, heap);
    let state = ExecutionState::new(
        registry,
        &dispatch_class,
        &lambda_info.impl_method,
        method_idx,
        &impl_args,
    )?;
    Ok(Some(ExecutorInvocation {
        state,
        impl_desc: Some(lambda_info.impl_desc),
        sam_desc: Some(lambda_info.sam_desc),
    }))
}

fn prepare_executor_invocation(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    task: duke_gc::ExecutorTask,
) -> Result<ExecutorInvocation> {
    let (method, descriptor) = executor_task_signature(task.kind);
    if let Some(invocation) =
        prepare_lambda_executor_invocation(registry, loader, heap, output, task.task_ref, method, descriptor)?
    {
        return Ok(invocation);
    }

    let task_class = heap.get(task.task_ref)?.class_name.clone();
    let (dispatch_class, method_idx) =
        resolve_method_in_hierarchy(registry, loader, &task_class, method, descriptor).ok_or_else(
            || Error::AbstractMethodError {
                class_name: task_class.clone(),
                method_name: method.to_string(),
            },
        )?;
    ensure_initialized(
        registry,
        loader,
        heap,
        output,
        &dispatch_class,
        task_class.as_str(),
    )?;
    let state = ExecutionState::new(
        registry,
        &dispatch_class,
        method,
        method_idx,
        &[Slot::Reference(Some(task.task_ref))],
    )?;
    Ok(ExecutorInvocation {
        state,
        impl_desc: None,
        sam_desc: None,
    })
}

fn run_executor_state_to_completion(
    mut state: ExecutionState,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> Result<Option<Slot>> {
    loop {
        let mut shared_guard = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let CompletionVm {
            registry,
            heap,
            output,
            live_workers,
        } = &mut *shared_guard;
        let outcome = execution::run_execution(
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
            ExecutionOutcome::Returned(result) => return Ok(result),
            ExecutionOutcome::ThreadAction(action) => {
                handle_thread_action(action, shared, runtime, loader)?;
            }
            ExecutionOutcome::Yield => {
                std::thread::sleep(std::time::Duration::from_micros(1));
            }
        }
    }
}

fn store_future_failure(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    future_ref: u64,
    class_name: &str,
) -> Result<()> {
    let cause_ref = if let Some(exception_ref) = take_uncaught_java_exception_ref(class_name) {
        exception_ref
    } else {
        materialize_java_exception_object(registry, loader, heap, class_name)?
    };
    heap.write_field(
        future_ref,
        FUTURE_EXCEPTION_FIELD,
        Slot::Reference(Some(cause_ref)),
    )?;
    heap.write_field(future_ref, FUTURE_STATE_FIELD, Slot::Int(FUTURE_FAILED))?;
    Ok(())
}

fn store_future_success(
    heap: &mut duke_gc::Heap,
    task: duke_gc::ExecutorTask,
    result: Option<Slot>,
    impl_desc: Option<&str>,
    sam_desc: Option<&str>,
) -> Result<()> {
    if future_state(heap, task.future_ref)? == FUTURE_CANCELLED {
        return Ok(());
    }
    let result = match task.kind {
        duke_gc::ExecutorTaskKind::Runnable => {
            extract_field_arg(heap, task.future_ref, FUTURE_RESULT_FIELD)?
        }
        duke_gc::ExecutorTaskKind::Callable => result.unwrap_or(Slot::Reference(None)),
    };
    let result = if let (Some(impl_desc), Some(sam_desc)) = (impl_desc, sam_desc) {
        autobox_if_needed(Some(result), impl_desc, sam_desc, heap)?.unwrap_or(Slot::Reference(None))
    } else {
        result
    };
    heap.write_field(task.future_ref, FUTURE_RESULT_FIELD, result)?;
    heap.remember_reference_write(task.future_ref, result);
    heap.write_field(task.future_ref, FUTURE_STATE_FIELD, Slot::Int(FUTURE_DONE))?;
    Ok(())
}

fn run_executor_task(
    task: duke_gc::ExecutorTask,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> Result<()> {
    {
        let mut shared_guard = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if future_state(&shared_guard.heap, task.future_ref)? == FUTURE_CANCELLED {
            return Ok(());
        }
        shared_guard
            .heap
            .write_field(task.future_ref, FUTURE_STATE_FIELD, Slot::Int(FUTURE_RUNNING))?;
    }

    let invocation = {
        let mut shared_guard = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let CompletionVm {
            registry,
            heap,
            output,
            ..
        } = &mut *shared_guard;
        let result = prepare_executor_invocation(registry, loader.as_ref(), heap, output, task);
        drop(shared_guard);
        result
    };

    let invocation = match invocation {
        Ok(invocation) => invocation,
        Err(Error::JavaException { class_name }) => {
            let mut shared_guard = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let CompletionVm { registry, heap, .. } = &mut *shared_guard;
            let result =
                store_future_failure(registry, loader.as_ref(), heap, task.future_ref, &class_name);
            drop(shared_guard);
            result?;
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    let impl_desc = invocation.impl_desc.clone();
    let sam_desc = invocation.sam_desc.clone();
    match run_executor_state_to_completion(invocation.state, shared, runtime, loader) {
        Ok(result) => {
            let mut shared_guard = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let result = store_future_success(
                &mut shared_guard.heap,
                task,
                result,
                impl_desc.as_deref(),
                sam_desc.as_deref(),
            );
            drop(shared_guard);
            result
        }
        Err(Error::JavaException { class_name }) => {
            let mut shared_guard = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let CompletionVm { registry, heap, .. } = &mut *shared_guard;
            let result =
                store_future_failure(registry, loader.as_ref(), heap, task.future_ref, &class_name);
            drop(shared_guard);
            result?;
            Ok(())
        }
        Err(err) => Err(err),
    }
}

fn run_executor_worker(
    executor: &duke_gc::ExecutorShared,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> Result<()> {
    loop {
        let task = {
            let mut guard = executor
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            loop {
                if let Some(task) = guard.queue.pop_front() {
                    guard.active = guard.active.saturating_add(1);
                    break task;
                }
                if guard.shutdown {
                    guard.workers = guard.workers.saturating_sub(1);
                    guard.refresh_terminated();
                    drop(guard);
                    executor.available.notify_all();
                    return Ok(());
                }
                guard = executor
                    .available
                    .wait(guard)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        };

        let result = run_executor_task(task, shared, runtime, loader);
        {
            let mut guard = executor
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.active = guard.active.saturating_sub(1);
            guard.refresh_terminated();
            drop(guard);
            executor.available.notify_all();
        }
        result?;
    }
}

fn spawn_executor_worker(
    executor: std::sync::Arc<duke_gc::ExecutorShared>,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) {
    let shared_clone = std::sync::Arc::clone(shared);
    let runtime_clone = std::sync::Arc::clone(runtime);
    let loader_clone = std::sync::Arc::clone(loader);
    let handle = std::thread::spawn(move || {
        let result = run_executor_worker(&executor, &shared_clone, &runtime_clone, &loader_clone);
        {
            let mut shared = shared_clone
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            shared.live_workers = shared.live_workers.saturating_sub(1);
        }
        result
    });
    let mut runtime_guard = runtime
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let worker_id = runtime_guard.next_executor_worker_id;
    runtime_guard.next_executor_worker_id = runtime_guard.next_executor_worker_id.wrapping_add(1);
    runtime_guard.executor_handles.insert(worker_id, handle);
}

fn enqueue_executor_task(
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
    executor_ref: u64,
    future_ref: u64,
    task_ref: u64,
    kind: duke_gc::ExecutorTaskKind,
) -> Result<()> {
    let executor = {
        let shared_guard = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        executor_shared(&shared_guard.heap, executor_ref)?
    };
    let mut workers_to_spawn = 0usize;
    {
        let mut guard = executor
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.shutdown {
            return Ok(());
        }
        guard.queue.push_back(duke_gc::ExecutorTask {
            future_ref,
            task_ref,
            kind,
        });
        while guard.workers < guard.max_workers
            && guard.workers < guard.queue.len().saturating_add(guard.active)
        {
            guard.workers = guard.workers.saturating_add(1);
            workers_to_spawn = workers_to_spawn.saturating_add(1);
        }
        drop(guard);
    }
    for _ in 0..workers_to_spawn {
        {
            let mut shared_guard = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            shared_guard.live_workers = shared_guard.live_workers.saturating_add(1);
        }
        spawn_executor_worker(std::sync::Arc::clone(&executor), shared, runtime, loader);
    }
    executor.available.notify_all();
    Ok(())
}

fn handle_thread_action(
    action: NativeThreadAction,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> Result<()> {
    match action {
        NativeThreadAction::Start { thread_ref } => {
            spawn_java_thread(shared, runtime, loader, thread_ref)
        }
        NativeThreadAction::Sleep(duration) => {
            std::thread::sleep(duration);
            Ok(())
        }
        NativeThreadAction::Join { thread_id } => join_java_thread(runtime, thread_id),
        NativeThreadAction::Retry => {
            std::thread::sleep(std::time::Duration::from_micros(1));
            Ok(())
        }
        NativeThreadAction::ExecutorSubmit {
            executor_ref,
            future_ref,
            task_ref,
            kind,
        } => enqueue_executor_task(shared, runtime, loader, executor_ref, future_ref, task_ref, kind),
    }
}

fn run_thread_to_completion(
    mut state: ExecutionState,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> Result<()> {
    loop {
        let mut shared_guard = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let CompletionVm {
            registry,
            heap,
            output,
            live_workers,
        } = &mut *shared_guard;
        let outcome = execution::run_execution(
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
) -> Result<()> {
    {
        let mut shared_guard = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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

    let host_key = next_thread_host_key();
    let thread_id = {
        let mut runtime = runtime.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let thread_id = runtime.threads.allocate_thread_id();
        runtime
            .threads
            .register(threading::ThreadRecord::new(thread_ref, thread_id));
        thread_id
    };

    let entry = {
        let mut shared_guard = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        {
            let thread = shared_guard.heap.get_mut(thread_ref)?;
            thread.fields[THREAD_ID_SLOT] = Slot::Int(thread_id);
            thread.fields[THREAD_HOST_KEY_SLOT] = Slot::Int(host_key);
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
        let _ = runtime.lock().unwrap_or_else(std::sync::PoisonError::into_inner).threads.mark_finished(thread_id);
        return Ok(());
    };

    let state = {
        let shared = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        ExecutionState::new(&shared.registry, &dispatch_class, "run", method_idx, &args)?
    };

    let shared_clone = std::sync::Arc::clone(shared);
    let runtime_clone = std::sync::Arc::clone(runtime);
    let loader_clone = std::sync::Arc::clone(loader);
    let handle = std::thread::spawn(move || {
        let host_thread_id = std::thread::current().id();
        register_java_host_thread(host_key, host_thread_id);
        let interrupted_before_start = {
            let shared = shared_clone
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            matches!(
                shared
                    .heap
                    .get(thread_ref)
                    .ok()
                    .and_then(|thread| thread.fields.get(THREAD_INTERRUPTED_SLOT))
                    .copied(),
                Some(Slot::Int(value)) if value != 0
            )
        };
        if interrupted_before_start {
            interrupt_host_thread(host_thread_id);
        }
        let result = run_thread_to_completion(state, &shared_clone, &runtime_clone, &loader_clone);
        {
            let mut shared = shared_clone.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            shared.live_workers = shared.live_workers.saturating_sub(1);
        }
        let _ = runtime_clone
            .lock()
            .unwrap()
            .threads
            .mark_finished_by_java_ref(thread_ref);
        unregister_java_host_thread(host_key);
        result
    });
    runtime.lock().unwrap_or_else(std::sync::PoisonError::into_inner).handles.insert(thread_id, handle);
    Ok(())
}

/// Execute a Java entrypoint and keep the VM alive until any spawned worker
/// threads have either finished or been joined.
///
/// # Errors
///
/// Returns `Error` if class resolution, method dispatch, or bytecode
/// execution fails in any thread.
/// Starts execution of a specific Java method within a given class.
///
/// **Why it exists:** This function resolves the requested class and method, builds the
/// initial execution frame, and begins interpreting instructions. It bridges the gap
/// between the user requesting a class to run and the `execute` loop.
///
/// # Arguments
/// * `registry` - The class registry for resolving types.
/// * `loader` - The class loader for fetching dependencies.
/// * `heap` - The garbage collector heap.
/// * `stdout` - The standard output stream.
/// * `class_name` - The internal name of the class (e.g., `"java/lang/String"`).
/// * `method_name` - The name of the method to execute (e.g., `"main"`).
/// * `descriptor` - The method descriptor (e.g., `"([Ljava/lang/String;)V"`).
/// * `args` - The arguments to pass to the method.
///
/// # Returns
///
/// * `Ok(Some(Slot))` - If the method completes and returns a value.
/// * `Ok(None)` - If the method is `void` and completes.
/// * `Err(Error)` - If the method throws an unhandled exception or encounters a fatal VM error.
///
/// Execute a Java entrypoint and keep the VM alive until any spawned worker
/// threads have either finished or been joined.
///
/// # Errors
///
/// Returns `Error` if class resolution, method dispatch, or bytecode
/// execution fails in any thread.
///
/// # Panics
///
/// Panics if a `Mutex` protecting shared VM state is poisoned by a
/// panicking thread, or if the `Arc` cannot be unwound after all threads
/// have joined. Also panics if the completion runtime is unexpectedly released early.
#[allow(clippy::too_many_arguments, clippy::missing_panics_doc)]
pub fn execute_class_to_completion<L>(
    registry: &mut ClassRegistry,
    loader: L,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> Result<Option<Slot>>
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

    let run_result: Result<Option<Slot>> = loop {
        let mut shared_guard = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let CompletionVm {
            registry,
            heap,
            output,
            live_workers,
        } = &mut *shared_guard;
        let outcome = execution::run_execution(
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
        .map_err(|_| Error::Unimplemented {
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

fn line_number_for_bci(line_number_table: &[(u16, u16)], bci: usize) -> i32 {
    line_number_table
        .iter()
        .filter(|(start_pc, _)| usize::from(*start_pc) <= bci)
        .max_by_key(|(start_pc, _)| *start_pc)
        .map_or(-1, |(_, line)| i32::from(*line))
}

fn stack_frame_for_method(
    registry: &ClassRegistry,
    class_name: &str,
    method_idx: usize,
    bci: usize,
) -> Option<NativeStackFrame> {
    let method = registry.get(class_name).ok()?.methods.get(method_idx)?;
    let line_number = if method.is_native {
        -2
    } else {
        line_number_for_bci(&method.line_number_table, bci)
    };
    Some(NativeStackFrame {
        class_name: class_name.to_string(),
        method_name: method.name.clone(),
        file_name: method.source_file.clone(),
        line_number,
    })
}

fn capture_stack_trace_snapshot(
    registry: &ClassRegistry,
    current_class: &str,
    method_idx: usize,
    bci: usize,
    call_stack: &[CallFrame],
) -> Vec<NativeStackFrame> {
    let mut frames = Vec::with_capacity(call_stack.len() + 1);
    if let Some(frame) = stack_frame_for_method(registry, current_class, method_idx, bci) {
        frames.push(frame);
    }
    for caller in call_stack.iter().rev() {
        let caller_bci = registry
            .get(&caller.class_name)
            .ok()
            .and_then(|ctx| ctx.methods.get(caller.method_idx))
            .and_then(|method| {
                caller
                    .resume_idx
                    .checked_sub(1)
                    .and_then(|idx| method.instructions.get(idx))
                    .map(|(pc, _)| *pc)
            })
            .unwrap_or(0);
        if let Some(frame) =
            stack_frame_for_method(registry, &caller.class_name, caller.method_idx, caller_bci)
        {
            frames.push(frame);
        }
    }
    frames
}

fn is_throwable_class(registry: &ClassRegistry, class_name: &str) -> bool {
    let mut current = Some(class_name.to_string());
    while let Some(name) = current {
        if name == "java/lang/Throwable" {
            return true;
        }
        current = registry.get(&name).ok().and_then(|ctx| ctx.super_class.clone());
    }
    false
}

fn native_needs_stack_snapshot(
    registry: &ClassRegistry,
    native_class: &str,
    native_method: &str,
    native_desc: &str,
) -> bool {
    (native_method == "fillInStackTrace" && native_desc == "()Ljava/lang/Throwable;")
        || (native_method == "<init>"
            && matches!(
                native_desc,
                "()V" | "(Ljava/lang/String;)V" | "(Ljava/lang/String;Ljava/lang/Throwable;)V"
            )
            && is_throwable_class(registry, native_class))
}

#[allow(clippy::too_many_arguments)]
fn native_control_for_call(
    registry: &ClassRegistry,
    current_class: &str,
    method_idx: usize,
    bci: usize,
    call_stack: &[CallFrame],
    native_class: &str,
    native_method: &str,
    native_desc: &str,
) -> NativeControl {
    let mut control = NativeControl::default();
    if native_needs_stack_snapshot(registry, native_class, native_method, native_desc) {
        control.set_stack_trace(capture_stack_trace_snapshot(
            registry,
            current_class,
            method_idx,
            bci,
            call_stack,
        ));
    }
    control
}

/// Build a [`ClassContext`] from a parsed [`duke_classfile::ClassFile`].
///
/// Decodes all methods with a Code attribute and extracts field metadata.
/// Methods without Code (abstract, native) are silently skipped.
#[must_use]
#[allow(clippy::too_many_lines)]
fn build_method_entries(cf: &duke_classfile::ClassFile) -> Vec<MethodEntry> {
    use duke_bytecode::decode;
    use duke_classfile::MethodAccessFlags;
    use duke_classfile::types::{AttributeData, CpEntry};

    let source_file = cf.attributes.iter().find_map(|a| {
        if let AttributeData::SourceFile { sourcefile_index } = &a.data {
            match cf.constant_pool.get(sourcefile_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => Some(s.clone()),
                _ => None,
            }
        } else {
            None
        }
    });

    cf
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
            let is_native = m.access_flags.contains(MethodAccessFlags::NATIVE);
            let is_abstract = m.access_flags.contains(MethodAccessFlags::ABSTRACT);
            let code = m.attributes.iter().find_map(|a| {
                if let AttributeData::Code(c) = &a.data {
                    Some(c)
                } else {
                    None
                }
            });
            // Native and abstract methods have no Code attribute — emit a
            // stub MethodEntry so they appear in the method table for resolution.
            // The interpreter will dispatch these through NativeRegistry or error
            // on abstract calls.
            let Some(code) = code else {
                if is_native || is_abstract {
                    return Some(MethodEntry {
                        name,
                        descriptor,
                        is_public: m.access_flags.contains(MethodAccessFlags::PUBLIC),
                        is_static: m.access_flags.contains(MethodAccessFlags::STATIC),
                        is_native,
                        is_abstract,
                        instructions: std::sync::Arc::new([]),
                        max_stack: 0,
                        max_locals: 0,
                        exception_table: vec![],
                        pc_to_idx: std::sync::Arc::new(std::collections::HashMap::new()),
                        line_number_table: Vec::new(),
                        source_file: source_file.clone(),
                    });
                }
                return None; // no Code and not native/abstract — malformed, skip
            };
            let line_number_table = code
                .attributes
                .iter()
                .find_map(|attr| {
                    if let AttributeData::LineNumberTable(entries) = &attr.data {
                        Some(
                            entries
                                .iter()
                                .map(|entry| (entry.start_pc, entry.line_number))
                                .collect::<Vec<_>>(),
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
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
                is_native,
                is_abstract,
                instructions: instructions.into(),
                max_stack: code.max_stack,
                max_locals: code.max_locals,
                exception_table,
                pc_to_idx: std::sync::Arc::new(pc_to_idx_map),
                line_number_table,
                source_file: source_file.clone(),
            })
        })
        .collect()
}

fn build_field_entries(cf: &duke_classfile::ClassFile) -> (Vec<FieldEntry>, Vec<Slot>, usize) {
    use duke_classfile::FieldAccessFlags;
    use duke_classfile::types::CpEntry;

    let mut fields = Vec::with_capacity(cf.fields.len());
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


    (fields, static_fields, instance_count)
}

/// Constructs a `ClassContext` from a parsed `ClassFile`.
///
/// The `ClassContext` serves as the runtime representation of a loaded class.
/// It bridges the raw structure provided by `duke_classfile` and the execution environment
/// required by the interpreter. It encapsulates resolved metadata (like the class name and superclass),
/// method representations (including code and handlers), fields (both static and instance), and
/// bootstrap methods required for dynamic invocation (`invokedynamic`).
///
/// # Arguments
///
/// * `cf` - A reference to the parsed `duke_classfile::ClassFile`.
///
/// # Examples
///
/// ```
/// # use duke_interpreter::build_class_context;
/// # use duke_classfile::{ClassFile, ClassAccessFlags};
/// # use duke_classfile::types::{CpIndex, CpEntry};
/// // A minimal class file representation of `java/lang/Object`.
/// let cf = ClassFile {
///     minor_version: 0,
///     major_version: 52,
///     constant_pool: vec![
///         // Index 0 is implicit in Java constant pools, but duke_classfile uses 0-indexed vec
///         // Let's create a minimal valid pool where index 0 is a Utf8 and index 1 is a Class.
///         Some(CpEntry::Utf8("java/lang/Object".to_string())),
///         Some(CpEntry::Class { name_index: CpIndex(0) }),
///     ],
///     access_flags: ClassAccessFlags::empty(),
///     this_class: CpIndex(1), // Points to the Class entry at index 1
///     super_class: CpIndex(0), // No superclass
///     interfaces: vec![],
///     fields: vec![],
///     methods: vec![],
///     attributes: vec![],
/// };
///
/// let context = build_class_context(&cf);
/// assert_eq!(context.class_name, "java/lang/Object"); // Name is correctly resolved
/// ```
#[must_use]
pub fn build_class_context(cf: &duke_classfile::ClassFile) -> ClassContext {
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

    let methods = build_method_entries(cf);
    let (fields, static_fields, instance_field_count) = build_field_entries(cf);
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
        instance_field_count,
        bootstrap_methods,
        load_source: ClassLoadSource::Classfile,
    }
}

/// Resolve a CP Class entry to its name string.
fn resolve_class_name(cp: &[Option<CpEntry>], cp_idx: usize) -> Result<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Class { name_index }) => {
            match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => Err(Error::InvalidCpIndex {
                    index: name_index.0 as usize,
                }),
            }
        }
        _ => Err(Error::InvalidCpIndex { index: cp_idx }),
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

fn cp_utf8_string(cp: &[Option<CpEntry>], cp_idx: usize) -> Result<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Utf8(s)) => Ok(s.clone()),
        _ => Err(Error::InvalidCpIndex { index: cp_idx }),
    }
}

fn annotation_descriptor_to_internal_name(descriptor: &str) -> String {
    descriptor
        .strip_prefix('L')
        .and_then(|s| s.strip_suffix(';'))
        .unwrap_or(descriptor)
        .to_string()
}

fn class_literal_descriptor_to_key(descriptor: &str) -> String {
    if descriptor.starts_with('L') && descriptor.ends_with(';') {
        annotation_descriptor_to_internal_name(descriptor)
    } else {
        descriptor.to_string()
    }
}

fn cp_annotation_const(
    cp: &[Option<CpEntry>],
    cp_idx: usize,
) -> Option<ReflectedAnnotationConst> {
    match cp.get(cp_idx).and_then(|e| e.as_ref())? {
        CpEntry::Integer(value) => Some(ReflectedAnnotationConst::Int(*value)),
        CpEntry::Long(value) => Some(ReflectedAnnotationConst::Long(*value)),
        CpEntry::Float(value) => Some(ReflectedAnnotationConst::Float(*value)),
        CpEntry::Double(value) => Some(ReflectedAnnotationConst::Double(*value)),
        CpEntry::Utf8(value) => Some(ReflectedAnnotationConst::String(value.clone())),
        _ => None,
    }
}

fn resolve_annotation_value(
    cp: &[Option<CpEntry>],
    value: &duke_classfile::types::ElementValue,
) -> Option<ReflectedAnnotationValue> {
    use duke_classfile::types::ElementValue;
    match value {
        ElementValue::ConstValueIndex(index) => cp_annotation_const(cp, index.0 as usize)
            .map(ReflectedAnnotationValue::Const),
        ElementValue::EnumConstValue {
            type_name_index,
            const_name_index,
        } => {
            let type_descriptor = cp_utf8_string(cp, type_name_index.0 as usize).ok()?;
            let const_name = cp_utf8_string(cp, const_name_index.0 as usize).ok()?;
            Some(ReflectedAnnotationValue::Enum {
                type_name: annotation_descriptor_to_internal_name(&type_descriptor),
                const_name,
            })
        }
        ElementValue::ClassInfoIndex(index) => {
            let descriptor = cp_utf8_string(cp, index.0 as usize).ok()?;
            Some(ReflectedAnnotationValue::Class(
                class_literal_descriptor_to_key(&descriptor),
            ))
        }
        ElementValue::AnnotationValue(annotation) => resolve_annotation(cp, annotation)
            .map(Box::new)
            .map(ReflectedAnnotationValue::Annotation),
        ElementValue::ArrayValue(values) => values
            .iter()
            .map(|value| resolve_annotation_value(cp, value))
            .collect::<Option<Vec<_>>>()
            .map(ReflectedAnnotationValue::Array),
    }
}

fn resolve_annotation(
    cp: &[Option<CpEntry>],
    annotation: &duke_classfile::types::Annotation,
) -> Option<ReflectedAnnotation> {
    let descriptor = cp_utf8_string(cp, annotation.type_index.0 as usize).ok()?;
    let elements = annotation
        .element_value_pairs
        .iter()
        .map(|pair| {
            Some(ReflectedAnnotationElement {
                name: cp_utf8_string(cp, pair.element_name_index.0 as usize).ok()?,
                value: resolve_annotation_value(cp, &pair.value)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ReflectedAnnotation {
        type_name: annotation_descriptor_to_internal_name(&descriptor),
        elements,
    })
}

fn runtime_visible_annotations_from_attrs(
    cp: &[Option<CpEntry>],
    attrs: &[duke_classfile::types::AttributeInfo],
) -> Vec<ReflectedAnnotation> {
    attrs
        .iter()
        .find_map(|attr| {
            if let duke_classfile::types::AttributeData::RuntimeVisibleAnnotations(annotations) =
                &attr.data
            {
                Some(
                    annotations
                        .iter()
                        .filter_map(|annotation| resolve_annotation(cp, annotation))
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn annotation_default_from_attrs(
    cp: &[Option<CpEntry>],
    attrs: &[duke_classfile::types::AttributeInfo],
) -> Option<ReflectedAnnotationValue> {
    attrs.iter().find_map(|attr| {
        if let duke_classfile::types::AttributeData::AnnotationDefault(value) = &attr.data {
            resolve_annotation_value(cp, value)
        } else {
            None
        }
    })
}

fn reflected_class_info_from_loader(
    loader: &dyn ClassLoader,
    internal_name: &str,
) -> Option<ReflectedClassInfo> {
    use duke_classfile::{FieldAccessFlags, MethodAccessFlags};

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
                annotations: runtime_visible_annotations_from_attrs(
                    &class_file.constant_pool,
                    &method.attributes,
                ),
                annotation_default: annotation_default_from_attrs(
                    &class_file.constant_pool,
                    &method.attributes,
                ),
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
                annotations: runtime_visible_annotations_from_attrs(
                    &class_file.constant_pool,
                    &field.attributes,
                ),
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
        annotations: runtime_visible_annotations_from_attrs(
            &class_file.constant_pool,
            &class_file.attributes,
        ),
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
) -> Result<ReflectedClassInfo> {
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
                    annotations: Vec::new(),
                    annotation_default: None,
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
                    annotations: Vec::new(),
                })
                .collect();
            return Ok(ReflectedClassInfo {
                internal_name: internal_name.clone(),
                binary_name: internal_name_to_binary_name(&internal_name),
                super_class: ctx.super_class.clone(),
                interfaces: ctx.interfaces.clone(),
                methods,
                fields,
                annotations: Vec::new(),
            });
        }
        Err(Error::ClassNotFound { .. }) => {}
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
            annotations: Vec::new(),
            annotation_default: None,
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
            annotations: Vec::new(),
        })
        .collect();

    Ok(ReflectedClassInfo {
        internal_name: internal_name.clone(),
        binary_name: internal_name_to_binary_name(&internal_name),
        super_class: ctx.super_class.clone(),
        interfaces: ctx.interfaces.clone(),
        methods,
        fields,
        annotations: Vec::new(),
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

fn allocate_class_object(heap: &mut duke_gc::Heap, class_key: &str) -> Result<u64> {
    if let Some(class_ref) = heap.find_string_backed_object("java/lang/Class", class_key) {
        return Ok(class_ref);
    }
    let class_ref = heap.allocate("java/lang/Class".to_string(), 0);
    heap.get_mut(class_ref)?.string_value = Some(class_key.to_string());
    Ok(class_ref)
}

fn class_key_from_ref(heap: &duke_gc::Heap, class_ref: u64) -> Result<String> {
    heap.get(class_ref)?
        .string_value
        .clone()
        .ok_or(Error::InvalidRef { address: class_ref })
}

fn class_internal_name_from_ref(heap: &duke_gc::Heap, class_ref: u64) -> Result<String> {
    Ok(class_internal_name_from_key(&class_key_from_ref(heap, class_ref)?).to_string())
}

fn allocate_reference_array(
    heap: &mut duke_gc::Heap,
    array_class_name: &str,
    elements: &[u64],
) -> Result<u64> {
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
) -> Result<u64> {
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

fn reflection_member_name_slot(heap: &duke_gc::Heap, member_ref: u64) -> Result<Slot> {
    heap.get(member_ref)?
        .fields
        .get(REFLECTION_MEMBER_NAME_FIELD)
        .copied()
        .ok_or(Error::InvalidRef {
            address: member_ref,
        })
}

fn reflection_member_declaring_class_slot(heap: &duke_gc::Heap, member_ref: u64) -> Result<Slot> {
    heap.get(member_ref)?
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied()
        .ok_or(Error::InvalidRef {
            address: member_ref,
        })
}

fn reflection_member_declaring_class_name_slot(
    heap: &mut duke_gc::Heap,
    member_ref: u64,
) -> Result<Slot> {
    let Slot::Reference(Some(class_ref)) =
        reflection_member_declaring_class_slot(heap, member_ref)?
    else {
        return Err(Error::InvalidRef {
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
) -> Result<String> {
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
        _ => Err(Error::TypeMismatch {
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
) -> Result<Slot> {
    let class_key = descriptor_class_key_from_source(ops, descriptor, source_class)?;
    let class_ref = allocate_class_object(heap, &class_key)?;
    Ok(Slot::Reference(Some(class_ref)))
}

fn reflection_array_elements(heap: &duke_gc::Heap, args_slot: Slot) -> Result<Vec<Slot>> {
    match args_slot {
        Slot::Reference(None) => Ok(Vec::new()),
        Slot::Reference(Some(array_ref)) => Ok(heap.get(array_ref)?.fields.clone()),
        _ => Err(Error::TypeMismatch {
            expected: "reference array",
            got: "other",
        }),
    }
}

fn class_descriptor_from_class_ref(heap: &duke_gc::Heap, class_ref: u64) -> Result<String> {
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    if internal_name.starts_with('[') || internal_name.len() == 1 {
        Ok(internal_name)
    } else {
        Ok(format!("L{internal_name};"))
    }
}

fn parameter_descriptor_from_class_array(heap: &duke_gc::Heap, slot: Slot) -> Result<String> {
    let params = reflection_array_elements(heap, slot)?;
    let mut descriptor = String::from("(");
    for param in params {
        let class_ref = match param {
            Slot::Reference(Some(class_ref)) => class_ref,
            Slot::Reference(None) => return Err(Error::NullPointerException),
            _ => {
                return Err(Error::TypeMismatch {
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
) -> Result<Option<(String, ReflectedFieldInfo)>> {
    lookup_public_reflected_field_inner(ops, class, field_name, &mut HashSet::new())
}

fn lookup_public_reflected_field_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    field_name: &str,
    visited: &mut HashSet<String>,
) -> Result<Option<(String, ReflectedFieldInfo)>> {
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
) -> Result<Option<(String, ReflectedMethodInfo)>> {
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
) -> Result<Option<(String, ReflectedMethodInfo)>> {
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
) -> Result<Vec<(String, ReflectedFieldInfo)>> {
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
) -> Result<()> {
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
) -> Result<Vec<(String, ReflectedMethodInfo)>> {
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
) -> Result<()> {
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

fn reflected_field_handle(heap: &duke_gc::Heap, field_ref: u64) -> Result<ReflectedFieldHandle> {
    let field_obj = heap.get(field_ref)?;
    let Some(Slot::Reference(Some(declaring_class_ref))) = field_obj
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied()
    else {
        return Err(Error::InvalidRef { address: field_ref });
    };
    let Some(Slot::Reference(Some(name_ref))) =
        field_obj.fields.get(REFLECTION_MEMBER_NAME_FIELD).copied()
    else {
        return Err(Error::InvalidRef { address: field_ref });
    };
    let Some(Slot::Reference(Some(descriptor_ref))) = field_obj
        .fields
        .get(REFLECTION_MEMBER_DESCRIPTOR_FIELD)
        .copied()
    else {
        return Err(Error::InvalidRef { address: field_ref });
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
            .ok_or(Error::NullPointerException)?,
        descriptor: heap
            .get(descriptor_ref)?
            .string_value
            .clone()
            .ok_or(Error::NullPointerException)?,
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
) -> Result<ReflectedMethodHandle> {
    let method_obj = heap.get(method_ref)?;
    let Some(Slot::Reference(Some(declaring_class_ref))) = method_obj
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied()
    else {
        return Err(Error::InvalidRef {
            address: method_ref,
        });
    };
    let Some(Slot::Reference(Some(name_ref))) =
        method_obj.fields.get(REFLECTION_MEMBER_NAME_FIELD).copied()
    else {
        return Err(Error::InvalidRef {
            address: method_ref,
        });
    };
    let Some(Slot::Reference(Some(descriptor_ref))) = method_obj
        .fields
        .get(REFLECTION_MEMBER_DESCRIPTOR_FIELD)
        .copied()
    else {
        return Err(Error::InvalidRef {
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
            .ok_or(Error::NullPointerException)?,
        descriptor: heap
            .get(descriptor_ref)?
            .string_value
            .clone()
            .ok_or(Error::NullPointerException)?,
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
) -> Result<Vec<Slot>> {
    let arg_types = parse_arg_types(descriptor);
    if arg_types.len() != invoke_arg_slots.len() {
        return Err(Error::TypeMismatch {
            expected: "matching reflective argument count",
            got: "different count",
        });
    }

    let mut invoke_args = Vec::with_capacity(arg_types.len() + usize::from(!is_static));
    if !is_static {
        match target_slot {
            Slot::Reference(Some(_)) => invoke_args.push(target_slot),
            _ => return Err(Error::NullPointerException),
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

fn unbox_reflection_argument(heap: &duke_gc::Heap, descriptor: char, arg: Slot) -> Result<Slot> {
    match descriptor {
        'L' | '[' => match arg {
            Slot::Reference(_) => Ok(arg),
            _ => Err(Error::TypeMismatch {
                expected: "reference",
                got: "other",
            }),
        },
        'B' | 'C' | 'I' | 'S' | 'Z' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(Error::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Int(value)) => Ok(Slot::Int(*value)),
                _ => Err(Error::TypeMismatch {
                    expected: "boxed int-like primitive",
                    got: "other",
                }),
            }
        }
        'J' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(Error::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Long(value)) => Ok(Slot::Long(*value)),
                _ => Err(Error::TypeMismatch {
                    expected: "boxed long",
                    got: "other",
                }),
            }
        }
        'F' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(Error::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Float(value)) => Ok(Slot::Float(*value)),
                _ => Err(Error::TypeMismatch {
                    expected: "boxed float",
                    got: "other",
                }),
            }
        }
        'D' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(Error::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Double(value)) => Ok(Slot::Double(*value)),
                _ => Err(Error::TypeMismatch {
                    expected: "boxed double",
                    got: "other",
                }),
            }
        }
        _ => Err(Error::Unimplemented {
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
) -> Result<Slot> {
    match return_type {
        'V' => Ok(Slot::Reference(None)),
        'L' | '[' => Ok(result.unwrap_or(Slot::Reference(None))),
        'B' => {
            let Some(Slot::Int(value)) = result else {
                return Err(Error::TypeMismatch {
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
                return Err(Error::TypeMismatch {
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
                return Err(Error::TypeMismatch {
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
                return Err(Error::TypeMismatch {
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
                return Err(Error::TypeMismatch {
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
                return Err(Error::TypeMismatch {
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
                return Err(Error::TypeMismatch {
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
                return Err(Error::TypeMismatch {
                    expected: "boolean result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Boolean".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        _ => Err(Error::Unimplemented {
            mnemonic: "reflection primitive boxing",
        }),
    }
}

/// Push a constant pool value onto the frame's operand stack.
fn ldc_push(frame: &mut Frame, cp: &[Option<CpEntry>], idx: usize) -> Result<()> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Integer(v)) => frame.push(Slot::Int(*v)),
        Some(CpEntry::Float(v)) => frame.push(Slot::Float(*v)),
        Some(CpEntry::Long(v)) => frame.push(Slot::Long(*v)),
        Some(CpEntry::Double(v)) => frame.push(Slot::Double(*v)),
        _ => Err(Error::InvalidCpIndex { index: idx }),
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
    if let Some(annotation_type) = annotation_proxy_type(from) {
        let target = class_internal_name_from_key(to);
        return matches!(target, "java/lang/Object" | "java/lang/annotation/Annotation")
            || target == annotation_type;
    }

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
    // Lambda proxies implement their SAM interface (and transitively java/lang/Object).
    if from_key.starts_with("$$Lambda$")
        && let Some(lambda_info) = registry.get_lambda(&from_key)
        && (lambda_info.sam_interface == to_key || lambda_info.sam_interface == to_internal)
    {
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
                    let method = &ctx.methods[idx];
                    if method.is_native {
                        // Declared native in classfile — dispatch through NativeRegistry
                        return MethodHierarchyLookup::NativeOverride;
                    }
                    if !method.is_abstract {
                        return MethodHierarchyLookup::Bytecode(current, idx);
                    }
                    // Abstract method — keep walking to find a concrete implementation
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
fn resolve_methodref(cp: &[Option<CpEntry>], idx: usize) -> Result<(String, String, String)> {
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
                        _ => return Err(Error::InvalidMethodref { index: idx }),
                    }
                }
                _ => return Err(Error::InvalidMethodref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(Error::InvalidMethodref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(Error::InvalidMethodref { index: idx }),
                    };
                    Ok((class_name, name, desc))
                }
                _ => Err(Error::InvalidMethodref { index: nat_idx }),
            }
        }
        _ => Err(Error::InvalidMethodref { index: idx }),
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
fn resolve_fieldref(cp: &[Option<CpEntry>], idx: usize) -> Result<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Fieldref {
            class_index,
            name_and_type_index,
        }) => {
            let class_name = match cp.get(class_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(Error::InvalidFieldref { index: idx }),
                    }
                }
                _ => return Err(Error::InvalidFieldref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(Error::InvalidFieldref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(Error::InvalidFieldref { index: idx }),
                    };
                    Ok((class_name, name, desc))
                }
                _ => Err(Error::InvalidFieldref { index: nat_idx }),
            }
        }
        _ => Err(Error::InvalidFieldref { index: idx }),
    }
}

/// Resolve a `MethodHandle` CP entry to (`reference_kind`, `class_name`, `method_name`, descriptor).
fn resolve_method_handle(
    cp: &[Option<CpEntry>],
    cp_idx: usize,
) -> Result<(u8, String, String, String)> {
    let (kind, ref_idx) = match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::MethodHandle {
            reference_kind,
            reference_index,
        }) => (*reference_kind, reference_index.0 as usize),
        _ => return Err(Error::InvalidCpIndex { index: cp_idx }),
    };
    let (class_name, method_name, descriptor) = resolve_methodref(cp, ref_idx)?;
    Ok((kind, class_name, method_name, descriptor))
}

/// Resolve a `NameAndType` CP entry to (name, descriptor).
fn resolve_name_and_type(cp: &[Option<CpEntry>], cp_idx: usize) -> Result<(String, String)> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::NameAndType {
            name_index,
            descriptor_index,
        }) => {
            let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(Error::InvalidCpIndex {
                        index: name_index.0 as usize,
                    });
                }
            };
            let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(Error::InvalidCpIndex {
                        index: descriptor_index.0 as usize,
                    });
                }
            };
            Ok((name, desc))
        }
        _ => Err(Error::InvalidCpIndex { index: cp_idx }),
    }
}

/// Resolve a CP String entry to its UTF-8 content. Also handles bare Utf8 entries.
fn resolve_cp_string(cp: &[Option<CpEntry>], cp_idx: usize) -> Result<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::String { string_index }) => {
            match cp.get(string_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => Err(Error::InvalidCpIndex {
                    index: string_index.0 as usize,
                }),
            }
        }
        Some(CpEntry::Utf8(s)) => Ok(s.clone()),
        _ => Err(Error::InvalidCpIndex { index: cp_idx }),
    }
}

/// Parse argument type descriptors from a JVM method descriptor like `(IZLjava/lang/String;)V`.
/// Returns a Vec of single-char type codes: 'I', 'Z', 'L' (for object refs), '[' (for arrays), etc.
fn parse_arg_types(descriptor: &str) -> Vec<char> {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut types = Vec::with_capacity(params.len());
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
/// Adapts `args` to match the parameter types in `descriptor`:
/// - Inserts a padding `Slot::Int(0)` after each Long/Double slot.
/// - Unboxes `Slot::Reference(Some(r))` → `Slot::Int/Long/Float/Double` when
///   the corresponding descriptor param type is `I`, `J`, `F`, or `D`.
///   This handles method references like `Integer::sum` used as `BinaryOperator<Integer>`.
fn adapt_args_for_impl_desc(args: &[Slot], descriptor: &str, heap: &duke_gc::Heap) -> Vec<Slot> {
    let param_types = parse_arg_types(descriptor);
    if param_types.is_empty() || args.is_empty() {
        return args.to_vec();
    }
    let mut result = Vec::with_capacity(args.len() + 4);
    for (slot, &type_char) in args.iter().zip(param_types.iter()) {
        let adapted = match (slot, type_char) {
            // Unbox Reference → int
            (Slot::Reference(Some(r)), 'I' | 'B' | 'S' | 'C' | 'Z') => heap
                .get(*r)
                .ok()
                .and_then(|obj| obj.fields.first().copied())
                .filter(|s| matches!(s, Slot::Int(_)))
                .unwrap_or(*slot),
            // Unbox Reference → long
            (Slot::Reference(Some(r)), 'J') => heap
                .get(*r)
                .ok()
                .and_then(|obj| obj.fields.first().copied())
                .filter(|s| matches!(s, Slot::Long(_)))
                .unwrap_or(*slot),
            // Unbox Reference → double
            (Slot::Reference(Some(r)), 'D') => heap
                .get(*r)
                .ok()
                .and_then(|obj| obj.fields.first().copied())
                .filter(|s| matches!(s, Slot::Double(_)))
                .unwrap_or(*slot),
            // Unbox Reference → float
            (Slot::Reference(Some(r)), 'F') => heap
                .get(*r)
                .ok()
                .and_then(|obj| obj.fields.first().copied())
                .filter(|s| matches!(s, Slot::Float(_)))
                .unwrap_or(*slot),
            _ => *slot,
        };
        result.push(adapted);
        if type_char == 'J' || type_char == 'D' {
            result.push(Slot::Int(0)); // wide padding
        }
    }
    // Preserve trailing args beyond descriptor param count (captures).
    if args.len() > param_types.len() {
        result.extend_from_slice(&args[param_types.len()..]);
    }
    result
}

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

/// Pop args from the operand stack and store them in the locals array.
///
/// Wide types (J = long, D = double) occupy two local variable slots in the JVM.
/// For each such type, the value is stored at `local_idx` and `local_idx + 1` is
/// left as the default padding (`Slot::Int(0)`).
///
/// For methods without any wide-type params, this behaves identically to the
/// previous simple sequential assignment.
fn pop_typed_args_into_locals(
    param_types: &[char],
    frame: &mut Frame,
    locals: &mut [Slot],
    start_idx: usize,
) -> Result<()> {
    let count = param_types.len();
    let mut args = vec![Slot::Int(0); count];
    for i in (0..count).rev() {
        args[i] = frame.pop()?;
    }
    let mut local_idx = start_idx;
    for (slot, &tc) in args.iter().zip(param_types.iter()) {
        if local_idx < locals.len() {
            locals[local_idx] = *slot;
        }
        local_idx += 1;
        if tc == 'J' || tc == 'D' {
            local_idx += 1; // wide type: skip the padding slot
        }
    }
    Ok(())
}

/// Return the first character of the return type portion of a method descriptor.
fn desc_return_char(desc: &str) -> Option<char> {
    desc.split_once(')').and_then(|(_, ret)| ret.chars().next())
}

/// Autobox a primitive return value when the SAM descriptor expects a reference.
///
/// This is needed for bound method references like `s::length` where the impl
/// returns `int` but the SAM interface (`Supplier.get()`) returns `Object`.
fn autobox_if_needed(
    result: Option<Slot>,
    impl_desc: &str,
    sam_desc: &str,
    heap: &mut duke_gc::Heap,
) -> Result<Option<Slot>> {
    let Some(slot) = result else {
        return Ok(None);
    };
    if !matches!(desc_return_char(sam_desc), Some('L' | '[')) {
        return Ok(Some(slot));
    }
    match (desc_return_char(impl_desc), slot) {
        (Some('I'), Slot::Int(v)) => {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Int(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        (Some('Z'), Slot::Int(v)) => {
            let r = heap.allocate("java/lang/Boolean".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Int(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        (Some('J'), Slot::Long(v)) => {
            let r = heap.allocate("java/lang/Long".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Long(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        (Some('D'), Slot::Double(v)) => {
            let r = heap.allocate("java/lang/Double".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Double(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        (Some('F'), Slot::Float(v)) => {
            let r = heap.allocate("java/lang/Float".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Float(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        _ => Ok(Some(slot)),
    }
}

fn parse_arg_descriptors(descriptor: &str) -> Vec<String> {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut descriptors = Vec::with_capacity(params.len());
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
    let mut count = registry.get(class_name).map_or(0, |c| c.instance_field_count);
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
) -> Result<u64> {
    registry.ensure_loaded(class_name, loader)?;
    let exc_ref = heap.allocate(
        class_name.to_string(),
        total_instance_field_count(registry, class_name),
    );
    init_object_fields(registry, heap, exc_ref, class_name);
    if let Some(message) = pop_pending_java_exception_message(class_name) {
        heap.get_mut(exc_ref)?.string_value = Some(message);
    }
    if let Some(cause) = pop_pending_java_exception_cause(class_name)
        && let Ok(obj) = heap.get_mut(exc_ref)
        && !obj.fields.is_empty()
    {
        obj.fields[THROWABLE_CAUSE_FIELD] = cause;
        heap.remember_reference_write(exc_ref, cause);
    }
    Ok(exc_ref)
}

type PendingExceptionMessages = std::sync::Mutex<HashMap<String, VecDeque<String>>>;
type PendingExceptionCauses = std::sync::Mutex<HashMap<String, VecDeque<Slot>>>;
type UncaughtExceptionRefs = std::sync::Mutex<HashMap<std::thread::ThreadId, VecDeque<(String, u64)>>>;

fn pending_java_exception_messages() -> &'static PendingExceptionMessages {
    static MESSAGES: OnceLock<PendingExceptionMessages> = OnceLock::new();
    MESSAGES.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

fn push_pending_java_exception_message(class_name: &str, message: String) {
    let mut messages = pending_java_exception_messages()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    messages
        .entry(class_name.to_string())
        .or_default()
        .push_back(message);
}

fn pop_pending_java_exception_message(class_name: &str) -> Option<String> {
    let mut messages = pending_java_exception_messages()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let queue = messages.get_mut(class_name)?;
    let message = queue.pop_front();
    if queue.is_empty() {
        messages.remove(class_name);
    }
    message
}

fn pending_java_exception_causes() -> &'static PendingExceptionCauses {
    static CAUSES: OnceLock<PendingExceptionCauses> = OnceLock::new();
    CAUSES.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

fn push_pending_java_exception_cause(class_name: &str, cause: Slot) {
    let mut causes = pending_java_exception_causes()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    causes
        .entry(class_name.to_string())
        .or_default()
        .push_back(cause);
}

fn pop_pending_java_exception_cause(class_name: &str) -> Option<Slot> {
    let mut causes = pending_java_exception_causes()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let queue = causes.get_mut(class_name)?;
    let cause = queue.pop_front();
    if queue.is_empty() {
        causes.remove(class_name);
    }
    cause
}

fn uncaught_java_exception_refs() -> &'static UncaughtExceptionRefs {
    static REFS: OnceLock<UncaughtExceptionRefs> = OnceLock::new();
    REFS.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

pub(crate) fn record_uncaught_java_exception_ref(class_name: &str, exception_ref: u64) {
    let mut refs = uncaught_java_exception_refs()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    refs.entry(std::thread::current().id())
        .or_default()
        .push_back((class_name.to_string(), exception_ref));
}

fn take_uncaught_java_exception_ref(class_name: &str) -> Option<u64> {
    let mut refs = uncaught_java_exception_refs()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let thread_id = std::thread::current().id();
    let queue = refs.get_mut(&thread_id)?;
    let position = queue
        .iter()
        .position(|(queued_class, _)| queued_class == class_name)
        .unwrap_or(0);
    let (_, exception_ref) = queue.remove(position)?;
    if queue.is_empty() {
        refs.remove(&thread_id);
    }
    drop(refs);
    Some(exception_ref)
}

fn allocate_reflection_instance(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    class: &str,
) -> Result<u64> {
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
fn field_slot_idx(registry: &ClassRegistry, target_class: &str, name: &str) -> Result<usize> {
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

    Err(Error::InvalidFieldref { index: 0 })
}

/// Index of a named static field within `ctx.static_fields`.
fn static_field_idx(ctx: &ClassContext, name: &str) -> Result<usize> {
    ctx.fields
        .iter()
        .filter(|f| f.is_static)
        .position(|f| f.name == name)
        .ok_or(Error::InvalidFieldref { index: 0 })
}

// ---------------------------------------------------------------------------
// StringBuilder natives
// ---------------------------------------------------------------------------

/// Native: `Character.isDigit(C)Z`
pub(crate) fn native_char_is_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_ascii_digit()))))
}

/// Native: `Character.isLetter(C)Z`
pub(crate) fn native_char_is_letter(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphabetic()))))
}

/// Native: `Character.isWhitespace(C)Z`
pub(crate) fn native_char_is_whitespace(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_whitespace()))))
}

/// Native: `Character.isUpperCase(C)Z`
pub(crate) fn native_char_is_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_uppercase()))))
}

/// Native: `Character.isLowerCase(C)Z`
pub(crate) fn native_char_is_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_lowercase()))))
}

/// Native: `Character.toUpperCase(C)C`
pub(crate) fn native_char_to_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    let upper = ch.to_uppercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(upper as i32)))
}

/// Native: `Character.toLowerCase(C)C`
pub(crate) fn native_char_to_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    let lower = ch.to_lowercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(lower as i32)))
}

/// Native: `Character.isLetterOrDigit(C)Z`
pub(crate) fn native_char_is_letter_or_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphanumeric()))))
}

/// Native: `Character.valueOf(C)Ljava/lang/Character;` — box a char.
pub(crate) fn native_char_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(Error::TypeMismatch {
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

// ---------------------------------------------------------------------------
// ArrayList natives
// ---------------------------------------------------------------------------

/// Native: `Double.isNaN(D)Z` — returns 1 if value is NaN.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_isnan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let double_val = |s: &Slot| -> Result<f64> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Double(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => double_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = double_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    // Use total_cmp: implements Java's total order where NaN > +∞ > … > -∞.
    Ok(Some(Slot::Int(ordering_to_int(a.total_cmp(&b)))))
}

// ---- Arrays natives ----

/// Native: `IntStream.sorted()IntStream` — returns a new sorted `IntStream`.
pub(crate) fn native_int_stream_sorted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let primary = extract_slot_arg(args, 0);
    let secondary = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let primary = extract_first_field_arg(heap, this_ref)?;
    let secondary = extract_field_arg(heap, this_ref, 1)?;
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
) -> Result<i32> {
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
) -> Result<Option<Slot>> {
    let left = extract_slot_arg(args, 0);
    let right = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let left = extract_first_field_arg(heap, this_ref)?;
    let right = extract_field_arg(heap, this_ref, 1)?;
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
) -> Result<Option<Slot>> {
    let left = extract_slot_arg(args, 0);
    let right = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let left = extract_first_field_arg(heap, this_ref)?;
    let right = extract_field_arg(heap, this_ref, 1)?;
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
) -> Result<Option<Slot>> {
    let original = extract_slot_arg(args, 0);
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let original = extract_first_field_arg(heap, this_ref)?;
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
) -> Result<bool> {
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
) -> Result<Option<Slot>> {
    let first = extract_slot_arg(args, 0);
    let second = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let input = extract_slot_arg(args, 1);
    let first = extract_first_field_arg(heap, this_ref)?;
    let second = extract_field_arg(heap, this_ref, 1)?;
    let mid = invoke_function_apply(first, input, heap, out, ops)?;
    invoke_function_apply(second, mid, heap, out, ops).map(Some)
}

/// Native: `Consumer.andThen(Consumer)Consumer` — chains two consumers sequentially.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_consumer_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let first = extract_slot_arg(args, 0);
    let second = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/AndThenConsumer".to_string(), 2);
    heap.get_mut(r)?.fields[0] = first;
    heap.get_mut(r)?.fields[1] = second;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `AndThenConsumer.accept(O)V` — runs first then second consumer.
pub(crate) fn native_and_then_consumer_accept(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let arg = extract_slot_arg(args, 1);
    let first = extract_first_field_arg(heap, this_ref)?;
    let second = extract_field_arg(heap, this_ref, 1)?;
    invoke_consumer_accept(first, arg, heap, out, ops)?;
    invoke_consumer_accept(second, arg, heap, out, ops)?;
    Ok(None)
}

fn invoke_consumer_accept(
    consumer: Slot,
    arg: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<()> {
    let Slot::Reference(Some(c_ref)) = consumer else {
        return Ok(());
    };
    let c_class = heap.get(c_ref)?.class_name.clone();
    ops.invoke(
        heap,
        out,
        &c_class,
        "accept",
        "(Ljava/lang/Object;)V",
        vec![consumer, arg],
    )?;
    Ok(())
}

/// Native: `Function.compose(Function)Function` — `f.compose(g)` = `f(g(x))`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_function_compose(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let outer = extract_slot_arg(args, 0);
    let inner = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let input = extract_slot_arg(args, 1);
    let outer = extract_first_field_arg(heap, this_ref)?;
    let inner = extract_field_arg(heap, this_ref, 1)?;
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
) -> Result<Slot> {
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

/// Native: `BiFunction.andThen(Function)BiFunction` — returns `BiFunctionAndThen` proxy.
pub(crate) fn native_bifunction_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bifunction = extract_slot_arg(args, 0);
    let after = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/BiFunctionAndThen".to_string(), 2);
    heap.get_mut(r)?.fields[0] = bifunction;
    heap.get_mut(r)?.fields[1] = after;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `BiFunctionAndThen.apply(Object,Object)Object` — calls wrapped bifunction then after.
pub(crate) fn native_bifunction_and_then_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let bifunction = extract_first_field_arg(heap, this_ref)?;
    let after = extract_field_arg(heap, this_ref, 1)?;
    let Slot::Reference(Some(bf_ref)) = bifunction else {
        return Ok(Some(Slot::Reference(None)));
    };
    let bf_class = heap.get(bf_ref)?.class_name.clone();
    let mid = ops
        .invoke(
            heap,
            out,
            &bf_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![bifunction, a, b],
        )?
        .unwrap_or(Slot::Reference(None));
    Ok(Some(invoke_function_apply(after, mid, heap, out, ops)?))
}

/// Native: `LongStream.sum()J` — sums all elements.
pub(crate) fn native_long_stream_sum(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

/// Native: `DoubleStream.sum()D` — sums all elements.
pub(crate) fn native_double_stream_sum(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let sum: f64 = stream_elements(heap, stream_ref)?
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

/// Extracts stream payload slots from `fields[1..=size]`, safely handling empty streams and
/// malformed `size` headers.
fn stream_elements(heap: &duke_gc::Heap, stream_ref: u64) -> Result<Vec<Slot>> {
    let obj = heap.get(stream_ref)?;
    let declared_size = match obj.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let actual_size = declared_size.min(obj.fields.len().saturating_sub(1));
    if actual_size == 0 {
        return Ok(Vec::new());
    }
    Ok(obj.fields[1..=actual_size].to_vec())
}

/// Extract long elements from a `duke/util/LongStream`.
fn long_stream_elems(heap: &duke_gc::Heap, ref_: u64) -> Vec<i64> {
    stream_elements(heap, ref_)
        .map(|elems| {
            elems.into_iter()
                .filter_map(|s| {
                    if let Slot::Long(n) = s {
                        Some(n)
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
    stream_elements(heap, ref_)
        .map(|elems| {
            elems.into_iter()
                .filter_map(|s| {
                    if let Slot::Double(d) = s {
                        Some(d)
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let identity = match args.get(1).copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let fn_slot = extract_slot_arg(args, 2);
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    if !present {
        return Err(Error::MethodNotFound {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}

// ---- Collectors.summingInt / averagingInt ----

/// Native: `IntStream.findFirst()OptionalInt`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_find_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let key = extract_slot_arg(args, 0);
    let val = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(map_ref))], heap, out, control)?;
    // args[0] is the Object[] array of Map.Entry objects (anewarray layout: fields = elements)
    if let Some(Slot::Reference(Some(arr_ref))) = args.first().copied() {
        let entries: Vec<Slot> = heap.get(arr_ref)?.fields.clone();
        for entry_slot in entries {
            let Slot::Reference(Some(entry_ref)) = entry_slot else {
                continue;
            };
            let key = extract_first_field_arg(heap, entry_ref)?;
            let val = extract_field_arg(heap, entry_ref, 1)?;
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

/// Native: `IntStream.takeWhile(IntPredicate)IntStream` — keeps prefix while predicate holds.
pub(crate) fn native_int_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut kept = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut dropping = true;
    let mut kept = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut kept = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut dropping = true;
    let mut kept = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut kept = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
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
    let mut kept = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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

/// Native: `IntStream.limit(long)IntStream` — truncate to at most n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<f64> = double_stream_elems(heap, r).into_iter().skip(n).collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, elems)))))
}

/// Native: `DoubleStream.forEach(DoubleConsumer)V`
pub(crate) fn native_double_stream_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let identity = match args.get(1).copied() {
        Some(Slot::Double(d)) => d,
        _ => 0.0,
    };
    let fn_slot = extract_slot_arg(args, 2);
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
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
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
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
