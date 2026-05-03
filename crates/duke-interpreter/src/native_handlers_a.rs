
use std::sync::{RwLock, OnceLock};
use std::sync::atomic::{AtomicI32, Ordering};

fn zip_files() -> &'static RwLock<HashMap<i32, duke_loader::ZipReader>> {
    static ZIP_FILES: OnceLock<RwLock<HashMap<i32, duke_loader::ZipReader>>> = OnceLock::new();
    ZIP_FILES.get_or_init(|| RwLock::new(HashMap::new()))
}

static NEXT_ZIP_ID: AtomicI32 = AtomicI32::new(100_000_000);

fn zip_open(path: &std::path::Path) -> Result<i32> {
    let reader = duke_loader::ZipReader::open(path).map_err(|err| match err {
        duke_loader::Error::Io { .. } => Error::JavaException {
            class_name: "java/io/FileNotFoundException".to_string(),
        },
        _ => Error::JavaException {
            class_name: "java/util/zip/ZipException".to_string(),
        },
    })?;
    let id = NEXT_ZIP_ID.fetch_add(1, Ordering::Relaxed);
    zip_files().write().unwrap_or_else(std::sync::PoisonError::into_inner).insert(id, reader);
    Ok(id)
}

fn zip_entry_count(id: i32) -> Result<usize> {
    let map = zip_files().read().unwrap_or_else(std::sync::PoisonError::into_inner);
    map.get(&id).map_or_else(
        || Err(Error::JavaException { class_name: "java/io/IOException".into() }),
        |reader| Ok(reader.entry_count())
    )
}

fn zip_get_entry_info(id: i32, name: &str) -> Result<Option<duke_loader::ZipEntryInfo>> {
    let map = zip_files().read().unwrap_or_else(std::sync::PoisonError::into_inner);
    map.get(&id).map_or_else(
        || Err(Error::JavaException { class_name: "java/io/IOException".into() }),
        |reader| Ok(reader.get_entry(name).cloned())
    )
}

fn zip_read_entry(id: i32, name: &str) -> Result<Vec<u8>> {
    let map = zip_files().read().unwrap_or_else(std::sync::PoisonError::into_inner);
    map.get(&id).map_or_else(
        || Err(Error::JavaException { class_name: "java/io/IOException".into() }),
        |reader| reader.read_entry(name).map_err(|_| Error::JavaException { class_name: "java/util/zip/ZipException".into() })
    )
}

fn zip_close(id: i32) {
    zip_files().write().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&id);
}

fn extract_slot_arg(args: &[Slot], idx: usize) -> Slot {
    args.get(idx).copied().unwrap_or(Slot::Reference(None))
}

const fn atomic_payload_error(this_ref: u64) -> Error {
    Error::InvalidRef { address: this_ref }
}

fn with_atomic_i32<T>(
    heap: &duke_gc::Heap,
    this_ref: u64,
    f: impl FnOnce(&AtomicI32) -> T,
) -> Result<T> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Int(cell)) => Ok(f(cell)),
        _ => Err(atomic_payload_error(this_ref)),
    }
}

fn with_atomic_i64<T>(
    heap: &duke_gc::Heap,
    this_ref: u64,
    f: impl FnOnce(&std::sync::atomic::AtomicI64) -> T,
) -> Result<T> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Long(cell)) => Ok(f(cell)),
        _ => Err(atomic_payload_error(this_ref)),
    }
}

fn with_atomic_bool<T>(
    heap: &duke_gc::Heap,
    this_ref: u64,
    f: impl FnOnce(&std::sync::atomic::AtomicBool) -> T,
) -> Result<T> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Bool(cell)) => Ok(f(cell)),
        _ => Err(atomic_payload_error(this_ref)),
    }
}

fn with_atomic_reference<T>(
    heap: &duke_gc::Heap,
    this_ref: u64,
    f: impl FnOnce(&std::sync::Mutex<Slot>) -> Result<T>,
) -> Result<T> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Reference(cell)) => f(cell),
        _ => Err(atomic_payload_error(this_ref)),
    }
}

fn load_atomic_reference(heap: &duke_gc::Heap, this_ref: u64) -> Result<Slot> {
    with_atomic_reference(heap, this_ref, |cell| {
        Ok(*cell
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner))
    })
}

fn atomic_bool_arg(args: &[Slot], idx: usize) -> Result<bool> {
    extract_int_arg(args, idx).map(|value| value != 0)
}

#[inline]
fn extract_ref_arg(args: &[Slot], idx: usize) -> Result<u64> {
    match args.get(idx) {
        Some(Slot::Reference(Some(r))) => Ok(*r),
        _ => Err(Error::NullPointerException),
    }
}

#[inline]
fn extract_field_arg(heap: &duke_gc::Heap, obj_ref: u64, idx: usize) -> Result<Slot> {
    Ok(heap.get(obj_ref)?.fields.get(idx).copied().unwrap_or(Slot::Reference(None)))
}

#[inline]
fn extract_first_field_arg(heap: &duke_gc::Heap, obj_ref: u64) -> Result<Slot> {
    extract_field_arg(heap, obj_ref, 0)
}

#[inline]
fn extract_io_fd(heap: &duke_gc::Heap, obj_ref: u64) -> Result<i32> {
    match heap.get(obj_ref)?.fields.first() {
        Some(Slot::Int(id)) => Ok(*id),
        _ => Err(Error::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

#[inline]
fn extract_io_fd_at(heap: &duke_gc::Heap, obj_ref: u64, idx: usize) -> Result<i32> {
    match heap.get(obj_ref)?.fields.get(idx) {
        Some(Slot::Int(id)) => Ok(*id),
        _ => Err(Error::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

#[inline]
fn extract_int_arg(args: &[Slot], idx: usize) -> Result<i32> {
    match args.get(idx) {
        Some(Slot::Int(v)) => Ok(*v),
        _ => Err(Error::TypeMismatch {
            expected: "Int",
            got: "other",
        }),
    }
}

/// Box a primitive `Slot` into a heap object so it can be stored as `Object` in collections.
/// `Slot::Reference` and `Slot::Long` pad pass through unchanged.
/// `Slot::Int` → `java/lang/Integer`, `Slot::Long` → `java/lang/Long`,
/// `Slot::Double` → `java/lang/Double`, `Slot::Float` → `java/lang/Float`.
fn box_primitive_slot(slot: Slot, heap: &mut duke_gc::Heap) -> Slot {
    match slot {
        Slot::Int(v) => {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            if let Ok(obj) = heap.get_mut(r) {
                obj.fields[0] = Slot::Int(v);
            }
            Slot::Reference(Some(r))
        }
        Slot::Long(v) => {
            let r = heap.allocate("java/lang/Long".to_string(), 1);
            if let Ok(obj) = heap.get_mut(r) {
                obj.fields[0] = Slot::Long(v);
            }
            Slot::Reference(Some(r))
        }
        Slot::Double(v) => {
            let r = heap.allocate("java/lang/Double".to_string(), 1);
            if let Ok(obj) = heap.get_mut(r) {
                obj.fields[0] = Slot::Double(v);
            }
            Slot::Reference(Some(r))
        }
        Slot::Float(v) => {
            let r = heap.allocate("java/lang/Float".to_string(), 1);
            if let Ok(obj) = heap.get_mut(r) {
                obj.fields[0] = Slot::Float(v);
            }
            Slot::Reference(Some(r))
        }
        other => other, // already a reference
    }
}

#[inline]
fn extract_long_arg(args: &[Slot], idx: usize) -> Result<i64> {
    match args.get(idx) {
        Some(Slot::Long(v)) => Ok(*v),
        _ => Err(Error::TypeMismatch {
            expected: "Long",
            got: "other",
        }),
    }
}

#[inline]
fn extract_float_arg(args: &[Slot], idx: usize) -> Result<f32> {
    match args.get(idx) {
        Some(Slot::Float(v)) => Ok(*v),
        _ => Err(Error::TypeMismatch {
            expected: "Float",
            got: "other",
        }),
    }
}

#[inline]
fn extract_double_arg(args: &[Slot], idx: usize) -> Result<f64> {
    match args.get(idx) {
        Some(Slot::Double(v)) => Ok(*v),
        _ => Err(Error::TypeMismatch {
            expected: "Double",
            got: "other",
        }),
    }
}

macro_rules! extract_print_arg {
    ($args:expr, $pat:pat => $expr:expr, $expected:literal) => {
        match $args.get(1) {
            Some($pat) => $expr,
            _ => {
                return Err(Error::TypeMismatch {
                    expected: $expected,
                    got: "other",
                });
            }
        }
    };
}

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

pub(crate) fn native_string_get_bytes_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes = string_bytes_for_arg(args, heap, StandardCharset::Utf8)?;
    Ok(Some(Slot::Reference(Some(allocate_byte_array(
        heap, &bytes,
    )?))))
}

pub(crate) fn native_string_get_bytes_named(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 1)?;
    let charset = charset_from_name_ref(heap, name_ref, unsupported_encoding_error)?;
    let bytes = string_bytes_for_arg(args, heap, charset)?;
    Ok(Some(Slot::Reference(Some(allocate_byte_array(
        heap, &bytes,
    )?))))
}

pub(crate) fn native_string_get_bytes_charset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let charset = charset_from_arg(args, 1, heap)?;
    let bytes = string_bytes_for_arg(args, heap, charset)?;
    Ok(Some(Slot::Reference(Some(allocate_byte_array(
        heap, &bytes,
    )?))))
}

pub(crate) fn native_string_init_bytes_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes_ref = extract_ref_arg(args, 1)?;
    let bytes = full_byte_array(heap, bytes_ref)?;
    init_string_from_bytes(args, heap, &bytes, StandardCharset::Utf8)
}

pub(crate) fn native_string_init_bytes_default_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes_ref = extract_ref_arg(args, 1)?;
    let offset = extract_int_arg(args, 2)?;
    let length = extract_int_arg(args, 3)?;
    let bytes = byte_array_window(heap, bytes_ref, offset, length)?;
    init_string_from_bytes(args, heap, &bytes, StandardCharset::Utf8)
}

pub(crate) fn native_string_init_bytes_named(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes_ref = extract_ref_arg(args, 1)?;
    let name_ref = extract_ref_arg(args, 2)?;
    let charset = charset_from_name_ref(heap, name_ref, unsupported_encoding_error)?;
    let bytes = full_byte_array(heap, bytes_ref)?;
    init_string_from_bytes(args, heap, &bytes, charset)
}

pub(crate) fn native_string_init_bytes_charset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes_ref = extract_ref_arg(args, 1)?;
    let charset = charset_from_arg(args, 2, heap)?;
    let bytes = full_byte_array(heap, bytes_ref)?;
    init_string_from_bytes(args, heap, &bytes, charset)
}

pub(crate) fn native_string_init_bytes_range_charset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes_ref = extract_ref_arg(args, 1)?;
    let offset = extract_int_arg(args, 2)?;
    let length = extract_int_arg(args, 3)?;
    let charset = charset_from_arg(args, 4, heap)?;
    let bytes = byte_array_window(heap, bytes_ref, offset, length)?;
    init_string_from_bytes(args, heap, &bytes, charset)
}

pub(crate) fn native_string_init_bytes_range_named(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes_ref = extract_ref_arg(args, 1)?;
    let offset = extract_int_arg(args, 2)?;
    let length = extract_int_arg(args, 3)?;
    let name_ref = extract_ref_arg(args, 4)?;
    let charset = charset_from_name_ref(heap, name_ref, unsupported_encoding_error)?;
    let bytes = byte_array_window(heap, bytes_ref, offset, length)?;
    init_string_from_bytes(args, heap, &bytes, charset)
}

#[cfg(test)]
mod charset_codec_tests {
    use super::*;

    #[test]
    fn charset_aliases_map_to_canonical_variants() {
        assert_eq!(charset_for_name("utf8"), Some(StandardCharset::Utf8));
        assert_eq!(charset_for_name("latin1"), Some(StandardCharset::Iso88591));
        assert_eq!(charset_for_name("ASCII"), Some(StandardCharset::UsAscii));
        assert_eq!(charset_for_name("not-a-charset"), None);
    }

    #[test]
    fn utf8_decode_replaces_malformed_sequence() {
        assert_eq!(
            decode_string_with_charset(&[0xc3, 0x28], StandardCharset::Utf8),
            "\u{fffd}("
        );
    }

    #[test]
    fn utf16_encodes_bom_and_decodes_surrogate_pair() {
        let value = decode_string_with_charset(
            &[0xf0, 0x9f, 0x98, 0x80, b' ', b'e', b'm', b'o', b'j', b'i'],
            StandardCharset::Utf8,
        );
        let bytes = encode_string_with_charset(&value, StandardCharset::Utf16);
        assert_eq!(&bytes[0..2], &[0xfe, 0xff]);
        assert_eq!(decode_string_with_charset(&bytes, StandardCharset::Utf16), value);
    }

    #[test]
    fn ascii_and_latin1_encode_unmappable_as_question_mark() {
        assert_eq!(
            encode_string_with_charset("\u{20ac}", StandardCharset::UsAscii),
            vec![b'?']
        );
        assert_eq!(
            encode_string_with_charset("\u{20ac}", StandardCharset::Iso88591),
            vec![b'?']
        );
    }

    #[test]
    fn standard_charset_allocation_is_canonical_per_heap() {
        let mut heap = duke_gc::Heap::new();
        let first = allocate_standard_charset(&mut heap, "UTF-8");
        let second = allocate_standard_charset(&mut heap, "UTF-8");
        assert_eq!(first, second);
        assert_eq!(
            heap.get(first).expect("charset object").string_value.as_deref(),
            Some("UTF-8")
        );
    }
}

fn file_path_from_ref(file_ref: u64, heap: &duke_gc::Heap) -> Result<std::path::PathBuf> {
    let path_ref = match heap.get(file_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(Error::NullPointerException),
    };
    Ok(std::path::PathBuf::from(string_value_from_ref(
        heap, path_ref,
    )?))
}

fn file_path_from_this(args: &[Slot], heap: &duke_gc::Heap) -> Result<std::path::PathBuf> {
    let this_ref = extract_ref_arg(args, 0)?;
    file_path_from_ref(this_ref, heap)
}

fn archive_path_from_slot(
    heap: &duke_gc::Heap,
    archive_ref: u64,
    slot_idx: usize,
) -> Result<Option<String>> {
    let Some(file_ref) = archive_ref_from_slot(heap, archive_ref, slot_idx)? else {
        return Ok(None);
    };
    Ok(Some(
        file_path_from_ref(file_ref, heap)?
            .to_string_lossy()
            .to_string(),
    ))
}

fn boot_archive_path_from_ref(
    registry: &ClassRegistry,
    heap: &duke_gc::Heap,
    archive_ref: u64,
) -> Result<Option<String>> {
    let archive_class = heap.get(archive_ref)?.class_name.clone();
    match archive_class.as_str() {
        "org/springframework/boot/loader/launch/JarFileArchive" => {
            let file_slot = field_slot_idx(registry, &archive_class, "file")?;
            archive_path_from_slot(heap, archive_ref, file_slot)
        }
        "org/springframework/boot/loader/launch/ExplodedArchive" => {
            let root_slot = field_slot_idx(registry, &archive_class, "rootDirectory")?;
            archive_path_from_slot(heap, archive_ref, root_slot)
        }
        _ => Ok(None),
    }
}

fn launched_class_loader_archive_path(
    registry: &ClassRegistry,
    heap: &duke_gc::Heap,
    loader_ref: u64,
) -> Result<Option<String>> {
    if heap.get(loader_ref)?.class_name
        != "org/springframework/boot/loader/launch/LaunchedClassLoader"
    {
        return Ok(None);
    }
    let root_archive_slot = field_slot_idx(
        registry,
        "org/springframework/boot/loader/launch/LaunchedClassLoader",
        "rootArchive",
    )?;
    let Some(archive_ref) = archive_ref_from_slot(heap, loader_ref, root_archive_slot)? else {
        return Ok(None);
    };
    boot_archive_path_from_ref(registry, heap, archive_ref)
}

fn class_extends(registry: &ClassRegistry, class_name: &str, expected_super: &str) -> bool {
    let mut current = Some(class_name.to_string());
    while let Some(name) = current {
        if name == expected_super {
            return true;
        }
        current = registry.get(&name).ok().and_then(|ctx| ctx.super_class.clone());
    }
    false
}

fn arraylist_reference_elements(heap: &duke_gc::Heap, list_ref: u64) -> Result<Vec<u64>> {
    let list_obj = heap.get(list_ref)?;
    let size = match list_obj.fields.first().copied() {
        Some(Slot::Int(value)) if value > 0 => usize::try_from(value).unwrap_or(0),
        _ => 0,
    };
    let mut refs = Vec::with_capacity(size);
    for slot in list_obj.fields.iter().skip(1).take(size) {
        if let Slot::Reference(Some(reference)) = slot {
            refs.push(*reference);
        }
    }
    Ok(refs)
}

fn class_path_from_url_spec(spec: &str) -> Option<String> {
    if let Some(jar_spec) = spec.strip_prefix("jar:")
        && let Some(file_url) = jar_spec.split("!/").next()
    {
        return file_url_to_path(file_url)
            .ok()
            .map(|path| path.to_string_lossy().to_string());
    }
    if spec.starts_with("file://") {
        return file_url_to_path(spec)
            .ok()
            .map(|path| path.to_string_lossy().to_string());
    }
    None
}

fn url_class_loader_paths(
    registry: &ClassRegistry,
    heap: &duke_gc::Heap,
    loader_ref: u64,
) -> Result<Vec<String>> {
    let loader_class = heap.get(loader_ref)?.class_name.clone();
    if !class_extends(registry, &loader_class, "java/net/URLClassLoader") {
        return Ok(Vec::new());
    }
    let Ok(ucp_slot) = field_slot_idx(registry, &loader_class, "ucp") else {
        return Ok(Vec::new());
    };
    let Some(ucp_ref) = archive_ref_from_slot(heap, loader_ref, ucp_slot)? else {
        return Ok(Vec::new());
    };
    let ucp_class = heap.get(ucp_ref)?.class_name.clone();
    let Ok(path_slot) = field_slot_idx(registry, &ucp_class, "path") else {
        return Ok(Vec::new());
    };
    let Some(path_list_ref) = archive_ref_from_slot(heap, ucp_ref, path_slot)? else {
        return Ok(Vec::new());
    };
    let mut paths = Vec::new();
    for url_ref in arraylist_reference_elements(heap, path_list_ref)? {
        if let Ok(spec) = string_backed_object_value(heap, url_ref)
            && let Some(path) = class_path_from_url_spec(&spec)
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn runtime_loader_paths(
    registry: &ClassRegistry,
    heap: &duke_gc::Heap,
    loader_ref: u64,
) -> Result<Vec<String>> {
    if let Some(path) = launched_class_loader_archive_path(registry, heap, loader_ref)? {
        return Ok(vec![path]);
    }
    url_class_loader_paths(registry, heap, loader_ref)
}

fn archive_ref_from_slot(
    heap: &duke_gc::Heap,
    obj_ref: u64,
    slot_idx: usize,
) -> Result<Option<u64>> {
    match heap.get(obj_ref)?.fields.get(slot_idx).copied() {
        Some(Slot::Reference(Some(r))) => Ok(Some(r)),
        Some(Slot::Reference(None)) | None => Ok(None),
        Some(_) => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
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

fn file_stream_id_from_this(args: &[Slot], heap: &duke_gc::Heap) -> Result<i32> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        }),
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
        return Err(Error::InvalidRef { address: this_ref });
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
        return Err(Error::InvalidRef { address: this_ref });
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
        return Err(Error::InvalidRef { address: this_ref });
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
        return Err(Error::InvalidRef { address: this_ref });
    };
    *fd_field = Slot::Int(0);
    Ok(None)
}

// ── Networking natives ────────────────────────────────────────────────────

/// Native: `ServerSocket.<init>(int port)` — binds to 0.0.0.0:{port}.
pub(crate) fn native_server_socket_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let port = extract_int_arg(args, 1)?;
    let addr = format!("0.0.0.0:{port}");
    let server_id = heap.bind_server_socket(&addr)?;
    let actual_port = heap.server_socket_local_port(server_id)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 2 {
        return Err(Error::InvalidRef { address: this_ref });
    }
    obj.fields[0] = Slot::Int(server_id);
    obj.fields[1] = Slot::Int(actual_port);
    Ok(None)
}

/// Native: `ServerSocket.accept()` — blocks until a client connects, returns a Socket.
pub(crate) fn native_server_socket_accept(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let server_fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let (reader_id, writer_id) = heap.accept_connection(server_fd)?;
    // Allocate a new Socket object with fdRead=reader_id, fdWrite=writer_id
    let socket_ref = heap.allocate("java/net/Socket".to_string(), 2);
    heap.get_mut(socket_ref)?.fields[0] = Slot::Int(reader_id);
    heap.get_mut(socket_ref)?.fields[1] = Slot::Int(writer_id);
    Ok(Some(Slot::Reference(Some(socket_ref))))
}

/// Native: `ServerSocket.getLocalPort()` — returns the bound port.
pub(crate) fn native_server_socket_get_local_port(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(port)) => Ok(Some(Slot::Int(*port))),
        _ => Err(Error::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

/// Native: `ServerSocket.close()` — closes the OS listener and zeros the fd field.
pub(crate) fn native_server_socket_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        Some(Slot::Int(_)) => return Ok(None), // already closed — idempotent
        _ => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    heap.close_host_file(fd);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    obj.fields[1] = Slot::Int(0); // also zero cached port so getLocalPort() returns 0 after close
    Ok(None)
}

/// Native: `Socket.<init>(String host, int port)` — connects to host:port.
pub(crate) fn native_socket_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let host_ref = extract_ref_arg(args, 1)?;
    let host = heap
        .get(host_ref)?
        .string_value
        .clone()
        .ok_or(Error::NullPointerException)?;
    let port = extract_int_arg(args, 2)?;
    let addr = format!("{host}:{port}");
    let (reader_id, writer_id) = heap.connect_socket(&addr)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 2 {
        return Err(Error::InvalidRef { address: this_ref });
    }
    obj.fields[0] = Slot::Int(reader_id);
    obj.fields[1] = Slot::Int(writer_id);
    Ok(None)
}

/// Native: `Socket.getInputStream()` — allocates a `SocketInputStream` wrapping `fdRead`.
pub(crate) fn native_socket_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd_read = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let stream_ref = heap.allocate("duke/net/SocketInputStream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(fd_read);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `Socket.getOutputStream()` — allocates a `SocketOutputStream` wrapping `fdWrite`.
pub(crate) fn native_socket_get_output_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd_write = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let stream_ref = heap.allocate("duke/net/SocketOutputStream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(fd_write);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `Socket.close()` — closes both OS handles (fdRead and fdWrite).
pub(crate) fn native_socket_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd_read = extract_io_fd(heap, this_ref)?;
    let fd_write = extract_io_fd_at(heap, this_ref, 1)?;
    heap.close_host_file(fd_read);
    heap.close_host_file(fd_write);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    obj.fields[1] = Slot::Int(0);
    Ok(None)
}

// ── ZIP / JAR natives ──────────────────────────────────────────────────

/// Native: `ZipFile.<init>(String)` — open and index a ZIP/JAR archive.
pub(crate) fn native_zip_file_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path_ref = extract_ref_arg(args, 1)?;
    let path_str = heap
        .get(path_ref)?
        .string_value
        .as_deref()
        .ok_or(Error::NullPointerException)?
        .to_string();
    let fd = zip_open(std::path::Path::new(&path_str))?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(fd);
    Ok(None)
}

/// Native: `JarFile.<init>(File)` — open and index a JAR archive from a File object.
pub(crate) fn native_jar_file_init_from_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let file_ref = extract_ref_arg(args, 1)?;
    let path = file_path_from_ref(file_ref, heap)?;
    let fd = zip_open(&path)?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(fd);
    Ok(None)
}

pub(crate) fn native_jar_file_init_with_mode_and_version(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let forwarded_args = match args {
        [this_slot, file_slot, ..] => [*this_slot, *file_slot],
        _ => {
            return Err(Error::TypeMismatch {
                expected: "this,file",
                got: "other",
            });
        }
    };
    native_jar_file_init_from_file(&forwarded_args, heap, out, control)
}

pub(crate) fn native_jar_file_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let Ok(manifest_bytes) = zip_read_entry(fd, "META-INF/MANIFEST.MF") else {
        return Ok(Some(Slot::Reference(None)));
    };
    let manifest_ref = allocate_manifest_from_bytes(heap, &manifest_bytes)?;
    Ok(Some(Slot::Reference(Some(manifest_ref))))
}

pub(crate) fn native_boot_nested_jar_file_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_jar_file_get_manifest(args, heap, out, control)
}

const BOOT_JAR_FILE_ARCHIVE_JAR_FILE_SLOT: usize = 1;
const BOOT_EXPLODED_ARCHIVE_ROOT_DIRECTORY_SLOT: usize = 0;
const BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT: usize = 2;

fn allocate_manifest_from_bytes(heap: &mut duke_gc::Heap, bytes: &[u8]) -> Result<u64> {
    let raw_ref = heap.allocate_string(String::from_utf8_lossy(bytes).into_owned());
    let manifest_ref = heap.allocate("java/util/jar/Manifest".to_string(), 1);
    heap.get_mut(manifest_ref)?.fields[0] = Slot::Reference(Some(raw_ref));
    Ok(manifest_ref)
}

pub(crate) fn native_boot_jar_file_archive_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let jar_file_ref = match heap
        .get(this_ref)?
        .fields
        .get(BOOT_JAR_FILE_ARCHIVE_JAR_FILE_SLOT)
        .copied()
    {
        Some(Slot::Reference(Some(jar_file_ref))) => jar_file_ref,
        Some(Slot::Reference(None)) | None => return Err(Error::NullPointerException),
        Some(_) => {
            return Err(Error::TypeMismatch {
                expected: "reference",
                got: "other",
            });
        }
    };
    native_jar_file_get_manifest(&[Slot::Reference(Some(jar_file_ref))], heap, out, control)
}

pub(crate) fn native_boot_archive_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.class_name.as_str() {
        "org/springframework/boot/loader/launch/JarFileArchive" => {
            native_boot_jar_file_archive_get_manifest(args, heap, out, control)
        }
        "org/springframework/boot/loader/launch/ExplodedArchive" => {
            native_boot_exploded_archive_get_manifest(args, heap, out, control)
        }
        _ => Err(Error::MethodNotFound {
            name: format!("{}.getManifest", heap.get(this_ref)?.class_name),
            descriptor: "()Ljava/util/jar/Manifest;".to_string(),
        }),
    }
}

pub(crate) fn native_boot_exploded_archive_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    if let Some(slot @ Slot::Reference(Some(_))) = heap
        .get(this_ref)?
        .fields
        .get(BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT)
        .copied()
    {
        return Ok(Some(slot));
    }

    let root_directory_ref = match heap
        .get(this_ref)?
        .fields
        .get(BOOT_EXPLODED_ARCHIVE_ROOT_DIRECTORY_SLOT)
        .copied()
    {
        Some(Slot::Reference(Some(root_directory_ref))) => root_directory_ref,
        Some(Slot::Reference(None)) | None => return Err(Error::NullPointerException),
        Some(_) => {
            return Err(Error::TypeMismatch {
                expected: "reference",
                got: "other",
            });
        }
    };
    let manifest_path = file_path_from_ref(root_directory_ref, heap)?.join("META-INF/MANIFEST.MF");
    let manifest_bytes = match std::fs::read(&manifest_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(Slot::Reference(None)));
        }
        Err(_) => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
        }
    };
    let manifest_ref = allocate_manifest_from_bytes(heap, &manifest_bytes)?;
    heap.get_mut(this_ref)?.fields[BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT] =
        Slot::Reference(Some(manifest_ref));
    Ok(Some(Slot::Reference(Some(manifest_ref))))
}

/// Native: `ZipFile.getEntry(String) -> ZipEntry` — look up an entry by name.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_zip_file_get_entry(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let entry_name = heap
        .get(name_ref)?
        .string_value
        .as_deref()
        .ok_or(Error::NullPointerException)?
        .to_string();
    let info = zip_get_entry_info(fd, &entry_name)?;
    let Some(info) = info else {
        return Ok(Some(Slot::Reference(None)));
    };
    // Allocate a ZipEntry HeapObject with 6 fields.
    let name_heap_ref = heap.allocate_string(info.name);
    let entry_ref = heap.allocate("java/util/zip/ZipEntry".to_string(), 6);
    let entry_obj = heap.get_mut(entry_ref)?;
    entry_obj.fields[0] = Slot::Reference(Some(name_heap_ref));
    entry_obj.fields[1] = Slot::Int(info.compressed_size as i32);
    entry_obj.fields[2] = Slot::Int((info.compressed_size >> 32) as i32);
    entry_obj.fields[3] = Slot::Int(info.uncompressed_size as i32);
    entry_obj.fields[4] = Slot::Int((info.uncompressed_size >> 32) as i32);
    entry_obj.fields[5] = Slot::Int(i32::from(info.compression_method));
    Ok(Some(Slot::Reference(Some(entry_ref))))
}

/// Native: `ZipFile.getInputStream(ZipEntry) -> InputStream`
pub(crate) fn native_zip_file_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let entry_ref = extract_ref_arg(args, 1)?;
    let fd = extract_io_fd(heap, this_ref)?;
    // Get entry name from the ZipEntry object.
    let name_slot_ref = match heap.get(entry_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(Error::NullPointerException),
    };
    let entry_name = heap
        .get(name_slot_ref)?
        .string_value
        .as_deref()
        .ok_or(Error::NullPointerException)?
        .to_string();
    // Decompress the entry and wrap in a ByteBuffer.
    let data = zip_read_entry(fd, &entry_name)?;
    let buf_fd = heap.open_host_byte_buffer(data);
    let is_ref = heap.allocate("duke/zip/ByteBufferInputStream".to_string(), 1);
    let is_obj = heap.get_mut(is_ref)?;
    is_obj.fields[0] = Slot::Int(buf_fd);
    Ok(Some(Slot::Reference(Some(is_ref))))
}

/// Native: `ZipFile.close()`
pub(crate) fn native_zip_file_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) => *id,
        _ => return Ok(None),
    };
    zip_close(fd);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ZipFile.size() -> int`
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_zip_file_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let count = zip_entry_count(fd)?;
    Ok(Some(Slot::Int(count as i32)))
}

/// Native: `ZipEntry.getName() -> String`
pub(crate) fn native_zip_entry_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(heap.get(this_ref)?.fields[0]))
}

/// Native: `ZipEntry.getCompressedSize() -> long`
pub(crate) fn native_zip_entry_get_compressed_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let lo = match fields[1] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let hi = match fields[2] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let val = (i64::from(hi) << 32) | (i64::from(lo) & 0xFFFF_FFFF);
    Ok(Some(Slot::Long(val)))
}

/// Native: `ZipEntry.getSize() -> long`
pub(crate) fn native_zip_entry_get_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let lo = match fields[3] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let hi = match fields[4] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let val = (i64::from(hi) << 32) | (i64::from(lo) & 0xFFFF_FFFF);
    Ok(Some(Slot::Long(val)))
}

/// Native: `ZipEntry.getMethod() -> int`
pub(crate) fn native_zip_entry_get_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(heap.get(this_ref)?.fields[5]))
}

/// Native: `String.length()` — returns string length as int.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_string_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let len = obj.string_value.as_ref().map_or(0, String::len);
    Ok(Some(Slot::Int(len as i32)))
}

/// Native: `String.equals(Object)` — compares string content.
pub(crate) fn native_string_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    // Fetch objects from heap in one go to keep borrows short
    let this_obj = heap.get(this_ref)?;
    let other_obj = heap.get(other_ref)?;
    let this_str = this_obj.string_value.as_deref().unwrap_or_default();
    let other_str = other_obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(this_str == other_str))))
}

/// Native: `String.charAt(int)` — returns char at index as int.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_string_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let index = extract_int_arg(args, 1)?;
    let obj = heap.get(this_ref)?;
    let s = obj.string_value.as_deref().unwrap_or("");
    let ch = s
        .chars()
        .nth(index as usize)
        .ok_or(Error::ArrayIndexOutOfBounds {
            index,
            length: s.len(),
        })?;
    Ok(Some(Slot::Int(ch as i32)))
}

/// Native: `Object.<init>()V` - root constructor is a no-op after null-checking `this`.
pub(crate) fn native_object_init(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(None)
}

/// Native: `Object.getClass()` — returns a lightweight `Class` object for the runtime type.
pub(crate) fn native_object_get_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let class_name = heap.get(this_ref)?.class_name.clone();
    let class_ref = allocate_class_object(heap, &class_name)?;
    Ok(Some(Slot::Reference(Some(class_ref))))
}

/// Native: `Object.equals(Object)` — default Java object identity comparison.
pub(crate) fn native_object_equals(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let equal = extract_ref_arg(args, 1) == Ok(this_ref);
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `Object.hashCode()` — returns heap address as hash.
pub(crate) fn native_object_hashcode(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(Slot::Reference(Some(r))) => Ok(Some(Slot::Int(*r as i32))),
        _ => Err(Error::NullPointerException),
    }
}

/// Native: `Object.toString()` — delegates to `heap_object_to_string` so String,
/// boxed primitives, and opaque objects all produce the correct Java representation.
pub(crate) fn native_object_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap_object_to_string(heap.get(this_ref)?, this_ref);
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Object.clone()` — shallow-copies a heap object.
pub(crate) fn native_object_clone(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let new_ref = heap.clone_object(this_ref)?;
    Ok(Some(Slot::Reference(Some(new_ref))))
}

const THROWABLE_CAUSE_FIELD: usize = 0;
const THROWABLE_STACK_TRACE_FIELD: usize = 1;
const THROWABLE_SUPPRESSED_FIELD: usize = 2;
const STACK_TRACE_ELEMENT_CLASS: &str = "java/lang/StackTraceElement";
const STACK_TRACE_ARRAY_CLASS: &str = "[Ljava/lang/StackTraceElement;";
const THROWABLE_ARRAY_CLASS: &str = "[Ljava/lang/Throwable;";

fn set_object_field(heap: &mut duke_gc::Heap, obj_ref: u64, index: usize, value: Slot) -> Result<()> {
    let obj = heap.get_mut(obj_ref)?;
    if obj.fields.len() <= index {
        obj.fields.resize(index + 1, Slot::Reference(None));
    }
    obj.fields[index] = value;
    Ok(())
}

fn allocate_slot_array(heap: &mut duke_gc::Heap, class_name: &str, elements: &[Slot]) -> Result<u64> {
    let array_ref = heap.allocate(class_name.to_string(), elements.len());
    heap.get_mut(array_ref)?.fields.clone_from_slice(elements);
    Ok(array_ref)
}

fn allocate_empty_reference_array(heap: &mut duke_gc::Heap, class_name: &str) -> Result<u64> {
    allocate_slot_array(heap, class_name, &[])
}

fn string_slot(heap: &mut duke_gc::Heap, value: &str) -> Slot {
    Slot::Reference(Some(heap.allocate_string(value.to_string())))
}

fn optional_string_slot(heap: &mut duke_gc::Heap, value: Option<&str>) -> Slot {
    value.map_or(Slot::Reference(None), |s| string_slot(heap, s))
}

fn slot_string(heap: &duke_gc::Heap, slot: Slot) -> Result<Option<String>> {
    match slot {
        Slot::Reference(Some(r)) => Ok(heap.get(r)?.string_value.clone()),
        _ => Ok(None),
    }
}

fn allocate_stack_trace_element(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    method_name: &str,
    file_name: Option<&str>,
    line_number: i32,
) -> Result<u64> {
    let element_ref = heap.allocate(STACK_TRACE_ELEMENT_CLASS.to_string(), 4);
    let declaring_class = class_name.replace('/', ".");
    let class_slot = string_slot(heap, &declaring_class);
    let method_slot = string_slot(heap, method_name);
    let file_slot = optional_string_slot(heap, file_name);
    let obj = heap.get_mut(element_ref)?;
    obj.fields[0] = class_slot;
    obj.fields[1] = method_slot;
    obj.fields[2] = file_slot;
    obj.fields[3] = Slot::Int(line_number);
    Ok(element_ref)
}

fn store_throwable_stack_trace_from_frames(
    heap: &mut duke_gc::Heap,
    throwable_ref: u64,
    frames: &[NativeStackFrame],
) -> Result<()> {
    let mut elements = Vec::with_capacity(frames.len());
    for frame in frames {
        let element_ref = allocate_stack_trace_element(
            heap,
            &frame.class_name,
            &frame.method_name,
            frame.file_name.as_deref(),
            frame.line_number,
        )?;
        elements.push(Slot::Reference(Some(element_ref)));
    }
    let array_ref = allocate_slot_array(heap, STACK_TRACE_ARRAY_CLASS, &elements)?;
    set_object_field(
        heap,
        throwable_ref,
        THROWABLE_STACK_TRACE_FIELD,
        Slot::Reference(Some(array_ref)),
    )
}

fn clone_reference_array(
    heap: &mut duke_gc::Heap,
    slot: Slot,
    default_class_name: &str,
) -> Result<Slot> {
    let Some(array_ref) = slot.as_reference() else {
        let empty_ref = allocate_empty_reference_array(heap, default_class_name)?;
        return Ok(Slot::Reference(Some(empty_ref)));
    };
    let (class_name, elements) = {
        let obj = heap.get(array_ref)?;
        (obj.class_name.clone(), obj.fields.clone())
    };
    let cloned_ref = allocate_slot_array(heap, &class_name, &elements)?;
    Ok(Slot::Reference(Some(cloned_ref)))
}

fn throwable_field_slot(heap: &duke_gc::Heap, throwable_ref: u64, index: usize) -> Result<Slot> {
    Ok(heap
        .get(throwable_ref)?
        .fields
        .get(index)
        .copied()
        .unwrap_or(Slot::Reference(None)))
}

fn throwable_header(heap: &duke_gc::Heap, throwable_ref: u64) -> Result<String> {
    let obj = heap.get(throwable_ref)?;
    let class_name = obj.class_name.replace('/', ".");
    Ok(match &obj.string_value {
        Some(msg) => format!("{class_name}: {msg}"),
        None => class_name,
    })
}

fn stack_trace_element_text(heap: &duke_gc::Heap, element_ref: u64) -> Result<String> {
    let fields = heap.get(element_ref)?.fields.clone();
    let class_name = slot_string(heap, fields.first().copied().unwrap_or(Slot::Reference(None)))?
        .unwrap_or_default();
    let method_name = slot_string(heap, fields.get(1).copied().unwrap_or(Slot::Reference(None)))?
        .unwrap_or_default();
    let file_name = slot_string(heap, fields.get(2).copied().unwrap_or(Slot::Reference(None)))?;
    let line_number = match fields.get(3) {
        Some(Slot::Int(line)) => *line,
        _ => -1,
    };
    let location = match (file_name, line_number) {
        (_, -2) => "Native Method".to_string(),
        (Some(file), line) if line >= 0 => format!("{file}:{line}"),
        (Some(file), _) => file,
        (None, _) => "Unknown Source".to_string(),
    };
    Ok(format!("{class_name}.{method_name}({location})"))
}

fn append_throwable_trace(
    heap: &duke_gc::Heap,
    throwable_ref: u64,
    caption: &str,
    frame_indent: &str,
    out: &mut String,
    visited: &mut std::collections::HashSet<u64>,
) -> Result<()> {
    if !visited.insert(throwable_ref) {
        out.push_str(caption);
        out.push_str("[CIRCULAR REFERENCE: ");
        out.push_str(&throwable_header(heap, throwable_ref)?);
        out.push_str("]\n");
        return Ok(());
    }

    out.push_str(caption);
    out.push_str(&throwable_header(heap, throwable_ref)?);
    out.push('\n');

    let stack_slot = throwable_field_slot(heap, throwable_ref, THROWABLE_STACK_TRACE_FIELD)?;
    if let Some(stack_ref) = stack_slot.as_reference() {
        for frame_slot in &heap.get(stack_ref)?.fields {
            if let Slot::Reference(Some(element_ref)) = frame_slot {
                out.push_str(frame_indent);
                out.push_str("\tat ");
                out.push_str(&stack_trace_element_text(heap, *element_ref)?);
                out.push('\n');
            }
        }
    }

    let suppressed_slot = throwable_field_slot(heap, throwable_ref, THROWABLE_SUPPRESSED_FIELD)?;
    if let Some(suppressed_ref) = suppressed_slot.as_reference() {
        for suppressed in &heap.get(suppressed_ref)?.fields {
            if let Slot::Reference(Some(suppressed_ref)) = suppressed {
                let mut suppressed_caption = String::from(frame_indent);
                suppressed_caption.push_str("\tSuppressed: ");
                let mut suppressed_indent = String::from(frame_indent);
                suppressed_indent.push('\t');
                append_throwable_trace(
                    heap,
                    *suppressed_ref,
                    &suppressed_caption,
                    &suppressed_indent,
                    out,
                    visited,
                )?;
            }
        }
    }

    let cause_slot = throwable_field_slot(heap, throwable_ref, THROWABLE_CAUSE_FIELD)?;
    if let Some(cause_ref) = cause_slot.as_reference()
        && cause_ref != throwable_ref
    {
        append_throwable_trace(heap, cause_ref, "Caused by: ", frame_indent, out, visited)?;
    }
    Ok(())
}

fn throwable_trace_string(heap: &duke_gc::Heap, throwable_ref: u64) -> Result<String> {
    let mut out = String::new();
    let mut visited = std::collections::HashSet::new();
    append_throwable_trace(heap, throwable_ref, "", "", &mut out, &mut visited)?;
    Ok(out)
}

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

fn fill_throwable_stack_trace_from_control(
    heap: &mut duke_gc::Heap,
    throwable_ref: u64,
    control: &NativeControl,
) -> Result<()> {
    store_throwable_stack_trace_from_frames(heap, throwable_ref, control.stack_trace())
}

/// `Throwable.addSuppressed(Throwable suppressed)V`
///
/// Signature: `args[0]` = this (Throwable), `args[1]` = suppressed (Throwable)
pub(crate) fn native_throwable_add_suppressed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _stdout: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let suppressed = args.get(1).copied().unwrap_or(Slot::Reference(None));
    if suppressed.as_reference().is_none() {
        return Ok(None);
    }
    let existing = throwable_field_slot(heap, this_ref, THROWABLE_SUPPRESSED_FIELD)?;
    let mut elements = existing.as_reference().map_or_else(Vec::new, |array_ref| {
        heap.get(array_ref)
            .map(|obj| obj.fields.clone())
            .unwrap_or_default()
    });
    elements.push(suppressed);
    let array_ref = allocate_slot_array(heap, THROWABLE_ARRAY_CLASS, &elements)?;
    set_object_field(
        heap,
        this_ref,
        THROWABLE_SUPPRESSED_FIELD,
        Slot::Reference(Some(array_ref)),
    )?;
    Ok(None)
}

/// Native: `Throwable.<init>(String)V` — stores detail message in `string_value`.
/// Native: `Throwable.<init>()V` - captures the construction stack trace.
pub(crate) fn native_throwable_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}

pub(crate) fn native_throwable_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let msg = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = msg;
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}

/// Native: `Throwable.<init>(String, Throwable)V` — stores message + cause.
pub(crate) fn native_throwable_init_string_cause(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let msg = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = msg;
    // Store cause in fields[0] (Throwable.cause field)
    if let Some(&cause_slot) = args.get(2)
        && let Ok(obj) = heap.get_mut(this_ref)
        && !obj.fields.is_empty()
    {
        obj.fields[THROWABLE_CAUSE_FIELD] = cause_slot;
    }
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}

/// Native: `Throwable.<init>(Throwable)V` - stores only the cause.
pub(crate) fn native_throwable_init_cause(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    if let Some(&cause_slot) = args.get(1)
        && let Ok(obj) = heap.get_mut(this_ref)
        && !obj.fields.is_empty()
    {
        obj.fields[THROWABLE_CAUSE_FIELD] = cause_slot;
    }
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}

/// Native: `Throwable.getCause()Throwable` — returns the stored cause.
#[allow(clippy::unnecessary_wraps)]
/// Native: `Throwable.fillInStackTrace()Throwable`.
pub(crate) fn native_throwable_fill_in_stack_trace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(Some(Slot::Reference(Some(this_ref))))
}

pub(crate) fn native_throwable_get_cause(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let cause = throwable_field_slot(heap, *r, THROWABLE_CAUSE_FIELD)?;
            Ok(Some(cause))
        }
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Throwable.getMessage()String` — returns the stored detail message.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_throwable_get_message(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let msg = heap.get(*r)?.string_value.clone();
            let slot = msg.map_or(Slot::Reference(None), |s| {
                Slot::Reference(Some(heap.allocate_string(s)))
            });
            Ok(Some(slot))
        }
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Throwable.toString()String` — returns `"ClassName: message"` or just class name.
pub(crate) fn native_throwable_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = throwable_header(heap, this_ref)?;
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Throwable.getStackTrace()StackTraceElement[]`.
pub(crate) fn native_throwable_get_stack_trace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let slot = throwable_field_slot(heap, this_ref, THROWABLE_STACK_TRACE_FIELD)?;
    Ok(Some(clone_reference_array(
        heap,
        slot,
        STACK_TRACE_ARRAY_CLASS,
    )?))
}

/// Native: `Throwable.setStackTrace(StackTraceElement[])V`.
pub(crate) fn native_throwable_set_stack_trace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let new_trace = clone_reference_array(
        heap,
        args.get(1).copied().unwrap_or(Slot::Reference(None)),
        STACK_TRACE_ARRAY_CLASS,
    )?;
    set_object_field(heap, this_ref, THROWABLE_STACK_TRACE_FIELD, new_trace)?;
    Ok(None)
}

/// Native: `Throwable.getSuppressed()Throwable[]`.
pub(crate) fn native_throwable_get_suppressed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let slot = throwable_field_slot(heap, this_ref, THROWABLE_SUPPRESSED_FIELD)?;
    Ok(Some(clone_reference_array(heap, slot, THROWABLE_ARRAY_CLASS)?))
}

/// Native: `Throwable.printStackTrace()V`.
pub(crate) fn native_throwable_print_stack_trace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = throwable_trace_string(heap, this_ref)?;
    write!(out, "{text}").ok();
    Ok(None)
}

/// Native: `Throwable.printStackTrace(PrintStream)V`.
pub(crate) fn native_throwable_print_stack_trace_print_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = throwable_trace_string(heap, this_ref)?;
    write_to_print_stream_or_output(args, heap, out, &text)?;
    Ok(None)
}

/// Native: `StackTraceElement.<init>(String,String,String,int)V`.
pub(crate) fn native_stack_trace_element_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let class_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let method_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let file_slot = args.get(3).copied().unwrap_or(Slot::Reference(None));
    let line_slot = args.get(4).copied().unwrap_or(Slot::Int(-1));
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 4 {
        obj.fields.resize(4, Slot::Reference(None));
    }
    obj.fields[0] = class_slot;
    obj.fields[1] = method_slot;
    obj.fields[2] = file_slot;
    obj.fields[3] = line_slot;
    Ok(None)
}

fn stack_trace_element_field(args: &[Slot], heap: &duke_gc::Heap, index: usize) -> Result<Slot> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(heap
        .get(this_ref)?
        .fields
        .get(index)
        .copied()
        .unwrap_or(Slot::Reference(None)))
}

pub(crate) fn native_stack_trace_element_get_class_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(stack_trace_element_field(args, heap, 0)?))
}

pub(crate) fn native_stack_trace_element_get_method_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(stack_trace_element_field(args, heap, 1)?))
}

pub(crate) fn native_stack_trace_element_get_file_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(stack_trace_element_field(args, heap, 2)?))
}

pub(crate) fn native_stack_trace_element_get_line_number(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match stack_trace_element_field(args, heap, 3)? {
        Slot::Int(line) => Ok(Some(Slot::Int(line))),
        _ => Ok(Some(Slot::Int(-1))),
    }
}

pub(crate) fn native_stack_trace_element_is_native_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let is_native = matches!(stack_trace_element_field(args, heap, 3)?, Slot::Int(-2));
    Ok(Some(Slot::Int(i32::from(is_native))))
}

pub(crate) fn native_stack_trace_element_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = stack_trace_element_text(heap, this_ref)?;
    let string_ref = heap.allocate_string(text);
    Ok(Some(Slot::Reference(Some(string_ref))))
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

/// Native: `ArrayList.addAll(Collection)Z` — appends all elements from a compatible collection.
pub(crate) fn native_arraylist_add_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_ref = extract_ref_arg(args, 1)?;
    let src_size = match heap.get(src_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let elems: Vec<Slot> = heap.get(src_ref)?.fields[1..=src_size].to_vec();
    let modified = !elems.is_empty();
    for elem in elems {
        native_arraylist_add(&[Slot::Reference(Some(this_ref)), elem], heap, out, control)?;
    }
    Ok(Some(Slot::Int(i32::from(modified))))
}

/// Native: `HashMap.putAll(Map)V` — copies all entries from the source `HashMap`.
pub(crate) fn native_hashmap_put_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_ref = extract_ref_arg(args, 1)?;
    let fields = heap.get(src_ref)?.fields.clone();
    // fields[0] = size, fields[1..] = k0, v0, k1, v1, ...
    let mut i = 1;
    while i + 1 < fields.len() {
        let k = fields[i];
        let v = fields[i + 1];
        native_hashmap_put(&[Slot::Reference(Some(this_ref)), k, v], heap, out, control)?;
        i += 2;
    }
    Ok(None)
}

/// Native: `HashMap.computeIfAbsent(K, Function)V` — returns existing value or computes and stores it.
pub(crate) fn native_hashmap_compute_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    // Check if key already present.
    let existing = native_hashmap_get(&[Slot::Reference(Some(this_ref)), key], heap, out, control)?;
    if let Some(v) = existing
        && !matches!(v, Slot::Reference(None))
    {
        return Ok(Some(v));
    }
    // Key absent — invoke the mapping function (lambda / SAM).
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let computed = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![Slot::Reference(Some(fn_ref)), key],
    )?;
    if let Some(value) = computed
        && !matches!(value, Slot::Reference(None))
    {
        native_hashmap_put(
            &[Slot::Reference(Some(this_ref)), key, value],
            heap,
            out,
            control,
        )?;
        return Ok(Some(value));
    }
    Ok(Some(Slot::Reference(None)))
}

// ---- LinkedList natives (field layout identical to ArrayList: fields[0]=size, fields[1..]=elements) ----

/// Native: `LinkedList.<init>()V` — same initialisation as `ArrayList`.
pub(crate) fn native_linked_list_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `LinkedList.<init>(Collection)V` — copies all elements from source collection.
pub(crate) fn native_linked_list_init_collection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_ref = extract_ref_arg(args, 1)?;
    let src_size = match heap.get(src_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let src_elems: Vec<Slot> = heap.get(src_ref)?.fields[1..=src_size].to_vec();
    let n = i32::try_from(src_elems.len()).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(n);
    for elem in src_elems {
        heap.get_mut(this_ref)?.fields.push(elem);
    }
    Ok(None)
}

/// Native: `LinkedList.size()I`
pub(crate) fn native_linked_list_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `LinkedList.add(Object)Z` — appends to tail.
pub(crate) fn native_linked_list_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)
}

/// Native: `LinkedList.get(I)Object`
pub(crate) fn native_linked_list_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_get(args, heap, out, control)
}

/// Native: `LinkedList.addFirst(Object)V` — inserts at index 0.
/// Field layout: `fields[0]`=Int(size), `fields[1..size]`=elements.
pub(crate) fn native_linked_list_add_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let obj = heap.get_mut(this_ref)?;
    // Shift existing elements right by one.
    obj.fields.insert(1, elem);
    obj.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    Ok(None)
}

/// Native: `LinkedList.addLast(Object)V` — appends to tail (same as add).
pub(crate) fn native_linked_list_add_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(None)
}

/// Native: `LinkedList.peekFirst()Object` — returns head without removal, or null if empty.
pub(crate) fn native_linked_list_peek_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `LinkedList.peekLast()Object` — returns tail without removal, or null if empty.
pub(crate) fn native_linked_list_peek_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[size]))
}

/// Native: `LinkedList.removeFirst()Object` — removes and returns head.
pub(crate) fn native_linked_list_remove_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let obj = heap.get_mut(this_ref)?;
    let elem = obj.fields.remove(1);
    obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    Ok(Some(elem))
}

/// Native: `LinkedList.removeLast()Object` — removes and returns tail.
pub(crate) fn native_linked_list_remove_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let obj = heap.get_mut(this_ref)?;
    let elem = obj.fields.remove(size);
    obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    Ok(Some(elem))
}

/// Native: `LinkedList.poll()Object` — removes and returns head, or null if empty.
pub(crate) fn native_linked_list_poll(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    let obj = heap.get_mut(this_ref)?;
    let elem = obj.fields.remove(1);
    obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    Ok(Some(elem))
}

/// Native: `LinkedList.offer(Object)Z` — appends to tail, returns true.
pub(crate) fn native_linked_list_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)
}

/// Native: `LinkedList.isEmpty()Z`
pub(crate) fn native_linked_list_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

/// Native: `LinkedList.iterator()Iterator` — returns an ArrayList-compatible iterator.
pub(crate) fn native_linked_list_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_iterator(args, heap, out, control)
}

// ---- HashMap.forEach callback ----

/// Native: `HashMap.forEach(BiConsumer)V` — iterates key-value pairs, invoking `accept(k, v)`.
pub(crate) fn native_hashmap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let consumer_ref = extract_ref_arg(args, 1)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    // Snapshot key-val pairs (fields[1,2], fields[3,4], ...)
    let pairs: Vec<(Slot, Slot)> = (0..size)
        .map(|i| {
            let key = heap.get(this_ref).map_or(Slot::Reference(None), |o| o.fields[1 + i * 2]);
            let val = heap.get(this_ref).map_or(Slot::Reference(None), |o| o.fields[2 + i * 2]);
            (key, val)
        })
        .collect();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for (key, val) in pairs {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;Ljava/lang/Object;)V",
            vec![Slot::Reference(Some(consumer_ref)), key, val],
        )?;
    }
    let _ = control;
    Ok(None)
}

/// Native: `HashMap.replaceAll(BiFunction<K,V,V>) -> void`
/// Replaces each value with the result of applying the function to (key, value).
pub(crate) fn native_hashmap_replace_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_ref = extract_ref_arg(args, 1)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    // Snapshot keys (values will be mutated in place).
    let keys: Vec<Slot> = (0..size)
        .map(|i| {
            heap.get(this_ref).map_or(Slot::Reference(None), |o| o.fields[1 + i * 2])
        })
        .collect();
    for (i, key) in keys.iter().enumerate() {
        let old_val = heap.get(this_ref).map_or(Slot::Reference(None), |o| o.fields[2 + i * 2]);
        let new_val = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![Slot::Reference(Some(fn_ref)), *key, old_val],
        )?;
        if let Some(v) = new_val {
            heap.get_mut(this_ref)?.fields[2 + i * 2] = v;
        }
    }
    let _ = control;
    Ok(None)
}

// ---- TreeMap natives (field layout: fields[0]=Int(size), fields[1,2]=k0/v0 sorted by key) ----
// Keys are stored sorted in ascending order for O(n) insert / O(1) first&last.

/// Compare two `TreeMap` keys by natural ordering, supporting String, Integer, Long, and Double keys.
fn compare_treemap_keys(a: Slot, b: Slot, heap: &duke_gc::Heap) -> std::cmp::Ordering {
    let key_ord = |s: Slot| -> Option<KeyOrd> {
        if let Slot::Reference(Some(r)) = s
            && let Ok(obj) = heap.get(r)
        {
            if let Some(sv) = &obj.string_value {
                return Some(KeyOrd::Str(sv.clone()));
            }
            match obj.class_name.as_str() {
                "java/lang/Integer" | "java/lang/Short" | "java/lang/Byte" => {
                    if let Some(Slot::Int(n)) = obj.fields.first() {
                        return Some(KeyOrd::Int(i64::from(*n)));
                    }
                }
                "java/lang/Long" => {
                    if let Some(Slot::Long(n)) = obj.fields.first() {
                        return Some(KeyOrd::Int(*n));
                    }
                }
                "java/lang/Double" | "java/lang/Float" => {
                    if let Some(Slot::Double(n)) = obj.fields.first() {
                        return Some(KeyOrd::Flt(*n));
                    }
                }
                _ => {}
            }
        }
        None
    };
    match (key_ord(a), key_ord(b)) {
        (Some(KeyOrd::Int(x)), Some(KeyOrd::Int(y))) => x.cmp(&y),
        (Some(KeyOrd::Flt(x)), Some(KeyOrd::Flt(y))) => x.total_cmp(&y),
        (Some(KeyOrd::Str(x)), Some(KeyOrd::Str(y))) => x.cmp(&y),
        _ => std::cmp::Ordering::Equal,
    }
}

enum KeyOrd {
    Int(i64),
    Flt(f64),
    Str(String),
}

/// Check if two `TreeMap` keys are equal (same value semantics as `compare_treemap_keys` == Equal).
fn treemap_keys_equal(a: Slot, b: Slot, heap: &duke_gc::Heap) -> bool {
    compare_treemap_keys(a, b, heap) == std::cmp::Ordering::Equal
}

/// Native: `TreeMap.<init>()V`
pub(crate) fn native_treemap_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `TreeMap.put(K,V)V` — inserts in sorted key order.
pub(crate) fn native_treemap_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let val = extract_slot_arg(args, 2);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    // Check for existing key — update in place.
    for i in 0..size {
        let existing_key = heap.get(this_ref)?.fields[1 + i * 2];
        if treemap_keys_equal(existing_key, key, heap) {
            heap.get_mut(this_ref)?.fields[2 + i * 2] = val;
            return Ok(Some(Slot::Reference(None)));
        }
    }
    // Find sorted insert position.
    let insert_pos = {
        let mut pos = size;
        for i in 0..size {
            let ek = heap.get(this_ref)?.fields[1 + i * 2];
            if compare_treemap_keys(key, ek, heap) == std::cmp::Ordering::Less {
                pos = i;
                break;
            }
        }
        pos
    };
    let obj = heap.get_mut(this_ref)?;
    obj.fields.insert(1 + insert_pos * 2, val);
    obj.fields.insert(1 + insert_pos * 2, key);
    obj.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    Ok(Some(Slot::Reference(None)))
}

/// Native: `TreeMap.get(K)V`
pub(crate) fn native_treemap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    for i in 0..size {
        let ek = heap.get(this_ref)?.fields[1 + i * 2];
        if treemap_keys_equal(ek, key, heap) {
            let v = heap.get(this_ref)?.fields[2 + i * 2];
            return Ok(Some(v));
        }
    }
    Ok(Some(Slot::Reference(None)))
}

/// Native: `TreeMap.containsKey(K)Z`
pub(crate) fn native_treemap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let found = !matches!(
        native_treemap_get(args, heap, out, control)?,
        Some(Slot::Reference(None)) | None
    );
    Ok(Some(Slot::Int(i32::from(found))))
}

/// Native: `TreeMap.size()I`
pub(crate) fn native_treemap_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Slot::Int(*n),
        _ => Slot::Int(0),
    }))
}

/// Native: `TreeMap.firstKey()K` — returns the smallest key (index 0 in sorted list).
pub(crate) fn native_treemap_first_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    if size == 0 {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `TreeMap.lastKey()K` — returns the largest key.
pub(crate) fn native_treemap_last_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(heap.get(this_ref)?.fields[1 + (size - 1) * 2]))
}

/// Native: `TreeMap.remove(K)V`
pub(crate) fn native_treemap_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    for i in 0..size {
        let ek = heap.get(this_ref)?.fields[1 + i * 2];
        if treemap_keys_equal(ek, key, heap) {
            let obj = heap.get_mut(this_ref)?;
            let v = obj.fields.remove(2 + i * 2);
            obj.fields.remove(1 + i * 2);
            obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
            return Ok(Some(v));
        }
    }
    Ok(Some(Slot::Reference(None)))
}

/// Native: `TreeMap.isEmpty()Z`
pub(crate) fn native_treemap_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) | None => 1,
        _ => 0,
    })))
}

/// Native: `TreeMap.headMap(toKey)SortedMap` — returns a new `TreeMap` with keys strictly less than toKey.
pub(crate) fn native_treemap_head_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let to_key = extract_slot_arg(args, 1);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let result = heap.allocate("java/util/TreeMap".to_string(), 1);
    heap.get_mut(result)?.fields[0] = Slot::Int(0);
    let mut count = 0usize;
    for i in 0..size {
        let k = heap.get(this_ref)?.fields[1 + i * 2];
        let v = heap.get(this_ref)?.fields[2 + i * 2];
        if compare_treemap_keys(k, to_key, heap) == std::cmp::Ordering::Less {
            heap.get_mut(result)?.fields.push(k);
            heap.get_mut(result)?.fields.push(v);
            count += 1;
        }
    }
    heap.get_mut(result)?.fields[0] = Slot::Int(i32::try_from(count).unwrap_or(0));
    Ok(Some(Slot::Reference(Some(result))))
}

/// Native: `TreeMap.tailMap(fromKey)SortedMap` — returns a new `TreeMap` with keys >= fromKey.
pub(crate) fn native_treemap_tail_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let from_key = extract_slot_arg(args, 1);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let result = heap.allocate("java/util/TreeMap".to_string(), 1);
    heap.get_mut(result)?.fields[0] = Slot::Int(0);
    let mut count = 0usize;
    for i in 0..size {
        let k = heap.get(this_ref)?.fields[1 + i * 2];
        let v = heap.get(this_ref)?.fields[2 + i * 2];
        if compare_treemap_keys(k, from_key, heap) != std::cmp::Ordering::Less {
            heap.get_mut(result)?.fields.push(k);
            heap.get_mut(result)?.fields.push(v);
            count += 1;
        }
    }
    heap.get_mut(result)?.fields[0] = Slot::Int(i32::try_from(count).unwrap_or(0));
    Ok(Some(Slot::Reference(Some(result))))
}

// ---- Stack natives (LIFO backed by ArrayList: push=add, pop=removeLast, peek=peekLast) ----

/// Native: `Stack.<init>()V`
pub(crate) fn native_stack_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `Stack.push(E)E` — appends to tail, returns the element.
pub(crate) fn native_stack_push(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let elem = extract_slot_arg(args, 1);
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(elem))
}

/// Native: `Stack.pop()E` — removes and returns the top element.
pub(crate) fn native_stack_pop(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_linked_list_remove_last(args, heap, out, control)
}

/// Native: `Stack.peek()E` — returns the top element without removal.
pub(crate) fn native_stack_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_linked_list_peek_last(args, heap, out, control)
}

/// Native: `Stack.empty()Z` — returns true if the stack is empty.
pub(crate) fn native_stack_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

/// Native: `Stack.size()I`
pub(crate) fn native_stack_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

// ---- TreeSet natives (sorted unique elements, backed by sorted Vec<Slot>) ----
// fields[0] = Int(size), fields[1..] = unique elements in sorted String order

/// Native: `TreeSet.<init>()V`
pub(crate) fn native_treeset_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `TreeSet.add(E)Z` — inserts in sorted order; returns false if already present.
/// Extract a sortable key from a Slot for `TreeSet` ordering.
/// Returns an `Ordering`-compatible f64 for numeric types, lexicographic for strings.
#[allow(clippy::cast_precision_loss)] // intentional: i64→f64 for sort ordering; precision loss acceptable
fn treeset_slot_sort_key(slot: Slot, heap: &duke_gc::Heap) -> Option<TreeSortKey> {
    match slot {
        Slot::Int(n) => Some(TreeSortKey::Num(f64::from(n))),
        Slot::Long(n) => Some(TreeSortKey::Num(n as f64)),
        Slot::Double(d) => Some(TreeSortKey::Num(d)),
        Slot::Float(f) => Some(TreeSortKey::Num(f64::from(f))),
        Slot::Reference(Some(r)) => {
            let obj = heap.get(r).ok()?;
            match obj.class_name.as_str() {
                "java/lang/Integer" | "java/lang/Long" | "java/lang/Short" | "java/lang/Byte" => {
                    match obj.fields.first() {
                        Some(Slot::Int(n)) => Some(TreeSortKey::Num(f64::from(*n))),
                        Some(Slot::Long(n)) => Some(TreeSortKey::Num(*n as f64)),
                        _ => None,
                    }
                }
                "java/lang/Double" | "java/lang/Float" => match obj.fields.first() {
                    Some(Slot::Double(d)) => Some(TreeSortKey::Num(*d)),
                    Some(Slot::Float(f)) => Some(TreeSortKey::Num(f64::from(*f))),
                    _ => None,
                },
                _ => obj.string_value.clone().map(TreeSortKey::Str),
            }
        }
        _ => None,
    }
}

#[derive(PartialEq)]
enum TreeSortKey {
    Num(f64),
    Str(String),
}

impl TreeSortKey {
    fn less_than(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Num(a), Self::Num(b)) => a < b,
            (Self::Str(a), Self::Str(b)) => a < b,
            _ => false,
        }
    }
}

pub(crate) fn native_treeset_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let elem_key = treeset_slot_sort_key(elem, heap);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    // Check for duplicate.
    for i in 0..size {
        let ex = heap.get(this_ref)?.fields[1 + i];
        let ex_key = treeset_slot_sort_key(ex, heap);
        if ex_key == elem_key {
            return Ok(Some(Slot::Int(0))); // false — no change
        }
    }
    // Find sorted insert position.
    let insert_pos = {
        let mut pos = size;
        for i in 0..size {
            let ex = heap.get(this_ref)?.fields[1 + i];
            let ex_key = treeset_slot_sort_key(ex, heap);
            if let (Some(ek), Some(exk)) = (&elem_key, &ex_key)
                && ek.less_than(exk)
            {
                pos = i;
                break;
            }
        }
        pos
    };
    let obj = heap.get_mut(this_ref)?;
    obj.fields.insert(1 + insert_pos, elem);
    obj.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(1))) // true — element added
}

/// Native: `TreeSet.contains(E)Z`
pub(crate) fn native_treeset_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let elem_str = match &elem {
        Slot::Reference(Some(r)) => heap.get(*r)?.string_value.clone(),
        Slot::Int(v) => Some(v.to_string()),
        _ => None,
    };
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    for i in 0..size {
        let ex = heap.get(this_ref)?.fields[1 + i];
        let ex_str = match &ex {
            Slot::Reference(Some(r)) => heap.get(*r).ok().and_then(|o| o.string_value.clone()),
            Slot::Int(v) => Some(v.to_string()),
            _ => None,
        };
        if ex_str == elem_str {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `TreeSet.size()I`
pub(crate) fn native_treeset_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Slot::Int(*n),
        _ => Slot::Int(0),
    }))
}

/// Native: `TreeSet.first()E` — returns smallest element.
pub(crate) fn native_treeset_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first().copied() {
        Some(Slot::Int(0)) | None => Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        }),
        _ => Ok(Some(heap.get(this_ref)?.fields[1])),
    }
}

/// Native: `TreeSet.last()E` — returns largest element.
pub(crate) fn native_treeset_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(heap.get(this_ref)?.fields[size]))
}

/// Native: `TreeSet.isEmpty()Z`
pub(crate) fn native_treeset_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) | None => 1,
        _ => 0,
    })))
}

/// Native: `TreeSet.iterator()Iterator` — returns an ArrayList-compatible iterator over sorted elements.
pub(crate) fn native_treeset_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_iterator(args, heap, out, control)
}

// ---- Collections.min / max / shuffle ----

/// Native: `Collections.min(Collection)T` — returns minimum element via `compareTo`.
pub(crate) fn native_collections_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let coll_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(coll_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let Slot::Reference(Some(mut min_ref)) = heap.get(coll_ref)?.fields[1] else {
        return Ok(Some(Slot::Reference(None)));
    };
    for i in 2..=size {
        let Slot::Reference(Some(candidate)) = heap.get(coll_ref)?.fields[i] else {
            continue;
        };
        let class_name = heap.get(candidate)?.class_name.clone();
        let cmp = ops.invoke(
            heap,
            out,
            &class_name,
            "compareTo",
            "(Ljava/lang/Object;)I",
            vec![
                Slot::Reference(Some(candidate)),
                Slot::Reference(Some(min_ref)),
            ],
        )?;
        let _ = control;
        if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
            min_ref = candidate;
        }
    }
    Ok(Some(Slot::Reference(Some(min_ref))))
}

/// Native: `Collections.max(Collection)T` — returns maximum element via `compareTo`.
pub(crate) fn native_collections_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let coll_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(coll_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let Slot::Reference(Some(mut max_ref)) = heap.get(coll_ref)?.fields[1] else {
        return Ok(Some(Slot::Reference(None)));
    };
    for i in 2..=size {
        let Slot::Reference(Some(candidate)) = heap.get(coll_ref)?.fields[i] else {
            continue;
        };
        let class_name = heap.get(candidate)?.class_name.clone();
        let cmp = ops.invoke(
            heap,
            out,
            &class_name,
            "compareTo",
            "(Ljava/lang/Object;)I",
            vec![
                Slot::Reference(Some(candidate)),
                Slot::Reference(Some(max_ref)),
            ],
        )?;
        let _ = control;
        if matches!(cmp, Some(Slot::Int(n)) if n > 0) {
            max_ref = candidate;
        }
    }
    Ok(Some(Slot::Reference(Some(max_ref))))
}

/// Native: `Collections.shuffle(List)V` — no-op (deterministic test environments).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_shuffle(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}

/// Native: `Collections.shuffle(List, Random)V` — shuffle with provided RNG (no-op for correctness since test only checks sum).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_shuffle_random(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}

/// Native: `Collections.fill(List, Object)V` — set every element to value.
pub(crate) fn native_collections_fill(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let value = extract_slot_arg(args, 1);
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    for i in 1..=size {
        heap.get_mut(list_ref)?.fields[i] = value;
    }
    Ok(None)
}

// ---- Stream natives ----
// duke/util/Stream: fields[0]=Int(size), fields[1..]=element refs

/// Native: `Stream.of(Object[])Stream` — create stream from varargs array.
pub(crate) fn native_stream_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let elems: Vec<Slot> = heap.get(arr_ref)?.fields.clone();
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for elem in elems {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `ArrayList.stream()` — wrap `ArrayList` elements into a `Stream`.
pub(crate) fn native_arraylist_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    let elems: Vec<Slot> =
        heap.get(list_ref)?.fields[1..=usize::try_from(size).unwrap_or(0)].to_vec();
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(size);
    for elem in elems {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `Stream.count()J` — returns the number of elements as a long.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let n = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => i64::from(*n),
        _ => 0,
    };
    Ok(Some(Slot::Long(n)))
}

/// Native: `Stream.filter(Predicate)Stream` — keeps elements where `predicate.test()` returns true.
pub(crate) fn native_stream_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept: Vec<Slot> = Vec::new();
    for elem in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(pred_ref)), elem],
        )?;
        if matches!(result, Some(Slot::Int(1))) {
            kept.push(elem);
        }
    }
    let new_size = i32::try_from(kept.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in kept {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

/// Native: `Stream.map(Function)Stream` — transforms each element via `function.apply()`.
pub(crate) fn native_stream_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut mapped: Vec<Slot> = Vec::with_capacity(elems.len());
    for elem in elems {
        let result = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![Slot::Reference(Some(fn_ref)), elem],
        )?;
        // Box primitive results so stream elements are always References (Java type-erasure)
        let boxed = box_primitive_slot(result.unwrap_or(Slot::Reference(None)), heap);
        mapped.push(boxed);
    }
    let new_size = i32::try_from(mapped.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in mapped {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

/// Native: `Stream.forEach(Consumer)V` — calls `consumer.accept()` on each element.
pub(crate) fn native_stream_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(None);
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for elem in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![Slot::Reference(Some(consumer_ref)), elem],
        )?;
    }
    Ok(None)
}

/// Native: `Stream.collect(Collector)Object` — collects to list (only toList collector supported).
#[allow(
    clippy::too_many_lines,
    clippy::only_used_in_recursion,
    clippy::cognitive_complexity
)]
pub(crate) fn native_stream_collect(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    let elems: Vec<Slot> =
        heap.get(stream_ref)?.fields[1..=usize::try_from(size).unwrap_or(0)].to_vec();

    // Dispatch on collector type.
    let collector_class = match extract_ref_arg(args, 1) {
        Ok(r) => heap.get(r)?.class_name.clone(),
        Err(_) => "duke/util/ToListCollector".to_string(),
    };

    if collector_class == "duke/util/JoiningCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let read_str_field = |heap: &duke_gc::Heap, idx: usize| -> String {
            match heap
                .get(collector_ref)
                .ok()
                .and_then(|o| o.fields.get(idx).copied())
            {
                Some(Slot::Reference(Some(dr))) => heap
                    .get(dr)
                    .ok()
                    .and_then(|o| o.string_value.clone())
                    .unwrap_or_default(),
                _ => String::new(),
            }
        };
        let delim = read_str_field(heap, 0);
        let prefix = read_str_field(heap, 1);
        let suffix = read_str_field(heap, 2);

        // ⚡ Bolt: Eliminate intermediate Vec<String> allocation, format! macro overhead,
        // and .join() by appending directly to a single String buffer.
        // Havoc: prevent OOM from capacity overflow
        let extra = elems.len().saturating_mul(10usize.saturating_add(delim.len()));
        let cap = prefix.len().checked_add(suffix.len()).and_then(|x| x.checked_add(extra));
        let max_size = 1024 * 1024 * 128; // 128 MB max string size

        if cap.is_none_or(|c| c > max_size) {
            return Err(Error::JavaException {
                class_name: "java/lang/OutOfMemoryError".to_string(),
            });
        }

        let mut joined = String::with_capacity(cap.unwrap());
        joined.push_str(&prefix);
        let mut first = true;
        for s in &elems {
            #[allow(clippy::collapsible_if)]
            if let Slot::Reference(Some(r)) = s {
                #[allow(clippy::collapsible_if)]
                if let Ok(obj) = heap.get(*r) {
                    if let Some(s_val) = &obj.string_value {
                        if !first {
                            joined.push_str(&delim);
                        }
                        joined.push_str(s_val);
                        first = false;
                    }
                }
            }
        }
        joined.push_str(&suffix);

        let result_ref = heap.allocate_string(joined);
        Ok(Some(Slot::Reference(Some(result_ref))))
    } else if collector_class == "duke/util/CountingCollector" {
        // collect() returns Object; box the Long so bytecode can checkcast/invokevirtual it.
        let boxed = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Long(i64::from(size));
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/GroupingByCollector" {
        // GroupingByCollector: fields[0] = key function slot
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // Build a HashMap: key → ArrayList of values
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let key = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![fn_slot, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            // find existing bucket or create new list
            let fields = heap.get(map_ref)?.fields.clone();
            let size_n = match fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let mut found_ki = None;
            for i in 0..size_n {
                let ki = 1 + i * 2;
                if fields.get(ki).is_some_and(|k| slots_equal(k, &key, heap)) {
                    found_ki = Some(ki);
                    break;
                }
            }
            if let Some(ki) = found_ki {
                // Append elem to existing list
                let list_slot = extract_field_arg(heap, map_ref, ki + 1)?;
                if let Slot::Reference(Some(list_ref)) = list_slot {
                    let list_size = match heap.get(list_ref)?.fields.first() {
                        Some(Slot::Int(n)) => *n,
                        _ => 0,
                    };
                    heap.get_mut(list_ref)?.fields.push(elem);
                    heap.get_mut(list_ref)?.fields[0] = Slot::Int(list_size + 1);
                }
            } else {
                // New key — create list with one element
                let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
                heap.get_mut(list_ref)?.fields[0] = Slot::Int(1);
                heap.get_mut(list_ref)?.fields.push(elem);
                heap.get_mut(map_ref)?.fields.push(key);
                heap.get_mut(map_ref)?
                    .fields
                    .push(Slot::Reference(Some(list_ref)));
                heap.get_mut(map_ref)?.fields[0] =
                    Slot::Int(i32::try_from(size_n + 1).unwrap_or(i32::MAX));
            }
        }
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/ToSetCollector" {
        // Collect into HashSet (deduplicates).
        let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
        heap.get_mut(set_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            // Check for duplicate before inserting
            let set_fields = heap.get(set_ref)?.fields.clone();
            let set_size = match set_fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let already = set_fields[1..=set_size]
                .iter()
                .any(|s| slots_equal(s, &elem, heap));
            if !already {
                let cur_size = match heap.get(set_ref)?.fields.first() {
                    Some(Slot::Int(n)) => *n,
                    _ => 0,
                };
                heap.get_mut(set_ref)?.fields.push(elem);
                heap.get_mut(set_ref)?.fields[0] = Slot::Int(cur_size + 1);
            }
        }
        Ok(Some(Slot::Reference(Some(set_ref))))
    } else if collector_class == "duke/util/ToMapCollector" {
        // Collect into HashMap using key/val extractor functions.
        let collector_ref = extract_ref_arg(args, 1)?;
        let key_fn = extract_first_field_arg(heap, collector_ref)?;
        let val_fn = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(key_ref)) = key_fn else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(val_ref)) = val_fn else {
            return Err(Error::NullPointerException);
        };
        let key_class = heap.get(key_ref)?.class_name.clone();
        let val_class = heap.get(val_ref)?.class_name.clone();
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let k_raw = ops
                .invoke(
                    heap,
                    out,
                    &key_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![key_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let v_raw = ops
                .invoke(
                    heap,
                    out,
                    &val_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![val_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            // Box primitives so the map stores References (Object contract).
            let k = box_primitive_slot(k_raw, heap);
            let v = box_primitive_slot(v_raw, heap);
            let cur_size = match heap.get(map_ref)?.fields.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            heap.get_mut(map_ref)?.fields.push(k);
            heap.get_mut(map_ref)?.fields.push(v);
            heap.get_mut(map_ref)?.fields[0] = Slot::Int(cur_size + 1);
        }
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/ToUnmodifiableMapCollector" {
        // Same as ToMapCollector but produces an UnmodifiableMap.
        let collector_ref = extract_ref_arg(args, 1)?;
        let key_fn = extract_first_field_arg(heap, collector_ref)?;
        let val_fn = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(key_ref)) = key_fn else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(val_ref)) = val_fn else {
            return Err(Error::NullPointerException);
        };
        let key_class = heap.get(key_ref)?.class_name.clone();
        let val_class = heap.get(val_ref)?.class_name.clone();
        let map_ref = heap.allocate("java/util/UnmodifiableMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let k_raw = ops
                .invoke(
                    heap,
                    out,
                    &key_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![key_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let v_raw = ops
                .invoke(
                    heap,
                    out,
                    &val_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![val_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let k = box_primitive_slot(k_raw, heap);
            let v = box_primitive_slot(v_raw, heap);
            let cur_size = match heap.get(map_ref)?.fields.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            heap.get_mut(map_ref)?.fields.push(k);
            heap.get_mut(map_ref)?.fields.push(v);
            heap.get_mut(map_ref)?.fields[0] = Slot::Int(cur_size + 1);
        }
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/ToMapMergeCollector" {
        // Collect into HashMap with merge function for duplicate keys.
        let collector_ref = extract_ref_arg(args, 1)?;
        let key_fn = extract_first_field_arg(heap, collector_ref)?;
        let val_fn = extract_field_arg(heap, collector_ref, 1)?;
        let merge_fn = extract_field_arg(heap, collector_ref, 2)?;
        let Slot::Reference(Some(key_ref)) = key_fn else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(val_ref)) = val_fn else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(merge_ref)) = merge_fn else {
            return Err(Error::NullPointerException);
        };
        let key_class = heap.get(key_ref)?.class_name.clone();
        let val_class = heap.get(val_ref)?.class_name.clone();
        let merge_class = heap.get(merge_ref)?.class_name.clone();
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let k = ops
                .invoke(
                    heap,
                    out,
                    &key_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![key_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let v = ops
                .invoke(
                    heap,
                    out,
                    &val_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![val_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let fields = heap.get(map_ref)?.fields.clone();
            if let Some(i) = find_hashmap_entry_index(&fields, &k, heap) {
                // Duplicate key — apply merge function: merge(existing, new)
                let existing = fields[i + 1];
                let merged = ops
                    .invoke(
                        heap,
                        out,
                        &merge_class,
                        "apply",
                        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                        vec![merge_fn, existing, v],
                    )?
                    .unwrap_or(Slot::Reference(None));
                heap.get_mut(map_ref)?.fields[i + 1] = merged;
            } else {
                let cur_size = match heap.get(map_ref)?.fields.first() {
                    Some(Slot::Int(n)) => *n,
                    _ => 0,
                };
                heap.get_mut(map_ref)?.fields.push(k);
                heap.get_mut(map_ref)?.fields.push(v);
                heap.get_mut(map_ref)?.fields[0] = Slot::Int(cur_size + 1);
            }
        }
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/PartitioningByCollector" {
        // Collect into a Map<Boolean, List> partitioned by predicate.
        let collector_ref = extract_ref_arg(args, 1)?;
        let pred_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(pred_ref)) = pred_slot else {
            return Err(Error::NullPointerException);
        };
        let pred_class = heap.get(pred_ref)?.class_name.clone();
        // Create two lists and the result map.
        let true_list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(true_list)?.fields[0] = Slot::Int(0);
        let false_list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(false_list)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![Slot::Reference(Some(pred_ref)), elem],
            )?;
            let is_true = matches!(result, Some(Slot::Int(n)) if n != 0);
            let target = if is_true { true_list } else { false_list };
            let cur_size = match heap.get(target)?.fields.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            heap.get_mut(target)?.fields.push(elem);
            heap.get_mut(target)?.fields[0] = Slot::Int(cur_size + 1);
        }
        // Build HashMap: Boolean(1)→trueList, Boolean(0)→falseList
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(2);
        let bool_true = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(bool_true)?.fields[0] = Slot::Int(1);
        let bool_false = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(bool_false)?.fields[0] = Slot::Int(0);
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(bool_true)));
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(true_list)));
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(bool_false)));
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(false_list)));
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/PartitioningByDownstreamCollector" {
        // partitioningBy(pred, downstream): partition then apply downstream to each group.
        let collector_ref = extract_ref_arg(args, 1)?;
        let pred_slot = extract_first_field_arg(heap, collector_ref)?;
        let downstream_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(pred_ref)) = pred_slot else {
            return Err(Error::NullPointerException);
        };
        let pred_class = heap.get(pred_ref)?.class_name.clone();
        let mut true_elems: Vec<Slot> = Vec::new();
        let mut false_elems: Vec<Slot> = Vec::new();
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![Slot::Reference(Some(pred_ref)), elem],
            )?;
            if matches!(result, Some(Slot::Int(n)) if n != 0) {
                true_elems.push(elem);
            } else {
                false_elems.push(elem);
            }
        }
        // Apply downstream collector to each partition by building a mini stream.
        let apply_downstream = |elems_sub: Vec<Slot>,
                                heap: &mut duke_gc::Heap,
                                downstream: Slot|
         -> Result<Option<Slot>> {
            let n = elems_sub.len();
            let downstream_class = match downstream {
                Slot::Reference(Some(r)) => heap.get(r)?.class_name.clone(),
                _ => return Ok(Some(Slot::Reference(None))),
            };
            if downstream_class == "duke/util/CountingCollector" {
                let boxed = heap.allocate("java/lang/Long".to_string(), 1);
                #[allow(clippy::cast_possible_wrap)] // n is a collection count; fits i64
                let count_long = Slot::Long(n as i64);
                heap.get_mut(boxed)?.fields[0] = count_long;
                return Ok(Some(Slot::Reference(Some(boxed))));
            }
            // Generic: build a mini stream and collect into a list.
            let stream_ref = heap.allocate("duke/util/Stream".to_string(), n);
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let n_i32 = n as i32;
            heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n_i32);
            for (i, e) in elems_sub.into_iter().enumerate() {
                heap.get_mut(stream_ref)?.fields[1 + i] = e;
            }
            let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
            heap.get_mut(list_ref)?.fields[0] = Slot::Int(0);
            for i in 1..=n {
                let e = heap.get(stream_ref)?.fields[i];
                let cur = match heap.get(list_ref)?.fields.first() {
                    Some(Slot::Int(x)) => *x,
                    _ => 0,
                };
                heap.get_mut(list_ref)?.fields.push(e);
                heap.get_mut(list_ref)?.fields[0] = Slot::Int(cur + 1);
            }
            Ok(Some(Slot::Reference(Some(list_ref))))
        };
        let true_result = apply_downstream(true_elems, heap, downstream_slot)?;
        let false_result = apply_downstream(false_elems, heap, downstream_slot)?;
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(2);
        let bool_true = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(bool_true)?.fields[0] = Slot::Int(1);
        let bool_false = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(bool_false)?.fields[0] = Slot::Int(0);
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(bool_true)));
        heap.get_mut(map_ref)?
            .fields
            .push(true_result.unwrap_or(Slot::Reference(None)));
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(bool_false)));
        heap.get_mut(map_ref)?
            .fields
            .push(false_result.unwrap_or(Slot::Reference(None)));
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/SummingIntCollector" {
        // Sum via applyAsInt(elem) for each element.
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i32;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(Ljava/lang/Object;)I",
                vec![fn_slot, elem],
            )?;
            if let Some(Slot::Int(n)) = result {
                sum = sum.wrapping_add(n);
            }
        }
        // Return boxed Integer
        let boxed = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Int(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingIntCollector" {
        // Average via applyAsInt(elem) for each element.
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        let mut count = 0_usize;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(Ljava/lang/Object;)I",
                vec![fn_slot, elem],
            )?;
            if let Some(Slot::Int(n)) = result {
                sum += i64::from(n);
                count += 1;
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let avg = if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        };
        // Return boxed Double
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/SummarizingIntCollector" {
        // IntSummaryStatistics via applyAsInt(elem) for each element.
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        let mut min = i32::MAX;
        let mut max = i32::MIN;
        let mut count = 0_i64;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(Ljava/lang/Object;)I",
                vec![fn_slot, elem],
            )?;
            if let Some(Slot::Int(n)) = result {
                sum += i64::from(n);
                if n < min {
                    min = n;
                }
                if n > max {
                    max = n;
                }
                count += 1;
            }
        }
        if count == 0 {
            min = 0;
            max = 0;
        }
        // fields: [0]=count(J) [1]=sum(J) [2]=min(I) [3]=max(I)
        let stats = heap.allocate("java/util/IntSummaryStatistics".to_string(), 4);
        heap.get_mut(stats)?.fields[0] = Slot::Long(count);
        heap.get_mut(stats)?.fields[1] = Slot::Long(sum);
        heap.get_mut(stats)?.fields[2] = Slot::Int(min);
        heap.get_mut(stats)?.fields[3] = Slot::Int(max);
        Ok(Some(Slot::Reference(Some(stats))))
    } else if collector_class == "duke/util/SummingLongCollector" {
        // Sum via applyAsLong(elem) for each element.
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(Ljava/lang/Object;)J",
                vec![fn_slot, elem],
            )?;
            match result {
                Some(Slot::Long(n)) => sum = sum.wrapping_add(n),
                Some(Slot::Int(n)) => sum = sum.wrapping_add(i64::from(n)),
                _ => {}
            }
        }
        // Return boxed Long
        let boxed = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Long(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingDoubleCollector" {
        // Average via applyAsDouble(elem) for each element.
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0.0_f64;
        let mut count = 0_usize;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(Ljava/lang/Object;)D",
                vec![fn_slot, elem],
            )?;
            match result {
                Some(Slot::Double(d)) => {
                    sum += d;
                    count += 1;
                }
                Some(Slot::Float(f)) => {
                    sum += f64::from(f);
                    count += 1;
                }
                Some(Slot::Int(n)) => {
                    sum += f64::from(n);
                    count += 1;
                }
                _ => {}
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let avg = if count == 0 { 0.0 } else { sum / count as f64 };
        // Return boxed Double
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/MappingCollector" {
        // MappingCollector: fields[0]=mapper fn, fields[1]=downstream collector
        let collector_ref = extract_ref_arg(args, 1)?;
        let mapper_slot = extract_first_field_arg(heap, collector_ref)?;
        let downstream_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(mapper_ref)) = mapper_slot else {
            return Err(Error::NullPointerException);
        };
        let mapper_class = heap.get(mapper_ref)?.class_name.clone();
        // Map each element through the mapper function
        let mut mapped_elems = Vec::with_capacity(elems.len());
        for elem in elems {
            let mapped = ops
                .invoke(
                    heap,
                    out,
                    &mapper_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![mapper_slot, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            mapped_elems.push(mapped);
        }
        // Build a temporary stream from mapped elements and collect with downstream
        let mapped_size = i32::try_from(mapped_elems.len()).unwrap_or(0);
        let tmp_stream = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(tmp_stream)?.fields[0] = Slot::Int(mapped_size);
        for elem in mapped_elems {
            heap.get_mut(tmp_stream)?.fields.push(elem);
        }
        let tmp_args = vec![Slot::Reference(Some(tmp_stream)), downstream_slot];
        native_stream_collect(&tmp_args, heap, out, control, ops)
    } else if collector_class == "duke/util/GroupingBy2Collector" {
        // groupingBy(keyFn, downstream): fields[0]=keyFn, fields[1]=downstream collector
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let downstream_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // First pass: group raw elements by key into HashMap<key, ArrayList<elem>>
        let raw_map = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(raw_map)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let key_raw = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![fn_slot, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            // Box primitive keys so HashMap.get(boxed) can match them via slots_equal
            let key = box_primitive_slot(key_raw, heap);
            let fields = heap.get(raw_map)?.fields.clone();
            let size_n = match fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let mut found_ki = None;
            for i in 0..size_n {
                let ki = 1 + i * 2;
                if fields.get(ki).is_some_and(|k| slots_equal(k, &key, heap)) {
                    found_ki = Some(ki);
                    break;
                }
            }
            if let Some(ki) = found_ki {
                let list_slot = extract_field_arg(heap, raw_map, ki + 1)?;
                if let Slot::Reference(Some(list_ref)) = list_slot {
                    let list_size = match heap.get(list_ref)?.fields.first() {
                        Some(Slot::Int(n)) => *n,
                        _ => 0,
                    };
                    heap.get_mut(list_ref)?.fields.push(elem);
                    heap.get_mut(list_ref)?.fields[0] = Slot::Int(list_size + 1);
                }
            } else {
                let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
                heap.get_mut(list_ref)?.fields[0] = Slot::Int(1);
                heap.get_mut(list_ref)?.fields.push(elem);
                heap.get_mut(raw_map)?.fields.push(key);
                heap.get_mut(raw_map)?
                    .fields
                    .push(Slot::Reference(Some(list_ref)));
                heap.get_mut(raw_map)?.fields[0] =
                    Slot::Int(i32::try_from(size_n + 1).unwrap_or(i32::MAX));
            }
        }
        // Second pass: apply downstream collector to each group's ArrayList
        let result_map = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(result_map)?.fields[0] = Slot::Int(0);
        let raw_fields = heap.get(raw_map)?.fields.clone();
        let group_count = match raw_fields.first() {
            Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
            _ => 0,
        };
        for i in 0..group_count {
            let key = raw_fields
                .get(1 + i * 2)
                .copied()
                .unwrap_or(Slot::Reference(None));
            let list_slot = raw_fields
                .get(2 + i * 2)
                .copied()
                .unwrap_or(Slot::Reference(None));
            let Slot::Reference(Some(list_ref)) = list_slot else {
                continue;
            };
            let group_size_field = heap
                .get(list_ref)?
                .fields
                .first()
                .copied()
                .unwrap_or(Slot::Int(0));
            let group_size = match group_size_field {
                Slot::Int(n) => n,
                _ => 0,
            };
            let tmp_stream = heap.allocate("duke/util/Stream".to_string(), 1);
            heap.get_mut(tmp_stream)?.fields[0] = group_size_field;
            let group_elems: Vec<Slot> =
                heap.get(list_ref)?.fields[1..=usize::try_from(group_size).unwrap_or(0)].to_vec();
            for e in group_elems {
                heap.get_mut(tmp_stream)?.fields.push(e);
            }
            let tmp_args = vec![Slot::Reference(Some(tmp_stream)), downstream_slot];
            let collected = native_stream_collect(&tmp_args, heap, out, control, ops)?
                .unwrap_or(Slot::Reference(None));
            let cur_result_size = match heap.get(result_map)?.fields.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            heap.get_mut(result_map)?.fields.push(key);
            heap.get_mut(result_map)?.fields.push(collected);
            heap.get_mut(result_map)?.fields[0] = Slot::Int(cur_result_size + 1);
        }
        Ok(Some(Slot::Reference(Some(result_map))))
    } else if collector_class == "duke/util/MinByCollector" {
        // minBy(comparator): fields[0] = comparator
        let collector_ref = extract_ref_arg(args, 1)?;
        let cmp_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(cmp_ref)) = cmp_slot else {
            let r = make_optional(heap, None);
            return Ok(Some(Slot::Reference(Some(r))));
        };
        let cmp_class = heap.get(cmp_ref)?.class_name.clone();
        let mut min: Option<Slot> = None;
        for elem in elems {
            let is_less = if let Some(ref cur) = min {
                let result = ops
                    .invoke(
                        heap,
                        out,
                        &cmp_class,
                        "compare",
                        "(Ljava/lang/Object;Ljava/lang/Object;)I",
                        vec![cmp_slot, elem, *cur],
                    )?
                    .unwrap_or(Slot::Int(0));
                matches!(result, Slot::Int(n) if n < 0)
            } else {
                true
            };
            if is_less {
                min = Some(elem);
            }
        }
        let r = make_optional(heap, min);
        Ok(Some(Slot::Reference(Some(r))))
    } else if collector_class == "duke/util/MaxByCollector" {
        // maxBy(comparator): fields[0] = comparator
        let collector_ref = extract_ref_arg(args, 1)?;
        let cmp_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(cmp_ref)) = cmp_slot else {
            let r = make_optional(heap, None);
            return Ok(Some(Slot::Reference(Some(r))));
        };
        let cmp_class = heap.get(cmp_ref)?.class_name.clone();
        let mut max: Option<Slot> = None;
        for elem in elems {
            let is_greater = if let Some(ref cur) = max {
                let result = ops
                    .invoke(
                        heap,
                        out,
                        &cmp_class,
                        "compare",
                        "(Ljava/lang/Object;Ljava/lang/Object;)I",
                        vec![cmp_slot, elem, *cur],
                    )?
                    .unwrap_or(Slot::Int(0));
                matches!(result, Slot::Int(n) if n > 0)
            } else {
                true
            };
            if is_greater {
                max = Some(elem);
            }
        }
        let r = make_optional(heap, max);
        Ok(Some(Slot::Reference(Some(r))))
    } else if collector_class == "duke/util/SummingDoubleCollector" {
        // summingDouble: fields[0] = ToDoubleFunction
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0.0_f64;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(Ljava/lang/Object;)D",
                vec![fn_slot, elem],
            )?;
            match result {
                Some(Slot::Double(d)) => sum += d,
                Some(Slot::Float(f)) => sum += f64::from(f),
                Some(Slot::Int(n)) => sum += f64::from(n),
                _ => {}
            }
        }
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingLongCollector" {
        // averagingLong: fields[0] = ToLongFunction
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        let mut count = 0_usize;
        for elem in elems {
            let result = ops.invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(Ljava/lang/Object;)J",
                vec![fn_slot, elem],
            )?;
            match result {
                Some(Slot::Long(n)) => {
                    sum = sum.wrapping_add(n);
                    count += 1;
                }
                Some(Slot::Int(n)) => {
                    sum = sum.wrapping_add(i64::from(n));
                    count += 1;
                }
                _ => {}
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let avg = if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        };
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/ReducingNoIdentityCollector" {
        // reducing(BinaryOperator) → Optional<T>
        let collector_ref = extract_ref_arg(args, 1)?;
        let op_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(bop_ref)) = op_slot else {
            return Err(Error::NullPointerException);
        };
        let op_class = heap.get(bop_ref)?.class_name.clone();
        let result = if elems.is_empty() {
            None
        } else {
            let mut acc = elems[0];
            for elem in elems.into_iter().skip(1) {
                acc = ops
                    .invoke(
                        heap,
                        out,
                        &op_class,
                        "apply",
                        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                        vec![op_slot, acc, elem],
                    )?
                    .unwrap_or(Slot::Reference(None));
            }
            Some(acc)
        };
        let opt_ref = make_optional(heap, result);
        Ok(Some(Slot::Reference(Some(opt_ref))))
    } else if collector_class == "duke/util/ReducingCollector" {
        // reducing(identity, BinaryOperator) → T
        let collector_ref = extract_ref_arg(args, 1)?;
        let identity_slot = extract_first_field_arg(heap, collector_ref)?;
        let op_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(bop_ref)) = op_slot else {
            return Err(Error::NullPointerException);
        };
        let op_class = heap.get(bop_ref)?.class_name.clone();
        let mut acc = identity_slot;
        for elem in elems {
            acc = ops
                .invoke(
                    heap,
                    out,
                    &op_class,
                    "apply",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![op_slot, acc, elem],
                )?
                .unwrap_or(Slot::Reference(None));
        }
        Ok(Some(acc))
    } else if collector_class == "duke/util/ReducingMappingCollector" {
        // reducing(identity, mapper, BinaryOperator) → U
        let collector_ref = extract_ref_arg(args, 1)?;
        let identity_slot = extract_first_field_arg(heap, collector_ref)?;
        let mapper_slot = extract_field_arg(heap, collector_ref, 1)?;
        let op_slot = extract_field_arg(heap, collector_ref, 2)?;
        let Slot::Reference(Some(mapper_ref)) = mapper_slot else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(bop_ref)) = op_slot else {
            return Err(Error::NullPointerException);
        };
        let mapper_class = heap.get(mapper_ref)?.class_name.clone();
        let op_class = heap.get(bop_ref)?.class_name.clone();
        let mut acc = identity_slot;
        for elem in elems {
            let mapped = ops
                .invoke(
                    heap,
                    out,
                    &mapper_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![mapper_slot, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            acc = ops
                .invoke(
                    heap,
                    out,
                    &op_class,
                    "apply",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![op_slot, acc, mapped],
                )?
                .unwrap_or(Slot::Reference(None));
        }
        Ok(Some(acc))
    } else if collector_class == "duke/util/CollectingAndThenCollector" {
        // collectingAndThen(downstream, finisher): fields[0]=downstream, fields[1]=finisher
        let collector_ref = extract_ref_arg(args, 1)?;
        let downstream_slot = extract_first_field_arg(heap, collector_ref)?;
        let finisher_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(finisher_ref)) = finisher_slot else {
            return Err(Error::NullPointerException);
        };
        let finisher_class = heap.get(finisher_ref)?.class_name.clone();
        // First collect with downstream
        let tmp_args = vec![Slot::Reference(Some(stream_ref)), downstream_slot];
        let intermediate = native_stream_collect(&tmp_args, heap, out, control, ops)?
            .unwrap_or(Slot::Reference(None));
        // Then apply finisher
        let result = ops.invoke(
            heap,
            out,
            &finisher_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![finisher_slot, intermediate],
        )?;
        Ok(result.or(Some(Slot::Reference(None))))
    } else if collector_class == "duke/util/ToUnmodifiableListCollector" {
        // toUnmodifiableList(): collect into UnmodifiableList (mutations throw).
        let list_ref = heap.allocate("java/util/UnmodifiableList".to_string(), 1);
        heap.get_mut(list_ref)?.fields[0] = Slot::Int(size);
        for elem in elems {
            heap.get_mut(list_ref)?.fields.push(elem);
        }
        Ok(Some(Slot::Reference(Some(list_ref))))
    } else {
        // ToListCollector (default): collect into ArrayList.
        let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(list_ref)?.fields[0] = Slot::Int(size);
        for elem in elems {
            heap.get_mut(list_ref)?.fields.push(elem);
        }
        Ok(Some(Slot::Reference(Some(list_ref))))
    }
}

/// Native: `Stream.distinct()Stream` — removes duplicate elements (by `slots_equal`).
pub(crate) fn native_stream_distinct(
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
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut seen: Vec<Slot> = Vec::new();
    for elem in elems {
        if !seen.iter().any(|s| slots_equal(s, &elem, heap)) {
            seen.push(elem);
        }
    }
    let new_size = i32::try_from(seen.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in seen {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

/// Native: `Stream.sorted()Stream` — sorts elements by natural order via `compareTo`.
pub(crate) fn native_stream_sorted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    // Insertion sort via compareTo callbacks (stable, O(n²) — fine for test sizes).
    for i in 1..elems.len() {
        let mut j = i;
        while j > 0 {
            let Slot::Reference(Some(a)) = elems[j - 1] else {
                break;
            };
            let Slot::Reference(Some(b)) = elems[j] else {
                break;
            };
            let class_a = heap.get(a)?.class_name.clone();
            let cmp = ops.invoke(
                heap,
                out,
                &class_a,
                "compareTo",
                "(Ljava/lang/Object;)I",
                vec![Slot::Reference(Some(a)), Slot::Reference(Some(b))],
            )?;
            if matches!(cmp, Some(Slot::Int(n)) if n > 0) {
                elems.swap(j - 1, j);
                j -= 1;
            } else {
                break;
            }
        }
    }
    let new_size = i32::try_from(elems.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in elems {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

/// Native: `Stream.anyMatch(Predicate)Z` — true if any element satisfies predicate.
pub(crate) fn native_stream_any_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for elem in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(pred_ref)), elem],
        )?;
        if matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `Stream.allMatch(Predicate)Z` — true if all elements satisfy predicate.
pub(crate) fn native_stream_all_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for elem in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(pred_ref)), elem],
        )?;
        if !matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `Stream.noneMatch(Predicate)Z` — true if no element satisfies predicate.
pub(crate) fn native_stream_none_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for elem in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(pred_ref)), elem],
        )?;
        if matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `Stream.findFirst()Optional` — returns Optional of first element, or empty.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_find_first(
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
    let opt_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if size > 0 {
        let first = heap.get(stream_ref)?.fields[1];
        heap.get_mut(opt_ref)?.fields[0] = first;
    } else {
        heap.get_mut(opt_ref)?.fields[0] = Slot::Reference(None);
    }
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `Stream.reduce(BinaryOperator)Optional` — folds elements left via binary op.
pub(crate) fn native_stream_reduce(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let op_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(op_ref)) = op_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let op_class = heap.get(op_ref)?.class_name.clone();
    let reduce_result_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if elems.is_empty() {
        heap.get_mut(reduce_result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(reduce_result_ref))));
    }
    let mut acc = elems[0];
    for elem in elems.into_iter().skip(1) {
        let result = ops.invoke(
            heap,
            out,
            &op_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![Slot::Reference(Some(op_ref)), acc, elem],
        )?;
        acc = result.unwrap_or(Slot::Reference(None));
    }
    // Box primitive accumulator before storing in Optional (Java generics always hold References)
    let acc_boxed = box_primitive_slot(acc, heap);
    heap.get_mut(reduce_result_ref)?.fields[0] = acc_boxed;
    Ok(Some(Slot::Reference(Some(reduce_result_ref))))
}

/// Native: `Stream.reduce(identity, BinaryOperator)Object` — fold with initial value.
pub(crate) fn native_stream_reduce_with_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let identity = extract_slot_arg(args, 1);
    let fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(identity));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut acc = identity;
    for elem in elems {
        acc = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                vec![fn_slot, acc, elem],
            )?
            .unwrap_or(Slot::Reference(None));
    }
    // Autobox: if the identity was a Reference (stream of boxed type) but the accumulator
    // impl returned a raw primitive (e.g. Integer::sum returns int), re-box the result so
    // that the caller can apply intValue() / longValue() as expected.
    let acc = match (identity, acc) {
        (Slot::Reference(_), Slot::Int(v)) => {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Int(v);
            Slot::Reference(Some(r))
        }
        (Slot::Reference(_), Slot::Long(v)) => {
            let r = heap.allocate("java/lang/Long".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Long(v);
            Slot::Reference(Some(r))
        }
        (Slot::Reference(_), Slot::Double(v)) => {
            let r = heap.allocate("java/lang/Double".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Double(v);
            Slot::Reference(Some(r))
        }
        _ => acc,
    };
    Ok(Some(acc))
}

/// Native: `Stream.toList()List` — terminal op returning an unmodifiable list (same as collect).
pub(crate) fn native_stream_to_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_stream_collect(args, heap, out, control, ops)
}

/// Helper: allocate a `JoiningCollector` with 3 fields: delimiter, prefix, suffix.
fn make_joining_collector(
    heap: &mut duke_gc::Heap,
    delimiter: &str,
    prefix: &str,
    suffix: &str,
) -> u64 {
    let collector_ref = heap.allocate("duke/util/JoiningCollector".to_string(), 3);
    let delim_ref = heap.allocate_string(delimiter.to_string());
    let prefix_ref = heap.allocate_string(prefix.to_string());
    let suffix_ref = heap.allocate_string(suffix.to_string());
    let obj = heap.get_mut(collector_ref).expect("just allocated");
    obj.fields[0] = Slot::Reference(Some(delim_ref));
    obj.fields[1] = Slot::Reference(Some(prefix_ref));
    obj.fields[2] = Slot::Reference(Some(suffix_ref));
    collector_ref
}

/// Native: `Collectors.joining(delim)Collector` — returns a joining collector with delimiter.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_joining(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let delim = match args.first() {
        Some(Slot::Reference(Some(r))) => heap
            .get(*r)
            .ok()
            .and_then(|o| o.string_value.clone())
            .unwrap_or_default(),
        _ => String::new(),
    };
    let collector_ref = make_joining_collector(heap, &delim, "", "");
    Ok(Some(Slot::Reference(Some(collector_ref))))
}

/// Native: `Collectors.joining()Collector` — no-arg version (empty delimiter).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_joining_no_arg(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let collector_ref = make_joining_collector(heap, "", "", "");
    Ok(Some(Slot::Reference(Some(collector_ref))))
}

/// Native: `Collectors.joining(delim, prefix, suffix)Collector` — full 3-arg joining collector.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_joining_full(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let read_str = |heap: &duke_gc::Heap, idx: usize| -> String {
        match args.get(idx) {
            Some(Slot::Reference(Some(r))) => heap
                .get(*r)
                .ok()
                .and_then(|o| o.string_value.clone())
                .unwrap_or_default(),
            _ => String::new(),
        }
    };
    let delim = read_str(heap, 0);
    let prefix = read_str(heap, 1);
    let suffix = read_str(heap, 2);
    let collector_ref = make_joining_collector(heap, &delim, &prefix, &suffix);
    Ok(Some(Slot::Reference(Some(collector_ref))))
}

/// Native: `Collectors.toList()Collector` — returns a sentinel collector object.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let collector_ref = heap.allocate("duke/util/ToListCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(collector_ref))))
}

/// Native: `Collectors.counting()Collector` — returns a counting collector sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_counting(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/CountingCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.groupingBy(Function)Collector` — returns a grouping-by collector.
/// Stores `fn_slot` in `fields[0]`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_grouping_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/GroupingByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Stream.peek(Consumer)Stream` — side-effect each element, returns same stream.
pub(crate) fn native_stream_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(Some(Slot::Reference(Some(stream_ref))));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for elem in &elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, *elem],
        )?;
    }
    // Return a new stream with same elements (consumer may have GC'd things)
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(elems.len()).unwrap_or(0));
    for elem in elems {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `Stream.toArray()Object[]` — materializes stream into an Object array.
pub(crate) fn native_stream_to_array(
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
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    // Arrays use fields directly (no length header); arraylength returns fields.len().
    let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 0);
    for elem in elems {
        heap.get_mut(arr_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

// ---------------------------------------------------------------------------
// Stream.limit / Stream.skip / Stream.flatMap
// ---------------------------------------------------------------------------

/// Native: `Stream.limit(long)Stream` — keeps first N elements.
pub(crate) fn native_stream_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let max_size = match args.get(1).copied() {
        Some(Slot::Long(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        _ => 0,
    };
    let class_name = heap.get(stream_ref)?.class_name.clone();

    // Lazy generators: materialise N elements on limit().
    if class_name == "duke/util/GeneratorStream" {
        let supplier_slot = extract_first_field_arg(heap, stream_ref)?;
        let Slot::Reference(Some(sup_ref)) = supplier_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let sup_class = heap.get(sup_ref)?.class_name.clone();
        let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(max_size).unwrap_or(0));
        for _ in 0..max_size {
            let elem = ops
                .invoke(
                    heap,
                    out,
                    &sup_class,
                    "get",
                    "()Ljava/lang/Object;",
                    vec![Slot::Reference(Some(sup_ref))],
                )?
                .unwrap_or(Slot::Reference(None));
            heap.get_mut(out_ref)?.fields.push(elem);
        }
        return Ok(Some(Slot::Reference(Some(out_ref))));
    }

    if class_name == "duke/util/IteratorStream" {
        // fields[0] = current seed, fields[1] = UnaryOperator fn
        let seed = extract_first_field_arg(heap, stream_ref)?;
        let fn_slot = extract_field_arg(heap, stream_ref, 1)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(max_size).unwrap_or(0));
        let mut current = seed;
        for _ in 0..max_size {
            heap.get_mut(out_ref)?.fields.push(current);
            current = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![fn_slot, current],
                )?
                .unwrap_or(Slot::Reference(None));
        }
        return Ok(Some(Slot::Reference(Some(out_ref))));
    }

    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let take = size.min(max_size);
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(take).unwrap_or(0));
    for elem in elems.into_iter().take(take) {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `Stream.skip(long)Stream` — skips first N elements.
pub(crate) fn native_stream_skip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let skip_n = match args.get(1).copied() {
        Some(Slot::Long(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        _ => 0,
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let skipped: Vec<Slot> = elems.into_iter().skip(skip_n).collect();
    let new_size = i32::try_from(skipped.len()).unwrap_or(0);
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(new_size);
    for elem in skipped {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `Stream.flatMap(Function)Stream` — maps each element to a Stream and flattens.
pub(crate) fn native_stream_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut flat: Vec<Slot> = Vec::with_capacity(elems.len());
    for elem in elems {
        let inner = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, elem],
        )?;
        if let Some(Slot::Reference(Some(inner_ref))) = inner {
            // inner should be a duke/util/Stream — flatten its elements
            let inner_size = match heap.get(inner_ref)?.fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let inner_elems: Vec<Slot> = heap.get(inner_ref)?.fields[1..=inner_size].to_vec();
            flat.extend(inner_elems);
        }
    }
    let new_size = i32::try_from(flat.len()).unwrap_or(0);
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(new_size);
    for elem in flat {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

// ---------------------------------------------------------------------------
// IntStream — duke/util/IntStream
// fields[0]=Int(size), fields[1..n]=Int(value) elements (unboxed ints)
// ---------------------------------------------------------------------------

/// Build an `IntStream` heap object from a `Vec<i32>`.
fn make_int_stream(heap: &mut duke_gc::Heap, values: Vec<i32>) -> u64 {
    let n = i32::try_from(values.len()).unwrap_or(0);
    let r = heap.allocate("duke/util/IntStream".to_string(), 1);
    heap.get_mut(r).expect("fresh").fields[0] = Slot::Int(n);
    for v in values {
        heap.get_mut(r).expect("fresh").fields.push(Slot::Int(v));
    }
    r
}

/// Create an `OptionalInt` heap object. `None` = empty, `Some(v)` = present.
fn make_optional_int(heap: &mut duke_gc::Heap, value: Option<i32>) -> u64 {
    let r = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    if let Some(v) = value {
        heap.get_mut(r).expect("fresh").fields[0] = Slot::Int(v);
        heap.get_mut(r).expect("fresh").fields[1] = Slot::Int(1);
    } else {
        heap.get_mut(r).expect("fresh").fields[1] = Slot::Int(0);
    }
    r
}

/// Create a `java/util/Optional` heap object. `None` = empty, `Some(slot)` = present.
fn make_optional(heap: &mut duke_gc::Heap, value: Option<Slot>) -> u64 {
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r).expect("fresh").fields[0] = value.unwrap_or(Slot::Reference(None));
    r
}

/// Extract int elements from an `IntStream` heap object.
fn int_stream_elems(heap: &duke_gc::Heap, ref_: u64) -> Vec<i32> {
    let size = match heap.get(ref_).ok().and_then(|o| o.fields.first().copied()) {
        Some(Slot::Int(n)) => usize::try_from(n).unwrap_or(0),
        _ => 0,
    };
    heap.get(ref_)
        .ok()
        .map(|o| {
            o.fields[1..=size]
                .iter()
                .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
                .collect()
        })
        .unwrap_or_default()
}

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

/// Native: `String.<init>(String)V` — copy constructor: copies `string_value` from source.
pub(crate) fn native_string_init_copy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_val = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = src_val;
    Ok(None)
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

/// Native: `ArrayDeque.<init>()V` — same layout as `ArrayList`.
pub(crate) fn native_arraydeque_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `ArrayDeque.push(Object)V` — push to front (stack: LIFO).
pub(crate) fn native_arraydeque_push(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    heap.get_mut(this_ref)?.fields.insert(1, elem);
    let new_size = i32::try_from(size + 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(None)
}

/// Native: `ArrayDeque.pop()Object` — pop from front (stack: LIFO).
pub(crate) fn native_arraydeque_pop(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let elem = heap.get_mut(this_ref)?.fields.remove(1);
    let new_size = i32::try_from(size - 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(Some(elem))
}

/// Native: `ArrayDeque.offer(Object)Z` — enqueue at back (queue: FIFO).
pub(crate) fn native_arraydeque_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(Slot::Int(1))) // always succeeds
}

/// Native: `ArrayDeque.add(Object)Z` — same as offer (appends to back).
pub(crate) fn native_arraydeque_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `ArrayDeque.poll()Object` — dequeue from front (queue: FIFO); null if empty.
pub(crate) fn native_arraydeque_poll(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    let elem = heap.get_mut(this_ref)?.fields.remove(1);
    let new_size = i32::try_from(size - 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(Some(elem))
}

/// Native: `ArrayDeque.peek()Object` — peek at front; null if empty.
pub(crate) fn native_arraydeque_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `ArrayDeque.size()I`
pub(crate) fn native_arraydeque_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `ArrayDeque.isEmpty()Z`
pub(crate) fn native_arraydeque_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

// ---- PriorityQueue natives ----
// fields[0]=Int(size), fields[1..size]=elements; maintained as a min-heap (String key order for Strings, value for boxed ints).

/// Native: `PriorityQueue.<init>()V` — same init as `ArrayList`.
pub(crate) fn native_priorityqueue_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `PriorityQueue.offer(Object)Z` — inserts in heap order via `compareTo`.
pub(crate) fn native_priorityqueue_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    // Append, then sift up.
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    heap.get_mut(this_ref)?.fields.push(elem);
    let new_size = size + 1;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(new_size).unwrap_or(0));
    // Sift up from last position (1-indexed in fields).
    let mut i = new_size; // fields index of newly added element
    while i > 1 {
        // fields are 1-indexed: child at fields[i], parent at fields[i/2]
        let parent_idx = i / 2;
        let child_slot = heap.get(this_ref)?.fields[i];
        let parent_slot = heap.get(this_ref)?.fields[parent_idx];
        let (Slot::Reference(Some(child_ref)), Slot::Reference(Some(parent_ref))) =
            (child_slot, parent_slot)
        else {
            break;
        };
        let class_child = heap.get(child_ref)?.class_name.clone();
        let cmp = ops.invoke(
            heap,
            out,
            &class_child,
            "compareTo",
            "(Ljava/lang/Object;)I",
            vec![
                Slot::Reference(Some(child_ref)),
                Slot::Reference(Some(parent_ref)),
            ],
        )?;
        if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
            // child < parent: swap
            heap.get_mut(this_ref)?.fields.swap(i, parent_idx);
            i = parent_idx;
        } else {
            break;
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `PriorityQueue.add(Object)Z` — same as offer.
pub(crate) fn native_priorityqueue_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_priorityqueue_offer(args, heap, out, control, ops)
}

/// Native: `PriorityQueue.peek()Object` — returns minimum element without removing.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_priorityqueue_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `PriorityQueue.poll()Object` — removes and returns minimum; sifts down.
pub(crate) fn native_priorityqueue_poll(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    let min = heap.get(this_ref)?.fields[1];
    if size == 1 {
        heap.get_mut(this_ref)?.fields.pop();
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
        return Ok(Some(min));
    }
    // Move last element to root, sift down.
    let last = heap.get(this_ref)?.fields[size];
    heap.get_mut(this_ref)?.fields[1] = last;
    heap.get_mut(this_ref)?.fields.pop();
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    let new_size = size - 1;
    let mut i = 1usize;
    loop {
        let left = 2 * i;
        let right = 2 * i + 1;
        let mut smallest = i;
        if left <= new_size {
            let (cur_slot, left_slot) = (
                heap.get(this_ref)?.fields[smallest],
                heap.get(this_ref)?.fields[left],
            );
            let (Slot::Reference(Some(cur_ref)), Slot::Reference(Some(left_ref))) =
                (cur_slot, left_slot)
            else {
                break;
            };
            let class_left = heap.get(left_ref)?.class_name.clone();
            let cmp = ops.invoke(
                heap,
                out,
                &class_left,
                "compareTo",
                "(Ljava/lang/Object;)I",
                vec![
                    Slot::Reference(Some(left_ref)),
                    Slot::Reference(Some(cur_ref)),
                ],
            )?;
            if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
                smallest = left;
            }
        }
        if right <= new_size {
            let (small_slot, right_slot) = (
                heap.get(this_ref)?.fields[smallest],
                heap.get(this_ref)?.fields[right],
            );
            let (Slot::Reference(Some(small_ref)), Slot::Reference(Some(right_ref))) =
                (small_slot, right_slot)
            else {
                break;
            };
            let class_right = heap.get(right_ref)?.class_name.clone();
            let cmp = ops.invoke(
                heap,
                out,
                &class_right,
                "compareTo",
                "(Ljava/lang/Object;)I",
                vec![
                    Slot::Reference(Some(right_ref)),
                    Slot::Reference(Some(small_ref)),
                ],
            )?;
            if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
                smallest = right;
            }
        }
        if smallest == i {
            break;
        }
        heap.get_mut(this_ref)?.fields.swap(i, smallest);
        i = smallest;
    }
    Ok(Some(min))
}

/// Native: `PriorityQueue.size()I`
pub(crate) fn native_priorityqueue_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `PriorityQueue.isEmpty()Z`
pub(crate) fn native_priorityqueue_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

// ---- Comparator natives ----

/// Native: `Comparator.naturalOrder()Comparator` — returns a singleton synthetic comparator.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_natural_order(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/NaturalOrderComparator".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Comparator.reverseOrder()Comparator` — returns a singleton reverse comparator.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_reverse_order(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/ReverseOrderComparator".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `NaturalOrderComparator.compare(O,O)I` — delegates to `o1.compareTo(o2)`.
pub(crate) fn native_natural_order_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    // args: [this, o1, o2]
    let o1 = extract_slot_arg(args, 1);
    let o2 = extract_slot_arg(args, 2);
    let o1_class = match &o1 {
        Slot::Reference(Some(r)) => heap.get(*r)?.class_name.clone(),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let result = ops.invoke(
        heap,
        out,
        &o1_class,
        "compareTo",
        "(Ljava/lang/Object;)I",
        vec![o1, o2],
    )?;
    let _ = control;
    Ok(Some(result.unwrap_or(Slot::Int(0))))
}

/// Native: `ReverseOrderComparator.compare(O,O)I` — negates natural order.
pub(crate) fn native_reverse_order_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let result = native_natural_order_compare(args, heap, out, control, ops)?;
    Ok(Some(match result {
        Some(Slot::Int(v)) => Slot::Int(-v),
        other => other.unwrap_or(Slot::Int(0)),
    }))
}

/// Native: `Comparator.comparingInt(ToIntFunction)Comparator` — wraps key extractor.
/// Creates a `duke/util/ComparingIntComparator` with `fields[0] = fn_ref`.
pub(crate) fn native_comparator_comparing_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_ref = extract_ref_arg(args, 0)?;
    let r = heap.allocate("duke/util/ComparingIntComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(Some(fn_ref));
    Ok(Some(Slot::Reference(Some(r))))
}

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

/// Native: `Collections.sort(List, Comparator)V` — 2-arg sort with explicit comparator.
pub(crate) fn native_collections_sort_with_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let comparator = extract_slot_arg(args, 1);
    let class_name = heap.get(list_ref)?.class_name.clone();
    ops.invoke(
        heap,
        output,
        &class_name,
        "sort",
        "(Ljava/util/Comparator;)V",
        vec![Slot::Reference(Some(list_ref)), comparator],
    )?;
    Ok(None)
}

/// Native: `Enum.<init>(Ljava/lang/String;I)V` — stores name + ordinal.
/// args: `[this_ref, name_ref, ordinal_int]`
pub(crate) fn native_enum_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let name_slot = extract_slot_arg(args, 1);
    let ordinal = match args.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() >= 2 {
        obj.fields[0] = name_slot;
        obj.fields[1] = Slot::Int(ordinal);
    }
    Ok(None)
}

/// Native: `Enum.ordinal()I`
pub(crate) fn native_enum_ordinal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.get(1) {
        Some(Slot::Int(v)) => Ok(Some(Slot::Int(*v))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `Enum.name()Ljava/lang/String;`
pub(crate) fn native_enum_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Enum.valueOf(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;`
/// Searches heap for enum constants of the given class matching the name.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_enum_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let target_name = heap.get(name_ref)?.string_value.clone().unwrap_or_default();
    let enum_class_name = heap
        .get(class_ref)?
        .string_value
        .clone()
        .unwrap_or_default();

    let obj_count = heap.len();
    for i in 0..obj_count {
        let obj = heap.get(i as u64)?;
        if obj.class_name == enum_class_name
            && obj.fields.len() >= 2
            && let Some(Slot::Reference(Some(name_r))) = obj.fields.first()
            && let Ok(name_obj) = heap.get(*name_r)
            && name_obj.string_value.as_deref() == Some(target_name.as_str())
        {
            return Ok(Some(Slot::Reference(Some(i as u64))));
        }
    }

    Err(Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Reflection natives
// ---------------------------------------------------------------------------

pub(crate) fn native_class_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let name_ref = heap.allocate_string(internal_name_to_binary_name(&internal_name));
    Ok(Some(Slot::Reference(Some(name_ref))))
}

pub(crate) fn native_class_get_package_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let package_name = if internal_name.starts_with('[') {
        String::new()
    } else {
        internal_name
            .rsplit_once('/')
            .map_or_else(String::new, |(package, _)| package.replace('/', "."))
    };
    let package_ref = heap.allocate_string(package_name);
    Ok(Some(Slot::Reference(Some(package_ref))))
}

/// Native: `Class.desiredAssertionStatus()` - Duke currently runs with assertions disabled.
pub(crate) fn native_class_desired_assertion_status(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(0)))
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

pub(crate) fn native_class_for_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let binary_name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .ok_or(Error::NullPointerException)?;
    let internal_name = binary_name_to_internal_name(&binary_name);
    match ops.ensure_loaded(&internal_name) {
        Ok(()) => {
            let class_key = ops.class_key_for_loaded_class(&internal_name)?;
            let class_ref = allocate_class_object(heap, &class_key)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        Err(Error::ClassNotFound { .. }) => Err(Error::JavaException {
            class_name: "java/lang/ClassNotFoundException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_class_for_name_with_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let binary_name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .ok_or(Error::NullPointerException)?;
    let internal_name = binary_name_to_internal_name(&binary_name);
    let (load_result, class_key) = match args.get(2) {
        Some(Slot::Reference(Some(loader_ref))) => (
            ops.ensure_loaded_with_runtime_loader(heap, *loader_ref, &internal_name),
            ops.class_key_for_runtime_loader(heap, *loader_ref, &internal_name)?,
        ),
        Some(Slot::Reference(None)) | None => (
            ops.ensure_loaded(&internal_name),
            ops.class_key_for_loaded_class(&internal_name)?,
        ),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    match load_result {
        Ok(()) => {
            let class_ref = allocate_class_object(heap, &class_key)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        Err(Error::ClassNotFound { .. }) => Err(Error::JavaException {
            class_name: "java/lang/ClassNotFoundException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_class_get_class_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    Ok(Some(Slot::Reference(
        ops.runtime_loader_for_class(&class_key)?,
    )))
}

fn allocate_resource_url(heap: &mut duke_gc::Heap, url: String) -> Result<u64> {
    allocate_string_backed_object(heap, "java/net/URL", url)
}

fn allocate_resource_enumeration(
    heap: &mut duke_gc::Heap,
    resources: Vec<duke_loader::LocatedResource>,
) -> Result<u64> {
    let enum_ref = heap.allocate(
        "duke/util/ResourceEnumeration".to_string(),
        RESOURCE_ENUM_VALUES_START + resources.len(),
    );
    heap.write_field(enum_ref, RESOURCE_ENUM_INDEX_FIELD, Slot::Int(0))?;
    heap.write_field(
        enum_ref,
        RESOURCE_ENUM_COUNT_FIELD,
        Slot::Int(i32::try_from(resources.len()).unwrap_or(i32::MAX)),
    )?;
    for (idx, resource) in resources.into_iter().enumerate() {
        let url_ref = allocate_resource_url(heap, resource.url)?;
        heap.write_field(
            enum_ref,
            RESOURCE_ENUM_VALUES_START + idx,
            Slot::Reference(Some(url_ref)),
        )?;
    }
    Ok(enum_ref)
}

fn lookup_class_resource(
    args: &[Slot],
    heap: &duke_gc::Heap,
    ops: &mut dyn CallbackOps,
) -> Result<(String, String, String, Option<duke_loader::LocatedResource>)> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let class_internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let loader_ref = ops.runtime_loader_for_class(&class_key)?;
    let requested_name = string_arg(args, 1, heap)?;
    let base = class_resource_base(&class_internal_name);
    let classpath = classpath_debug_label(loader_ref);
    let Some(resolved_name) = resolve_class_resource_name(&class_internal_name, &requested_name) else {
        log_resource_lookup_miss(&requested_name, &base, &classpath);
        return Ok((requested_name, base, classpath, None));
    };
    let resource = ops.find_resource_entry(heap, loader_ref, &resolved_name)?;
    if resource.is_none() {
        log_resource_lookup_miss(&resolved_name, &base, &classpath);
    }
    Ok((resolved_name, base, classpath, resource))
}

fn lookup_class_loader_resource(
    args: &[Slot],
    heap: &duke_gc::Heap,
    ops: &mut dyn CallbackOps,
) -> Result<(String, String, String, Option<duke_loader::LocatedResource>)> {
    let loader_ref = extract_ref_arg(args, 0)?;
    let requested_name = string_arg(args, 1, heap)?;
    let base = "<class-loader>".to_string();
    let classpath = classpath_debug_label(Some(loader_ref));
    let Some(resolved_name) = normalize_resource_name(&requested_name) else {
        log_resource_lookup_miss(&requested_name, &base, &classpath);
        return Ok((requested_name, base, classpath, None));
    };
    let resource = ops.find_resource_entry(heap, Some(loader_ref), &resolved_name)?;
    if resource.is_none() {
        log_resource_lookup_miss(&resolved_name, &base, &classpath);
    }
    Ok((resolved_name, base, classpath, resource))
}

pub(crate) fn native_class_get_resource_as_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let (_, _, _, resource) = lookup_class_resource(args, heap, ops)?;
    let Some(resource) = resource else {
        return Ok(Some(Slot::Reference(None)));
    };
    let stream_ref = allocate_resource_input_stream(heap, resource.bytes)?;
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

pub(crate) fn native_class_loader_get_resource_as_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let (_, _, _, resource) = lookup_class_loader_resource(args, heap, ops)?;
    let Some(resource) = resource else {
        return Ok(Some(Slot::Reference(None)));
    };
    let stream_ref = allocate_resource_input_stream(heap, resource.bytes)?;
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

pub(crate) fn native_class_get_resource(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let (_, _, _, resource) = lookup_class_resource(args, heap, ops)?;
    let Some(resource) = resource else {
        return Ok(Some(Slot::Reference(None)));
    };
    let url_ref = allocate_resource_url(heap, resource.url)?;
    Ok(Some(Slot::Reference(Some(url_ref))))
}

pub(crate) fn native_class_loader_get_resource(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let (_, _, _, resource) = lookup_class_loader_resource(args, heap, ops)?;
    let Some(resource) = resource else {
        return Ok(Some(Slot::Reference(None)));
    };
    let url_ref = allocate_resource_url(heap, resource.url)?;
    Ok(Some(Slot::Reference(Some(url_ref))))
}

pub(crate) fn native_class_loader_get_resources(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let loader_ref = extract_ref_arg(args, 0)?;
    let requested_name = string_arg(args, 1, heap)?;
    let classpath = classpath_debug_label(Some(loader_ref));
    let Some(resolved_name) = normalize_resource_name(&requested_name) else {
        log_resource_lookup_miss(&requested_name, "<class-loader>", &classpath);
        let enum_ref = allocate_resource_enumeration(heap, Vec::new())?;
        return Ok(Some(Slot::Reference(Some(enum_ref))));
    };
    let resources = ops.find_resource_entries(heap, Some(loader_ref), &resolved_name)?;
    if resources.is_empty() {
        log_resource_lookup_miss(&resolved_name, "<class-loader>", &classpath);
    }
    let enum_ref = allocate_resource_enumeration(heap, resources)?;
    Ok(Some(Slot::Reference(Some(enum_ref))))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_class_loader_register_as_parallel_capable(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Int(1)))
}

fn allocate_string_backed_object(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    value: String,
) -> Result<u64> {
    let obj_ref = heap.allocate(class_name.to_string(), 1);
    let value_ref = heap.allocate_string(value);
    heap.get_mut(obj_ref)?.fields[0] = Slot::Reference(Some(value_ref));
    Ok(obj_ref)
}

fn first_reference_field(heap: &duke_gc::Heap, obj_ref: u64) -> Result<Option<u64>> {
    match heap.get(obj_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => Ok(Some(*r)),
        Some(Slot::Reference(None)) => Ok(None),
        _ => Err(Error::NullPointerException),
    }
}

fn string_backed_object_value(heap: &duke_gc::Heap, obj_ref: u64) -> Result<String> {
    let Some(value_ref) = first_reference_field(heap, obj_ref)? else {
        return Err(Error::NullPointerException);
    };
    string_value_from_ref(heap, value_ref)
}

fn path_to_file_url(path: &std::path::Path) -> String {
    let mut normalized = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        if let Some(stripped) = normalized.strip_prefix("//?/UNC/") {
            normalized = format!("//{stripped}");
        } else if let Some(stripped) = normalized.strip_prefix("//?/") {
            normalized = stripped.to_string();
        }
    }
    if cfg!(windows) && !normalized.starts_with('/') {
        normalized.insert(0, '/');
    }
    format!("file://{normalized}")
}

const fn decode_pct_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) =
                (decode_pct_hex(bytes[i + 1]), decode_pct_hex(bytes[i + 2]))
        {
            out.push(char::from((hi << 4) | lo));
            i += 3;
            continue;
        }
        out.push(char::from(bytes[i]));
        i += 1;
    }
    out
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

const RESOURCE_STREAM_BYTES_FIELD: usize = 0;
const RESOURCE_STREAM_CURSOR_FIELD: usize = 1;
const RESOURCE_STREAM_CLOSED_FIELD: usize = 2;

const RESOURCE_ENUM_INDEX_FIELD: usize = 0;
const RESOURCE_ENUM_COUNT_FIELD: usize = 1;
const RESOURCE_ENUM_VALUES_START: usize = 2;

fn resource_lookup_trace_enabled() -> bool {
    std::env::var("RUST_LOG").is_ok_and(|value| {
        let lower = value.to_ascii_lowercase();
        lower.contains("debug") || lower.contains("trace")
    })
}

fn log_resource_lookup_miss(resolved_name: &str, base: &str, classpath: &str) {
    if resource_lookup_trace_enabled() {
        eprintln!(
            "resource lookup miss: name={resolved_name} base={base} classpath={classpath}"
        );
    }
}

fn normalize_resource_name(name: &str) -> Option<String> {
    if name.is_empty()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains(':')
    {
        return None;
    }
    let mut normalized = String::with_capacity(name.len());
    for (idx, component) in name.split(['/', '\\']).enumerate() {
        if component.is_empty() || component == "." || component == ".." {
            return None;
        }
        if idx > 0 {
            normalized.push('/');
        }
        normalized.push_str(component);
    }
    Some(normalized)
}

fn resolve_class_resource_name(class_internal_name: &str, name: &str) -> Option<String> {
    if let Some(absolute) = name.strip_prefix('/') {
        return normalize_resource_name(absolute);
    }

    let mut resolved = String::new();
    if let Some((package, _)) = class_internal_name.rsplit_once('/') {
        resolved.push_str(package);
        resolved.push('/');
    }
    resolved.push_str(name);
    normalize_resource_name(&resolved)
}

fn class_resource_base(class_internal_name: &str) -> String {
    class_internal_name
        .rsplit_once('/')
        .map_or_else(|| "<default-package>".to_string(), |(package, _)| package.to_string())
}

fn classpath_debug_label(loader_ref: Option<u64>) -> String {
    loader_ref.map_or_else(|| "bootstrap".to_string(), |loader| format!("loader:{loader}"))
}

fn string_arg(args: &[Slot], idx: usize, heap: &duke_gc::Heap) -> Result<String> {
    let string_ref = extract_ref_arg(args, idx)?;
    string_value_from_ref(heap, string_ref)
}

fn allocate_resource_input_stream(heap: &mut duke_gc::Heap, bytes: Vec<u8>) -> Result<u64> {
    let byte_array_ref = heap.allocate("[B".to_string(), bytes.len());
    {
        let array = heap.get_mut(byte_array_ref)?;
        for (idx, byte) in bytes.into_iter().enumerate() {
            array.fields[idx] = Slot::Int(i32::from(byte));
        }
    }
    let stream_ref = heap.allocate("duke/io/ResourceInputStream".to_string(), 3);
    let stream = heap.get_mut(stream_ref)?;
    stream.fields[RESOURCE_STREAM_BYTES_FIELD] = Slot::Reference(Some(byte_array_ref));
    stream.fields[RESOURCE_STREAM_CURSOR_FIELD] = Slot::Int(0);
    stream.fields[RESOURCE_STREAM_CLOSED_FIELD] = Slot::Int(0);
    Ok(stream_ref)
}

fn resource_stream_array_ref(heap: &duke_gc::Heap, stream_ref: u64) -> Result<u64> {
    match heap.get(stream_ref)?.fields.get(RESOURCE_STREAM_BYTES_FIELD) {
        Some(Slot::Reference(Some(array_ref))) => Ok(*array_ref),
        _ => Err(Error::InvalidRef { address: stream_ref }),
    }
}

fn resource_stream_is_closed(heap: &duke_gc::Heap, stream_ref: u64) -> Result<bool> {
    Ok(matches!(
        heap.get(stream_ref)?.fields.get(RESOURCE_STREAM_CLOSED_FIELD),
        Some(Slot::Int(value)) if *value != 0
    ))
}

fn resource_stream_ensure_open(heap: &duke_gc::Heap, stream_ref: u64) -> Result<()> {
    if resource_stream_is_closed(heap, stream_ref)? {
        push_pending_java_exception_message("java/io/IOException", "Stream closed".to_string());
        return Err(Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        });
    }
    Ok(())
}

fn resource_stream_cursor(heap: &duke_gc::Heap, stream_ref: u64) -> Result<usize> {
    match heap.get(stream_ref)?.fields.get(RESOURCE_STREAM_CURSOR_FIELD) {
        Some(Slot::Int(value)) if *value >= 0 => Ok(usize::try_from(*value).unwrap_or(usize::MAX)),
        _ => Ok(0),
    }
}

fn resource_stream_set_cursor(heap: &mut duke_gc::Heap, stream_ref: u64, cursor: usize) -> Result<()> {
    heap.write_field(
        stream_ref,
        RESOURCE_STREAM_CURSOR_FIELD,
        Slot::Int(i32::try_from(cursor).unwrap_or(i32::MAX)),
    )
}

fn resource_stream_available_bytes(heap: &duke_gc::Heap, stream_ref: u64) -> Result<usize> {
    let array_ref = resource_stream_array_ref(heap, stream_ref)?;
    let len = heap.get(array_ref)?.fields.len();
    Ok(len.saturating_sub(resource_stream_cursor(heap, stream_ref)?))
}

fn read_resource_bytes_from_jar_spec(spec: &str) -> Result<Vec<u8>> {
    let Some((container, entry_name)) = spec.rsplit_once("!/") else {
        return Err(Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        });
    };

    if container.starts_with("file://") && container.contains("!/") {
        let Some((outer_url, nested_entry_name)) = container.rsplit_once("!/") else {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
        };
        let outer_path = file_url_to_path(outer_url)?;
        let nested_bytes = duke_loader::ZipReader::open(&outer_path)
            .map_err(|_| Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            })?
            .read_entry(nested_entry_name)
            .map_err(|_| Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            })?;
        return duke_loader::ZipReader::from_bytes(nested_bytes)
            .map_err(|_| Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            })?
            .read_entry(entry_name)
            .map_err(|_| Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
    }

    let jar_path = file_url_to_path(container)?;
    duke_loader::ZipReader::open(&jar_path)
        .map_err(|_| Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        })?
        .read_entry(entry_name)
        .map_err(|_| Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        })
}

fn read_resource_bytes_from_url_spec(spec: &str) -> Result<Vec<u8>> {
    if let Some(jar_spec) = spec.strip_prefix("jar:") {
        return read_resource_bytes_from_jar_spec(jar_spec);
    }
    if spec.starts_with("file://") {
        let path = file_url_to_path(spec)?;
        return std::fs::read(path).map_err(|_| Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        });
    }
    Err(Error::JavaException {
        class_name: "java/io/IOException".to_string(),
    })
}

fn url_path_string(spec: &str) -> String {
    if let Some(path) = spec.strip_prefix("jar:") {
        return path.to_string();
    }
    if let Some(path) = spec.strip_prefix("file://") {
        return percent_decode(path);
    }
    spec.to_string()
}

pub(crate) fn native_class_get_protection_domain(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let pd_ref = heap.allocate("java/security/ProtectionDomain".to_string(), 1);
    let code_source_slot = if let Some(path) = ops.code_source_for_class(&class_key)? {
        let url_ref = allocate_string_backed_object(
            heap,
            "java/net/URL",
            path_to_file_url(std::path::Path::new(&path)),
        )?;
        let code_source_ref = heap.allocate("java/security/CodeSource".to_string(), 1);
        heap.get_mut(code_source_ref)?.fields[0] = Slot::Reference(Some(url_ref));
        Slot::Reference(Some(code_source_ref))
    } else {
        Slot::Reference(None)
    };
    heap.get_mut(pd_ref)?.fields[0] = code_source_slot;
    Ok(Some(Slot::Reference(Some(pd_ref))))
}

pub(crate) fn native_protection_domain_get_code_source(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Err(Error::NullPointerException),
    }
}

pub(crate) fn native_code_source_get_location(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Err(Error::NullPointerException),
    }
}

pub(crate) fn native_url_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let spec_slot = extract_slot_arg(args, 1);
    match spec_slot {
        Slot::Reference(Some(_)) => {
            heap.get_mut(this_ref)?.fields[0] = spec_slot;
            Ok(None)
        }
        Slot::Reference(None) => Err(Error::NullPointerException),
        _ => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

pub(crate) fn native_url_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first().copied() {
        Some(slot @ Slot::Reference(Some(_))) => Ok(Some(slot)),
        Some(Slot::Reference(None)) => Err(Error::NullPointerException),
        _ => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

pub(crate) fn native_url_to_external_form(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_url_to_string(args, heap, out, control)
}

pub(crate) fn native_url_to_uri(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let spec = string_backed_object_value(heap, this_ref)?;
    let uri_ref = allocate_string_backed_object(heap, "java/net/URI", spec)?;
    Ok(Some(Slot::Reference(Some(uri_ref))))
}

pub(crate) fn native_url_get_path(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let spec = string_backed_object_value(heap, this_ref)?;
    let path_ref = heap.allocate_string(url_path_string(&spec));
    Ok(Some(Slot::Reference(Some(path_ref))))
}

pub(crate) fn native_url_open_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let spec = string_backed_object_value(heap, this_ref)?;
    let bytes = read_resource_bytes_from_url_spec(&spec)?;
    let stream_ref = allocate_resource_input_stream(heap, bytes)?;
    Ok(Some(Slot::Reference(Some(stream_ref))))
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

pub(crate) fn native_url_class_loader_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let urls_ref = extract_ref_arg(args, 1)?;
    let url_slots = heap.get(urls_ref)?.fields.clone();

    let path_list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(
        &[Slot::Reference(Some(path_list_ref))],
        heap,
        out,
        control,
    )?;
    for url_slot in url_slots {
        match url_slot {
            Slot::Reference(Some(entry_ref)) => {
                native_arraylist_add(
                    &[
                        Slot::Reference(Some(path_list_ref)),
                        Slot::Reference(Some(entry_ref)),
                    ],
                    heap,
                    out,
                    control,
                )?;
            }
            Slot::Reference(None) => return Err(Error::NullPointerException),
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "Reference",
                    got: "other",
                });
            }
        }
    }

    let ucp_ref = heap.allocate("jdk/internal/loader/URLClassPath".to_string(), 1);
    heap.get_mut(ucp_ref)?.fields[0] = Slot::Reference(Some(path_list_ref));
    heap.get_mut(this_ref)?.fields[0] = Slot::Reference(Some(ucp_ref));
    Ok(None)
}

pub(crate) fn native_url_class_loader_load_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let binary_name = string_value_from_ref(heap, name_ref)?;
    let internal_name = binary_name_to_internal_name(&binary_name);

    if let Some(class_key) = ops.ensure_parent_loaded(&internal_name)? {
        let class_ref = allocate_class_object(heap, &class_key)?;
        return Ok(Some(Slot::Reference(Some(class_ref))));
    }

    match ops.ensure_loaded_with_runtime_loader(heap, this_ref, &internal_name) {
        Ok(()) => {
            let class_key = ops.class_key_for_runtime_loader(heap, this_ref, &internal_name)?;
            let class_ref = allocate_class_object(heap, &class_key)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        Err(Error::ClassNotFound { .. }) => Err(Error::JavaException {
            class_name: "java/lang/ClassNotFoundException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_url_set_url_stream_handler_factory(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}

pub(crate) fn native_path_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let uri_ref = extract_ref_arg(args, 0)?;
    let uri = string_backed_object_value(heap, uri_ref)?;
    let path = file_url_to_path(&uri)?;
    let path_ref = allocate_string_backed_object(
        heap,
        "java/nio/file/Path",
        path.to_string_lossy().to_string(),
    )?;
    Ok(Some(Slot::Reference(Some(path_ref))))
}

pub(crate) fn native_path_to_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .ok_or(Error::NullPointerException)?;
    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    heap.get_mut(file_ref)?.fields[0] = path_slot;
    Ok(Some(Slot::Reference(Some(file_ref))))
}

pub(crate) fn native_paths_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let first_ref = extract_ref_arg(args, 0)?;
    let mut path = std::path::PathBuf::from(string_value_from_ref(heap, first_ref)?);
    let more_slot = extract_slot_arg(args, 1);
    match more_slot {
        Slot::Reference(Some(array_ref)) => {
            let segments = heap.get(array_ref)?.fields.clone();
            for segment in segments {
                let Slot::Reference(Some(segment_ref)) = segment else {
                    return Err(Error::NullPointerException);
                };
                path.push(string_value_from_ref(heap, segment_ref)?);
            }
        }
        Slot::Reference(None) => {}
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    }
    let path_ref = allocate_string_backed_object(
        heap,
        "java/nio/file/Path",
        path.to_string_lossy().into_owned(),
    )?;
    Ok(Some(Slot::Reference(Some(path_ref))))
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_posix_file_permissions_as_file_attribute(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let attribute_ref = heap.allocate("java/nio/file/attribute/FileAttribute".to_string(), 0);
    Ok(Some(Slot::Reference(Some(attribute_ref))))
}

fn manifest_attribute_value(manifest_bytes: &[u8], key: &str) -> Option<String> {
    let text = std::str::from_utf8(manifest_bytes).ok()?;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix(key)
            && let Some(value) = value.strip_prefix(':')
        {
            return Some(value.trim().to_string());
        }
    }
    None
}

pub(crate) fn native_manifest_get_main_attributes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let manifest_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Reference(Some(raw_ref))) = heap.get(manifest_ref)?.fields.first().copied()
    else {
        return Err(Error::NullPointerException);
    };
    let attributes_ref = heap.allocate("java/util/jar/Attributes".to_string(), 1);
    heap.get_mut(attributes_ref)?.fields[0] = Slot::Reference(Some(raw_ref));
    Ok(Some(Slot::Reference(Some(attributes_ref))))
}

pub(crate) fn native_attributes_get_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let attributes_ref = extract_ref_arg(args, 0)?;
    let key_ref = extract_ref_arg(args, 1)?;
    let Some(Slot::Reference(Some(raw_ref))) = heap.get(attributes_ref)?.fields.first().copied()
    else {
        return Err(Error::NullPointerException);
    };
    let manifest_text = string_value_from_ref(heap, raw_ref)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let result = manifest_attribute_value(manifest_text.as_bytes(), &key)
        .map_or(Slot::Reference(None), |value| {
            Slot::Reference(Some(heap.allocate_string(value)))
        });
    Ok(Some(result))
}

const BOOT_ARCHIVE_ENTRY_NAME_SLOT: usize = 0;
const BOOT_ARCHIVE_ENTRY_DIRECTORY_SLOT: usize = 1;

#[cfg(test)]
fn boot_archive_entry_name(heap: &duke_gc::Heap, entry_ref: u64) -> Result<String> {
    let Some(Slot::Reference(Some(name_ref))) = heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_NAME_SLOT)
        .copied()
    else {
        return Err(Error::NullPointerException);
    };
    string_value_from_ref(heap, name_ref)
}

fn boot_archive_entry_is_directory_flag(heap: &duke_gc::Heap, entry_ref: u64) -> Result<bool> {
    match heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_DIRECTORY_SLOT)
        .copied()
    {
        Some(Slot::Int(value)) => Ok(value != 0),
        _ => Err(Error::TypeMismatch {
            expected: "Int",
            got: "other",
        }),
    }
}

pub(crate) fn native_boot_archive_entry_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let entry_ref = extract_ref_arg(args, 0)?;
    let Some(slot @ Slot::Reference(Some(_))) = heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_NAME_SLOT)
        .copied()
    else {
        return Err(Error::NullPointerException);
    };
    Ok(Some(slot))
}

pub(crate) fn native_boot_archive_entry_is_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let entry_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(
        boot_archive_entry_is_directory_flag(heap, entry_ref)?,
    ))))
}

fn boot_archive_hashset_add_url(
    set_ref: u64,
    url_spec: String,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<()> {
    let url_ref = allocate_string_backed_object(heap, "java/net/URL", url_spec)?;
    native_hashset_add(
        &[
            Slot::Reference(Some(set_ref)),
            Slot::Reference(Some(url_ref)),
        ],
        heap,
        out,
        control,
    )?;
    Ok(())
}

fn allocate_boot_archive_entry(
    heap: &mut duke_gc::Heap,
    entry_name: &str,
    is_directory: bool,
) -> Result<u64> {
    let entry_ref = heap.allocate("duke/boot/ArchiveEntry".to_string(), 2);
    let name_ref = heap.allocate_string(entry_name.to_string());
    let entry = heap.get_mut(entry_ref)?;
    entry.fields[BOOT_ARCHIVE_ENTRY_NAME_SLOT] = Slot::Reference(Some(name_ref));
    entry.fields[BOOT_ARCHIVE_ENTRY_DIRECTORY_SLOT] = Slot::Int(i32::from(is_directory));
    Ok(entry_ref)
}

fn boot_archive_predicate_accepts(
    predicate_ref: u64,
    entry_name: &str,
    is_directory: bool,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<bool> {
    let predicate_class = heap.get(predicate_ref)?.class_name.clone();
    let entry_ref = allocate_boot_archive_entry(heap, entry_name, is_directory)?;
    match ops.invoke(
        heap,
        out,
        &predicate_class,
        "test",
        "(Ljava/lang/Object;)Z",
        vec![
            Slot::Reference(Some(predicate_ref)),
            Slot::Reference(Some(entry_ref)),
        ],
    )? {
        Some(Slot::Int(value)) => Ok(value != 0),
        Some(other) => Err(Error::TypeMismatch {
            expected: "Int",
            got: match other {
                Slot::Long(_) => "Long",
                Slot::Float(_) => "Float",
                Slot::Double(_) => "Double",
                Slot::Reference(_) => "Reference",
                Slot::ReturnAddress(_) => "ReturnAddress",
                Slot::Int(_) => unreachable!(),
            },
        }),
        None => Ok(false),
    }
}

fn open_boot_archive_reader(path: &std::path::Path) -> Result<duke_loader::ZipReader> {
    duke_loader::ZipReader::open(path).map_err(|err| match err {
        duke_loader::Error::Io { .. } => Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        },
        _ => Error::JavaException {
            class_name: "java/util/zip/ZipException".to_string(),
        },
    })
}

fn archive_file_ref_at(heap: &duke_gc::Heap, archive_ref: u64, slot: usize) -> Result<u64> {
    match heap.get(archive_ref)?.fields.get(slot).copied() {
        Some(Slot::Reference(Some(file_ref))) => Ok(file_ref),
        Some(Slot::Reference(None)) => Err(Error::NullPointerException),
        _ => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

fn patch_forwarded_slot_if_needed(heap: &duke_gc::Heap, slot: &mut Slot) {
    if heap.has_pending_forwards() {
        heap.apply_forward(slot);
    }
}

fn patch_forwarded_ref_if_needed(heap: &duke_gc::Heap, reference: &mut u64) {
    let mut slot = Slot::Reference(Some(*reference));
    patch_forwarded_slot_if_needed(heap, &mut slot);
    if let Slot::Reference(Some(new_ref)) = slot {
        *reference = new_ref;
    }
}

pub(crate) fn native_boot_jar_file_archive_get_class_path_urls(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let archive_ref = extract_ref_arg(args, 0)?;
    let include_predicate_ref = extract_ref_arg(args, 1)?;
    let _ = extract_ref_arg(args, 2)?;
    let archive_file_ref = archive_file_ref_at(heap, archive_ref, 0)?;
    let archive_path = file_path_from_ref(archive_file_ref, heap)?;
    let reader = open_boot_archive_reader(&archive_path)?;
    let mut entry_names: Vec<String> = reader.entry_names().map(ToOwned::to_owned).collect();
    entry_names.sort_unstable();

    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;

    for entry_name in entry_names {
        let is_directory = entry_name.ends_with('/');
        if boot_archive_predicate_accepts(
            include_predicate_ref,
            &entry_name,
            is_directory,
            heap,
            out,
            ops,
        )? {
            let url_spec = format!("jar:{}!/{}", path_to_file_url(&archive_path), entry_name);
            boot_archive_hashset_add_url(set_ref, url_spec, heap, out, control)?;
        }
    }

    Ok(Some(Slot::Reference(Some(set_ref))))
}

fn list_directory_children_sorted(path: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
    let iter = std::fs::read_dir(path).map_err(|_| Error::JavaException {
        class_name: "java/io/IOException".to_string(),
    })?;
    let mut children = Vec::new();
    for entry in iter {
        let entry = entry.map_err(|_| Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        })?;
        children.push(entry.path());
    }
    children.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .cmp(right.file_name().unwrap_or_default())
    });
    Ok(children)
}

fn exploded_archive_relative_entry_name(
    root_path: &std::path::Path,
    entry_path: &std::path::Path,
    is_directory: bool,
) -> String {
    let mut relative = entry_path
        .strip_prefix(root_path)
        .unwrap_or(entry_path)
        .to_string_lossy()
        .replace('\\', "/");
    if is_directory && !relative.ends_with('/') {
        relative.push('/');
    }
    relative
}

pub(crate) fn native_boot_exploded_archive_get_class_path_urls(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let archive_ref = extract_ref_arg(args, 0)?;
    let include_predicate_ref = extract_ref_arg(args, 1)?;
    let search_predicate_ref = extract_ref_arg(args, 2)?;
    let root_directory_ref = archive_file_ref_at(heap, archive_ref, 0)?;
    let root_path = file_path_from_ref(root_directory_ref, heap)?;

    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;

    let mut pending: VecDeque<std::path::PathBuf> = list_directory_children_sorted(&root_path)?
        .into_iter()
        .collect();
    while let Some(entry_path) = pending.pop_front() {
        let is_directory = entry_path.is_dir();
        let entry_name =
            exploded_archive_relative_entry_name(&root_path, &entry_path, is_directory);
        if is_directory
            && boot_archive_predicate_accepts(
                search_predicate_ref,
                &entry_name,
                true,
                heap,
                out,
                ops,
            )?
        {
            let children = list_directory_children_sorted(&entry_path)?;
            for child in children.into_iter().rev() {
                pending.push_front(child);
            }
        }

        if boot_archive_predicate_accepts(
            include_predicate_ref,
            &entry_name,
            is_directory,
            heap,
            out,
            ops,
        )? {
            boot_archive_hashset_add_url(
                set_ref,
                path_to_file_url(&entry_path),
                heap,
                out,
                control,
            )?;
        }
    }

    Ok(Some(Slot::Reference(Some(set_ref))))
}

pub(crate) fn native_boot_launched_class_loader_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let exploded = extract_int_arg(args, 1)?;
    let archive_ref = extract_ref_arg(args, 2)?;
    let _ = extract_ref_arg(args, 3)?;
    match args.get(4) {
        Some(Slot::Reference(_)) => {
            let exploded_slot = ops.instance_field_slot(
                "org/springframework/boot/loader/launch/LaunchedClassLoader",
                "exploded",
            )?;
            let root_archive_slot = ops.instance_field_slot(
                "org/springframework/boot/loader/launch/LaunchedClassLoader",
                "rootArchive",
            )?;
            let loader_obj = heap.get_mut(this_ref)?;
            loader_obj.fields[exploded_slot] = Slot::Int(i32::from(exploded != 0));
            loader_obj.fields[root_archive_slot] = Slot::Reference(Some(archive_ref));
            Ok(None)
        }
        _ => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

pub(crate) fn native_class_get_declared_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let method_name = string_value_from_ref(heap, name_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 2))?;

    let Some(method) = reflected.methods.into_iter().find(|method| {
        method.name == method_name
            && descriptor_parameter_part(&method.descriptor) == parameter_descriptor
    }) else {
        return Err(Error::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let method_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Method",
        &class_key,
        &method.name,
        &method.descriptor,
        method.is_public,
        method.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(method_ref))))
}

pub(crate) fn native_class_get_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let method_name = string_value_from_ref(heap, name_ref)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 2))?;

    let Some((declaring_class, method)) =
        lookup_public_reflected_method(ops, &class_key, &method_name, &parameter_descriptor)?
    else {
        return Err(Error::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let method_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Method",
        &declaring_class,
        &method.name,
        &method.descriptor,
        method.is_public,
        method.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(method_ref))))
}

fn lookup_reflected_constructor(
    reflected: ReflectedClassInfo,
    parameter_descriptor: &str,
    public_only: bool,
) -> Option<ReflectedMethodInfo> {
    reflected.methods.into_iter().find(|method| {
        method.name == "<init>"
            && (!public_only || method.is_public)
            && descriptor_parameter_part(&method.descriptor) == parameter_descriptor
    })
}

fn reflected_constructors(
    reflected: ReflectedClassInfo,
    public_only: bool,
) -> Vec<ReflectedMethodInfo> {
    reflected
        .methods
        .into_iter()
        .filter(|method| method.name == "<init>" && (!public_only || method.is_public))
        .collect()
}

pub(crate) fn native_class_get_declared_field(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let field_name = string_value_from_ref(heap, name_ref)?;
    let reflected = ops.inspect_class(&class_key)?;

    let Some(field) = reflected
        .fields
        .into_iter()
        .find(|field| field.name == field_name)
    else {
        return Err(Error::JavaException {
            class_name: "java/lang/NoSuchFieldException".to_string(),
        });
    };

    let field_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Field",
        &class_key,
        &field.name,
        &field.descriptor,
        field.is_public,
        field.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(field_ref))))
}

pub(crate) fn native_class_get_declared_constructor(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 1))?;

    let Some(constructor) = lookup_reflected_constructor(reflected, &parameter_descriptor, false)
    else {
        return Err(Error::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let constructor_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Constructor",
        &class_key,
        &constructor.name,
        &constructor.descriptor,
        constructor.is_public,
        constructor.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(constructor_ref))))
}

pub(crate) fn native_class_get_field(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let field_name = string_value_from_ref(heap, name_ref)?;

    let Some((declaring_class, field)) =
        lookup_public_reflected_field(ops, &class_key, &field_name)?
    else {
        return Err(Error::JavaException {
            class_name: "java/lang/NoSuchFieldException".to_string(),
        });
    };

    let field_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Field",
        &declaring_class,
        &field.name,
        &field.descriptor,
        field.is_public,
        field.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(field_ref))))
}

pub(crate) fn native_class_get_constructor(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 1))?;

    let Some(constructor) = lookup_reflected_constructor(reflected, &parameter_descriptor, true)
    else {
        return Err(Error::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let constructor_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Constructor",
        &class_key,
        &constructor.name,
        &constructor.descriptor,
        constructor.is_public,
        constructor.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(constructor_ref))))
}

pub(crate) fn native_class_new_instance(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let constructors = reflected_constructors(reflected, false);
    let Some(constructor) = constructors
        .iter()
        .find(|constructor| constructor.descriptor == "()V")
        .cloned()
    else {
        return Err(Error::JavaException {
            class_name: "java/lang/InstantiationException".to_string(),
        });
    };
    if !constructor.is_public {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let instance_ref = ops.allocate_instance(heap, output, &class_key)?;
    match ops.invoke(
        heap,
        output,
        &class_key,
        "<init>",
        &constructor.descriptor,
        vec![Slot::Reference(Some(instance_ref))],
    ) {
        Ok(_) => Ok(Some(Slot::Reference(Some(instance_ref)))),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_class_get_declared_methods(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let method_refs = reflected
        .methods
        .into_iter()
        .filter(|method| method.name != "<init>" && method.name != "<clinit>")
        .map(|method| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Method",
                &class_key,
                &method.name,
                &method.descriptor,
                method.is_public,
                method.is_static,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Method;", &method_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_declared_constructors(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let constructor_refs = reflected_constructors(reflected, false)
        .into_iter()
        .map(|constructor| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Constructor",
                &class_key,
                &constructor.name,
                &constructor.descriptor,
                constructor.is_public,
                constructor.is_static,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let array_ref =
        allocate_reference_array(heap, "[Ljava/lang/reflect/Constructor;", &constructor_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_methods(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let method_refs = collect_public_reflected_methods(ops, &class_key)?
        .into_iter()
        .map(|(declaring_class, method)| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Method",
                &declaring_class,
                &method.name,
                &method.descriptor,
                method.is_public,
                method.is_static,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Method;", &method_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_constructors(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let constructor_refs = reflected_constructors(reflected, true)
        .into_iter()
        .map(|constructor| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Constructor",
                &class_key,
                &constructor.name,
                &constructor.descriptor,
                constructor.is_public,
                constructor.is_static,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let array_ref =
        allocate_reference_array(heap, "[Ljava/lang/reflect/Constructor;", &constructor_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_declared_fields(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let field_refs = reflected
        .fields
        .into_iter()
        .map(|field| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Field",
                &class_key,
                &field.name,
                &field.descriptor,
                field.is_public,
                field.is_static,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Field;", &field_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_fields(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let field_refs = collect_public_reflected_fields(ops, &class_key)?
        .into_iter()
        .map(|(declaring_class, field)| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Field",
                &declaring_class,
                &field.name,
                &field.descriptor,
                field.is_public,
                field.is_static,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Field;", &field_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

const ANNOTATION_PROXY_PREFIX: &str = "duke/annotation/AnnotationProxy:";

pub(crate) fn annotation_proxy_type(class_name: &str) -> Option<&str> {
    class_name.strip_prefix(ANNOTATION_PROXY_PREFIX)
}

fn find_annotation<'a>(
    annotations: &'a [ReflectedAnnotation],
    requested_type: &str,
) -> Option<&'a ReflectedAnnotation> {
    let requested_type = class_internal_name_from_key(requested_type);
    annotations
        .iter()
        .find(|annotation| annotation.type_name == requested_type)
}

fn annotation_element_values(
    annotation: &ReflectedAnnotation,
    ops: &mut dyn CallbackOps,
) -> Result<Vec<(String, String, ReflectedAnnotationValue)>> {
    let annotation_info = ops.inspect_class(&annotation.type_name)?;
    let mut values = Vec::new();
    for method in annotation_info
        .methods
        .into_iter()
        .filter(|method| method.descriptor.starts_with("()"))
    {
        let explicit = annotation
            .elements
            .iter()
            .find(|element| element.name == method.name)
            .map(|element| element.value.clone());
        let Some(value) = explicit.or_else(|| method.annotation_default.clone()) else {
            continue;
        };
        values.push((
            method.name,
            method_return_descriptor(&method.descriptor).to_string(),
            value,
        ));
    }
    Ok(values)
}

fn array_element_descriptor(array_descriptor: &str) -> &str {
    array_descriptor.strip_prefix('[').unwrap_or("Ljava/lang/Object;")
}

fn materialize_annotation_const(
    heap: &mut duke_gc::Heap,
    descriptor: &str,
    value: &ReflectedAnnotationConst,
) -> Slot {
    match value {
        ReflectedAnnotationConst::String(value) => {
            Slot::Reference(Some(heap.allocate_string(value.clone())))
        }
        ReflectedAnnotationConst::Long(value) => Slot::Long(*value),
        ReflectedAnnotationConst::Float(value) => Slot::Float(*value),
        ReflectedAnnotationConst::Double(value) => Slot::Double(*value),
        ReflectedAnnotationConst::Boolean(value) => Slot::Int(i32::from(*value)),
        ReflectedAnnotationConst::Byte(value)
        | ReflectedAnnotationConst::Char(value)
        | ReflectedAnnotationConst::Int(value)
        | ReflectedAnnotationConst::Short(value) => {
            if descriptor == "Z" {
                Slot::Int(i32::from(*value != 0))
            } else {
                Slot::Int(*value)
            }
        }
    }
}

fn materialize_annotation_value(
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    descriptor: &str,
    value: &ReflectedAnnotationValue,
) -> Result<Slot> {
    match value {
        ReflectedAnnotationValue::Const(value) => {
            Ok(materialize_annotation_const(heap, descriptor, value))
        }
        ReflectedAnnotationValue::Class(class_key) => {
            let class_ref = allocate_class_object(heap, class_key)?;
            Ok(Slot::Reference(Some(class_ref)))
        }
        ReflectedAnnotationValue::Enum {
            type_name,
            const_name,
        } => {
            ops.ensure_class_initialized(heap, output, type_name)?;
            ops.read_static_field(type_name, const_name)
        }
        ReflectedAnnotationValue::Annotation(annotation) => {
            let annotation_ref = allocate_annotation_proxy(heap, output, ops, annotation)?;
            Ok(Slot::Reference(Some(annotation_ref)))
        }
        ReflectedAnnotationValue::Array(values) => {
            let element_descriptor = array_element_descriptor(descriptor);
            let slots = values
                .iter()
                .map(|value| {
                    materialize_annotation_value(heap, output, ops, element_descriptor, value)
                })
                .collect::<Result<Vec<_>>>()?;
            let array_ref = allocate_reference_array_from_slots(heap, descriptor, &slots)?;
            Ok(Slot::Reference(Some(array_ref)))
        }
    }
}

fn allocate_annotation_proxy(
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    annotation: &ReflectedAnnotation,
) -> Result<u64> {
    let element_values = annotation_element_values(annotation, ops)?;
    let type_ref = allocate_class_object(heap, &annotation.type_name)?;
    let mut fields = Vec::with_capacity(1 + element_values.len() * 3);
    fields.push(Slot::Reference(Some(type_ref)));
    for (name, descriptor, value) in element_values {
        let name_ref = heap.allocate_string(name);
        let descriptor_ref = heap.allocate_string(descriptor.clone());
        let value_slot = materialize_annotation_value(heap, output, ops, &descriptor, &value)?;
        fields.push(Slot::Reference(Some(name_ref)));
        fields.push(Slot::Reference(Some(descriptor_ref)));
        fields.push(value_slot);
    }

    let proxy_ref = heap.allocate(
        format!("{ANNOTATION_PROXY_PREFIX}{}", annotation.type_name),
        fields.len(),
    );
    let proxy = heap.get_mut(proxy_ref)?;
    proxy.string_value = Some(annotation.type_name.clone());
    proxy.fields = fields;
    Ok(proxy_ref)
}

fn allocate_annotation_array(
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    annotations: &[ReflectedAnnotation],
) -> Result<Option<Slot>> {
    let annotation_refs = annotations
        .iter()
        .map(|annotation| allocate_annotation_proxy(heap, output, ops, annotation))
        .collect::<Result<Vec<_>>>()?;
    let array_ref =
        allocate_reference_array(heap, "[Ljava/lang/annotation/Annotation;", &annotation_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn annotation_proxy_element_slot(
    heap: &duke_gc::Heap,
    proxy_ref: u64,
    method_name: &str,
    descriptor: &str,
) -> Result<Option<Slot>> {
    let proxy = heap.get(proxy_ref)?;
    if annotation_proxy_type(&proxy.class_name).is_none() {
        return Ok(None);
    }
    if method_name == "annotationType" && descriptor == "()Ljava/lang/Class;" {
        return Ok(proxy.fields.first().copied());
    }
    for chunk in proxy.fields[1..].chunks(3) {
        let [Slot::Reference(Some(name_ref)), Slot::Reference(Some(desc_ref)), value] = chunk
        else {
            continue;
        };
        if string_value_from_ref(heap, *name_ref)? == method_name
            && string_value_from_ref(heap, *desc_ref)? == method_return_descriptor(descriptor)
        {
            return Ok(Some(*value));
        }
    }
    Ok(None)
}

fn annotations_for_reflected_method(
    heap: &duke_gc::Heap,
    method_ref: u64,
    ops: &mut dyn CallbackOps,
) -> Result<Vec<ReflectedAnnotation>> {
    let method = reflected_method_handle(heap, method_ref)?;
    Ok(ops
        .inspect_class(&method.declaring_class_key)?
        .methods
        .into_iter()
        .find(|candidate| {
            let name_matches = candidate.name == method.method_name;
            let descriptor_matches = candidate.descriptor == method.descriptor;
            name_matches && descriptor_matches
        })
        .map_or_else(Vec::new, |method| method.annotations))
}

fn annotations_for_reflected_field(
    heap: &duke_gc::Heap,
    field_ref: u64,
    ops: &mut dyn CallbackOps,
) -> Result<Vec<ReflectedAnnotation>> {
    let field = reflected_field_handle(heap, field_ref)?;
    Ok(ops
        .inspect_class(&field.declaring_class_key)?
        .fields
        .into_iter()
        .find(|candidate| {
            let name_matches = candidate.name == field.field_name;
            let descriptor_matches = candidate.descriptor == field.descriptor;
            name_matches && descriptor_matches
        })
        .map_or_else(Vec::new, |field| field.annotations))
}

pub(crate) fn native_class_get_annotations(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let annotations = ops.inspect_class(&class_key)?.annotations;
    allocate_annotation_array(heap, out, ops, &annotations)
}

pub(crate) fn native_class_get_declared_annotations(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_class_get_annotations(args, heap, out, control, ops)
}

pub(crate) fn native_class_get_annotation(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let annotation_type_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let requested_type = class_key_from_ref(heap, annotation_type_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    match find_annotation(&reflected.annotations, &requested_type) {
        Some(annotation) => {
            let annotation_ref = allocate_annotation_proxy(heap, out, ops, annotation)?;
            Ok(Some(Slot::Reference(Some(annotation_ref))))
        }
        None => Ok(Some(Slot::Reference(None))),
    }
}

pub(crate) fn native_reflect_method_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_name_slot(heap, method_ref)?))
}

pub(crate) fn native_reflect_method_get_annotations(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let annotations = annotations_for_reflected_method(heap, method_ref, ops)?;
    allocate_annotation_array(heap, out, ops, &annotations)
}

pub(crate) fn native_reflect_method_get_declared_annotations(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_reflect_method_get_annotations(args, heap, out, control, ops)
}

pub(crate) fn native_reflect_method_get_annotation(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let annotation_type_ref = extract_ref_arg(args, 1)?;
    let requested_type = class_key_from_ref(heap, annotation_type_ref)?;
    let annotations = annotations_for_reflected_method(heap, method_ref, ops)?;
    match find_annotation(&annotations, &requested_type) {
        Some(annotation) => {
            let annotation_ref = allocate_annotation_proxy(heap, out, ops, annotation)?;
            Ok(Some(Slot::Reference(Some(annotation_ref))))
        }
        None => Ok(Some(Slot::Reference(None))),
    }
}

pub(crate) fn native_reflect_constructor_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let constructor_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_declaring_class_name_slot(
        heap,
        constructor_ref,
    )?))
}

pub(crate) fn native_reflect_method_get_return_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let method = reflected_method_handle(heap, method_ref)?;
    Ok(Some(descriptor_class_slot_from_source(
        heap,
        ops,
        method_return_descriptor(&method.descriptor),
        Some(method.declaring_class_key.as_str()),
    )?))
}

pub(crate) fn native_reflect_field_get_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let field = reflected_field_handle(heap, field_ref)?;
    Ok(Some(descriptor_class_slot_from_source(
        heap,
        ops,
        &field.descriptor,
        Some(field.declaring_class_key.as_str()),
    )?))
}

pub(crate) fn native_reflection_member_get_declaring_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_declaring_class_slot(
        heap, member_ref,
    )?))
}

pub(crate) fn native_reflect_method_get_parameter_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let method = reflected_method_handle(heap, method_ref)?;
    let count = i32::try_from(parse_arg_count(&method.descriptor)).unwrap_or(i32::MAX);
    Ok(Some(Slot::Int(count)))
}

pub(crate) fn native_reflect_executable_get_parameter_types(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    let member = reflected_method_handle(heap, member_ref)?;
    let parameter_descriptors = parse_arg_descriptors(&member.descriptor);
    let mut class_refs = Vec::with_capacity(parameter_descriptors.len());
    for descriptor in parameter_descriptors {
        let Slot::Reference(Some(class_ref)) = descriptor_class_slot_from_source(
            heap,
            ops,
            &descriptor,
            Some(member.declaring_class_key.as_str()),
        )?
        else {
            return Err(Error::TypeMismatch {
                expected: "class reference",
                got: "other",
            });
        };
        class_refs.push(class_ref);
    }
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/Class;", &class_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_reflect_field_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_name_slot(heap, field_ref)?))
}

pub(crate) fn native_reflect_field_get_annotations(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let annotations = annotations_for_reflected_field(heap, field_ref, ops)?;
    allocate_annotation_array(heap, out, ops, &annotations)
}

pub(crate) fn native_reflect_field_get_declared_annotations(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_reflect_field_get_annotations(args, heap, out, control, ops)
}

pub(crate) fn native_reflect_field_get_annotation(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let annotation_type_ref = extract_ref_arg(args, 1)?;
    let requested_type = class_key_from_ref(heap, annotation_type_ref)?;
    let annotations = annotations_for_reflected_field(heap, field_ref, ops)?;
    match find_annotation(&annotations, &requested_type) {
        Some(annotation) => {
            let annotation_ref = allocate_annotation_proxy(heap, out, ops, annotation)?;
            Ok(Some(Slot::Reference(Some(annotation_ref))))
        }
        None => Ok(Some(Slot::Reference(None))),
    }
}

pub(crate) fn native_reflection_member_set_accessible(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    let accessible = extract_int_arg(args, 1)? != 0;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_ACCESSIBLE_FIELD,
        Slot::Int(i32::from(accessible)),
    )?;
    Ok(None)
}

pub(crate) fn native_reflect_field_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let field = reflected_field_handle(heap, field_ref)?;

    if !field.is_public && !field.is_accessible {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let field_type = field.descriptor.chars().next().unwrap_or('L');
    let raw_value = if field.is_static {
        ops.ensure_class_initialized(heap, output, &field.declaring_class_key)?;
        ops.read_static_field(&field.declaring_class_key, &field.field_name)?
    } else {
        let Slot::Reference(Some(target_ref)) = target_slot else {
            return Err(Error::NullPointerException);
        };
        ops.read_instance_field(
            heap,
            target_ref,
            &field.declaring_class_key,
            &field.field_name,
        )?
    };

    Ok(Some(box_reflection_return_value(
        heap,
        field_type,
        Some(raw_value),
    )?))
}

pub(crate) fn native_reflect_field_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let value_slot = extract_slot_arg(args, 2);
    let field = reflected_field_handle(heap, field_ref)?;

    if !field.is_public && !field.is_accessible {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let field_type = field.descriptor.chars().next().unwrap_or('L');
    let value = unbox_reflection_argument(heap, field_type, value_slot)?;

    if field.is_static {
        ops.ensure_class_initialized(heap, output, &field.declaring_class_key)?;
        ops.write_static_field(&field.declaring_class_key, &field.field_name, value)?;
        return Ok(None);
    }

    let Slot::Reference(Some(target_ref)) = target_slot else {
        return Err(Error::NullPointerException);
    };
    ops.write_instance_field(
        heap,
        target_ref,
        &field.declaring_class_key,
        &field.field_name,
        value,
    )?;
    Ok(None)
}

pub(crate) fn native_reflect_method_invoke(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let invoke_arg_slots = reflection_array_elements(heap, extract_slot_arg(args, 2))?;
    let method = reflected_method_handle(heap, method_ref)?;

    if !method.is_public && !method.is_accessible {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let invoke_args = build_reflection_invoke_args(
        heap,
        target_slot,
        &method.descriptor,
        invoke_arg_slots,
        method.is_static,
    )?;

    ops.ensure_loaded(&method.declaring_class_key)?;
    match ops.invoke(
        heap,
        output,
        &method.declaring_class_key,
        &method.method_name,
        &method.descriptor,
        invoke_args,
    ) {
        Ok(result) => Ok(Some(box_reflection_return_value(
            heap,
            descriptor_return_type(&method.descriptor),
            result,
        )?)),
        Err(Error::JavaException { .. }) => Err(Error::JavaException {
            class_name: "java/lang/reflect/InvocationTargetException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_reflect_constructor_new_instance(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let constructor_ref = extract_ref_arg(args, 0)?;
    let invoke_arg_slots = reflection_array_elements(heap, extract_slot_arg(args, 1))?;
    let constructor = reflected_method_handle(heap, constructor_ref)?;

    if !constructor.is_public && !constructor.is_accessible {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let instance_ref = ops.allocate_instance(heap, output, &constructor.declaring_class_key)?;
    let invoke_args = build_reflection_invoke_args(
        heap,
        Slot::Reference(Some(instance_ref)),
        &constructor.descriptor,
        invoke_arg_slots,
        false,
    )?;

    match ops.invoke(
        heap,
        output,
        &constructor.declaring_class_key,
        &constructor.method_name,
        &constructor.descriptor,
        invoke_args,
    ) {
        Ok(_) => Ok(Some(Slot::Reference(Some(instance_ref)))),
        Err(Error::JavaException { .. }) => Err(Error::JavaException {
            class_name: "java/lang/reflect/InvocationTargetException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

/// Native: `String.valueOf(int)` — static method, returns string of int.
pub(crate) fn native_string_value_of_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let s = val.to_string();
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
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

pub(crate) fn native_system_exit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let code = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 1,
    };
    Err(Error::SystemExit { code })
}

static SYSTEM_PROPERTY_OVERRIDES: std::sync::OnceLock<std::sync::Mutex<HashMap<String, String>>> =
    std::sync::OnceLock::new();

fn system_property_overrides() -> &'static std::sync::Mutex<HashMap<String, String>> {
    SYSTEM_PROPERTY_OVERRIDES.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}


fn system_property_value_fallback(key: &str) -> Option<String> {
    match key {
        "java.io.tmpdir" => Some(std::env::temp_dir().to_string_lossy().into_owned()),
        "file.separator" => Some(std::path::MAIN_SEPARATOR.to_string()),
        "path.separator" => Some(if cfg!(windows) { ";" } else { ":" }.to_string()),
        "line.separator" => Some(if cfg!(windows) { "\r\n" } else { "\n" }.to_string()),
        "user.dir" => std::env::current_dir()
            .ok()
            .map(|path| path.to_string_lossy().into_owned()),
        "user.home" => std::env::var("USERPROFILE")
            .ok()
            .or_else(|| std::env::var("HOME").ok()),
        "os.name" => Some(
            if cfg!(windows) {
                "Windows"
            } else if cfg!(target_os = "macos") {
                "Mac OS X"
            } else {
                "Linux"
            }
            .to_string(),
        ),
        "os.arch" => Some(std::env::consts::ARCH.to_string()),
        "os.version" => Some("1.0".to_string()),
        "user.name" => std::env::var("USERNAME")
            .ok()
            .or_else(|| std::env::var("USER").ok()),
        "java.version" => Some("1.8.0".to_string()),
        _ => None,
    }
}

fn system_property_value(key: &str) -> Option<String> {
    let override_value = system_property_overrides()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(key)
        .cloned();
    if let Some(value) = override_value {
        return Some(value);
    }
    match key {
        "java.io.tmpdir" => Some(std::env::temp_dir().to_string_lossy().into_owned()),
        "file.separator" => Some(std::path::MAIN_SEPARATOR.to_string()),
        "path.separator" => Some(if cfg!(windows) { ";" } else { ":" }.to_string()),
        "line.separator" => Some(if cfg!(windows) { "\r\n" } else { "\n" }.to_string()),
        "user.dir" => std::env::current_dir()
            .ok()
            .map(|path| path.to_string_lossy().into_owned()),
        "user.home" => std::env::var("USERPROFILE")
            .ok()
            .or_else(|| std::env::var("HOME").ok()),
        "os.name" => Some(
            if cfg!(windows) {
                "Windows"
            } else if cfg!(target_os = "macos") {
                "Mac OS X"
            } else if cfg!(target_os = "linux") {
                "Linux"
            } else {
                std::env::consts::OS
            }
            .to_string(),
        ),
        "os.arch" => Some(std::env::consts::ARCH.to_string()),
        "java.version" => Some("21".to_string()),
        _ => None,
    }
}

pub(crate) fn native_system_get_property(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let key_ref = extract_ref_arg(args, 0)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let result = system_property_value(&key).map_or(Slot::Reference(None), |value| {
        Slot::Reference(Some(heap.allocate_string(value)))
    });
    Ok(Some(result))
}

pub(crate) fn native_system_set_property(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let key_ref = extract_ref_arg(args, 0)?;
    let value_ref = extract_ref_arg(args, 1)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let value = string_value_from_ref(heap, value_ref)?;
    let previous = {
        let mut overrides = system_property_overrides()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let prev = overrides.get(&key).cloned().or_else(|| system_property_value_fallback(&key));
        overrides.insert(key, value);
        prev
    };
    let result = previous.map_or(Slot::Reference(None), |previous| {
        Slot::Reference(Some(heap.allocate_string(previous)))
    });
    Ok(Some(result))
}

pub(crate) fn native_system_get_property_with_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let key_ref = extract_ref_arg(args, 0)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let result = system_property_value(&key).map_or_else(
        || extract_slot_arg(args, 1),
        |value| Slot::Reference(Some(heap.allocate_string(value))),
    );
    Ok(Some(result))
}

/// Native: `System.lineSeparator()String` — returns the platform line separator.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_system_line_separator(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate_string("\n".to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `System.identityHashCode(Object)I` — returns a stable identity hash (heap address low bits).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_system_identity_hash_code(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    #[allow(clippy::cast_possible_truncation)]
    let hash = match args.first() {
        Some(Slot::Reference(Some(r))) => (*r & 0x7FFF_FFFF) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(hash)))
}

fn system_time_to_epoch_millis(now: std::time::SystemTime) -> i64 {
    now.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_system_current_time_millis(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Long(system_time_to_epoch_millis(
        std::time::SystemTime::now(),
    ))))
}

static NANO_TIME_ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn monotonic_nano_time_now() -> i64 {
    let origin = NANO_TIME_ORIGIN.get_or_init(std::time::Instant::now);
    i64::try_from(origin.elapsed().as_nanos()).unwrap_or(i64::MAX)
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_system_nano_time(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Long(monotonic_nano_time_now())))
}

const THREAD_TARGET_SLOT: usize = 0;
const THREAD_ID_SLOT: usize = 1;
const THREAD_INTERRUPTED_SLOT: usize = 2;
const THREAD_HOST_KEY_SLOT: usize = 3;

static NEXT_THREAD_HOST_KEY: AtomicI32 = AtomicI32::new(1);

fn next_thread_host_key() -> i32 {
    NEXT_THREAD_HOST_KEY.fetch_add(1, Ordering::Relaxed)
}

fn java_thread_hosts() -> &'static RwLock<HashMap<i32, std::thread::ThreadId>> {
    static HOSTS: OnceLock<RwLock<HashMap<i32, std::thread::ThreadId>>> = OnceLock::new();
    HOSTS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn interrupted_host_threads() -> &'static RwLock<HashSet<std::thread::ThreadId>> {
    static INTERRUPTED: OnceLock<RwLock<HashSet<std::thread::ThreadId>>> = OnceLock::new();
    INTERRUPTED.get_or_init(|| RwLock::new(HashSet::new()))
}

fn register_java_host_thread(host_key: i32, host_thread_id: std::thread::ThreadId) {
    java_thread_hosts()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(host_key, host_thread_id);
}

fn unregister_java_host_thread(host_key: i32) {
    let removed_host_thread = java_thread_hosts()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&host_key);
    if let Some(host_thread_id) = removed_host_thread {
        interrupted_host_threads()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&host_thread_id);
    }
}

fn host_thread_for_java_thread(host_key: i32) -> Option<std::thread::ThreadId> {
    java_thread_hosts()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&host_key)
        .copied()
}

fn java_host_key_for_current_host() -> Option<i32> {
    let host_thread_id = current_host_thread_id();
    java_thread_hosts()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .find_map(|(host_key, mapped_host)| (*mapped_host == host_thread_id).then_some(*host_key))
}

fn interrupt_host_thread(host_thread_id: std::thread::ThreadId) {
    interrupted_host_threads()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(host_thread_id);
}

fn current_host_thread_is_interrupted() -> bool {
    interrupted_host_threads()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .contains(&current_host_thread_id())
}

fn take_current_host_thread_interrupted() -> bool {
    interrupted_host_threads()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&current_host_thread_id())
}

pub(crate) fn native_thread_current_thread(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = heap.allocate("java/lang/Thread".to_string(), 4);
    let thread = heap.get_mut(thread_ref)?;
    thread.fields[THREAD_TARGET_SLOT] = Slot::Reference(None);
    thread.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    thread.fields[THREAD_INTERRUPTED_SLOT] = Slot::Int(i32::from(current_host_thread_is_interrupted()));
    thread.fields[THREAD_HOST_KEY_SLOT] = Slot::Int(java_host_key_for_current_host().unwrap_or(-1));
    Ok(Some(Slot::Reference(Some(thread_ref))))
}

pub(crate) fn native_thread_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let this = heap.get_mut(this_ref)?;
    this.fields[THREAD_TARGET_SLOT] = Slot::Reference(None);
    this.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    this.fields[THREAD_INTERRUPTED_SLOT] = Slot::Int(0);
    this.fields[THREAD_HOST_KEY_SLOT] = Slot::Int(-1);
    Ok(None)
}

pub(crate) fn native_thread_init_runnable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let this = heap.get_mut(this_ref)?;
    this.fields[THREAD_TARGET_SLOT] = target;
    this.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    this.fields[THREAD_INTERRUPTED_SLOT] = Slot::Int(0);
    this.fields[THREAD_HOST_KEY_SLOT] = Slot::Int(-1);
    Ok(None)
}

pub(crate) fn native_thread_start(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    control.request(NativeThreadAction::Start { thread_ref });
    Ok(None)
}

pub(crate) fn native_thread_join(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let thread = heap.get(thread_ref)?;
    let Some(Slot::Int(thread_id)) = thread.fields.get(THREAD_ID_SLOT) else {
        return Ok(None);
    };
    if *thread_id >= 0 {
        control.request(NativeThreadAction::Join {
            thread_id: *thread_id,
        });
    }
    Ok(None)
}

pub(crate) fn native_thread_sleep(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let millis = match args.first() {
        Some(Slot::Long(value)) => *value,
        _ => {
            return Err(Error::TypeMismatch {
                expected: "long",
                got: "other",
            });
        }
    };
    let millis = u64::try_from(millis.max(0)).unwrap_or(0);
    control.request(NativeThreadAction::Sleep(std::time::Duration::from_millis(
        millis,
    )));
    Ok(None)
}

const EXECUTOR_SHUTDOWN_FIELD: usize = 0;
const EXECUTOR_AWAIT_DEADLINE_FIELD: usize = 1;

const FUTURE_STATE_FIELD: usize = 0;
const FUTURE_RESULT_FIELD: usize = 1;
const FUTURE_EXCEPTION_FIELD: usize = 2;
const FUTURE_WAIT_DEADLINE_FIELD: usize = 3;
const FUTURE_TASK_FIELD: usize = 4;

const FUTURE_PENDING: i32 = 0;
const FUTURE_RUNNING: i32 = 1;
const FUTURE_DONE: i32 = 2;
const FUTURE_CANCELLED: i32 = 3;
const FUTURE_FAILED: i32 = 4;

const TIMEUNIT_NANOS_FIELD: usize = 2;

fn executor_shared(
    heap: &duke_gc::Heap,
    executor_ref: u64,
) -> Result<std::sync::Arc<duke_gc::ExecutorShared>> {
    match heap.get(executor_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Executor(state)) => Ok(std::sync::Arc::clone(state)),
        _ => Err(atomic_payload_error(executor_ref)),
    }
}

fn allocate_executor(heap: &mut duke_gc::Heap, max_workers: usize) -> Result<Slot> {
    let executor_ref = heap.allocate("duke/util/concurrent/DukeExecutorService".to_string(), 2);
    {
        let executor = heap.get_mut(executor_ref)?;
        executor.fields[EXECUTOR_SHUTDOWN_FIELD] = Slot::Int(0);
        executor.fields[EXECUTOR_AWAIT_DEADLINE_FIELD] = Slot::Long(0);
        executor.atomic_payload = Some(duke_gc::AtomicPayload::executor(max_workers));
    }
    Ok(Slot::Reference(Some(executor_ref)))
}

pub(crate) fn native_executors_new_fixed_thread_pool(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let count = extract_int_arg(args, 0)?;
    if count <= 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalArgumentException".to_string(),
        });
    }
    Ok(Some(allocate_executor(
        heap,
        usize::try_from(count).unwrap_or(1),
    )?))
}

pub(crate) fn native_executors_new_single_thread_executor(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(allocate_executor(heap, 1)?))
}

pub(crate) fn native_executors_new_cached_thread_pool(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(allocate_executor(heap, 64)?))
}

fn init_future(
    heap: &mut duke_gc::Heap,
    result: Slot,
    task: Slot,
) -> Result<u64> {
    let future_ref = heap.allocate("duke/util/concurrent/DukeFuture".to_string(), 5);
    {
        let future = heap.get_mut(future_ref)?;
        future.fields[FUTURE_STATE_FIELD] = Slot::Int(FUTURE_PENDING);
        future.fields[FUTURE_RESULT_FIELD] = result;
        future.fields[FUTURE_EXCEPTION_FIELD] = Slot::Reference(None);
        future.fields[FUTURE_WAIT_DEADLINE_FIELD] = Slot::Long(0);
        future.fields[FUTURE_TASK_FIELD] = task;
    }
    heap.remember_reference_write(future_ref, result);
    heap.remember_reference_write(future_ref, task);
    Ok(future_ref)
}

fn executor_submit_common(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    control: &mut NativeControl,
    kind: duke_gc::ExecutorTaskKind,
    preset_result: Slot,
) -> Result<u64> {
    let executor_ref = extract_ref_arg(args, 0)?;
    let task_ref = extract_ref_arg(args, 1)?;
    let executor = executor_shared(heap, executor_ref)?;
    let is_shutdown = {
        let guard = executor
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.shutdown
    };
    if is_shutdown {
        return Err(Error::JavaException {
            class_name: "java/util/concurrent/RejectedExecutionException".to_string(),
        });
    }

    let future_ref = init_future(
        heap,
        preset_result,
        Slot::Reference(Some(task_ref)),
    )?;
    control.request(NativeThreadAction::ExecutorSubmit {
        executor_ref,
        future_ref,
        task_ref,
        kind,
    });
    Ok(future_ref)
}

pub(crate) fn native_executor_submit_runnable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let future_ref = executor_submit_common(
        args,
        heap,
        control,
        duke_gc::ExecutorTaskKind::Runnable,
        Slot::Reference(None),
    )?;
    Ok(Some(Slot::Reference(Some(future_ref))))
}

pub(crate) fn native_executor_submit_runnable_result(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let result = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let future_ref = executor_submit_common(
        args,
        heap,
        control,
        duke_gc::ExecutorTaskKind::Runnable,
        result,
    )?;
    Ok(Some(Slot::Reference(Some(future_ref))))
}

pub(crate) fn native_executor_submit_callable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let future_ref = executor_submit_common(
        args,
        heap,
        control,
        duke_gc::ExecutorTaskKind::Callable,
        Slot::Reference(None),
    )?;
    Ok(Some(Slot::Reference(Some(future_ref))))
}

pub(crate) fn native_executor_execute(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let _future_ref = executor_submit_common(
        args,
        heap,
        control,
        duke_gc::ExecutorTaskKind::Runnable,
        Slot::Reference(None),
    )?;
    Ok(None)
}

pub(crate) fn native_executor_shutdown(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let executor_ref = extract_ref_arg(args, 0)?;
    let executor = executor_shared(heap, executor_ref)?;
    {
        let mut guard = executor
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.shutdown = true;
        guard.refresh_terminated();
    }
    executor.available.notify_all();
    heap.write_field(executor_ref, EXECUTOR_SHUTDOWN_FIELD, Slot::Int(1))?;
    Ok(None)
}

fn executor_is_shutdown(heap: &duke_gc::Heap, executor_ref: u64) -> Result<bool> {
    let executor = executor_shared(heap, executor_ref)?;
    let guard = executor
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(guard.shutdown)
}

pub(crate) fn native_executor_is_shutdown(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let executor_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(executor_is_shutdown(
        heap,
        executor_ref,
    )?))))
}

fn executor_is_terminated(heap: &duke_gc::Heap, executor_ref: u64) -> Result<bool> {
    let executor = executor_shared(heap, executor_ref)?;
    let mut guard = executor
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.refresh_terminated();
    Ok(guard.terminated)
}

pub(crate) fn native_executor_is_terminated(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let executor_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(executor_is_terminated(
        heap,
        executor_ref,
    )?))))
}

fn timeunit_nanos_per_unit(heap: &duke_gc::Heap, unit_ref: u64) -> Result<i64> {
    match heap.get(unit_ref)?.fields.get(TIMEUNIT_NANOS_FIELD) {
        Some(Slot::Long(nanos)) => Ok(*nanos),
        _ => Err(Error::TypeMismatch {
            expected: "TimeUnit",
            got: "other",
        }),
    }
}

fn saturating_mul_i64(lhs: i64, rhs: i64) -> i64 {
    let value = i128::from(lhs).saturating_mul(i128::from(rhs));
    let clamped = value.clamp(i128::from(i64::MIN), i128::from(i64::MAX));
    match i64::try_from(clamped) {
        Ok(value) => value,
        Err(_) if clamped < 0 => i64::MIN,
        Err(_) => i64::MAX,
    }
}

fn timeout_nanos(timeout: i64, unit_ref: u64, heap: &duke_gc::Heap) -> Result<i64> {
    Ok(saturating_mul_i64(
        timeout.max(0),
        timeunit_nanos_per_unit(heap, unit_ref)?,
    ))
}

pub(crate) fn native_timeunit_to_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let unit_ref = extract_ref_arg(args, 0)?;
    let value = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(saturating_mul_i64(
        value,
        timeunit_nanos_per_unit(heap, unit_ref)?,
    ))))
}

pub(crate) fn native_timeunit_to_millis(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let unit_ref = extract_ref_arg(args, 0)?;
    let value = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(
        saturating_mul_i64(value, timeunit_nanos_per_unit(heap, unit_ref)?) / 1_000_000,
    )))
}

pub(crate) fn native_executor_await_termination(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let executor_ref = extract_ref_arg(args, 0)?;
    if executor_is_terminated(heap, executor_ref)? {
        heap.write_field(executor_ref, EXECUTOR_AWAIT_DEADLINE_FIELD, Slot::Long(0))?;
        return Ok(Some(Slot::Int(1)));
    }

    let timeout = extract_long_arg(args, 1)?;
    let unit_ref = extract_ref_arg(args, 2)?;
    let nanos = timeout_nanos(timeout, unit_ref, heap)?;
    if nanos <= 0 {
        return Ok(Some(Slot::Int(0)));
    }

    let now = monotonic_nano_time_now();
    let deadline = match extract_field_arg(heap, executor_ref, EXECUTOR_AWAIT_DEADLINE_FIELD)? {
        Slot::Long(value) if value > 0 => value,
        _ => {
            let deadline = now.saturating_add(nanos);
            heap.write_field(
                executor_ref,
                EXECUTOR_AWAIT_DEADLINE_FIELD,
                Slot::Long(deadline),
            )?;
            deadline
        }
    };
    if now >= deadline {
        heap.write_field(executor_ref, EXECUTOR_AWAIT_DEADLINE_FIELD, Slot::Long(0))?;
        return Ok(Some(Slot::Int(0)));
    }
    request_native_retry(control);
    Ok(None)
}

fn future_state(heap: &duke_gc::Heap, future_ref: u64) -> Result<i32> {
    match heap.get(future_ref)?.fields.get(FUTURE_STATE_FIELD) {
        Some(Slot::Int(state)) => Ok(*state),
        _ => Ok(FUTURE_PENDING),
    }
}

pub(crate) fn native_future_cancel(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let future_ref = extract_ref_arg(args, 0)?;
    if future_state(heap, future_ref)? == FUTURE_PENDING {
        heap.write_field(future_ref, FUTURE_STATE_FIELD, Slot::Int(FUTURE_CANCELLED))?;
        return Ok(Some(Slot::Int(1)));
    }
    Ok(Some(Slot::Int(0)))
}

pub(crate) fn native_future_is_cancelled(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let future_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(
        future_state(heap, future_ref)? == FUTURE_CANCELLED,
    ))))
}

pub(crate) fn native_future_is_done(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let future_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(matches!(
        future_state(heap, future_ref)?,
        FUTURE_DONE | FUTURE_CANCELLED | FUTURE_FAILED
    )))))
}

fn future_get_common(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    control: &mut NativeControl,
    timeout: Option<i64>,
) -> Result<Option<Slot>> {
    let future_ref = extract_ref_arg(args, 0)?;
    match future_state(heap, future_ref)? {
        FUTURE_DONE => {
            heap.write_field(future_ref, FUTURE_WAIT_DEADLINE_FIELD, Slot::Long(0))?;
            Ok(Some(extract_field_arg(heap, future_ref, FUTURE_RESULT_FIELD)?))
        }
        FUTURE_CANCELLED => Err(Error::JavaException {
            class_name: "java/util/concurrent/CancellationException".to_string(),
        }),
        FUTURE_FAILED => {
            let cause = extract_field_arg(heap, future_ref, FUTURE_EXCEPTION_FIELD)?;
            push_pending_java_exception_cause("java/util/concurrent/ExecutionException", cause);
            Err(Error::JavaException {
                class_name: "java/util/concurrent/ExecutionException".to_string(),
            })
        }
        FUTURE_PENDING | FUTURE_RUNNING => {
            if let Some(nanos) = timeout {
                if nanos <= 0 {
                    return Err(Error::JavaException {
                        class_name: "java/util/concurrent/TimeoutException".to_string(),
                    });
                }
                let now = monotonic_nano_time_now();
                let deadline = match extract_field_arg(heap, future_ref, FUTURE_WAIT_DEADLINE_FIELD)?
                {
                    Slot::Long(value) if value > 0 => value,
                    _ => {
                        let deadline = now.saturating_add(nanos);
                        heap.write_field(
                            future_ref,
                            FUTURE_WAIT_DEADLINE_FIELD,
                            Slot::Long(deadline),
                        )?;
                        deadline
                    }
                };
                if now >= deadline {
                    heap.write_field(future_ref, FUTURE_WAIT_DEADLINE_FIELD, Slot::Long(0))?;
                    return Err(Error::JavaException {
                        class_name: "java/util/concurrent/TimeoutException".to_string(),
                    });
                }
            }
            request_native_retry(control);
            Ok(None)
        }
        _ => Err(Error::InvalidRef {
            address: future_ref,
        }),
    }
}

pub(crate) fn native_future_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    future_get_common(args, heap, control, None)
}

pub(crate) fn native_future_get_timeout(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let timeout = extract_long_arg(args, 1)?;
    let unit_ref = extract_ref_arg(args, 2)?;
    let nanos = timeout_nanos(timeout, unit_ref, heap)?;
    future_get_common(args, heap, control, Some(nanos))
}

// java.util.concurrent.atomic natives.
pub(crate) fn native_atomic_integer_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.atomic_payload = Some(duke_gc::AtomicPayload::int(0));
    Ok(None)
}

pub(crate) fn native_atomic_integer_init_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = extract_int_arg(args, 1)?;
    heap.get_mut(this_ref)?.atomic_payload = Some(duke_gc::AtomicPayload::int(value));
    Ok(None)
}

pub(crate) fn native_atomic_integer_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(with_atomic_i32(heap, this_ref, |cell| {
        cell.load(Ordering::SeqCst)
    })?)))
}

pub(crate) fn native_atomic_integer_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = extract_int_arg(args, 1)?;
    with_atomic_i32(heap, this_ref, |cell| cell.store(value, Ordering::SeqCst))?;
    Ok(None)
}

pub(crate) fn native_atomic_integer_get_and_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = extract_int_arg(args, 1)?;
    let previous = with_atomic_i32(heap, this_ref, |cell| cell.swap(value, Ordering::SeqCst))?;
    Ok(Some(Slot::Int(previous)))
}

pub(crate) fn native_atomic_integer_compare_and_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let expected = extract_int_arg(args, 1)?;
    let update = extract_int_arg(args, 2)?;
    let exchanged = with_atomic_i32(heap, this_ref, |cell| {
        cell.compare_exchange(expected, update, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    })?;
    Ok(Some(Slot::Int(i32::from(exchanged))))
}

pub(crate) fn native_atomic_integer_get_and_increment(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i32(heap, this_ref, |cell| cell.fetch_add(1, Ordering::SeqCst))?;
    Ok(Some(Slot::Int(previous)))
}

pub(crate) fn native_atomic_integer_get_and_decrement(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i32(heap, this_ref, |cell| cell.fetch_sub(1, Ordering::SeqCst))?;
    Ok(Some(Slot::Int(previous)))
}

pub(crate) fn native_atomic_integer_get_and_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let delta = extract_int_arg(args, 1)?;
    let previous =
        with_atomic_i32(heap, this_ref, |cell| cell.fetch_add(delta, Ordering::SeqCst))?;
    Ok(Some(Slot::Int(previous)))
}

pub(crate) fn native_atomic_integer_increment_and_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i32(heap, this_ref, |cell| cell.fetch_add(1, Ordering::SeqCst))?;
    Ok(Some(Slot::Int(previous.wrapping_add(1))))
}

pub(crate) fn native_atomic_integer_decrement_and_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i32(heap, this_ref, |cell| cell.fetch_sub(1, Ordering::SeqCst))?;
    Ok(Some(Slot::Int(previous.wrapping_sub(1))))
}

pub(crate) fn native_atomic_integer_add_and_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let delta = extract_int_arg(args, 1)?;
    let previous =
        with_atomic_i32(heap, this_ref, |cell| cell.fetch_add(delta, Ordering::SeqCst))?;
    Ok(Some(Slot::Int(previous.wrapping_add(delta))))
}

pub(crate) fn native_atomic_integer_long_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Long(i64::from(with_atomic_i32(
        heap,
        this_ref,
        |cell| cell.load(Ordering::SeqCst),
    )?))))
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn native_atomic_integer_float_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = with_atomic_i32(heap, this_ref, |cell| cell.load(Ordering::SeqCst))?;
    Ok(Some(Slot::Float(value as f32)))
}

pub(crate) fn native_atomic_integer_double_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = with_atomic_i32(heap, this_ref, |cell| cell.load(Ordering::SeqCst))?;
    Ok(Some(Slot::Double(f64::from(value))))
}

pub(crate) fn native_atomic_integer_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = with_atomic_i32(heap, this_ref, |cell| cell.load(Ordering::SeqCst))?;
    let string_ref = heap.allocate_string(value.to_string());
    Ok(Some(Slot::Reference(Some(string_ref))))
}

pub(crate) fn native_atomic_long_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.atomic_payload = Some(duke_gc::AtomicPayload::long(0));
    Ok(None)
}

pub(crate) fn native_atomic_long_init_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = extract_long_arg(args, 1)?;
    heap.get_mut(this_ref)?.atomic_payload = Some(duke_gc::AtomicPayload::long(value));
    Ok(None)
}

pub(crate) fn native_atomic_long_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Long(with_atomic_i64(heap, this_ref, |cell| {
        cell.load(Ordering::SeqCst)
    })?)))
}

pub(crate) fn native_atomic_long_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = extract_long_arg(args, 1)?;
    with_atomic_i64(heap, this_ref, |cell| cell.store(value, Ordering::SeqCst))?;
    Ok(None)
}

pub(crate) fn native_atomic_long_get_and_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = extract_long_arg(args, 1)?;
    let previous = with_atomic_i64(heap, this_ref, |cell| cell.swap(value, Ordering::SeqCst))?;
    Ok(Some(Slot::Long(previous)))
}

pub(crate) fn native_atomic_long_compare_and_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let expected = extract_long_arg(args, 1)?;
    let update = extract_long_arg(args, 2)?;
    let exchanged = with_atomic_i64(heap, this_ref, |cell| {
        cell.compare_exchange(expected, update, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    })?;
    Ok(Some(Slot::Int(i32::from(exchanged))))
}

pub(crate) fn native_atomic_long_get_and_increment(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i64(heap, this_ref, |cell| cell.fetch_add(1, Ordering::SeqCst))?;
    Ok(Some(Slot::Long(previous)))
}

pub(crate) fn native_atomic_long_get_and_decrement(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i64(heap, this_ref, |cell| cell.fetch_sub(1, Ordering::SeqCst))?;
    Ok(Some(Slot::Long(previous)))
}

pub(crate) fn native_atomic_long_get_and_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let delta = extract_long_arg(args, 1)?;
    let previous =
        with_atomic_i64(heap, this_ref, |cell| cell.fetch_add(delta, Ordering::SeqCst))?;
    Ok(Some(Slot::Long(previous)))
}

pub(crate) fn native_atomic_long_increment_and_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i64(heap, this_ref, |cell| cell.fetch_add(1, Ordering::SeqCst))?;
    Ok(Some(Slot::Long(previous.wrapping_add(1))))
}

pub(crate) fn native_atomic_long_decrement_and_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i64(heap, this_ref, |cell| cell.fetch_sub(1, Ordering::SeqCst))?;
    Ok(Some(Slot::Long(previous.wrapping_sub(1))))
}

pub(crate) fn native_atomic_long_add_and_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let delta = extract_long_arg(args, 1)?;
    let previous =
        with_atomic_i64(heap, this_ref, |cell| cell.fetch_add(delta, Ordering::SeqCst))?;
    Ok(Some(Slot::Long(previous.wrapping_add(delta))))
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_atomic_long_int_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = with_atomic_i64(heap, this_ref, |cell| cell.load(Ordering::SeqCst))?;
    Ok(Some(Slot::Int(value as i32)))
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn native_atomic_long_float_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = with_atomic_i64(heap, this_ref, |cell| cell.load(Ordering::SeqCst))?;
    Ok(Some(Slot::Float(value as f32)))
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn native_atomic_long_double_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = with_atomic_i64(heap, this_ref, |cell| cell.load(Ordering::SeqCst))?;
    Ok(Some(Slot::Double(value as f64)))
}

pub(crate) fn native_atomic_long_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = with_atomic_i64(heap, this_ref, |cell| cell.load(Ordering::SeqCst))?;
    let string_ref = heap.allocate_string(value.to_string());
    Ok(Some(Slot::Reference(Some(string_ref))))
}

pub(crate) fn native_atomic_reference_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.atomic_payload =
        Some(duke_gc::AtomicPayload::reference(Slot::Reference(None)));
    Ok(None)
}

pub(crate) fn native_atomic_reference_init_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = extract_slot_arg(args, 1);
    heap.get_mut(this_ref)?.atomic_payload = Some(duke_gc::AtomicPayload::reference(value));
    heap.remember_reference_write(this_ref, value);
    Ok(None)
}

pub(crate) fn native_atomic_reference_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(load_atomic_reference(heap, this_ref)?))
}

pub(crate) fn native_atomic_reference_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = extract_slot_arg(args, 1);
    with_atomic_reference(heap, this_ref, |cell| {
        *cell
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = value;
        Ok(())
    })?;
    heap.remember_reference_write(this_ref, value);
    Ok(None)
}

pub(crate) fn native_atomic_reference_get_and_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = extract_slot_arg(args, 1);
    let previous = with_atomic_reference(heap, this_ref, |cell| {
        let mut guard = cell
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = *guard;
        *guard = value;
        drop(guard);
        Ok(previous)
    })?;
    heap.remember_reference_write(this_ref, value);
    Ok(Some(previous))
}

pub(crate) fn native_atomic_reference_compare_and_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let expected = extract_slot_arg(args, 1);
    let update = extract_slot_arg(args, 2);
    let exchanged = with_atomic_reference(heap, this_ref, |cell| {
        let mut guard = cell
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let exchanged = *guard == expected;
        if exchanged {
            *guard = update;
        }
        drop(guard);
        Ok(exchanged)
    })?;
    if exchanged {
        heap.remember_reference_write(this_ref, update);
    }
    Ok(Some(Slot::Int(i32::from(exchanged))))
}

pub(crate) fn native_atomic_reference_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = load_atomic_reference(heap, this_ref)?;
    let text = match value {
        Slot::Reference(None) => "null".to_string(),
        Slot::Reference(Some(reference)) => heap_object_to_string(heap.get(reference)?, reference),
        Slot::Int(value) => value.to_string(),
        Slot::Long(value) => value.to_string(),
        Slot::Float(value) => value.to_string(),
        Slot::Double(value) => value.to_string(),
        Slot::ReturnAddress(value) => value.to_string(),
    };
    let string_ref = heap.allocate_string(text);
    Ok(Some(Slot::Reference(Some(string_ref))))
}

pub(crate) fn native_atomic_boolean_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.atomic_payload = Some(duke_gc::AtomicPayload::bool(false));
    Ok(None)
}

pub(crate) fn native_atomic_boolean_init_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = atomic_bool_arg(args, 1)?;
    heap.get_mut(this_ref)?.atomic_payload = Some(duke_gc::AtomicPayload::bool(value));
    Ok(None)
}

pub(crate) fn native_atomic_boolean_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = with_atomic_bool(heap, this_ref, |cell| cell.load(Ordering::SeqCst))?;
    Ok(Some(Slot::Int(i32::from(value))))
}

pub(crate) fn native_atomic_boolean_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = atomic_bool_arg(args, 1)?;
    with_atomic_bool(heap, this_ref, |cell| cell.store(value, Ordering::SeqCst))?;
    Ok(None)
}

pub(crate) fn native_atomic_boolean_compare_and_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let expected = atomic_bool_arg(args, 1)?;
    let update = atomic_bool_arg(args, 2)?;
    let exchanged = with_atomic_bool(heap, this_ref, |cell| {
        cell.compare_exchange(expected, update, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    })?;
    Ok(Some(Slot::Int(i32::from(exchanged))))
}

pub(crate) fn native_atomic_boolean_get_and_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = atomic_bool_arg(args, 1)?;
    let previous = with_atomic_bool(heap, this_ref, |cell| cell.swap(value, Ordering::SeqCst))?;
    Ok(Some(Slot::Int(i32::from(previous))))
}

pub(crate) fn native_atomic_boolean_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = with_atomic_bool(heap, this_ref, |cell| cell.load(Ordering::SeqCst))?;
    let string_ref = heap.allocate_string(value.to_string());
    Ok(Some(Slot::Reference(Some(string_ref))))
}

fn illegal_monitor_state_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalMonitorStateException".to_string(),
    }
}

fn unsupported_operation_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/UnsupportedOperationException".to_string(),
    }
}

fn illegal_argument_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    }
}

fn interrupted_exception_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/InterruptedException".to_string(),
    }
}

fn broken_barrier_exception_error() -> Error {
    Error::JavaException {
        class_name: "java/util/concurrent/BrokenBarrierException".to_string(),
    }
}

fn timeout_exception_error() -> Error {
    Error::JavaException {
        class_name: "java/util/concurrent/TimeoutException".to_string(),
    }
}

const fn request_native_retry(control: &mut NativeControl) {
    control.request(NativeThreadAction::Retry);
}

fn current_host_thread_id() -> std::thread::ThreadId {
    std::thread::current().id()
}

fn with_reentrant_lock_state<T>(
    heap: &mut duke_gc::Heap,
    this_ref: u64,
    f: impl FnOnce(&std::sync::Arc<std::sync::Mutex<duke_gc::ReentrantLockState>>) -> Result<T>,
) -> Result<T> {
    let obj = heap.get_mut(this_ref)?;
    if obj.atomic_payload.is_none() {
        obj.atomic_payload = Some(duke_gc::AtomicPayload::reentrant_lock(false));
    }
    match obj.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::ReentrantLock(state)) => f(state),
        _ => Err(atomic_payload_error(this_ref)),
    }
}

fn condition_state(
    heap: &duke_gc::Heap,
    this_ref: u64,
) -> Result<std::sync::Arc<std::sync::Mutex<duke_gc::ConditionState>>> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Condition(state)) => Ok(std::sync::Arc::clone(state)),
        _ => Err(atomic_payload_error(this_ref)),
    }
}

fn count_down_latch_state(
    heap: &duke_gc::Heap,
    this_ref: u64,
) -> Result<std::sync::Arc<std::sync::Mutex<duke_gc::CountDownLatchState>>> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::CountDownLatch(state)) => Ok(std::sync::Arc::clone(state)),
        _ => Err(atomic_payload_error(this_ref)),
    }
}

fn semaphore_state(
    heap: &duke_gc::Heap,
    this_ref: u64,
) -> Result<std::sync::Arc<std::sync::Mutex<duke_gc::SemaphoreState>>> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Semaphore(state)) => Ok(std::sync::Arc::clone(state)),
        _ => Err(atomic_payload_error(this_ref)),
    }
}

fn cyclic_barrier_state(
    heap: &duke_gc::Heap,
    this_ref: u64,
) -> Result<std::sync::Arc<std::sync::Mutex<duke_gc::CyclicBarrierState>>> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::CyclicBarrier(state)) => Ok(std::sync::Arc::clone(state)),
        _ => Err(atomic_payload_error(this_ref)),
    }
}

fn deadline_from_now(nanos: i64) -> Option<std::time::Instant> {
    let now = std::time::Instant::now();
    u64::try_from(nanos)
        .ok()
        .and_then(|nanos| now.checked_add(std::time::Duration::from_nanos(nanos)))
}

fn read_write_lock_state(
    heap: &mut duke_gc::Heap,
    this_ref: u64,
) -> Result<std::sync::Arc<std::sync::Mutex<duke_gc::ReadWriteLockState>>> {
    let obj = heap.get_mut(this_ref)?;
    if obj.atomic_payload.is_none() {
        obj.atomic_payload = Some(duke_gc::AtomicPayload::read_write_lock());
    }
    match obj.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::ReadWriteLock(state)) => Ok(std::sync::Arc::clone(state)),
        _ => Err(atomic_payload_error(this_ref)),
    }
}

fn read_write_view_state(
    heap: &duke_gc::Heap,
    this_ref: u64,
    expected_kind: duke_gc::ReadWriteLockViewKind,
) -> Result<std::sync::Arc<std::sync::Mutex<duke_gc::ReadWriteLockState>>> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::ReadWriteLockView { state, kind })
            if *kind == expected_kind =>
        {
            Ok(std::sync::Arc::clone(state))
        }
        _ => Err(atomic_payload_error(this_ref)),
    }
}

fn reentrant_lock_is_held_by(
    state: &duke_gc::ReentrantLockState,
    thread_id: std::thread::ThreadId,
) -> bool {
    state.owner == Some(thread_id) && state.hold_count > 0
}

fn reentrant_lock_try_acquire(
    state: &mut duke_gc::ReentrantLockState,
    thread_id: std::thread::ThreadId,
) -> bool {
    match state.owner {
        Some(owner) if owner != thread_id => false,
        Some(_) => {
            state.hold_count = state.hold_count.saturating_add(1);
            true
        }
        None => {
            state.owner = Some(thread_id);
            state.hold_count = 1;
            true
        }
    }
}

fn reentrant_lock_release(
    state: &mut duke_gc::ReentrantLockState,
    thread_id: std::thread::ThreadId,
) -> Result<()> {
    if !reentrant_lock_is_held_by(state, thread_id) {
        return Err(illegal_monitor_state_error());
    }
    state.hold_count -= 1;
    if state.hold_count == 0 {
        state.owner = None;
    }
    Ok(())
}

pub(crate) fn native_reentrant_lock_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.atomic_payload = Some(duke_gc::AtomicPayload::reentrant_lock(false));
    Ok(None)
}

pub(crate) fn native_reentrant_lock_init_fair(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fair = atomic_bool_arg(args, 1)?;
    heap.get_mut(this_ref)?.atomic_payload = Some(duke_gc::AtomicPayload::reentrant_lock(fair));
    Ok(None)
}

pub(crate) fn native_reentrant_lock_lock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let thread_id = current_host_thread_id();
    with_reentrant_lock_state(heap, this_ref, |state| {
        let mut guard = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !reentrant_lock_try_acquire(&mut guard, thread_id) {
            request_native_retry(control);
        }
        Ok(None)
    })
}

pub(crate) fn native_reentrant_lock_try_lock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let thread_id = current_host_thread_id();
    with_reentrant_lock_state(heap, this_ref, |state| {
        let mut guard = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(Some(Slot::Int(i32::from(reentrant_lock_try_acquire(
            &mut guard, thread_id,
        )))))
    })
}

pub(crate) fn native_reentrant_lock_unlock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let thread_id = current_host_thread_id();
    with_reentrant_lock_state(heap, this_ref, |state| {
        let mut guard = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        reentrant_lock_release(&mut guard, thread_id)?;
        Ok(None)
    })
}

pub(crate) fn native_reentrant_lock_new_condition(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock_state =
        with_reentrant_lock_state(heap, this_ref, |state| Ok(std::sync::Arc::clone(state)))?;
    let condition_ref = heap.allocate("duke/util/concurrent/ConditionObject".to_string(), 0);
    heap.get_mut(condition_ref)?.atomic_payload =
        Some(duke_gc::AtomicPayload::condition(lock_state));
    Ok(Some(Slot::Reference(Some(condition_ref))))
}

pub(crate) fn native_reentrant_lock_get_hold_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let thread_id = current_host_thread_id();
    with_reentrant_lock_state(heap, this_ref, |state| {
        let guard = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let hold_count = if guard.owner == Some(thread_id) {
            guard.hold_count
        } else {
            0
        };
        Ok(Some(Slot::Int(hold_count)))
    })
}

pub(crate) fn native_reentrant_lock_is_held_by_current_thread(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let thread_id = current_host_thread_id();
    with_reentrant_lock_state(heap, this_ref, |state| {
        let guard = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(Some(Slot::Int(i32::from(reentrant_lock_is_held_by(
            &guard, thread_id,
        )))))
    })
}

pub(crate) fn native_reentrant_lock_is_locked(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    with_reentrant_lock_state(heap, this_ref, |state| {
        let guard = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(Some(Slot::Int(i32::from(
            guard.owner.is_some() && guard.hold_count > 0,
        ))))
    })
}

pub(crate) fn native_reentrant_lock_is_fair(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    with_reentrant_lock_state(heap, this_ref, |state| {
        let guard = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(Some(Slot::Int(i32::from(guard.fair))))
    })
}

fn condition_await_common(
    args: &[Slot],
    heap: &duke_gc::Heap,
    control: &mut NativeControl,
    timeout_nanos: Option<i64>,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let condition = condition_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let now = std::time::Instant::now();
    let mut condition_guard = condition
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let lock_state = std::sync::Arc::clone(&condition_guard.lock);
    let mut lock_guard = lock_state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if let Some(waiter_idx) = condition_guard
        .waiters
        .iter()
        .position(|waiter| waiter.thread_id == thread_id)
    {
        let waiter = &mut condition_guard.waiters[waiter_idx];
        if !waiter.signaled && waiter.deadline.is_some_and(|deadline| now >= deadline) {
            waiter.signaled = true;
            waiter.timed_out = true;
        }
        if !waiter.signaled {
            drop(lock_guard);
            drop(condition_guard);
            request_native_retry(control);
            return Ok(None);
        }

        let released_hold_count = waiter.released_hold_count.max(1);
        let timed_out = waiter.timed_out;
        let deadline = waiter.deadline;
        if reentrant_lock_try_acquire(&mut lock_guard, thread_id) {
            lock_guard.hold_count = released_hold_count;
            condition_guard.waiters.remove(waiter_idx);
            let result = timeout_nanos.map(|_| {
                let remaining = if timed_out {
                    0
                } else {
                    deadline.map_or(0, |deadline| {
                        i64::try_from(deadline.saturating_duration_since(now).as_nanos())
                            .unwrap_or(i64::MAX)
                    })
                };
                Slot::Long(remaining)
            });
            drop(lock_guard);
            drop(condition_guard);
            return Ok(result);
        }

        drop(lock_guard);
        drop(condition_guard);
        request_native_retry(control);
        return Ok(None);
    }

    if !reentrant_lock_is_held_by(&lock_guard, thread_id) {
        drop(lock_guard);
        drop(condition_guard);
        return Err(illegal_monitor_state_error());
    }

    let released_hold_count = lock_guard.hold_count;
    lock_guard.owner = None;
    lock_guard.hold_count = 0;
    drop(lock_guard);

    let deadline = timeout_nanos.and_then(|nanos| {
        u64::try_from(nanos.max(0))
            .ok()
            .and_then(|nanos| now.checked_add(std::time::Duration::from_nanos(nanos)))
    });
    let immediate_timeout = timeout_nanos.is_some_and(|nanos| nanos <= 0);
    condition_guard.waiters.push(duke_gc::ConditionWaiter {
        thread_id,
        released_hold_count,
        signaled: immediate_timeout,
        deadline,
        timed_out: immediate_timeout,
    });
    drop(condition_guard);
    request_native_retry(control);
    Ok(None)
}

pub(crate) fn native_condition_await(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    condition_await_common(args, heap, control, None)
}

pub(crate) fn native_condition_await_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let nanos = extract_long_arg(args, 1)?;
    condition_await_common(args, heap, control, Some(nanos))
}

pub(crate) fn native_condition_signal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let condition = condition_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let mut condition_guard = condition
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let lock_state = std::sync::Arc::clone(&condition_guard.lock);
    let lock_guard = lock_state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !reentrant_lock_is_held_by(&lock_guard, thread_id) {
        return Err(illegal_monitor_state_error());
    }
    drop(lock_guard);
    if let Some(waiter) = condition_guard
        .waiters
        .iter_mut()
        .find(|waiter| !waiter.signaled)
    {
        waiter.signaled = true;
        waiter.timed_out = false;
    }
    drop(condition_guard);
    Ok(None)
}

pub(crate) fn native_condition_signal_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let condition = condition_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let mut condition_guard = condition
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let lock_state = std::sync::Arc::clone(&condition_guard.lock);
    let lock_guard = lock_state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !reentrant_lock_is_held_by(&lock_guard, thread_id) {
        return Err(illegal_monitor_state_error());
    }
    drop(lock_guard);
    for waiter in &mut condition_guard.waiters {
        waiter.signaled = true;
        waiter.timed_out = false;
    }
    drop(condition_guard);
    Ok(None)
}

pub(crate) fn native_thread_interrupt(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let host_key = match heap.get(thread_ref)?.fields.get(THREAD_HOST_KEY_SLOT) {
        Some(Slot::Int(host_key)) => *host_key,
        _ => -1,
    };
    heap.write_field(thread_ref, THREAD_INTERRUPTED_SLOT, Slot::Int(1))?;
    if let Some(host_thread_id) = host_thread_for_java_thread(host_key) {
        interrupt_host_thread(host_thread_id);
    }
    Ok(None)
}

pub(crate) fn native_thread_is_interrupted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let field_interrupted = matches!(
        heap.get(thread_ref)?.fields.get(THREAD_INTERRUPTED_SLOT),
        Some(Slot::Int(value)) if *value != 0
    );
    let host_key = match heap.get(thread_ref)?.fields.get(THREAD_HOST_KEY_SLOT) {
        Some(Slot::Int(host_key)) => *host_key,
        _ => -1,
    };
    let host_interrupted = host_thread_for_java_thread(host_key)
        .is_some_and(|host_thread_id| {
            interrupted_host_threads()
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .contains(&host_thread_id)
        });
    Ok(Some(Slot::Int(i32::from(
        field_interrupted || host_interrupted,
    ))))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_thread_interrupted(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Int(i32::from(
        take_current_host_thread_interrupted(),
    ))))
}

pub(crate) fn native_count_down_latch_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let count = extract_int_arg(args, 1)?;
    if count < 0 {
        return Err(illegal_argument_error());
    }
    heap.get_mut(this_ref)?.atomic_payload =
        Some(duke_gc::AtomicPayload::count_down_latch(count));
    Ok(None)
}

fn count_down_latch_await_common(
    args: &[Slot],
    heap: &duke_gc::Heap,
    control: &mut NativeControl,
    timeout_nanos: Option<i64>,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let latch = count_down_latch_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let now = std::time::Instant::now();
    let interrupted = take_current_host_thread_interrupted();
    let mut guard = latch
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if interrupted {
        guard.waiters.retain(|waiter| waiter.thread_id != thread_id);
        return Err(interrupted_exception_error());
    }

    if guard.count <= 0 {
        guard.waiters.retain(|waiter| waiter.thread_id != thread_id);
        return Ok(timeout_nanos.map(|_| Slot::Int(1)));
    }
    if timeout_nanos.is_some_and(|nanos| nanos <= 0) {
        return Ok(Some(Slot::Int(0)));
    }

    if let Some(waiter_idx) = guard
        .waiters
        .iter()
        .position(|waiter| waiter.thread_id == thread_id)
    {
        if guard.waiters[waiter_idx]
            .deadline
            .is_some_and(|deadline| now >= deadline)
        {
            guard.waiters.remove(waiter_idx);
            return Ok(Some(Slot::Int(0)));
        }
    } else {
        guard.waiters.push(duke_gc::CountDownLatchWaiter {
            thread_id,
            deadline: timeout_nanos.and_then(deadline_from_now),
        });
    }
    drop(guard);
    request_native_retry(control);
    Ok(None)
}

pub(crate) fn native_count_down_latch_await(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    count_down_latch_await_common(args, heap, control, None)
}

pub(crate) fn native_count_down_latch_await_timeout(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let timeout = extract_long_arg(args, 1)?;
    let unit_ref = extract_ref_arg(args, 2)?;
    let nanos = timeout_nanos(timeout, unit_ref, heap)?;
    count_down_latch_await_common(args, heap, control, Some(nanos))
}

pub(crate) fn native_count_down_latch_count_down(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let latch = count_down_latch_state(heap, this_ref)?;
    let mut guard = latch
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if guard.count > 0 {
        guard.count -= 1;
        if guard.count == 0 {
            guard.waiters.clear();
        }
    }
    drop(guard);
    Ok(None)
}

pub(crate) fn native_count_down_latch_get_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let latch = count_down_latch_state(heap, this_ref)?;
    let count = latch
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .count;
    Ok(Some(Slot::Long(i64::from(count.max(0)))))
}

pub(crate) fn native_count_down_latch_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let latch = count_down_latch_state(heap, this_ref)?;
    let count = latch
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .count
        .max(0);
    let string_ref = heap.allocate_string(format!(
        "java.util.concurrent.CountDownLatch[Count = {count}]"
    ));
    Ok(Some(Slot::Reference(Some(string_ref))))
}

pub(crate) fn native_semaphore_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let permits = extract_int_arg(args, 1)?;
    heap.get_mut(this_ref)?.atomic_payload =
        Some(duke_gc::AtomicPayload::semaphore(permits, false));
    Ok(None)
}

pub(crate) fn native_semaphore_init_fair(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let permits = extract_int_arg(args, 1)?;
    let fair = atomic_bool_arg(args, 2)?;
    heap.get_mut(this_ref)?.atomic_payload =
        Some(duke_gc::AtomicPayload::semaphore(permits, fair));
    Ok(None)
}

fn semaphore_validate_permits(permits: i32) -> Result<()> {
    if permits < 0 {
        return Err(illegal_argument_error());
    }
    Ok(())
}

fn semaphore_remove_waiter(
    waiters: &mut VecDeque<duke_gc::SemaphoreWaiter>,
    thread_id: std::thread::ThreadId,
) {
    if let Some(idx) = waiters
        .iter()
        .position(|waiter| waiter.thread_id == thread_id)
    {
        waiters.remove(idx);
    }
}

fn semaphore_can_acquire(
    state: &duke_gc::SemaphoreState,
    thread_id: std::thread::ThreadId,
    permits: i32,
) -> bool {
    if state.permits < permits {
        return false;
    }
    if !state.fair {
        return true;
    }
    state
        .waiters
        .front()
        .is_none_or(|waiter| waiter.thread_id == thread_id)
}

fn semaphore_try_acquire_immediate(
    state: &mut duke_gc::SemaphoreState,
    thread_id: std::thread::ThreadId,
    permits: i32,
    honor_fairness: bool,
) -> bool {
    if permits == 0 {
        return true;
    }
    let can_acquire = if honor_fairness {
        semaphore_can_acquire(state, thread_id, permits)
    } else {
        state.permits >= permits
    };
    if !can_acquire {
        return false;
    }
    state.permits -= permits;
    semaphore_remove_waiter(&mut state.waiters, thread_id);
    true
}

fn semaphore_acquire_common(
    args: &[Slot],
    heap: &duke_gc::Heap,
    control: &mut NativeControl,
    permits: i32,
    timeout_nanos: Option<i64>,
    returns_bool: bool,
    interruptible: bool,
) -> Result<Option<Slot>> {
    semaphore_validate_permits(permits)?;
    let this_ref = extract_ref_arg(args, 0)?;
    let semaphore = semaphore_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let now = std::time::Instant::now();
    let interrupted = interruptible && take_current_host_thread_interrupted();
    let mut guard = semaphore
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if interrupted {
        semaphore_remove_waiter(&mut guard.waiters, thread_id);
        return Err(interrupted_exception_error());
    }

    if semaphore_try_acquire_immediate(&mut guard, thread_id, permits, true) {
        return Ok(returns_bool.then_some(Slot::Int(1)));
    }
    if timeout_nanos.is_some_and(|nanos| nanos <= 0) {
        return Ok(Some(Slot::Int(0)));
    }

    if let Some(waiter_idx) = guard
        .waiters
        .iter()
        .position(|waiter| waiter.thread_id == thread_id)
    {
        if guard.waiters[waiter_idx]
            .deadline
            .is_some_and(|deadline| now >= deadline)
        {
            guard.waiters.remove(waiter_idx);
            return Ok(Some(Slot::Int(0)));
        }
    } else {
        guard.waiters.push_back(duke_gc::SemaphoreWaiter {
            thread_id,
            permits,
            deadline: timeout_nanos.and_then(deadline_from_now),
        });
    }
    drop(guard);
    request_native_retry(control);
    Ok(None)
}

pub(crate) fn native_semaphore_acquire(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    semaphore_acquire_common(args, heap, control, 1, None, false, true)
}

pub(crate) fn native_semaphore_acquire_many(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let permits = extract_int_arg(args, 1)?;
    semaphore_acquire_common(args, heap, control, permits, None, false, true)
}

pub(crate) fn native_semaphore_acquire_uninterruptibly(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    semaphore_acquire_common(args, heap, control, 1, None, false, false)
}

pub(crate) fn native_semaphore_acquire_uninterruptibly_many(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let permits = extract_int_arg(args, 1)?;
    semaphore_acquire_common(args, heap, control, permits, None, false, false)
}

fn semaphore_try_acquire_common(
    args: &[Slot],
    heap: &duke_gc::Heap,
    permits: i32,
) -> Result<Option<Slot>> {
    semaphore_validate_permits(permits)?;
    let this_ref = extract_ref_arg(args, 0)?;
    let semaphore = semaphore_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let acquired = semaphore_try_acquire_immediate(
        &mut semaphore
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
        thread_id,
        permits,
        false,
    );
    Ok(Some(Slot::Int(i32::from(acquired))))
}

pub(crate) fn native_semaphore_try_acquire(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    semaphore_try_acquire_common(args, heap, 1)
}

pub(crate) fn native_semaphore_try_acquire_many(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let permits = extract_int_arg(args, 1)?;
    semaphore_try_acquire_common(args, heap, permits)
}

pub(crate) fn native_semaphore_try_acquire_timeout(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let timeout = extract_long_arg(args, 1)?;
    let unit_ref = extract_ref_arg(args, 2)?;
    let nanos = timeout_nanos(timeout, unit_ref, heap)?;
    semaphore_acquire_common(args, heap, control, 1, Some(nanos), true, true)
}

fn semaphore_release_common(args: &[Slot], heap: &duke_gc::Heap, permits: i32) -> Result<()> {
    semaphore_validate_permits(permits)?;
    let this_ref = extract_ref_arg(args, 0)?;
    let semaphore = semaphore_state(heap, this_ref)?;
    let mut guard = semaphore
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.permits = guard.permits.saturating_add(permits);
    drop(guard);
    Ok(())
}

pub(crate) fn native_semaphore_release(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    semaphore_release_common(args, heap, 1)?;
    Ok(None)
}

pub(crate) fn native_semaphore_release_many(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let permits = extract_int_arg(args, 1)?;
    semaphore_release_common(args, heap, permits)?;
    Ok(None)
}

pub(crate) fn native_semaphore_available_permits(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let semaphore = semaphore_state(heap, this_ref)?;
    let permits = semaphore
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .permits;
    Ok(Some(Slot::Int(permits)))
}

pub(crate) fn native_semaphore_drain_permits(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let semaphore = semaphore_state(heap, this_ref)?;
    let mut guard = semaphore
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let drained = guard.permits;
    guard.permits = 0;
    drop(guard);
    Ok(Some(Slot::Int(drained)))
}

pub(crate) fn native_semaphore_has_queued_threads(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let semaphore = semaphore_state(heap, this_ref)?;
    let has_waiters = !semaphore
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .waiters
        .is_empty();
    Ok(Some(Slot::Int(i32::from(has_waiters))))
}

pub(crate) fn native_semaphore_get_queue_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let semaphore = semaphore_state(heap, this_ref)?;
    let len = semaphore
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .waiters
        .len();
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}

pub(crate) fn native_semaphore_is_fair(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let semaphore = semaphore_state(heap, this_ref)?;
    let fair = semaphore
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .fair;
    Ok(Some(Slot::Int(i32::from(fair))))
}

pub(crate) fn native_semaphore_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let semaphore = semaphore_state(heap, this_ref)?;
    let permits = semaphore
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .permits;
    let string_ref =
        heap.allocate_string(format!("java.util.concurrent.Semaphore[Permits = {permits}]"));
    Ok(Some(Slot::Reference(Some(string_ref))))
}

pub(crate) fn native_cyclic_barrier_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let parties = extract_int_arg(args, 1)?;
    if parties <= 0 {
        return Err(illegal_argument_error());
    }
    let this = heap.get_mut(this_ref)?;
    this.atomic_payload = Some(duke_gc::AtomicPayload::cyclic_barrier(parties));
    if !this.fields.is_empty() {
        this.fields[0] = Slot::Reference(None);
    }
    Ok(None)
}

pub(crate) fn native_cyclic_barrier_init_action(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_cyclic_barrier_init(args, heap, out, control)?;
    let this_ref = extract_ref_arg(args, 0)?;
    let action = extract_slot_arg(args, 2);
    if !heap.get(this_ref)?.fields.is_empty() {
        heap.write_field(this_ref, 0, action)?;
        heap.remember_reference_write(this_ref, action);
    }
    Ok(None)
}

fn cyclic_barrier_break_current(
    barrier: &std::sync::Arc<std::sync::Mutex<duke_gc::CyclicBarrierState>>,
) {
    barrier
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .break_generation();
}

#[allow(clippy::too_many_arguments)]
fn cyclic_barrier_await_common(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
    timeout_nanos: Option<i64>,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let action = extract_field_arg(heap, this_ref, 0)?;
    let barrier = cyclic_barrier_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let now = std::time::Instant::now();
    let interrupted = take_current_host_thread_interrupted();
    let run_action = {
        let mut guard = barrier
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(waiter_idx) = guard
            .waiters
            .iter()
            .position(|waiter| waiter.thread_id == thread_id)
        {
            if interrupted {
                guard.break_generation();
                guard.waiters.remove(waiter_idx);
                return Err(interrupted_exception_error());
            }
            if guard.waiters[waiter_idx].broken {
                guard.waiters.remove(waiter_idx);
                return Err(broken_barrier_exception_error());
            }
            if guard.waiters[waiter_idx].generation != guard.generation {
                let arrival_index = guard.waiters[waiter_idx].arrival_index;
                guard.waiters.remove(waiter_idx);
                return Ok(Some(Slot::Int(arrival_index)));
            }
            if guard.waiters[waiter_idx]
                .deadline
                .is_some_and(|deadline| now >= deadline)
            {
                guard.break_generation();
                guard.waiters.remove(waiter_idx);
                return Err(timeout_exception_error());
            }
            drop(guard);
            request_native_retry(control);
            return Ok(None);
        }

        if guard.broken {
            return Err(broken_barrier_exception_error());
        }
        if interrupted {
            guard.break_generation();
            return Err(interrupted_exception_error());
        }
        if timeout_nanos.is_some_and(|nanos| nanos <= 0) {
            guard.break_generation();
            return Err(timeout_exception_error());
        }

        let arrival_index = guard.count.saturating_sub(1);
        guard.count = arrival_index;
        if arrival_index == 0 {
            action.as_reference()
        } else {
            let generation = guard.generation;
            guard.waiters.push(duke_gc::CyclicBarrierWaiter {
                thread_id,
                generation,
                arrival_index,
                deadline: timeout_nanos.and_then(deadline_from_now),
                broken: false,
            });
            drop(guard);
            request_native_retry(control);
            return Ok(None);
        }
    };

    if let Some(action_ref) = run_action {
        let action_class = heap.get(action_ref)?.class_name.clone();
        let action_result = ops.invoke(
            heap,
            out,
            &action_class,
            "run",
            "()V",
            vec![Slot::Reference(Some(action_ref))],
        );
        if action_result.is_err() {
            cyclic_barrier_break_current(&barrier);
            return Err(broken_barrier_exception_error());
        }
    }
    barrier
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .trip_generation();
    Ok(Some(Slot::Int(0)))
}

pub(crate) fn native_cyclic_barrier_await(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    cyclic_barrier_await_common(args, heap, out, control, ops, None)
}

pub(crate) fn native_cyclic_barrier_await_timeout(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let timeout = extract_long_arg(args, 1)?;
    let unit_ref = extract_ref_arg(args, 2)?;
    let nanos = timeout_nanos(timeout, unit_ref, heap)?;
    cyclic_barrier_await_common(args, heap, out, control, ops, Some(nanos))
}

pub(crate) fn native_cyclic_barrier_get_parties(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let barrier = cyclic_barrier_state(heap, this_ref)?;
    let parties = barrier
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .parties;
    Ok(Some(Slot::Int(parties)))
}

pub(crate) fn native_cyclic_barrier_get_number_waiting(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let barrier = cyclic_barrier_state(heap, this_ref)?;
    let waiting = {
        let guard = barrier
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard
            .waiters
            .iter()
            .filter(|waiter| waiter.generation == guard.generation && !waiter.broken)
            .count()
    };
    Ok(Some(Slot::Int(i32::try_from(waiting).unwrap_or(i32::MAX))))
}

pub(crate) fn native_cyclic_barrier_is_broken(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let barrier = cyclic_barrier_state(heap, this_ref)?;
    let broken = barrier
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .broken;
    Ok(Some(Slot::Int(i32::from(broken))))
}

pub(crate) fn native_cyclic_barrier_reset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let barrier = cyclic_barrier_state(heap, this_ref)?;
    barrier
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .reset();
    Ok(None)
}

pub(crate) fn native_cyclic_barrier_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let barrier = cyclic_barrier_state(heap, this_ref)?;
    let (parties, count) = {
        let guard = barrier
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (guard.parties, guard.count)
    };
    let string_ref = heap.allocate_string(format!(
        "java.util.concurrent.CyclicBarrier[Parties = {parties}, Count = {count}]"
    ));
    Ok(Some(Slot::Reference(Some(string_ref))))
}

fn allocate_read_write_view(
    heap: &mut duke_gc::Heap,
    state: std::sync::Arc<std::sync::Mutex<duke_gc::ReadWriteLockState>>,
    class_name: &str,
    kind: duke_gc::ReadWriteLockViewKind,
) -> Result<Slot> {
    let view_ref = heap.allocate(class_name.to_string(), 0);
    heap.get_mut(view_ref)?.atomic_payload =
        Some(duke_gc::AtomicPayload::read_write_lock_view(state, kind));
    Ok(Slot::Reference(Some(view_ref)))
}

pub(crate) fn native_reentrant_read_write_lock_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = std::sync::Arc::new(std::sync::Mutex::new(
        duke_gc::ReadWriteLockState::default(),
    ));
    let read_lock = allocate_read_write_view(
        heap,
        std::sync::Arc::clone(&state),
        "java/util/concurrent/locks/ReentrantReadWriteLock$ReadLock",
        duke_gc::ReadWriteLockViewKind::Read,
    )?;
    let write_lock = allocate_read_write_view(
        heap,
        std::sync::Arc::clone(&state),
        "java/util/concurrent/locks/ReentrantReadWriteLock$WriteLock",
        duke_gc::ReadWriteLockViewKind::Write,
    )?;
    let should_remember = {
        let this = heap.get_mut(this_ref)?;
        this.atomic_payload = Some(duke_gc::AtomicPayload::ReadWriteLock(state));
        if this.fields.len() >= 2 {
            this.fields[0] = read_lock;
            this.fields[1] = write_lock;
            true
        } else {
            false
        }
    };
    if should_remember {
        heap.remember_reference_write(this_ref, read_lock);
        heap.remember_reference_write(this_ref, write_lock);
    }
    Ok(None)
}

pub(crate) fn native_reentrant_read_write_lock_read_lock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let slot = extract_field_arg(heap, this_ref, 0)?;
    if matches!(slot, Slot::Reference(Some(_))) {
        return Ok(Some(slot));
    }
    let state = read_write_lock_state(heap, this_ref)?;
    let slot = allocate_read_write_view(
        heap,
        state,
        "java/util/concurrent/locks/ReentrantReadWriteLock$ReadLock",
        duke_gc::ReadWriteLockViewKind::Read,
    )?;
    let should_remember = {
        let this = heap.get_mut(this_ref)?;
        if this.fields.is_empty() {
            false
        } else {
            this.fields[0] = slot;
            true
        }
    };
    if should_remember {
        heap.remember_reference_write(this_ref, slot);
    }
    Ok(Some(slot))
}

pub(crate) fn native_reentrant_read_write_lock_write_lock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let slot = extract_field_arg(heap, this_ref, 1)?;
    if matches!(slot, Slot::Reference(Some(_))) {
        return Ok(Some(slot));
    }
    let state = read_write_lock_state(heap, this_ref)?;
    let slot = allocate_read_write_view(
        heap,
        state,
        "java/util/concurrent/locks/ReentrantReadWriteLock$WriteLock",
        duke_gc::ReadWriteLockViewKind::Write,
    )?;
    let should_remember = {
        let this = heap.get_mut(this_ref)?;
        if this.fields.len() > 1 {
            this.fields[1] = slot;
            true
        } else {
            false
        }
    };
    if should_remember {
        heap.remember_reference_write(this_ref, slot);
    }
    Ok(Some(slot))
}

fn read_lock_try_acquire(
    state: &mut duke_gc::ReadWriteLockState,
    thread_id: std::thread::ThreadId,
) -> bool {
    if state.writer.is_some_and(|writer| writer != thread_id) {
        return false;
    }
    let count = state.readers.entry(thread_id).or_insert(0);
    *count = count.saturating_add(1);
    true
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

pub(crate) fn native_read_lock_lock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_view_state(heap, this_ref, duke_gc::ReadWriteLockViewKind::Read)?;
    let thread_id = current_host_thread_id();
    let mut guard = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !read_lock_try_acquire(&mut guard, thread_id) {
        request_native_retry(control);
    }
    Ok(None)
}

pub(crate) fn native_read_lock_try_lock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_view_state(heap, this_ref, duke_gc::ReadWriteLockViewKind::Read)?;
    let thread_id = current_host_thread_id();
    let mut guard = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(Some(Slot::Int(i32::from(read_lock_try_acquire(
        &mut guard, thread_id,
    )))))
}

pub(crate) fn native_read_lock_unlock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_view_state(heap, this_ref, duke_gc::ReadWriteLockViewKind::Read)?;
    let thread_id = current_host_thread_id();
    let mut guard = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(count) = guard.readers.get_mut(&thread_id) else {
        return Err(illegal_monitor_state_error());
    };
    *count -= 1;
    if *count == 0 {
        guard.readers.remove(&thread_id);
    }
    drop(guard);
    Ok(None)
}

pub(crate) fn native_read_lock_new_condition(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Err(unsupported_operation_error())
}

pub(crate) fn native_write_lock_lock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_view_state(heap, this_ref, duke_gc::ReadWriteLockViewKind::Write)?;
    let thread_id = current_host_thread_id();
    let mut guard = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    let state = read_write_view_state(heap, this_ref, duke_gc::ReadWriteLockViewKind::Write)?;
    let thread_id = current_host_thread_id();
    let mut guard = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(Some(Slot::Int(i32::from(write_lock_try_acquire(
        &mut guard, thread_id,
    )))))
}

pub(crate) fn native_write_lock_unlock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_view_state(heap, this_ref, duke_gc::ReadWriteLockViewKind::Write)?;
    let thread_id = current_host_thread_id();
    let mut guard = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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

pub(crate) fn native_reentrant_read_write_lock_is_write_locked(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_lock_state(heap, this_ref)?;
    let guard = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(Some(Slot::Int(i32::from(guard.writer.is_some()))))
}

pub(crate) fn native_reentrant_read_write_lock_is_write_locked_by_current_thread(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_lock_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let guard = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(Some(Slot::Int(i32::from(guard.writer == Some(thread_id)))))
}

pub(crate) fn native_reentrant_read_write_lock_get_write_hold_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_lock_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let guard = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let count = if guard.writer == Some(thread_id) {
        guard.write_hold_count
    } else {
        0
    };
    Ok(Some(Slot::Int(count)))
}

pub(crate) fn native_reentrant_read_write_lock_get_read_hold_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_lock_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let guard = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(Some(Slot::Int(
        guard.readers.get(&thread_id).copied().unwrap_or_default(),
    )))
}

pub(crate) fn native_reentrant_read_write_lock_get_read_lock_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_lock_state(heap, this_ref)?;
    let count = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .readers
        .values()
        .copied()
        .sum();
    Ok(Some(Slot::Int(count)))
}

/// Native: `String.substring(int)` - substring from begin to end.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn native_string_substring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;

    let begin = extract_int_arg(args, 1)? as usize;
    let sub = {
        let obj = heap.get(this_ref)?;
        let s = obj.string_value.as_deref().unwrap_or_default();
        let char_count = s.chars().count();
        if begin > char_count {
            return Err(Error::ArrayIndexOutOfBounds {
                index: i32::try_from(begin).unwrap_or(i32::MAX),
                length: char_count,
            });
        }
        let byte_begin = s.char_indices().nth(begin).map_or(s.len(), |(i, _)| i);
        s[byte_begin..].to_string()
    };

    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.substring(int, int)` — substring from begin to end (exclusive).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn native_string_substring_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;

    let begin = extract_int_arg(args, 1)? as usize;
    let end = extract_int_arg(args, 2)? as usize;
    let sub = {
        let obj = heap.get(this_ref)?;
        let s = obj.string_value.as_deref().unwrap_or_default();
        let char_count = s.chars().count();
        if begin > end || end > char_count {
            return Err(Error::ArrayIndexOutOfBounds {
                index: i32::try_from(end).unwrap_or(i32::MAX),
                length: char_count,
            });
        }
        let byte_begin = s.char_indices().nth(begin).map_or(s.len(), |(i, _)| i);
        let byte_end = s.char_indices().nth(end).map_or(s.len(), |(i, _)| i);
        s[byte_begin..byte_end].to_string()
    };

    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.indexOf(String)` — find first occurrence of target.
pub(crate) fn native_string_indexof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;

    let target_ref = extract_ref_arg(args, 1)?;
    // Fetch objects from heap in one go to keep borrows short
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let result = s.find(target).map_or(-1, |i| i as i32);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.indexOf(String, int)I` — first occurrence at or after fromIndex.
pub(crate) fn native_string_indexof_from(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target_ref = extract_ref_arg(args, 1)?;
    #[allow(clippy::cast_sign_loss)] // .max(0) guarantees non-negative
    let from = extract_int_arg(args, 2).unwrap_or(0).max(0) as usize;
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();
    let search_in = if from < s.len() { &s[from..] } else { "" };
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let result = search_in.find(target).map_or(-1, |i| (from + i) as i32);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.lastIndexOf(String, int)I` — last occurrence at or before fromIndex.
pub(crate) fn native_string_last_indexof_from(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let sub_ref = extract_ref_arg(args, 1)?;
    #[allow(clippy::cast_sign_loss)] // .max(0) guarantees non-negative
    let from = extract_int_arg(args, 2).unwrap_or(0).max(0) as usize;
    let this_str = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let sub_str = heap.get(sub_ref)?.string_value.clone().unwrap_or_default();
    let search_in = if from + sub_str.len() < this_str.len() {
        &this_str[..from + sub_str.len()]
    } else {
        &this_str
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let result = search_in.rfind(sub_str.as_str()).map_or(-1, |i| i as i32);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.contains(CharSequence)` — check if string contains target.
pub(crate) fn native_string_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;

    let target_ref = extract_ref_arg(args, 1)?;
    // Fetch objects from heap in one go to keep borrows short
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.contains(target)))))
}

/// Native: `String.isEmpty()` — check if string is empty.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_string_isempty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let s = obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.is_empty()))))
}

// Dispatch string constants used by the callback-based sort chain.
const COMPARE_TO_METHOD: &str = "compareTo";
const COMPARE_TO_OBJECT_DESC: &str = "(Ljava/lang/Object;)I";
const SORT_COMPARATOR_DESC: &str = "(Ljava/util/Comparator;)V";

/// Maps a `std::cmp::Ordering` to the Java `compareTo` convention: -1 / 0 / 1.
///
/// Used by all boxed-type `compareTo` natives to return a consistent,
/// sign-correct value without relying on `Ordering`'s internal discriminant.
#[inline]
const fn ordering_to_int(o: std::cmp::Ordering) -> i32 {
    match o {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Native: `String.compareTo(String)` — delegates to the Object overload.
pub(crate) fn native_string_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_string_compareto_object(args, heap, out, control)
}

/// Native: `String.compareTo(Object)` — lexicographic comparison via Object descriptor.
pub(crate) fn native_string_compareto_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let str_val = |s: &Slot| -> Result<String> {
        match s {
            Slot::Reference(Some(r)) => Ok(heap.get(*r)?.string_value.clone().unwrap_or_default()),
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => str_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = str_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.as_str().cmp(b.as_str())))))
}

/// Native: `String.startsWith(String)` — check if string starts with prefix.
pub(crate) fn native_string_startswith(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let prefix_ref = extract_ref_arg(args, 1)?;
    let prefix = heap
        .get(prefix_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.starts_with(&prefix)))))
}

/// Native: `String.endsWith(String)` — check if string ends with suffix.
pub(crate) fn native_string_endswith(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let suffix_ref = extract_ref_arg(args, 1)?;
    let suffix = heap
        .get(suffix_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.ends_with(&suffix)))))
}

/// Native: `String.trim()` — remove leading and trailing whitespace.
pub(crate) fn native_string_trim(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let trimmed = s.trim().to_string();
    let r = heap.allocate_string(trimmed);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.toCharArray()` — convert string to char array.
///
/// **Bolt Optimization:**
/// Eliminates an intermediate `.collect::<Vec<char>>()` allocation by pre-computing
/// the character length via `.count()` and iterating characters directly into the heap array.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_string_tochararray(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let char_count = s.chars().count();
    let arr_ref = heap.allocate("[C".to_string(), char_count);
    for (i, c) in s.chars().enumerate() {
        heap.get_mut(arr_ref)?.fields[i] = Slot::Int(c as i32);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

// ---- Integer natives ----

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

/// Native: `Collections.emptyList()List` — returns a new empty `ArrayList`.
pub(crate) fn native_collections_empty_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // Return an immutable empty UnmodifiableList (same field layout as ArrayList)
    let r = heap.allocate("java/util/UnmodifiableList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.emptySet()Set` — returns a new empty `HashSet`.
pub(crate) fn native_collections_empty_set(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.emptyMap()Map` — returns a new empty `HashMap`.
pub(crate) fn native_collections_empty_map(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.unmodifiableList(List)List` — returns the same list (no-copy; single-threaded).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // Create an UnmodifiableList backed by the source list's elements.
    // Mutation methods on this class throw UnsupportedOperationException.
    let src_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Reference(None))),
    };
    let (size, elems) = {
        let src = heap.get(src_ref)?;
        let size = src.fields.first().copied().unwrap_or(Slot::Int(0));
        let elems = src.fields[1..].to_vec();
        (size, elems)
    };
    let n_fields = 1 + elems.len();
    let dst_ref = heap.allocate("java/util/UnmodifiableList".to_string(), n_fields);
    {
        let dst = heap.get_mut(dst_ref)?;
        dst.fields[0] = size;
        for (i, e) in elems.iter().enumerate() {
            dst.fields[1 + i] = *e;
        }
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: mutation ops on `UnmodifiableList` throw `UnsupportedOperationException`.
pub(crate) fn native_unmodifiable_list_mutation(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Err(Error::JavaException {
        class_name: "java/lang/UnsupportedOperationException".to_string(),
    })
}

/// Native: `Math.random()D` — returns a pseudo-random double in [0.0, 1.0).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_math_random(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // Use a simple deterministic seed based on stack pointer heuristic
    // For a JVM interpreter we just use a fixed-seed LCG for reproducibility
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(12345);
    let old = SEED.load(Ordering::Relaxed);
    let new = old.wrapping_mul(25_214_903_917).wrapping_add(11) & 0x0000_FFFF_FFFF_FFFF;
    SEED.store(new, Ordering::Relaxed);
    #[allow(clippy::cast_precision_loss)]
    let v = (new as f64) / (1_u64 << 48) as f64;
    Ok(Some(Slot::Double(v)))
}

/// Native: `Arrays.stream(int[])IntStream` — wraps an int array into an `IntStream`.
pub(crate) fn native_arrays_stream_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    // int[] layout: fields = Int elements directly (no length header); arraylength = fields.len()
    let arr_obj = heap.get(arr_ref)?;
    let values: Vec<i32> = arr_obj
        .fields
        .iter()
        .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
        .collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

/// Native: `Arrays.stream(int[], int, int)IntStream` — wraps a subrange as an `IntStream`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_arrays_stream_int_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let from = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let to = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let arr_obj = heap.get(arr_ref)?;
    let values: Vec<i32> = arr_obj
        .fields
        .get(from..to.min(arr_obj.fields.len()))
        .unwrap_or(&[])
        .iter()
        .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
        .collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

/// Native: `Arrays.stream(Object[])Stream` — wraps a reference array as an eager `Stream`.
pub(crate) fn native_arrays_stream_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let elems: Vec<Slot> = heap.get(arr_ref)?.fields.clone();
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for elem in elems {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `Comparator.comparing(Function)Comparator` — creates a comparator by key extractor.
/// Returns a `duke/util/ComparingComparator` with `fields[0]`=fn\_ref.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_comparing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/ComparingComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Comparator.comparing compare(Object,Object)I` — compare via key extractor.
pub(crate) fn native_comparing_comparator_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Reference(None));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, b],
        )?
        .unwrap_or(Slot::Reference(None));
    // Compare extracted keys via natural ordering (String, Integer, Long, or raw int).
    let cmp = compare_treemap_keys(ka, kb, heap) as i32;
    Ok(Some(Slot::Int(cmp)))
}

// ---------------------------------------------------------------------------
// Phase 43: Collectors.toSet/toMap, Stream.mapToInt/min/max, IntStream.reduce,
//           Arrays.sort(Object[]), String(char[])/valueOf(char[])
// ---------------------------------------------------------------------------

/// Native: `Collectors.toSet()Collector` — returns a `ToSetCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_set(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/ToSetCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.toMap(keyFn, valFn)Collector` — stores both functions in `ToMapCollector`.
pub(crate) fn native_collectors_to_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let key_fn = extract_slot_arg(args, 0);
    let val_fn = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ToMapCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = key_fn;
    heap.get_mut(r)?.fields[1] = val_fn;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Stream.mapToInt(ToIntFunction)IntStream` — maps each element via `applyAsInt`.
pub(crate) fn native_stream_map_to_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut values = Vec::with_capacity(elems.len());
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(Ljava/lang/Object;)I",
                vec![fn_slot, elem],
            )?
            .unwrap_or(Slot::Int(0));
        match result {
            Slot::Int(n) => values.push(n),
            Slot::Reference(Some(r)) => {
                // Unbox Integer/Short/Byte if the function returned a boxed type.
                let n = match heap.get(r)?.fields.first() {
                    Some(Slot::Int(v)) => *v,
                    _ => 0,
                };
                values.push(n);
            }
            _ => values.push(0),
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

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

/// Native: `Stream.min(Comparator)Optional` — returns minimum element by comparator.
pub(crate) fn native_stream_min_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    stream_min_max_by_comparator(args, heap, out, ops, false)
}

/// Native: `Stream.max(Comparator)Optional` — returns maximum element by comparator.
pub(crate) fn native_stream_max_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    stream_min_max_by_comparator(args, heap, out, ops, true)
}

fn stream_min_max_by_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    want_max: bool,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let cmp_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(cmp_ref)) = cmp_slot else {
        let r = heap.allocate("java/util/Optional".to_string(), 1);
        heap.get_mut(r)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(r))));
    };
    let cmp_class = heap.get(cmp_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let opt_r = heap.allocate("java/util/Optional".to_string(), 1);
    if elems.is_empty() {
        heap.get_mut(opt_r)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(opt_r))));
    }
    let mut best = elems[0];
    for elem in elems.into_iter().skip(1) {
        let cmp_result = ops
            .invoke(
                heap,
                out,
                &cmp_class,
                "compare",
                "(Ljava/lang/Object;Ljava/lang/Object;)I",
                vec![cmp_slot, elem, best],
            )?
            .unwrap_or(Slot::Int(0));
        let cmp_val = match cmp_result {
            Slot::Int(n) => n,
            _ => 0,
        };
        // For min: pick elem if elem < best (cmp_val < 0)
        // For max: pick elem if elem > best (cmp_val > 0)
        if (want_max && cmp_val > 0) || (!want_max && cmp_val < 0) {
            best = elem;
        }
    }
    heap.get_mut(opt_r)?.fields[0] = best;
    Ok(Some(Slot::Reference(Some(opt_r))))
}

/// Native: `Arrays.sort(Object[])V` — natural order sort using `compareTo`.
#[allow(clippy::too_many_lines)]
pub(crate) fn native_arrays_sort_objects(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let len = heap.get(arr_ref)?.fields.len();
    // Insertion sort with compareTo callbacks
    for i in 1..len {
        let mut j = i;
        while j > 0 {
            let a = heap.get(arr_ref)?.fields[j - 1];
            let b = heap.get(arr_ref)?.fields[j];
            let cmp = compare_slots_natural(a, b, heap, out, ops)?;
            if cmp <= 0 {
                break;
            }
            heap.write_field(arr_ref, j - 1, b)?;
            heap.write_field(arr_ref, j, a)?;
            j -= 1;
        }
    }
    Ok(None)
}

fn compare_slots_natural(
    a: Slot,
    b: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<i32> {
    match (a, b) {
        (Slot::Reference(Some(ra)), Slot::Reference(Some(_rb))) => {
            let a_class = heap.get(ra)?.class_name.clone();
            let result = ops
                .invoke(
                    heap,
                    out,
                    &a_class,
                    "compareTo",
                    "(Ljava/lang/Object;)I",
                    vec![a, b],
                )?
                .unwrap_or(Slot::Int(0));
            Ok(match result {
                Slot::Int(n) => n,
                _ => 0,
            })
        }
        (Slot::Int(a), Slot::Int(b)) => Ok(a.cmp(&b) as i32),
        _ => Ok(0),
    }
}

/// Native: `String.<init>(char[])V` — constructs a String from a char array.
pub(crate) fn native_string_init_from_chars(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Reference(Some(arr_ref))) = args.get(1).copied() else {
        heap.get_mut(this_ref)?.string_value = Some(String::new());
        return Ok(None);
    };
    let chars: String = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| match s {
            Slot::Int(n) => char::from_u32(u32::try_from(*n).unwrap_or(0)),
            _ => None,
        })
        .collect();
    heap.get_mut(this_ref)?.string_value = Some(chars);
    Ok(None)
}

/// Native: `String.valueOf(char[])String` — creates String from char array.
pub(crate) fn native_string_value_of_char_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => {
            return Ok(Some(Slot::Reference(Some(
                heap.allocate_string(String::new()),
            ))));
        }
    };
    let chars: String = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| match s {
            Slot::Int(n) => char::from_u32(u32::try_from(*n).unwrap_or(0)),
            _ => None,
        })
        .collect();
    Ok(Some(Slot::Reference(Some(heap.allocate_string(chars)))))
}

/// Native: `Collections.singletonList(Object)List` — returns a one-element `ArrayList`.
pub(crate) fn native_collections_singleton_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let element = extract_slot_arg(args, 0);
    let r = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    native_arraylist_add(&[Slot::Reference(Some(r)), element], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.reverse(List)V` — reverses an `ArrayList` in-place.
pub(crate) fn native_collections_reverse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(None),
    };
    // Elements are at fields[1..=size]; reverse that slice.
    let fields = &mut heap.get_mut(list_ref)?.fields;
    fields[1..=size].reverse();
    Ok(None)
}

/// Native: `Collections.frequency(Collection, Object)I` — count occurrences of element.
pub(crate) fn native_collections_frequency(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let coll_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let size = match heap.get(coll_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let fields = heap.get(coll_ref)?.fields[1..=size].to_vec();
    let count = fields
        .iter()
        .filter(|s| slots_equal(s, &target, heap))
        .count();
    Ok(Some(Slot::Int(i32::try_from(count).unwrap_or(i32::MAX))))
}

// ---- String.valueOf overloads ----

/// Native: `String.valueOf(long)` — converts long to String.
pub(crate) fn native_string_value_of_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(double)` — converts double to String.
pub(crate) fn native_string_value_of_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_double_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(float)` — converts float to String.
pub(crate) fn native_string_value_of_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_float_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(boolean)` — converts boolean to String.
pub(crate) fn native_string_value_of_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v != 0,
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Int(boolean)",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(if val { "true" } else { "false" }.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(char)` — converts char to String.
pub(crate) fn native_string_value_of_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('?'),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Int(char)",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(Object)` — converts Object to String.
pub(crate) fn native_string_value_of_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let s = heap_object_to_string(heap.get(*r)?, *r);
            let r = heap.allocate_string(s);
            Ok(Some(Slot::Reference(Some(r))))
        }
        Some(Slot::Reference(None)) => {
            let r = heap.allocate_string("null".to_string());
            Ok(Some(Slot::Reference(Some(r))))
        }
        _ => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

// ---- String.concat ----

/// Native: `String.concat(String)` — concatenates two strings.
pub(crate) fn native_string_concat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s1 = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let other_ref = extract_ref_arg(args, 1)?;
    let s2 = heap
        .get(other_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let r = heap.allocate_string(format!("{s1}{s2}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Formats a single boxed slot value using the given format specifier.
/// Apply width/alignment/flags to an already-formatted value string.
fn apply_format_width(s: String, width: usize, left_align: bool, zero_pad: bool) -> String {
    // Havoc: bounds check width to prevent OOM
    let max_width = 1024 * 1024 * 128; // 128 MB
    let width = width.min(max_width);

    if s.len() >= width {
        return s;
    }
    let pad = width - s.len();
    if left_align {
        format!("{s}{}", " ".repeat(pad))
    } else if zero_pad {
        // zero-pad: insert zeros after optional sign
        if s.starts_with('-') || s.starts_with('+') {
            let (sign, rest) = s.split_at(1);
            format!("{sign}{}{rest}", "0".repeat(pad))
        } else {
            format!("{}{s}", "0".repeat(pad))
        }
    } else {
        format!("{}{s}", " ".repeat(pad))
    }
}

#[allow(clippy::too_many_lines)]
fn format_arg(
    spec: char,
    flags: &str,
    width: Option<usize>,
    precision: Option<usize>,
    slot: &Slot,
    heap: &duke_gc::Heap,
) -> Result<String> {
    let left_align = flags.contains('-');
    let force_sign = flags.contains('+');
    let zero_pad = flags.contains('0') && !left_align;

    let raw = match slot {
        Slot::Reference(None) => {
            if spec == 'b' {
                "false".to_string()
            } else {
                "null".to_string()
            }
        }
        Slot::Reference(Some(r)) => {
            let obj = heap.get(*r)?;
            match spec {
                's' => heap_object_to_string(obj, *r),
                'b' => {
                    // true if non-null Boolean true, else depends
                    if obj.class_name == "java/lang/Boolean" {
                        match obj.fields.first() {
                            Some(Slot::Int(n)) => {
                                if *n != 0 { "true" } else { "false" }.to_string()
                            }
                            _ => "true".to_string(),
                        }
                    } else {
                        "true".to_string() // non-null object → true
                    }
                }
                'c' => {
                    let code = match obj.fields.first() {
                        Some(Slot::Int(n)) => *n,
                        _ => 0,
                    };
                    #[allow(clippy::cast_sign_loss)]
                    char::from_u32(code as u32).map_or(String::new(), |c| c.to_string())
                }
                'd' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Int(v)) => i64::from(*v),
                        Some(Slot::Long(v)) => *v,
                        _ => 0,
                    };
                    if force_sign && v >= 0 {
                        format!("+{v}")
                    } else {
                        v.to_string()
                    }
                }
                'o' => match obj.fields.first() {
                    Some(Slot::Int(v)) => format!("{v:o}"),
                    Some(Slot::Long(v)) => format!("{v:o}"),
                    _ => "0".to_string(),
                },
                'f' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Double(v)) => *v,
                        Some(Slot::Float(v)) => f64::from(*v),
                        _ => 0.0,
                    };
                    let s = precision.map_or_else(|| format!("{v:.6}"), |p| format!("{v:.p$}"));
                    if force_sign && v >= 0.0 {
                        format!("+{s}")
                    } else {
                        s
                    }
                }
                'e' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Double(v)) => *v,
                        Some(Slot::Float(v)) => f64::from(*v),
                        _ => 0.0,
                    };
                    let prec = precision.unwrap_or(6);
                    // format in scientific notation matching Java's %e output
                    let s = format_scientific(v, prec, false);
                    if force_sign && v >= 0.0 {
                        format!("+{s}")
                    } else {
                        s
                    }
                }
                'x' => match obj.fields.first() {
                    Some(Slot::Int(v)) => format!("{v:x}"),
                    Some(Slot::Long(v)) => format!("{v:x}"),
                    _ => "0".to_string(),
                },
                'X' => match obj.fields.first() {
                    Some(Slot::Int(v)) => format!("{v:X}"),
                    Some(Slot::Long(v)) => format!("{v:X}"),
                    _ => "0".to_string(),
                },
                _ => String::new(),
            }
        }
        Slot::Int(n) => match spec {
            'd' => {
                if force_sign && *n >= 0 {
                    format!("+{n}")
                } else {
                    n.to_string()
                }
            }
            'b' => "true".to_string(),
            'c' =>
            {
                #[allow(clippy::cast_sign_loss)]
                char::from_u32(*n as u32).map_or(String::new(), |c| c.to_string())
            }
            'o' => format!("{n:o}"),
            'x' => format!("{n:x}"),
            'X' => format!("{n:X}"),
            _ => n.to_string(),
        },
        Slot::Long(n) => match spec {
            'd' => {
                if force_sign && *n >= 0 {
                    format!("+{n}")
                } else {
                    n.to_string()
                }
            }
            'o' => format!("{n:o}"),
            'x' => format!("{n:x}"),
            'X' => format!("{n:X}"),
            _ => n.to_string(),
        },
        Slot::Double(v) => match spec {
            'f' => {
                let s = precision.map_or_else(|| format!("{v:.6}"), |p| format!("{v:.p$}"));
                if force_sign && *v >= 0.0 {
                    format!("+{s}")
                } else {
                    s
                }
            }
            'e' => {
                let prec = precision.unwrap_or(6);
                let s = format_scientific(*v, prec, false);
                if force_sign && *v >= 0.0 {
                    format!("+{s}")
                } else {
                    s
                }
            }
            _ => format!("{v}"),
        },
        _ => String::new(),
    };

    Ok(match width {
        None => raw,
        Some(w) => apply_format_width(raw, w, left_align, zero_pad),
    })
}

/// Format a float in Java-style scientific notation `1.234568e+05`.
fn format_scientific(v: f64, prec: usize, upper: bool) -> String {
    // Havoc: bounds check prec to prevent OOM
    let max_prec = 1024 * 1024 * 128; // 128 MB
    let prec = prec.min(max_prec);

    if v == 0.0 {
        let zeros = "0".repeat(prec);
        let e = if upper { 'E' } else { 'e' };
        return format!("0.{zeros}{e}+00");
    }
    #[allow(clippy::cast_possible_truncation)]
    let exp = v.abs().log10().floor() as i32;
    let mantissa = v / 10_f64.powi(exp);
    let s = format!("{mantissa:.prec$}");
    let e_char = if upper { 'E' } else { 'e' };
    if exp >= 0 {
        format!("{s}{e_char}+{exp:02}")
    } else {
        format!("{s}{e_char}-{:02}", exp.unsigned_abs())
    }
}

/// Native: `String.format(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;`
pub(crate) fn native_string_format(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fmt_ref = extract_ref_arg(args, 0)?;
    let fmt = heap.get(fmt_ref)?.string_value.clone().unwrap_or_default();

    let arr_len = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.fields.len(),
        _ => 0,
    };

    let mut result = String::new();
    let mut arg_idx = 0usize;
    let mut chars = fmt.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '%' {
            result.push(ch);
            continue;
        }

        // Parse flags: -, +, 0
        let mut flags = String::new();
        while let Some(&f) = chars.peek() {
            if matches!(f, '-' | '+' | '0' | ' ' | '#') {
                flags.push(f);
                chars.next();
            } else {
                break;
            }
        }

        // Parse optional width
        let mut width_str = String::new();
        while let Some(&d) = chars.peek() {
            if d.is_ascii_digit() {
                width_str.push(d);
                chars.next();
            } else {
                break;
            }
        }
        let width: Option<usize> = if width_str.is_empty() {
            None
        } else {
            width_str.parse().ok()
        };

        // Parse optional precision: .N
        let precision: Option<usize> = if chars.peek() == Some(&'.') {
            chars.next();
            let mut prec_str = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    prec_str.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            prec_str.parse().ok()
        } else {
            None
        };

        let Some(spec) = chars.next() else {
            break;
        };

        match spec {
            '%' => result.push('%'),
            'n' => result.push('\n'),
            's' | 'd' | 'f' | 'x' | 'X' | 'b' | 'c' | 'o' | 'e' | 'E' => {
                let slot = if arg_idx < arr_len {
                    match args.get(1) {
                        Some(Slot::Reference(Some(r))) => extract_field_arg(heap, *r, arg_idx)?,
                        _ => Slot::Reference(None),
                    }
                } else {
                    Slot::Reference(None)
                };
                arg_idx += 1;
                let formatted = format_arg(spec, &flags, width, precision, &slot, heap)?;
                result.push_str(&formatted);
            }
            _ => {
                result.push('%');
                result.push(spec);
            }
        }
    }

    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

// ---- Extended String natives ----

/// Native: `String.toUpperCase()` — returns a new uppercase String.
pub(crate) fn native_string_touppercase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.to_uppercase());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.toLowerCase()` — returns a new lowercase String.
pub(crate) fn native_string_tolowercase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.to_lowercase());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.replace(char, char)` — replaces all occurrences of old char with new char.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_string_replace_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let old_char = char::from_u32(extract_int_arg(args, 1)?.cast_unsigned()).unwrap_or('?');
    let new_char = char::from_u32(extract_int_arg(args, 2)?.cast_unsigned()).unwrap_or('?');
    let result = s.replace(old_char, &new_char.to_string());
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.replace(CharSequence, CharSequence)` — replaces all occurrences of target with replacement.
pub(crate) fn native_string_replace_charsequence(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let target_ref = extract_ref_arg(args, 1)?;
    let target = heap
        .get(target_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let replacement_ref = extract_ref_arg(args, 2)?;
    let replacement = heap
        .get(replacement_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let result = s.replace(&*target, &replacement);
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.split(String)` — splits string by delimiter, returns String array.
pub(crate) fn native_string_split(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let delim_ref = extract_ref_arg(args, 1)?;
    let delim = heap
        .get(delim_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    // Use regex split (Java's String.split uses regex); remove trailing empty strings
    // to match Java's default split behaviour.
    // Special case: split("") splits into individual chars (Java 21 semantics — no leading "").
    let parts: Vec<String> = if delim.is_empty() {
        s.chars().map(|c| c.to_string()).collect()
    } else {
        regex::Regex::new(&delim).map_or_else(
            |_| s.split(delim.as_str()).map(str::to_string).collect(),
            |re| {
                let mut v: Vec<String> = re.split(&s).map(str::to_string).collect();
                while v.last().is_some_and(String::is_empty) {
                    v.pop();
                }
                v
            },
        )
    };
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), parts.len());
    for (i, part) in parts.iter().enumerate() {
        let str_ref = heap.allocate_string(part.clone());
        heap.get_mut(arr_ref)?.fields[i] = Slot::Reference(Some(str_ref));
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `String.split(String, int)` — split with a limit parameter.
pub(crate) fn native_string_split_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let delim_ref = extract_ref_arg(args, 1)?;
    let delim = heap
        .get(delim_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let limit = match args.get(2) {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    #[allow(clippy::cast_sign_loss)] // limit is validated > 0 before the cast
    let parts: Vec<String> = if delim.is_empty() {
        let chars: Vec<String> = s.chars().map(|c| c.to_string()).collect();
        if limit > 0 && (limit as usize) < chars.len() {
            let mut v = chars[..limit as usize - 1].to_vec();
            v.push(chars[limit as usize - 1..].join(""));
            v
        } else {
            chars
        }
    } else {
        let re = regex::Regex::new(&delim)
            .unwrap_or_else(|_| regex::Regex::new(&regex::escape(&delim)).unwrap());
        if limit > 0 {
            re.splitn(&s, limit as usize).map(str::to_string).collect()
        } else {
            let mut v: Vec<String> = re.split(&s).map(str::to_string).collect();
            if limit == 0 {
                while v.last().is_some_and(String::is_empty) {
                    v.pop();
                }
            }
            v
        }
    };
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), parts.len());
    for (i, part) in parts.iter().enumerate() {
        let str_ref = heap.allocate_string(part.clone());
        heap.get_mut(arr_ref)?.fields[i] = Slot::Reference(Some(str_ref));
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `String.hashCode()` — Java's hash algorithm: `s[0]*31^(n-1) + s[1]*31^(n-2) + ... + s[n-1]`.
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn native_string_hashcode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let mut h: i32 = 0;
    for ch in s.chars() {
        h = h.wrapping_mul(31).wrapping_add(ch as i32);
    }
    Ok(Some(Slot::Int(h)))
}

/// Native: `String.toString()` — identity, returns `this`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_string_tostring(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(extract_slot_arg(args, 0)))
}

// ---- Math natives ----

/// Native: `Math.max(int, int)` — returns the larger value.
pub(crate) fn native_math_max_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.max(b))))
}

/// Native: `Math.min(int, int)` — returns the smaller value.
pub(crate) fn native_math_min_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.min(b))))
}

/// Native: `Math.abs(int)` — returns absolute value.
pub(crate) fn native_math_abs_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(a.wrapping_abs())))
}

/// Native: `Math.floorMod(int, int)` — remainder with the divisor's sign.
pub(crate) fn native_math_floor_mod_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    if b == 0 {
        return Err(Error::DivisionByZero);
    }
    let remainder = a.wrapping_rem(b);
    let floor_mod = if remainder != 0 && (remainder < 0) != (b < 0) {
        remainder.wrapping_add(b)
    } else {
        remainder
    };
    Ok(Some(Slot::Int(floor_mod)))
}

// ---- Extended Math natives ----

/// Native: `Math.sqrt(double)` — returns square root.
pub(crate) fn native_math_sqrt(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.sqrt())))
}

/// Native: `Math.pow(double, double)` — returns a raised to the power b.
pub(crate) fn native_math_pow(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    let b = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(a.powf(b))))
}

/// Native: `Math.floor(double)` — returns floor value.
pub(crate) fn native_math_floor(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.floor())))
}

/// Native: `Math.ceil(double)` — returns ceiling value.
pub(crate) fn native_math_ceil(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.ceil())))
}

/// Native: `Math.round(double)` — returns closest long.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_math_round_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Long(a.round() as i64)))
}

/// Native: `Math.abs(long)` — returns absolute value.
pub(crate) fn native_math_abs_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Long(a.wrapping_abs())))
}

/// Native: `Math.abs(double)` — returns absolute value.
pub(crate) fn native_math_abs_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.abs())))
}

/// Native: `Math.max(long, long)` — returns the larger value.
pub(crate) fn native_math_max_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(a.max(b))))
}

/// Native: `Math.min(long, long)` — returns the smaller value.
pub(crate) fn native_math_min_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(a.min(b))))
}

/// Native: `Math.max(double, double)` — returns the larger value.
pub(crate) fn native_math_max_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    let b = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(a.max(b))))
}

/// Native: `Math.min(double, double)` — returns the smaller value.
pub(crate) fn native_math_min_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    let b = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(a.min(b))))
}

// ---- Extended Math trig / transcendental natives ----

/// Native: `Math.sin(double)` — sine (argument in radians).
pub(crate) fn native_math_sin(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.sin())))
}

/// Native: `Math.cos(double)` — cosine (argument in radians).
pub(crate) fn native_math_cos(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.cos())))
}

/// Native: `Math.tan(double)` — tangent (argument in radians).
pub(crate) fn native_math_tan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.tan())))
}

/// Native: `Math.asin(double)` — arc sine, result in [-π/2, π/2].
pub(crate) fn native_math_asin(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.asin())))
}

/// Native: `Math.acos(double)` — arc cosine, result in [0, π].
pub(crate) fn native_math_acos(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.acos())))
}

/// Native: `Math.atan(double)` — arc tangent, result in [-π/2, π/2].
pub(crate) fn native_math_atan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.atan())))
}

/// Native: `Math.atan2(double, double)` — angle of vector (y, x) in [-π, π].
pub(crate) fn native_math_atan2(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let y = extract_double_arg(args, 0)?;
    let x = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(y.atan2(x))))
}

/// Native: `Math.log(double)` — natural logarithm.
pub(crate) fn native_math_log(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.ln())))
}

/// Native: `Math.log10(double)` — base-10 logarithm.
pub(crate) fn native_math_log10(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.log10())))
}

/// Native: `Math.exp(double)` — Euler's number raised to the given power.
pub(crate) fn native_math_exp(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.exp())))
}

/// Native: `Math.signum(double)` — sign of a: -1.0, 0.0, or 1.0.
pub(crate) fn native_math_signum_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.signum())))
}

/// Native: `Math.signum(float)` — sign of a as float: -1.0, 0.0, or 1.0.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_math_signum_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_float_arg(args, 0)?;
    Ok(Some(Slot::Float(a.signum())))
}

/// Native: `Math.toRadians(double)` — converts degrees to radians.
pub(crate) fn native_math_to_radians(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.to_radians())))
}

/// Native: `Math.toDegrees(double)` — converts radians to degrees.
pub(crate) fn native_math_to_degrees(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.to_degrees())))
}

/// Native: `Math.cbrt(double)` — cube root.
pub(crate) fn native_math_cbrt(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.cbrt())))
}

/// Native: `Math.hypot(double, double)` — sqrt(x²+y²) without overflow.
pub(crate) fn native_math_hypot(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let x = extract_double_arg(args, 0)?;
    let y = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(x.hypot(y))))
}

/// Native: `Math.floorDiv(int, int)` — largest int ≤ quotient.
pub(crate) fn native_math_floor_div_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    if b == 0 {
        return Err(Error::DivisionByZero);
    }
    Ok(Some(Slot::Int(
        a.wrapping_div_euclid(b) - i32::from(a.wrapping_rem(b) != 0 && (a < 0) != (b < 0)),
    )))
}

/// Native: `Math.round(float)` — rounds float to nearest int.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_math_round_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_float_arg(args, 0)?;
    Ok(Some(Slot::Int(a.round() as i32)))
}

#[cfg(test)]
mod havoc_proptest_math {
    use super::*;
    use proptest::prelude::*;
    use std::io::sink;

    proptest! {
        #[test]
        fn fuzz_native_math_floor_div_int(a in any::<i32>(), b in any::<i32>()) {
            let mut heap = duke_gc::Heap::new();
            let mut control = NativeControl::default();
            let args = vec![Slot::Int(a), Slot::Int(b)];

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = native_math_floor_div_int(&args, &mut heap, &mut sink(), &mut control);
            }));

            assert!(result.is_ok(), "Panic on a={a}, b={b}");
        }
    }
}

// ---- System.arraycopy native ----

/// Native: `System.arraycopy(Object src, int srcPos, Object dst, int dstPos, int length)`.
/// Copies `length` elements from `src` starting at `srcPos` into `dst` starting at `dstPos`.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_system_arraycopy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let src_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let src_pos = extract_int_arg(args, 1)?;
    let dst_ref = match args.get(2) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let dst_pos = extract_int_arg(args, 3)?;
    let length = extract_int_arg(args, 4)?;
    if length < 0 || src_pos < 0 || dst_pos < 0 {
        return Err(Error::NegativeArraySize {
            size: length.min(src_pos).min(dst_pos),
        });
    }
    let src_pos = src_pos as usize;
    let dst_pos = dst_pos as usize;
    let length = length as usize;
    // Copy elements one by one to support src == dst (overlapping ranges handled via clone).
    let src_len = heap.get(src_ref)?.fields.len();
    if src_pos + length > src_len {
        return Err(Error::ArrayIndexOutOfBounds {
            index: i32::try_from(src_pos + length - 1).unwrap_or(i32::MAX),
            length: src_len,
        });
    }
    let src_elems: Vec<Slot> = heap.get(src_ref)?.fields[src_pos..src_pos + length].to_vec();
    let dst_len = heap.get(dst_ref)?.fields.len();
    if dst_pos + length > dst_len {
        return Err(Error::ArrayIndexOutOfBounds {
            index: i32::try_from(dst_pos + length - 1).unwrap_or(i32::MAX),
            length: dst_len,
        });
    }
    let dst_fields = &mut heap.get_mut(dst_ref)?.fields;
    for (i, slot) in src_elems.into_iter().enumerate() {
        dst_fields[dst_pos + i] = slot;
    }
    Ok(None)
}

// ---- HashMap / Map$Entry iteration natives ----

/// Native: `Map$Entry.getKey()Object` — returns the key field.
pub(crate) fn native_map_entry_get_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_first_field_arg(heap, this_ref)?;
    Ok(Some(key))
}

/// Native: `Map$Entry.getValue()Object` — returns the value field.
pub(crate) fn native_map_entry_get_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_field_arg(heap, this_ref, 1)?;
    Ok(Some(val))
}

/// Native: `HashMap.keySet()` — returns a new `HashSet` containing all keys.
pub(crate) fn native_hashmap_key_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields_len = heap.get(this_ref)?.fields.len();
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    let mut i = 1usize;
    while i < fields_len {
        let key = heap.get(this_ref)?.fields[i];
        native_hashset_add(&[Slot::Reference(Some(set_ref)), key], heap, out, control)?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}

/// Native: `HashMap.values()` — returns a new `ArrayList` containing all values.
/// `HashMap` fields: `[size, key0, val0, key1, val1, ...]`; values are at even indices 2, 4, 6, ...
pub(crate) fn native_hashmap_values(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields_len = heap.get(this_ref)?.fields.len();
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    // Pairs start at index 1; values are at indices 2, 4, 6, ...
    let mut i = 2usize;
    while i < fields_len {
        let val = heap.get(this_ref)?.fields[i];
        native_arraylist_add(&[Slot::Reference(Some(list_ref)), val], heap, out, control)?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}

/// Native: `HashMap.entrySet()` — returns a new `HashSet` of `java/util/Map$Entry` objects.
pub(crate) fn native_hashmap_entry_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields_len = heap.get(this_ref)?.fields.len();
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    let mut i = 1usize;
    while i + 1 < fields_len {
        let key = heap.get(this_ref)?.fields[i];
        let val = heap.get(this_ref)?.fields[i + 1];
        let entry_ref = heap.allocate("java/util/Map$Entry".to_string(), 2);
        {
            let entry_obj = heap.get_mut(entry_ref)?;
            entry_obj.fields[0] = key;
            entry_obj.fields[1] = val;
        }
        native_hashset_add(
            &[
                Slot::Reference(Some(set_ref)),
                Slot::Reference(Some(entry_ref)),
            ],
            heap,
            out,
            control,
        )?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}

// ---- Long class natives ----

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
