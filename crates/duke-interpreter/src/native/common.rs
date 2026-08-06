
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




fn path_from_string_slot(
    args: &[Slot],
    idx: usize,
    heap: &duke_gc::Heap,
) -> Result<std::path::PathBuf> {
    let path_ref = extract_ref_arg(args, idx)?;
    let path = string_value_from_ref(heap, path_ref)?;
    Ok(std::path::PathBuf::from(path))
}

fn string_value_from_ref(heap: &duke_gc::Heap, string_ref: u64) -> Result<String> {
    read_string_bytes(heap, string_ref)
}

/// Decodes the real `java/lang/String` heap layout — slot 0 (`value:[B`) and
/// slot 1 (`coder:B`) — back into a Rust `String`, inverting the encoding that
/// [`duke_gc::Heap::set_string_layout`] applies: Latin-1 when `coder == 0`,
/// little-endian UTF-16 when `coder == 1`. Java `byte`s are signed, so the
/// backing-array octets are recovered through [`byte_from_slot`] (via
/// [`full_byte_array`]).
///
/// This is the slot-0 "source of truth" counterpart to reading the
/// `string_value` side-channel. A null `value` array (slot 0 is not a live `[B`
/// reference) yields `NullPointerException`, matching how the previous
/// [`string_value_from_ref`] treated a missing `string_value`.
///
/// In debug builds the decoded result is cross-checked against the
/// `string_value` side-channel (when present); a mismatch flags a String mint
/// path that populated the side-channel but not slot 0.
///
/// # Errors
/// Returns `Error::NullPointerException` if `string_ref` is not a live object
/// or its slot-0 `value` array is null.
fn read_string_bytes(heap: &duke_gc::Heap, string_ref: u64) -> Result<String> {
    let obj = heap.get(string_ref)?;
    let Some(Slot::Reference(Some(bytes_ref))) = obj.fields.first().copied() else {
        return Err(Error::NullPointerException);
    };
    let coder = match obj.fields.get(1) {
        Some(Slot::Int(c)) => *c,
        _ => 0,
    };
    let bytes = full_byte_array(heap, bytes_ref)?;
    let decoded: String = if coder == 1 {
        decode_utf16_bytes(&bytes, Utf16Endian::Little)
    } else {
        bytes.iter().map(|&b| char::from(b)).collect()
    };

    #[cfg(debug_assertions)]
    if let Some(expected) = obj.string_value.as_deref() {
        debug_assert_eq!(
            decoded.as_str(),
            expected,
            "read_string_bytes slot-0 decode disagreed with the string_value side-channel"
        );
    }

    Ok(decoded)
}

/// Reads the character content of a `CharSequence`-polymorphic heap reference
/// whose object may be either a real-layout `java/lang/String` or a non-String
/// character backing (`StringBuilder`/`StringBuffer` and similar).
///
/// A real-layout `java/lang/String` is decoded from slot 0 (`value:[B`) via
/// [`read_string_bytes`] — never from the `string_value` side-channel — so that
/// production `String` objects need not carry the redundant cache. Any other
/// object falls back to its own `string_value` char buffer. Returns `None` when
/// the object carries no readable characters (a non-String with no
/// `string_value`), letting callers apply their own `"null"`/default fallback.
fn charsequence_chars(heap: &duke_gc::Heap, string_ref: u64) -> Result<Option<String>> {
    let obj = heap.get(string_ref)?;
    if obj.class_name == "java/lang/String"
        && let Ok(decoded) = read_string_bytes(heap, string_ref)
    {
        return Ok(Some(decoded));
    }
    Ok(obj.string_value.clone())
}

/// `Object`/`CharSequence`-polymorphic `toString` that reads a real-layout
/// `java/lang/String` from slot 0 (via [`read_string_bytes`]) rather than the
/// `string_value` side-channel, delegating every other object (`StringBuilder`,
/// boxed primitives, opaque refs) to [`heap_object_to_string`]. This keeps
/// `String.valueOf(Object)`, `Object.toString`, `println(Object)`, `%s`
/// formatting, etc. off a `String` object's `string_value`.
fn heap_object_to_string_ref(heap: &duke_gc::Heap, obj_ref: u64) -> Result<String> {
    let obj = heap.get(obj_ref)?;
    if obj.class_name == "java/lang/String" {
        return Ok(read_string_bytes(heap, obj_ref).unwrap_or_default());
    }
    Ok(heap_object_to_string(obj, obj_ref))
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
        Slot::Reference(Some(obj_ref)) => heap_object_to_string_ref(heap, obj_ref),
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

/// Decodes UTF-16 bytes into a Rust String.
///
/// ⚡ Bolt: Removed the intermediate `Vec<u16>` allocation by passing a lazy iterator
/// (`chunks.by_ref().map(...)`) directly to `char::decode_utf16`. This avoids an $O(N)$
/// heap allocation for every UTF-16 decoding operation.
fn decode_utf16_bytes(bytes: &[u8], endian: Utf16Endian) -> String {
    let mut chunks = bytes.chunks_exact(2);
    let iter = chunks.by_ref().map(|chunk| {
        let pair = [chunk[0], chunk[1]];
        match endian {
            Utf16Endian::Big => u16::from_be_bytes(pair),
            Utf16Endian::Little => u16::from_le_bytes(pair),
        }
    });

    let mut decoded: String = char::decode_utf16(iter)
        .map(|item| item.unwrap_or(REPLACEMENT_CHAR))
        .collect();
    if !chunks.remainder().is_empty() {
        decoded.push(REPLACEMENT_CHAR);
    }
    decoded
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
    let value = string_value_from_ref(heap, this_ref).unwrap_or_default();
    Ok(encode_string_with_charset(&value, charset))
}

/// Populate a freshly-constructed `java/lang/String` receiver on a `<init>`
/// path. Sets the authoritative `string_value` side-channel AND mints the real
/// 4-slot layout (`value:[B`, `coder:B`) via [`Heap::set_string_layout`], so the
/// `new java/lang/String` + `<init>` mint path is coherent with the
/// `allocate_string` mint path (both leave slot0/slot1 populated). An empty
/// `value` still gets a valid zero-length `[B` in slot0.
fn store_string_init_value(heap: &mut duke_gc::Heap, this_ref: u64, value: String) -> Result<()> {
    heap.set_string_layout(this_ref, &value);
    heap.get_mut(this_ref)?.string_value = Some(value);
    Ok(())
}

fn init_string_from_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    bytes: &[u8],
    charset: StandardCharset,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let decoded = decode_string_with_charset(bytes, charset);
    store_string_init_value(heap, this_ref, decoded)?;
    Ok(None)
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

/// Native: `String.<init>([BB)V` — the package-private compact-strings
/// constructor `String(byte[] value, byte coder)`. The real JDK ctor performs no
/// copy or validation: `this.value = value; this.coder = coder;`. The incoming
/// `value` bytes are already in the JDK compact-strings encoding, which is the
/// same convention Duke's 4-slot layout uses (see
/// [`duke_gc::Heap::set_string_layout`]): Latin-1 when `coder == 0`,
/// little-endian UTF-16 when `coder == 1`. We decode `(value, coder)` into a Rust
/// `String` — inverting that encoding exactly as [`read_string_bytes`] does — and
/// mint the receiver through [`store_string_init_value`], which re-establishes
/// slot 0 (`value:[B`), slot 1 (`coder:B`), and the `string_value` cache
/// coherently. Because JDK's compact-strings encoding matches ours, the decode →
/// re-encode round-trips: `value`/`coder` are preserved.
///
/// Real-JDK boot (JDK 21 jimage) reaches this ctor in the `String` encode path;
/// without it the chain fails as `MethodNotFound java/lang/String.<init>([BB)V`.
pub(crate) fn native_string_init_bytes_coder(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let bytes_ref = extract_ref_arg(args, 1)?;
    let coder = extract_int_arg(args, 2)?;
    let bytes = full_byte_array(heap, bytes_ref)?;
    let decoded: String = if coder == 1 {
        decode_utf16_bytes(&bytes, Utf16Endian::Little)
    } else {
        bytes.iter().map(|&b| char::from(b)).collect()
    };
    store_string_init_value(heap, this_ref, decoded)?;
    Ok(None)
}

/// Native: `String.<init>([III)V` — construct a `String` from a range of a
/// code-point `int[]` (`new String(int[] codePoints, int offset, int count)`).
/// Used by `StringUtils.capitalize`, which rebuilds a string from its code
/// points after titlecasing the first.
pub(crate) fn native_string_init_code_points(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let array_ref = extract_ref_arg(args, 1)?;
    let offset = extract_int_arg(args, 2)?;
    let count = extract_int_arg(args, 3)?;
    if offset < 0 || count < 0 {
        return Err(index_out_of_bounds_error());
    }
    let start = usize::try_from(offset).map_err(|_| index_out_of_bounds_error())?;
    let n = usize::try_from(count).map_err(|_| index_out_of_bounds_error())?;
    let end = start.checked_add(n).ok_or_else(index_out_of_bounds_error)?;
    let fields = &heap.get(array_ref)?.fields;
    if end > fields.len() {
        return Err(index_out_of_bounds_error());
    }
    let mut decoded = String::with_capacity(n);
    for slot in &fields[start..end] {
        let cp = match slot {
            Slot::Int(v) => *v,
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "Int (code point)",
                    got: "other",
                });
            }
        };
        decoded.push(char::from_u32(cp.cast_unsigned()).unwrap_or(REPLACEMENT_CHAR));
    }
    store_string_init_value(heap, this_ref, decoded)?;
    Ok(None)
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





fn file_stream_id_from_this(args: &[Slot], heap: &duke_gc::Heap) -> Result<i32> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        }),
    }
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

    let mut buf = vec![0u8; len];
    let read_res = heap.read_host_file_bytes(file_id, &mut buf)?;
    if read_res < 0 {
        return Ok(Some(Slot::Int(-1)));
    }

    let count = usize::try_from(read_res).unwrap_or(0);
    for (idx, &byte) in buf.iter().enumerate().take(count) {
        heap.get_mut(array_ref)?.fields[idx] = Slot::Int(i32::from(byte));
    }

    Ok(Some(Slot::Int(read_res)))
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









// ── ZIP / JAR natives ──────────────────────────────────────────────────





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











/// Two `char` values are case-insensitively equal per the Java SE 21
/// `String.equalsIgnoreCase` contract: they match if equal directly, or if
/// `Character.toUpperCase(c1) == Character.toUpperCase(c2)`, or if
/// `Character.toLowerCase(c1) == Character.toLowerCase(c2)`. The two-step fold
/// catches code points the naive ASCII rule misses (e.g. the KELVIN SIGN
/// `U+212A` lower-cases to `'k'`).
fn chars_equal_ignore_case(c1: char, c2: char) -> bool {
    c1 == c2 || c1.to_uppercase().eq(c2.to_uppercase()) || c1.to_lowercase().eq(c2.to_lowercase())
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
        Slot::Reference(Some(r)) => Ok(Some(string_value_from_ref(heap, r)?)),
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









fn stack_trace_element_field(args: &[Slot], heap: &duke_gc::Heap, index: usize) -> Result<Slot> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(heap
        .get(this_ref)?
        .fields
        .get(index)
        .copied()
        .unwrap_or(Slot::Reference(None)))
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



// ---- Optional<T> natives ----









// ---- ArrayList / HashMap bulk operations ----




// ---- LinkedList natives (field layout identical to ArrayList: fields[0]=size, fields[1..]=elements) ----
















// ---- HashMap.forEach callback ----



// ---- TreeMap natives (field layout: fields[0]=Int(size), fields[1,2]=k0/v0 sorted by key) ----
// Keys are stored sorted in ascending order for O(n) insert / O(1) first&last.

/// Compare two `TreeMap` keys by natural ordering, supporting String, Integer, Long, and Double keys.
fn compare_treemap_keys(a: Slot, b: Slot, heap: &duke_gc::Heap) -> std::cmp::Ordering {
    let key_ord = |s: Slot| -> Option<KeyOrd> {
        if let Slot::Reference(Some(r)) = s
            && let Ok(obj) = heap.get(r)
        {
            if let Some(sv) = charsequence_chars(heap, r).ok().flatten() {
                return Some(KeyOrd::Str(sv));
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












// ---- Stack natives (LIFO backed by ArrayList: push=add, pop=removeLast, peek=peekLast) ----







// ---- TreeSet natives (sorted unique elements, backed by sorted Vec<Slot>) ----
// fields[0] = Int(size), fields[1..] = unique elements in sorted String order


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
                _ => charsequence_chars(heap, r).ok().flatten().map(TreeSortKey::Str),
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








// ---- Collections.min / max / shuffle ----






// ---- Stream natives ----
// duke/util/Stream: fields[0]=Int(size), fields[1..]=element refs

















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









// ---------------------------------------------------------------------------
// Stream.limit / Stream.skip / Stream.flatMap
// ---------------------------------------------------------------------------




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




// ---- ArrayDeque natives ----
// fields[0]=Int(size), fields[1..size]=elements (front at index 1)










// ---- PriorityQueue natives ----
// fields[0]=Int(size), fields[1..size]=elements; maintained as a min-heap (String key order for Strings, value for boxed ints).








// ---- Comparator natives ----












// ---------------------------------------------------------------------------
// Reflection natives
// ---------------------------------------------------------------------------






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

// Retained for existing (test) callers; the native call sites that used the
// one-shot post-invoke patch have migrated to `NativeRootScope` handles.
#[allow(dead_code)]
fn patch_forwarded_slot_if_needed(heap: &duke_gc::Heap, slot: &mut Slot) {
    if heap.has_pending_forwards() {
        heap.apply_forward(slot);
    }
}

#[allow(dead_code)]
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


























// ---------------------------------------------------------------------------
// println overloads (long, float, double, boolean, char, object)
// ---------------------------------------------------------------------------






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


// ---------------------------------------------------------------------------
// print overloads (long, float, double, boolean, char, object)
// ---------------------------------------------------------------------------







// ---------------------------------------------------------------------------
// System.exit
// ---------------------------------------------------------------------------


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







fn system_time_to_epoch_millis(now: std::time::SystemTime) -> i64 {
    now.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}


static NANO_TIME_ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn monotonic_nano_time_now() -> i64 {
    let origin = NANO_TIME_ORIGIN.get_or_init(std::time::Instant::now);
    i64::try_from(origin.elapsed().as_nanos()).unwrap_or(i64::MAX)
}


const THREAD_TARGET_SLOT: usize = 0;
const THREAD_ID_SLOT: usize = 1;
const THREAD_INTERRUPTED_SLOT: usize = 2;
const THREAD_HOST_KEY_SLOT: usize = 3;
const THREAD_CONTEXT_CLASS_LOADER_SLOT: usize = 4;

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







// ---- Integer natives ----















// ---- Integer bit / arithmetic operations ----













// ---- Long bit / arithmetic operations ----











// ---- java.util.Objects natives ----









// ---- Collections utilities ----












// ---------------------------------------------------------------------------
// Phase 43: Collectors.toSet/toMap, Stream.mapToInt/min/max, IntStream.reduce,
//           Arrays.sort(Object[]), String(char[])/valueOf(char[])
// ---------------------------------------------------------------------------








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






// ---- String.valueOf overloads ----







// ---- String.concat ----


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
                's' => heap_object_to_string_ref(heap, *r)?,
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


// ---- Extended String natives ----









// ---- Math natives ----





// ---- Extended Math natives ----












// ---- Extended Math trig / transcendental natives ----



















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


// ---- HashMap / Map$Entry iteration natives ----






// ---- Long class natives ----















// ---- Double class natives ----




// ---- Float class native ----











// ---- Boolean class native ----





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
    // Propagate a genuine invalid-reference error before defaulting; a live
    // String with a null `value` slot (no content) decodes to the empty string.
    heap.get(str_ref)?;
    Ok(read_string_bytes(heap, str_ref).unwrap_or_default())
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
            out.push_str(&heap_object_to_string_ref(heap, *r)?);
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
            // Shared trailing `frame.push(v1)?` across the JVMS forms is kept inline
            // for spec fidelity rather than hoisted out of the branches.
            #[allow(clippy::branches_sharing_code)]
            Instruction::Dup2 => {
                // JVMS §6.5 dup2. In Duke's model a category-2 value (long/double)
                // is a single `Slot`, so Form 2 duplicates one slot, not two.
                let v1 = frame.pop()?;
                if matches!(v1, Slot::Long(_) | Slot::Double(_)) {
                    // Form 2: single category-2 value.
                    frame.push(v1)?;
                    frame.push(v1)?;
                } else {
                    // Form 1: two category-1 values.
                    let v2 = frame.pop()?;
                    frame.push(v2)?;
                    frame.push(v1)?;
                    frame.push(v2)?;
                    frame.push(v1)?;
                }
            }
            #[allow(clippy::branches_sharing_code)]
            Instruction::Dup2X1 => {
                // JVMS §6.5 dup2_x1.
                let v1 = frame.pop()?;
                if matches!(v1, Slot::Long(_) | Slot::Double(_)) {
                    // Form 2: value1 category 2, value2 category 1.
                    let v2 = frame.pop()?;
                    frame.push(v1)?;
                    frame.push(v2)?;
                    frame.push(v1)?;
                } else {
                    // Form 1: three category-1 values.
                    let v2 = frame.pop()?;
                    let v3 = frame.pop()?;
                    frame.push(v2)?;
                    frame.push(v1)?;
                    frame.push(v3)?;
                    frame.push(v2)?;
                    frame.push(v1)?;
                }
            }
            #[allow(clippy::branches_sharing_code)]
            Instruction::Dup2X2 => {
                // JVMS §6.5 dup2_x2, all four forms in Duke's single-slot cat-2 model.
                let v1 = frame.pop()?;
                if matches!(v1, Slot::Long(_) | Slot::Double(_)) {
                    let v2 = frame.pop()?;
                    if matches!(v2, Slot::Long(_) | Slot::Double(_)) {
                        // Form 4: value1, value2 both category 2.
                        frame.push(v1)?;
                        frame.push(v2)?;
                        frame.push(v1)?;
                    } else {
                        // Form 2: value1 category 2; value2, value3 category 1.
                        let v3 = frame.pop()?;
                        frame.push(v1)?;
                        frame.push(v3)?;
                        frame.push(v2)?;
                        frame.push(v1)?;
                    }
                } else {
                    let v2 = frame.pop()?;
                    let v3 = frame.pop()?;
                    if matches!(v3, Slot::Long(_) | Slot::Double(_)) {
                        // Form 3: value1, value2 category 1; value3 category 2.
                        frame.push(v2)?;
                        frame.push(v1)?;
                        frame.push(v3)?;
                        frame.push(v2)?;
                        frame.push(v1)?;
                    } else {
                        // Form 1: all four category 1.
                        let v4 = frame.pop()?;
                        frame.push(v2)?;
                        frame.push(v1)?;
                        frame.push(v4)?;
                        frame.push(v3)?;
                        frame.push(v2)?;
                        frame.push(v1)?;
                    }
                }
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
    // JVMS 5.5: a class left in the "erroneous" state by a previously-failed
    // <clinit> must not be re-initialised — any attempt throws a fresh,
    // catchable NoClassDefFoundError naming the class.
    if registry.is_erroneous(class_name) {
        return Err(throw_no_class_def_found_error(class_name));
    }
    if registry.is_initialized(class_name) {
        return Ok(());
    }
    // Mark as initialized BEFORE running clinit to prevent infinite recursion.
    // (Single-threaded init model: we do not implement the full JVMS 5.5
    // per-thread "in progress" state machine — the recursion guard is a plain
    // membership flag, and on failure we move the class to the erroneous set.)
    registry.mark_initialized(class_name);

    // JVMS §5.5 step 7: a class's direct superclass must be initialized before
    // the class itself. The `mark_initialized` guard above makes this
    // cycle-safe; an erroneous/failed superclass propagates via `?`.
    if let Some(super_name) = registry
        .get(class_name)
        .ok()
        .and_then(|c| c.super_class.clone())
    {
        ensure_initialized(registry, loader, heap, stdout, &super_name, class_name)?;
    }

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
        let clinit_result = execute_class(
            registry,
            init_loader,
            heap,
            stdout,
            class_name,
            "<clinit>",
            "()V",
            &[],
        );
        if let Err(err) = clinit_result {
            // JVMS 5.5: the initialiser completed abruptly. Move the class to the
            // erroneous state and surface a *catchable* Java throwable so the
            // triggering opcode routes it through the caller's exception table.
            registry.mark_erroneous(class_name);
            return Err(map_clinit_failure(registry, loader, heap, err));
        }
        #[cfg(feature = "telemetry")]
        registry.telemetry.class_init_dag.record(
            class_name,
            _triggered_by,
            _clinit_start.elapsed().as_nanos() as u64,
        );
    }
    Ok(())
}

/// Build a catchable `NoClassDefFoundError` [`Error::JavaException`] whose detail
/// message is the internal (slash-form) class name — matching the real JVM.
///
/// The returned error is *catchable*: opcode handlers re-materialise it through
/// `throw_java!`, and the pending message is consumed at materialisation time to
/// populate the throwable's detail message.
fn throw_no_class_def_found_error(internal_class_name: &str) -> Error {
    push_pending_java_exception_message(
        "java/lang/NoClassDefFoundError",
        internal_class_name.to_string(),
    );
    Error::JavaException {
        class_name: "java/lang/NoClassDefFoundError".to_string(),
    }
}

/// Translate a `<clinit>` failure into the correct catchable throwable per
/// JVMS 5.5:
///
/// * If the thrown throwable is (a subclass of) `java/lang/Error`, it propagates
///   unwrapped — this is the commons-logging ladder case where an inner
///   `NoClassDefFoundError` must reach a `catch (LinkageError)`.
/// * Otherwise the throwable is wrapped in a freshly materialised
///   `java/lang/ExceptionInInitializerError` whose cause is the original object.
///
/// Non-`JavaException` errors (genuine VM-internal failures) are returned as-is.
fn map_clinit_failure(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    err: Error,
) -> Error {
    let Error::JavaException { class_name } = err else {
        return err;
    };
    // Errors propagate unwrapped.
    if is_assignable_from(registry, loader, &class_name, "java/lang/Error", None) {
        return Error::JavaException { class_name };
    }
    // Wrap Exceptions in ExceptionInInitializerError(cause = original).
    let cause_ref = take_uncaught_java_exception_ref(&class_name);
    let eiie_class = "java/lang/ExceptionInInitializerError";
    let Ok(eiie_ref) = materialize_java_exception_object(registry, loader, heap, eiie_class) else {
        // If we somehow cannot materialise the wrapper, fall back to the raw
        // exception rather than masking the failure.
        return Error::JavaException { class_name };
    };
    if let Some(cause) = cause_ref
        && set_object_field(
            heap,
            eiie_ref,
            THROWABLE_CAUSE_FIELD,
            Slot::Reference(Some(cause)),
        )
        .is_ok()
    {
        heap.remember_reference_write(eiie_ref, Slot::Reference(Some(cause)));
    }
    record_uncaught_java_exception_ref(eiie_class, eiie_ref);
    Error::JavaException {
        class_name: eiie_class.to_string(),
    }
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
    class_name: std::sync::Arc<str>,
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
    current_class: std::sync::Arc<str>,
    method_idx: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: std::sync::Arc<[(usize, Instruction)]>,
    frame: Frame,
    call_stack: Vec<CallFrame>,
    frame_pool: FramePool,
    /// Static dispatch cache for `invokestatic` and `invokespecial`.
    /// Key: (`caller_class`, `cp_idx`) → pre-resolved method data.
    dispatch_cache: HashMap<std::sync::Arc<str>, HashMap<u16, CachedDispatch>>,
    /// Polymorphic inline cache for `invokevirtual`.
    /// Key: (`caller_class`, `cp_idx`, `receiver_runtime_class`) → pre-resolved method data.
    vtable_cache: HashMap<std::sync::Arc<str>, HashMap<u16, HashMap<String, CachedDispatch>>>,
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
        let current_class = registry.intern_key(class_name);
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
    current_class: &mut std::sync::Arc<str>,
    call_stack: &mut Vec<CallFrame>,
    callee_class: std::sync::Arc<str>,
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
        instructions: std::sync::Arc::clone(instructions),
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
    // Resolve the method on the exact class first (fast path, unchanged
    // behaviour). If the class does not declare it, the method is inherited, so
    // walk the superclass chain exactly like the main interpreter dispatch does.
    // This fallback only fires on inputs that previously raised `MethodNotFound`
    // (a directed `ops.invoke` for an inherited method — e.g. a native calling a
    // Comparator's `compare` that lives on a superclass), so it cannot alter any
    // call that already resolved.
    let own_idx = {
        let ctx = registry.get(&class_name)?;
        ctx.methods
            .iter()
            .position(|m| m.name == method_name && m.descriptor == descriptor)
    };
    let (dispatch_class, entry_idx) = match own_idx {
        Some(idx) => (class_name.clone(), idx),
        None => resolve_method_in_hierarchy(registry, loader, &class_name, method_name, descriptor)
            .ok_or_else(|| Error::MethodNotFound {
                name: format!("{class_name}.{method_name}"),
                descriptor: descriptor.to_string(),
            })?,
    };

    let class_loader = registry.class_loader(&dispatch_class).cloned();
    let init_loader = class_loader.as_deref().map_or(loader, |v| v);
    ensure_initialized(registry, init_loader, heap, stdout, &dispatch_class, "")?;
    ExecutionState::new(registry, &dispatch_class, method_name, entry_idx, args)
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
    /// Bytecode of the caller method, cached so `do_return!` can restore it with
    /// a cheap `Arc` move instead of a `ClassRegistry` lookup + constant-pool
    /// re-resolution on every return. Mirrors the cached `pc_to_idx` above.
    instructions: std::sync::Arc<[(usize, Instruction)]>,
    resume_idx: usize,
    /// Class that was executing when this frame was pushed.
    class_name: std::sync::Arc<str>,
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
        || (native_class == "jdk/internal/reflect/Reflection"
            && native_method == "getCallerClass"
            && native_desc == "()Ljava/lang/Class;")
        // `Method.invoke` / `Constructor.newInstance` are caller-sensitive: the JVM
        // access check permits a class to reflectively invoke its OWN
        // (private/nestmate) members without `setAccessible(true)`. Capturing the
        // stack snapshot here makes `frames[0]` (the invoking Java frame) available
        // to the reflect natives so they can apply the same-class rule.
        || (native_class == "java/lang/reflect/Method"
            && native_method == "invoke"
            && native_desc == "(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;")
        || (native_class == "java/lang/reflect/Constructor"
            && native_method == "newInstance"
            && native_desc == "([Ljava/lang/Object;)Ljava/lang/Object;")
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
    use duke_classfile::{AttributeData, CpEntry};

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
    use duke_classfile::CpEntry;

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
/// # use duke_classfile::{CpIndex, CpEntry};
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
    use duke_classfile::{AttributeData, CpEntry};

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
    value: &duke_classfile::ElementValue,
) -> Option<ReflectedAnnotationValue> {
    use duke_classfile::ElementValue;
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
    annotation: &duke_classfile::Annotation,
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
    attrs: &[duke_classfile::AttributeInfo],
) -> Vec<ReflectedAnnotation> {
    attrs
        .iter()
        .find_map(|attr| {
            if let duke_classfile::AttributeData::RuntimeVisibleAnnotations(annotations) =
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
    attrs: &[duke_classfile::AttributeInfo],
) -> Option<ReflectedAnnotationValue> {
    attrs.iter().find_map(|attr| {
        if let duke_classfile::AttributeData::AnnotationDefault(value) = &attr.data {
            resolve_annotation_value(cp, value)
        } else {
            None
        }
    })
}

/// Resolve the `Signature` attribute (JVMS §4.7.9) from an attribute list to its
/// UTF-8 string, if present. Shared by the class-, field-, and method-level
/// reflection metadata so generic type information survives erasure.
fn signature_from_attrs(
    cp: &[Option<CpEntry>],
    attrs: &[duke_classfile::AttributeInfo],
) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if let duke_classfile::AttributeData::Signature { signature_index } = &attr.data {
            cp_utf8_string(cp, signature_index.0 as usize).ok()
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
                signature: signature_from_attrs(
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
                access_flags: field.access_flags.bits(),
                annotations: runtime_visible_annotations_from_attrs(
                    &class_file.constant_pool,
                    &field.attributes,
                ),
                signature: signature_from_attrs(&class_file.constant_pool, &field.attributes),
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
        access_flags: class_file.access_flags.bits(),
        annotations: runtime_visible_annotations_from_attrs(
            &class_file.constant_pool,
            &class_file.attributes,
        ),
        signature: signature_from_attrs(&class_file.constant_pool, &class_file.attributes),
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

/// Synthesize the SAM as a reflectively-visible method for a `$$Lambda$N` proxy.
///
/// The proxy's `ClassContext.methods` is deliberately empty so the SAM keeps
/// dispatching through the `Missing`-method lambda fallback in
/// invokevirtual/invokeinterface (see `ClassRegistry::register_lambda`). Reflective
/// enumeration (`getDeclaredMethods`/`getMethods`) must still list the SAM, so it is
/// synthesized here from the `LambdaInfo` side-channel — sourced ONLY at the
/// reflection-enumeration boundary, never added to `ClassContext.methods`. The SAM
/// is a concrete public instance method (the lambda's implemented functional method).
fn synthesized_lambda_sam(registry: &ClassRegistry, class: &str) -> Option<ReflectedMethodInfo> {
    registry.get_lambda(class).map(|info| ReflectedMethodInfo {
        name: info.sam_method.clone(),
        descriptor: info.sam_desc.clone(),
        is_public: true,
        is_static: false,
        annotations: Vec::new(),
        annotation_default: None,
        signature: None,
    })
}

// Threading `signature: Option<String>` through the class/field/method
// synthetic-stub literals tipped this data-plumbing fn just over the line limit.
#[allow(clippy::too_many_lines)]
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
            let mut methods: Vec<ReflectedMethodInfo> = ctx
                .methods
                .iter()
                .map(|method| ReflectedMethodInfo {
                    name: method.name.clone(),
                    descriptor: method.descriptor.clone(),
                    is_public: method.is_public,
                    is_static: method.is_static,
                    annotations: Vec::new(),
                    annotation_default: None,
                    signature: None,
                })
                .collect();
            // Lambda proxies carry an empty ClassContext.methods (dispatch
            // constraint); synthesize the SAM so reflection lists it.
            methods.extend(synthesized_lambda_sam(registry, &class_key));
            let fields = ctx
                .fields
                .iter()
                .map(|field| ReflectedFieldInfo {
                    name: field.name.clone(),
                    descriptor: field.descriptor.clone(),
                    is_public: true,
                    is_static: field.is_static,
                    access_flags: synthetic_field_access_flags(field.is_static),
                    annotations: Vec::new(),
                    signature: None,
                })
                .collect();
            return Ok(ReflectedClassInfo {
                internal_name: internal_name.clone(),
                binary_name: internal_name_to_binary_name(&internal_name),
                super_class: ctx.super_class.clone(),
                interfaces: ctx.interfaces.clone(),
                methods,
                fields,
                access_flags: SYNTHETIC_CLASS_ACCESS_FLAGS,
                annotations: Vec::new(),
                signature: None,
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
    let mut methods: Vec<ReflectedMethodInfo> = ctx
        .methods
        .iter()
        .map(|method| ReflectedMethodInfo {
            name: method.name.clone(),
            descriptor: method.descriptor.clone(),
            is_public: method.is_public,
            is_static: method.is_static,
            annotations: Vec::new(),
            annotation_default: None,
            signature: None,
        })
        .collect();
    // Lambda proxies carry an empty ClassContext.methods (dispatch constraint);
    // synthesize the SAM so reflection lists it.
    methods.extend(synthesized_lambda_sam(registry, &class_key));
    let fields = ctx
        .fields
        .iter()
        .map(|field| ReflectedFieldInfo {
            name: field.name.clone(),
            descriptor: field.descriptor.clone(),
            is_public: true,
            is_static: field.is_static,
            access_flags: synthetic_field_access_flags(field.is_static),
            annotations: Vec::new(),
            signature: None,
        })
        .collect();

    Ok(ReflectedClassInfo {
        internal_name: internal_name.clone(),
        binary_name: internal_name_to_binary_name(&internal_name),
        super_class: ctx.super_class.clone(),
        interfaces: ctx.interfaces.clone(),
        methods,
        fields,
        access_flags: SYNTHETIC_CLASS_ACCESS_FLAGS,
        annotations: Vec::new(),
        signature: None,
    })
}

const REFLECTION_MEMBER_DECLARING_CLASS_FIELD: usize = 0;
const REFLECTION_MEMBER_NAME_FIELD: usize = 1;
const REFLECTION_MEMBER_DESCRIPTOR_FIELD: usize = 2;
const REFLECTION_MEMBER_PUBLIC_FIELD: usize = 3;
const REFLECTION_MEMBER_STATIC_FIELD: usize = 4;
const REFLECTION_MEMBER_ACCESSIBLE_FIELD: usize = 5;

/// `ACC_INTERFACE` (§4.1) — set when a `Class` mirror denotes an interface.
const ACC_INTERFACE: u16 = 0x0200;

/// Classfile access flags reported for reflection over a synthetic (stub) class
/// that carries no real classfile flags. Matches the legacy `Class.getModifiers`
/// default of `ACC_PUBLIC`.
const SYNTHETIC_CLASS_ACCESS_FLAGS: u16 = 0x0001;

/// Reconstruct field access flags for a synthetic-class field so reflection over
/// a stub keeps reporting the legacy default (public, plus static when applicable).
const fn synthetic_field_access_flags(is_static: bool) -> u16 {
    if is_static { 0x0001 | 0x0008 } else { 0x0001 }
}

/// The modifier bits `Class.getModifiers()` reports: the JLS recognized class
/// modifiers (`ACC_PUBLIC|FINAL|INTERFACE|ABSTRACT|SYNTHETIC|ANNOTATION|ENUM`),
/// with `ACC_SUPER` (0x0020) and `ACC_MODULE` (0x8000) masked off, matching the
/// value `HotSpot`'s `JVM_GetClassModifiers` returns for a top-level class.
const CLASS_MODIFIER_MASK: u16 =
    0x0001 | 0x0010 | 0x0200 | 0x0400 | 0x1000 | 0x2000 | 0x4000;

/// The modifier bits `Field.getModifiers()` reports
/// (`JVM_RECOGNIZED_FIELD_MODIFIERS`): public/private/protected/static/final/
/// volatile/transient/synthetic/enum.
const FIELD_MODIFIER_MASK: u16 =
    0x0001 | 0x0002 | 0x0004 | 0x0008 | 0x0010 | 0x0040 | 0x0080 | 0x1000 | 0x4000;

/// Mask raw classfile class access flags to the set `Class.getModifiers()` reports.
#[must_use]
pub(crate) const fn class_modifiers_from_access_flags(access_flags: u16) -> i32 {
    (access_flags & CLASS_MODIFIER_MASK) as i32
}

/// Whether raw classfile class access flags denote an interface.
#[must_use]
pub(crate) const fn is_interface_from_access_flags(access_flags: u16) -> bool {
    access_flags & ACC_INTERFACE != 0
}

/// Mask raw classfile field access flags to the set `Field.getModifiers()` reports.
#[must_use]
pub(crate) const fn field_modifiers_from_access_flags(access_flags: u16) -> i32 {
    (access_flags & FIELD_MODIFIER_MASK) as i32
}

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
        field_name: string_value_from_ref(heap, name_ref)?,
        descriptor: string_value_from_ref(heap, descriptor_ref)?,
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
        method_name: string_value_from_ref(heap, name_ref)?,
        descriptor: string_value_from_ref(heap, descriptor_ref)?,
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
            // A ClassContext's interface entries are stored either as plain internal
            // names (classes built reflectively/synthetically via `build_class_context`,
            // which never runs the loader-resolution pass) or as loader-qualified keys
            // (classes resolved through `ensure_loaded_inner`, which rewrites each
            // interface to a `name\0loader:N` key). `to_key` here is loader-qualified.
            // A direct `iface == to_key` match therefore silently fails for the plain
            // case — e.g. `x instanceof org/apache/commons/logging/Log` on a
            // reflectively-built implementor returns a false negative. Interface
            // assignability in Duke's model is keyed by internal name, so compare on the
            // plain internal name (stripping any loader qualifier from both sides); this
            // is loader-agnostic and consistent with how interface entries are recorded.
            if class_internal_name_from_key(&iface) == to_internal {
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

/// Walk `start_class` and its super chain looking for a registered native handler
/// for `method_name`/`method_desc`. Mirrors the super-chain walk used by
/// invokevirtual so inherited `java/lang/Object` natives (e.g. `getClass`) resolve
/// even when the receiver is reached through an interface-typed callsite whose own
/// class declares no such native.
fn lookup_native_kind_in_super_chain(
    registry: &ClassRegistry,
    start_class: &str,
    method_name: &str,
    method_desc: &str,
) -> Option<HandlerKind> {
    let mut current = Some(start_class.to_string());
    while let Some(ref class) = current {
        if let Some(handler) =
            lookup_registered_native_kind(registry, class, method_name, method_desc)
        {
            return Some(handler);
        }
        current = registry.get(class).ok().and_then(|ctx| ctx.super_class.clone());
    }
    None
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
    // The operand stack holds the args in forward order (last param on top), so we
    // pop in reverse. Walk the destination local index backward in lock-step: the
    // total slot span accounts for wide J/D params occupying two slots, and each
    // param's slot is derived by subtracting its width before the store. This pops
    // args directly into the pre-sized `locals` buffer, avoiding the per-call
    // temporary `Vec` the previous implementation allocated on every invoke.
    let mut local_idx = start_idx;
    for &tc in param_types {
        local_idx += if tc == 'J' || tc == 'D' { 2 } else { 1 };
    }
    for &tc in param_types.iter().rev() {
        local_idx -= if tc == 'J' || tc == 'D' { 2 } else { 1 };
        let value = frame.pop()?;
        if local_idx < locals.len() {
            locals[local_idx] = value;
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
    if let Some(exception_ref) = take_uncaught_java_exception_ref(class_name) {
        return Ok(exception_ref);
    }

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

type PendingExceptionMessages = std::sync::Mutex<HashMap<(std::thread::ThreadId, String), VecDeque<String>>>;
type PendingExceptionCauses = std::sync::Mutex<HashMap<(std::thread::ThreadId, String), VecDeque<Slot>>>;
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
        .entry((std::thread::current().id(), class_name.to_string()))
        .or_default()
        .push_back(message);
}

fn pop_pending_java_exception_message(class_name: &str) -> Option<String> {
    let mut messages = pending_java_exception_messages()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let key = (std::thread::current().id(), class_name.to_string());
    let queue = messages.get_mut(&key)?;
    let message = queue.pop_front();
    if queue.is_empty() {
        messages.remove(&key);
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
        .entry((std::thread::current().id(), class_name.to_string()))
        .or_default()
        .push_back(cause);
}

fn pop_pending_java_exception_cause(class_name: &str) -> Option<Slot> {
    let mut causes = pending_java_exception_causes()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let key = (std::thread::current().id(), class_name.to_string());
    let queue = causes.get_mut(&key)?;
    let cause = queue.pop_front();
    if queue.is_empty() {
        causes.remove(&key);
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

/// Resolve a static field by walking the superclass chain (JVMS §5.4.3.2).
///
/// Returns the declaring class (where the static is physically stored) and the
/// local index into THAT class's `static_fields`. Static storage is per-class,
/// so callers must index the declaring class's `static_fields`, not the
/// subclass's (there is no cumulative offset as there is for instance fields).
fn resolve_static_field(
    registry: &ClassRegistry,
    target_class: &str,
    name: &str,
) -> Result<(String, usize)> {
    let mut current = target_class;

    while let Ok(ctx) = registry.get(current) {
        if let Some(idx) = ctx
            .fields
            .iter()
            .filter(|f| f.is_static)
            .position(|f| f.name == name)
        {
            return Ok((current.to_string(), idx));
        }

        if let Some(ref sc) = ctx.super_class {
            current = sc;
        } else {
            break;
        }
    }

    Err(Error::InvalidFieldref { index: 0 })
}

// ---------------------------------------------------------------------------
// StringBuilder natives
// ---------------------------------------------------------------------------













// ---- StringBuilder extended operations ----









// Character natives
// ---------------------------------------------------------------------------

/// Helper: extract a `char` from a `Slot::Int` argument.
fn slot_to_char(slot: &Slot) -> Result<char> {
    match slot {
        Slot::Int(v) => Ok(char::from_u32((*v).cast_unsigned()).unwrap_or('\0')),
        _ => Err(Error::TypeMismatch {
            expected: "Int (char)",
            got: "other",
        }),
    }
}

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

/// Simple (single-code-point) uppercase mapping, matching
/// `java.lang.Character.toUpperCase` semantics rather than Rust's full
/// `char::to_uppercase` (which can expand e.g. `ß` → `SS`). When the full
/// uppercase expansion is a single character we use it; otherwise the
/// character has no simple uppercase mapping and maps to itself.
fn simple_uppercase(ch: char) -> char {
    let mut it = ch.to_uppercase();
    match (it.next(), it.next()) {
        (Some(upper), None) => upper,
        _ => ch,
    }
}

/// Native: `Character.toTitleCase(C)C` / `Character.toTitleCase(I)I`.
///
/// Returns the simple (single-code-point) titlecase mapping. For the BMP this
/// equals the simple uppercase mapping except for the Latin digraph titlecase
/// characters, which map to their dedicated titlecase forms.
pub(crate) fn native_char_to_titlecase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    let title = match ch as u32 {
        0x01C4..=0x01C6 => '\u{01C5}',
        0x01C7..=0x01C9 => '\u{01C8}',
        0x01CA..=0x01CC => '\u{01CB}',
        0x01F1..=0x01F3 => '\u{01F2}',
        _ => simple_uppercase(ch),
    };
    Ok(Some(Slot::Int(title as i32)))
}

/// Native: `Character.charCount(I)I` — number of `char` values needed to
/// represent the given code point (2 for supplementary code points, else 1).
pub(crate) fn native_char_char_count(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let cp = match args.first().ok_or(Error::StackUnderflow)? {
        Slot::Int(v) => *v,
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Int (code point)",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Int(if cp >= 0x0001_0000 { 2 } else { 1 })))
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








fn collection_elements_from_ref(heap: &duke_gc::Heap, collection_ref: u64) -> Result<Vec<Slot>> {
    let collection = heap.get(collection_ref)?;
    match collection.class_name.as_str() {
        "java/util/ArrayList" | "java/util/HashSet" => {
            let size = match collection.fields.first() {
                Some(Slot::Int(size)) if *size >= 0 => usize::try_from(*size).unwrap_or(0),
                _ => 0,
            };
            Ok(collection
                .fields
                .iter()
                .skip(1)
                .take(size)
                .copied()
                .collect())
        }
        _ => Err(Error::TypeMismatch {
            expected: "java/util/Collection",
            got: "other",
        }),
    }
}

fn allocate_reference_array_from_slots(
    heap: &mut duke_gc::Heap,
    array_class_name: &str,
    elements: &[Slot],
) -> Result<u64> {
    let array_ref = heap.allocate(array_class_name.to_string(), elements.len());
    let array = heap.get_mut(array_ref)?;
    for slot in &mut array.fields {
        *slot = Slot::Reference(None);
    }
    for (idx, element) in elements.iter().enumerate() {
        array.fields[idx] = *element;
    }
    Ok(array_ref)
}



/// Native: `ArrayList.sort(Comparator)V` — sorts in-place using insertion sort,
/// calling `compareTo` on each element pair via the interpreter callback.
///
/// Only null Comparator (natural ordering via `compareTo`) is supported.
pub(crate) fn array_list_sort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    // args[0] = ArrayList ref, args[1] = Comparator (null = natural ordering)
    let list_ref = extract_ref_arg(args, 0)?;

    // args[1] = optional Comparator ref (null = natural ordering via compareTo)
    let comparator = args.get(1).copied();

    // Fix 2: guard against a negative size stored in fields[0].
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(Error::NegativeArraySize { size: *n }),
        _ => return Ok(None),
    };

    if size <= 1 {
        return Ok(None);
    }

    // Collect element refs (fields[1..=size]).
    let mut elems: Vec<u64> = (1..=size)
        .filter_map(|i| match heap.get(list_ref).ok()?.fields.get(i) {
            Some(Slot::Reference(Some(r))) => Some(*r),
            _ => None,
        })
        .collect();

    // Fix 3: malformed list → InvalidRef, not silent Ok(None).
    if elems.len() != size {
        return Err(Error::InvalidRef { address: list_ref });
    }

    // Insertion sort — O(n²), correct, easy to verify.
    for i in 1..elems.len() {
        let mut key = elems[i];
        let mut j = i;
        while j > 0 {
            let receiver = elems[j - 1];
            let cmp = if let Some(Slot::Reference(Some(comp_ref))) = comparator {
                let comp_class = heap.get(comp_ref)?.class_name.clone();
                ops.invoke(
                    heap,
                    output,
                    &comp_class,
                    "compare",
                    "(Ljava/lang/Object;Ljava/lang/Object;)I",
                    vec![
                        Slot::Reference(Some(comp_ref)),
                        Slot::Reference(Some(receiver)),
                        Slot::Reference(Some(key)),
                    ],
                )?
            } else {
                let class_name = heap.get(receiver)?.class_name.clone();
                ops.invoke(
                    heap,
                    output,
                    &class_name,
                    COMPARE_TO_METHOD,
                    COMPARE_TO_OBJECT_DESC,
                    vec![Slot::Reference(Some(receiver)), Slot::Reference(Some(key))],
                )?
            };

            // Patch stale young-gen refs if a minor GC fired during the callback.
            // Guarded by `has_pending_forwards` so the common (no-GC) path pays
            // only one bool check instead of O(n) HashMap probes.
            if heap.has_pending_forwards() {
                for elem in &mut elems {
                    let mut slot = Slot::Reference(Some(*elem));
                    heap.apply_forward(&mut slot);
                    if let Slot::Reference(Some(r)) = slot {
                        *elem = r;
                    }
                }
                // Re-read key after forwarding patch (it lives outside elems
                // during the innermost loop iteration).
                let mut key_slot = Slot::Reference(Some(key));
                heap.apply_forward(&mut key_slot);
                if let Slot::Reference(Some(r)) = key_slot {
                    key = r;
                }
            }

            // Fix 4: explicit error on non-Int compareTo return.
            match cmp {
                Some(Slot::Int(n)) if n <= 0 => break,
                Some(Slot::Int(_)) => {} // n > 0, keep shifting
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: "Int",
                        got: "other",
                    });
                }
            }
            elems[j] = elems[j - 1];
            j -= 1;
        }
        elems[j] = key;
    }

    // Write sorted elements back using the write barrier.
    for (i, &r) in elems.iter().enumerate() {
        heap.write_field(list_ref, i + 1, Slot::Reference(Some(r)))?;
    }

    Ok(None)
}

// ---------------------------------------------------------------------------
// Collections natives
// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// ServiceLoader natives
// ---------------------------------------------------------------------------

const SERVICE_LOADER_SERVICE_CLASS_FIELD: usize = 0;
const SERVICE_LOADER_LOADER_FIELD: usize = 1;
const SERVICE_LOADER_COUNT_FIELD: usize = 2;
const SERVICE_LOADER_PROVIDERS_START: usize = 3;

const SERVICE_ITER_SERVICE_CLASS_FIELD: usize = 0;
const SERVICE_ITER_LOADER_FIELD: usize = 1;
const SERVICE_ITER_INDEX_FIELD: usize = 2;
const SERVICE_ITER_COUNT_FIELD: usize = 3;
const SERVICE_ITER_PROVIDERS_START: usize = 4;

fn read_resource_enumeration_bytes(
    enum_ref: u64,
    heap: &mut duke_gc::Heap,
) -> Result<Vec<Vec<u8>>> {
    let mut files = Vec::new();
    loop {
        let has_more = match native_resource_enumeration_has_more_elements(
            &[Slot::Reference(Some(enum_ref))],
            heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
        )? {
            Some(Slot::Int(value)) => value != 0,
            _ => false,
        };
        if !has_more {
            break;
        }
        let url_slot = native_resource_enumeration_next_element(
            &[Slot::Reference(Some(enum_ref))],
            heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
        )?
        .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(url_ref)) = url_slot else {
            return Err(Error::NullPointerException);
        };
        let spec = string_backed_object_value(heap, url_ref)?;
        files.push(read_resource_bytes_from_url_spec(&spec)?);
    }
    Ok(files)
}

fn service_configuration_files_via_resources(
    heap: &mut duke_gc::Heap,
    ops: &mut dyn CallbackOps,
    loader_ref: Option<u64>,
    service_binary_name: &str,
) -> Result<Vec<Vec<u8>>> {
    let resource_name = format!("META-INF/services/{service_binary_name}");
    let enum_ref = if let Some(loader_ref) = loader_ref {
        let resource_name_ref = heap.allocate_string(resource_name);
        let enum_slot = native_class_loader_get_resources(
            &[
                Slot::Reference(Some(loader_ref)),
                Slot::Reference(Some(resource_name_ref)),
            ],
            heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
            ops,
        )?
        .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(enum_ref)) = enum_slot else {
            return Ok(Vec::new());
        };
        enum_ref
    } else {
        let resources = ops.find_resource_entries(heap, None, &resource_name)?;
        allocate_resource_enumeration(heap, resources)?
    };
    read_resource_enumeration_bytes(enum_ref, heap)
}

fn parse_service_provider_names(files: Vec<Vec<u8>>) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for bytes in files {
        let text = String::from_utf8(bytes).map_err(|_| Error::JavaException {
            class_name: "java/util/ServiceConfigurationError".to_string(),
        })?;
        for line in text.lines() {
            let live = line
                .split_once('#')
                .map_or(line, |(before, _)| before)
                .trim();
            if !live.is_empty() {
                names.push(live.to_string());
            }
        }
    }
    Ok(names)
}

fn service_loader_cause_type(err: &Error) -> &'static str {
    match err {
        Error::ClassNotFound { .. } => "java.lang.ClassNotFoundException",
        Error::JavaException { class_name } => match class_name.as_str() {
            "java/lang/ClassNotFoundException" => "java.lang.ClassNotFoundException",
            "java/lang/IllegalAccessException" => "java.lang.IllegalAccessException",
            "java/lang/InstantiationException" => "java.lang.InstantiationException",
            "java/lang/NoSuchMethodException" => "java.lang.NoSuchMethodException",
            _ => "java.lang.RuntimeException",
        },
        Error::InstantiationError { .. } => "java.lang.InstantiationException",
        Error::NullPointerException => "java.lang.NullPointerException",
        _ => "duke_runtime.Error",
    }
}

fn service_configuration_error(provider_name: &str, cause_type: &str) -> Error {
    push_pending_java_exception_message(
        "java/util/ServiceConfigurationError",
        format!("{provider_name}: provider construction failed due to {cause_type}"),
    );
    Error::JavaException {
        class_name: "java/util/ServiceConfigurationError".to_string(),
    }
}

fn allocate_service_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    ops: &mut dyn CallbackOps,
    loader_arg_index: Option<usize>,
) -> Result<Option<Slot>> {
    let service_class_ref = extract_ref_arg(args, 0)?;
    let service_internal_name = class_internal_name_from_ref(heap, service_class_ref)?;
    let service_binary_name = internal_name_to_binary_name(&service_internal_name);
    let loader_slot = loader_arg_index
        .and_then(|idx| args.get(idx).copied())
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(loader_ref) = loader_slot else {
        return Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        });
    };
    let files = service_configuration_files_via_resources(heap, ops, loader_ref, &service_binary_name)?;
    let provider_names = parse_service_provider_names(files)?;
    let loader_ref = heap.allocate(
        "java/util/ServiceLoader".to_string(),
        SERVICE_LOADER_PROVIDERS_START + provider_names.len(),
    );
    {
        let service_loader = heap.get_mut(loader_ref)?;
        service_loader.fields[SERVICE_LOADER_SERVICE_CLASS_FIELD] =
            Slot::Reference(Some(service_class_ref));
        service_loader.fields[SERVICE_LOADER_LOADER_FIELD] = loader_slot;
        service_loader.fields[SERVICE_LOADER_COUNT_FIELD] =
            Slot::Int(i32::try_from(provider_names.len()).unwrap_or(i32::MAX));
    }
    for (idx, provider_name) in provider_names.into_iter().enumerate() {
        let name_ref = heap.allocate_string(provider_name);
        heap.write_field(
            loader_ref,
            SERVICE_LOADER_PROVIDERS_START + idx,
            Slot::Reference(Some(name_ref)),
        )?;
    }
    Ok(Some(Slot::Reference(Some(loader_ref))))
}




fn service_iterator_index_and_count(heap: &duke_gc::Heap, iter_ref: u64) -> Result<(usize, usize)> {
    let iter = heap.get(iter_ref)?;
    let index = match iter.fields.get(SERVICE_ITER_INDEX_FIELD) {
        Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
        _ => 0,
    };
    let count = match iter.fields.get(SERVICE_ITER_COUNT_FIELD) {
        Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
        _ => 0,
    };
    Ok((index, count))
}

pub(crate) fn native_service_loader_iter_init(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(None)
}

pub(crate) fn native_service_loader_iter_has_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let iter_ref = extract_ref_arg(args, 0)?;
    let (index, count) = service_iterator_index_and_count(heap, iter_ref)?;
    Ok(Some(Slot::Int(i32::from(index < count))))
}

fn instantiate_service_provider(
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    iter_ref: u64,
) -> Result<u64> {
    let (index, count) = service_iterator_index_and_count(heap, iter_ref)?;
    if index >= count {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }

    let (loader_ref, provider_name_ref) = {
        let iter = heap.get(iter_ref)?;
        let loader_ref = match iter.fields.get(SERVICE_ITER_LOADER_FIELD) {
            Some(Slot::Reference(reference)) => *reference,
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "Reference",
                    got: "other",
                });
            }
        };
        let provider_name_ref = match iter.fields.get(SERVICE_ITER_PROVIDERS_START + index) {
            Some(Slot::Reference(Some(reference))) => *reference,
            _ => return Err(Error::NullPointerException),
        };
        (loader_ref, provider_name_ref)
    };
    let provider_binary_name = string_value_from_ref(heap, provider_name_ref)?;
    let provider_internal_name = binary_name_to_internal_name(&provider_binary_name);
    heap.get_mut(iter_ref)?.fields[SERVICE_ITER_INDEX_FIELD] =
        Slot::Int(i32::try_from(index + 1).unwrap_or(i32::MAX));

    let load_result = if let Some(loader_ref) = loader_ref {
        ops.ensure_loaded_with_runtime_loader(heap, loader_ref, &provider_internal_name)
    } else {
        ops.ensure_loaded(&provider_internal_name)
    };
    if let Err(err) = load_result {
        return Err(service_configuration_error(
            &provider_binary_name,
            service_loader_cause_type(&err),
        ));
    }
    let class_key = if let Some(loader_ref) = loader_ref {
        match ops.class_key_for_runtime_loader(heap, loader_ref, &provider_internal_name) {
            Ok(class_key) => class_key,
            Err(err) => {
                return Err(service_configuration_error(
                    &provider_binary_name,
                    service_loader_cause_type(&err),
                ));
            }
        }
    } else {
        match ops.class_key_for_loaded_class(&provider_internal_name) {
            Ok(class_key) => class_key,
            Err(err) => {
                return Err(service_configuration_error(
                    &provider_binary_name,
                    service_loader_cause_type(&err),
                ));
            }
        }
    };

    let reflected = match ops.inspect_class(&class_key) {
        Ok(reflected) => reflected,
        Err(err) => {
            return Err(service_configuration_error(
                &provider_binary_name,
                service_loader_cause_type(&err),
            ));
        }
    };
    let has_public_no_arg_ctor = reflected.methods.iter().any(|method| {
        method.name == "<init>" && method.descriptor == "()V" && method.is_public
    });
    if !has_public_no_arg_ctor {
        return Err(service_configuration_error(
            &provider_binary_name,
            "java.lang.NoSuchMethodException",
        ));
    }

    let instance_ref = match ops.allocate_instance(heap, output, &class_key) {
        Ok(instance_ref) => instance_ref,
        Err(err) => {
            return Err(service_configuration_error(
                &provider_binary_name,
                service_loader_cause_type(&err),
            ));
        }
    };
    match ops.invoke(
        heap,
        output,
        &class_key,
        "<init>",
        "()V",
        vec![Slot::Reference(Some(instance_ref))],
    ) {
        Ok(_) => Ok(instance_ref),
        Err(err) => Err(service_configuration_error(
            &provider_binary_name,
            service_loader_cause_type(&err),
        )),
    }
}

pub(crate) fn native_service_loader_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let iter_ref = extract_ref_arg(args, 0)?;
    let provider_ref = instantiate_service_provider(heap, output, ops, iter_ref)?;
    Ok(Some(Slot::Reference(Some(provider_ref))))
}


// ---------------------------------------------------------------------------
// ArrayListIterator natives
// ---------------------------------------------------------------------------





// ---- ArrayList extended methods ----










// ---- HashMap extended methods ----




// ---- Double.isNaN ----



// ---- Arrays natives ----







// ---------------------------------------------------------------------------
// Phase 44: Arrays.copyOfRange, List.subList, Comparator.reversed,
//           Collections.binarySearch, String.intern, ArrayList.removeIf
// ---------------------------------------------------------------------------





/// Native: `Comparator.reversed()Comparator` — wraps comparator to invert ordering.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_reversed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let delegate = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/ReversedComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = delegate;
    Ok(Some(Slot::Reference(Some(r))))
}




// ---- Arrays.asList ----


// ---- String extended operations (Java 11+) ----












// ---------------------------------------------------------------------------
// java.util.Random — 48-bit LCG (same multiplier/addend as Java's java.util.Random)
// fields[0] = Long(seed as i64); all bit-level casts are intentional.
// ---------------------------------------------------------------------------

#[allow(clippy::unreadable_literal)]
const RANDOM_MULTIPLIER: u64 = 25_214_903_917; // 0x5DEECE66D — Java's LCG multiplier
const RANDOM_ADDEND: u64 = 0xB;
const RANDOM_MASK: u64 = (1u64 << 48) - 1;

/// Advance the LCG and return `bits` high bits of the new state.
#[allow(clippy::cast_possible_truncation)]
const fn random_next(seed: u64, bits: u32) -> (u64, i32) {
    let new_seed = seed
        .wrapping_mul(RANDOM_MULTIPLIER)
        .wrapping_add(RANDOM_ADDEND)
        & RANDOM_MASK;
    let value = (new_seed >> (48 - bits)) as i32; // intentional truncation to bit pattern
    (new_seed, value)
}



/// Retrieve and advance seed from `fields[0]`, returning new seed and `bits` high bits.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
fn random_step(heap: &mut duke_gc::Heap, this_ref: u64, bits: u32) -> Result<(u64, i32)> {
    let old_seed = match heap.get(this_ref)?.fields.first().copied() {
        Some(Slot::Long(v)) => v as u64,
        _ => 0,
    };
    let (new_seed, value) = random_next(old_seed, bits);
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(new_seed as i64); // safe: < 2^48
    Ok((new_seed, value))
}







fn byte_array_from_ref(heap: &duke_gc::Heap, array_ref: u64) -> Result<Vec<u8>> {
    let arr = heap.get(array_ref)?;
    let mut bytes = Vec::with_capacity(arr.fields.len());
    for slot in &arr.fields {
        let Slot::Int(v) = slot else {
            return Err(Error::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        };
        bytes.push(v.to_le_bytes()[0]);
    }
    Ok(bytes)
}

fn alloc_byte_array(heap: &mut duke_gc::Heap, bytes: &[u8]) -> u64 {
    let array_ref = heap.allocate("[B".to_string(), bytes.len());
    if let Ok(array) = heap.get_mut(array_ref) {
        for (idx, byte) in bytes.iter().copied().enumerate() {
            array.fields[idx] = Slot::Int(i32::from(byte));
        }
    }
    array_ref
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Base64Variant {
    Standard,
    Mime,
    Url,
}

impl Base64Variant {
    const fn field_value(self) -> i32 {
        match self {
            Self::Standard => 0,
            Self::Mime => 1,
            Self::Url => 2,
        }
    }

    const fn from_field(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Standard),
            1 => Some(Self::Mime),
            2 => Some(Self::Url),
            _ => None,
        }
    }

    const fn alphabet(self) -> &'static [u8; 64] {
        match self {
            Self::Standard | Self::Mime => {
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
            }
            Self::Url => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
        }
    }
}

fn invalid_base64_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    }
}

fn allocate_base64_coder(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    variant: Base64Variant,
) -> Result<u64> {
    let coder_ref = heap.allocate(class_name.to_string(), 1);
    heap.get_mut(coder_ref)?.fields[0] = Slot::Int(variant.field_value());
    Ok(coder_ref)
}

fn base64_variant_arg(args: &[Slot], heap: &duke_gc::Heap) -> Result<Base64Variant> {
    let coder_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Int(value)) = heap.get(coder_ref)?.fields.first() else {
        return Err(Error::TypeMismatch {
            expected: "Base64 variant",
            got: "other",
        });
    };
    Base64Variant::from_field(*value).ok_or_else(invalid_base64_error)
}

fn encode_base64(input: &[u8], variant: Base64Variant) -> String {
    let alphabet = variant.alphabet();
    let mut out = Vec::with_capacity(input.len().div_ceil(3) * 4);
    let mut line_len = 0_usize;

    for chunk in input.chunks(3) {
        let first = u32::from(chunk[0]);
        let second = chunk.get(1).copied().map_or(0, u32::from);
        let third = chunk.get(2).copied().map_or(0, u32::from);
        let triple = (first << 16) | (second << 8) | third;
        let encoded = [
            alphabet[((triple >> 18) & 0x3f) as usize],
            alphabet[((triple >> 12) & 0x3f) as usize],
            if chunk.len() > 1 {
                alphabet[((triple >> 6) & 0x3f) as usize]
            } else {
                b'='
            },
            if chunk.len() > 2 {
                alphabet[(triple & 0x3f) as usize]
            } else {
                b'='
            },
        ];

        for byte in encoded {
            if variant == Base64Variant::Mime && line_len == 76 {
                out.extend_from_slice(b"\r\n");
                line_len = 0;
            }
            out.push(byte);
            line_len += 1;
        }
    }

    out.into_iter().map(char::from).collect()
}

fn base64_decode_value(byte: u8, variant: Base64Variant) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' if variant != Base64Variant::Url => Some(62),
        b'/' if variant != Base64Variant::Url => Some(63),
        b'-' if variant == Base64Variant::Url => Some(62),
        b'_' if variant == Base64Variant::Url => Some(63),
        _ => None,
    }
}

fn filtered_base64_input(input: &[u8], variant: Base64Variant) -> Vec<u8> {
    input
        .iter()
        .copied()
        .filter(|byte| {
            variant != Base64Variant::Mime || !matches!(byte, b'\r' | b'\n' | b' ' | b'\t')
        })
        .collect()
}

fn require_base64_value(byte: u8, variant: Base64Variant) -> Result<u8> {
    base64_decode_value(byte, variant).ok_or_else(invalid_base64_error)
}

fn push_base64_triplet(out: &mut Vec<u8>, values: [u8; 4]) {
    out.push((values[0] << 2) | (values[1] >> 4));
    out.push(((values[1] & 0x0f) << 4) | (values[2] >> 2));
    out.push(((values[2] & 0x03) << 6) | values[3]);
}

fn decode_base64(input: &[u8], variant: Base64Variant) -> Result<Vec<u8>> {
    let bytes = filtered_base64_input(input, variant);
    if bytes.is_empty() {
        return Ok(Vec::new());
    }

    let mut first_padding = None;
    for (idx, byte) in bytes.iter().copied().enumerate() {
        if byte == b'=' {
            first_padding.get_or_insert(idx);
        } else if first_padding.is_some() || base64_decode_value(byte, variant).is_none() {
            return Err(invalid_base64_error());
        }
    }

    let data_len = first_padding.unwrap_or(bytes.len());
    if let Some(first_padding_idx) = first_padding {
        let pad_count = bytes.len() - first_padding_idx;
        let data_remainder = data_len % 4;
        if pad_count > 2
            || !bytes.len().is_multiple_of(4)
            || (pad_count == 1 && data_remainder != 3)
            || (pad_count == 2 && data_remainder != 2)
        {
            return Err(invalid_base64_error());
        }
    } else if data_len % 4 == 1 {
        return Err(invalid_base64_error());
    }

    let mut out = Vec::with_capacity((data_len / 4) * 3 + 2);
    let mut idx = 0;
    while idx + 4 <= data_len {
        let values = [
            require_base64_value(bytes[idx], variant)?,
            require_base64_value(bytes[idx + 1], variant)?,
            require_base64_value(bytes[idx + 2], variant)?,
            require_base64_value(bytes[idx + 3], variant)?,
        ];
        push_base64_triplet(&mut out, values);
        idx += 4;
    }

    match data_len - idx {
        0 => {}
        2 => {
            let first = require_base64_value(bytes[idx], variant)?;
            let second = require_base64_value(bytes[idx + 1], variant)?;
            out.push((first << 2) | (second >> 4));
        }
        3 => {
            let first = require_base64_value(bytes[idx], variant)?;
            let second = require_base64_value(bytes[idx + 1], variant)?;
            let third = require_base64_value(bytes[idx + 2], variant)?;
            out.push((first << 2) | (second >> 4));
            out.push(((second & 0x0f) << 4) | (third >> 2));
        }
        _ => return Err(invalid_base64_error()),
    }

    Ok(out)
}











const DUKE_SECURITY_PROVIDER: &str = "DUKE";
const PRIVILEGED_ACTION_EXCEPTION: &str = "java/security/PrivilegedActionException";
const MESSAGE_DIGEST_SERVICE_TYPE: &str = "MessageDigest";
const MESSAGE_DIGEST_ALGORITHMS: [&str; 3] = ["SHA-256", "SHA-1", "MD5"];
const MESSAGE_DIGEST_ALGORITHM_FIELD: usize = 0;
const MESSAGE_DIGEST_BUFFER_FIELD: usize = 1;
const PROVIDER_SERVICE_PROVIDER_FIELD: usize = 0;
const PROVIDER_SERVICE_TYPE_FIELD: usize = 1;
const PROVIDER_SERVICE_ALGORITHM_FIELD: usize = 2;
const PRIVILEGED_ACTION_RUN_DESCRIPTOR: &str = "()Ljava/lang/Object;";

fn class_extends_via_callback(
    class_name: &str,
    target: &str,
    ops: &mut dyn CallbackOps,
) -> Result<bool> {
    let mut current = Some(class_name.to_string());
    let mut visited = HashSet::new();

    while let Some(name) = current {
        if name == target {
            return Ok(true);
        }
        if !visited.insert(name.clone()) {
            return Ok(false);
        }
        current = ops.inspect_class(&name)?.super_class;
    }

    Ok(false)
}

fn is_unchecked_exception_class(class_name: &str, ops: &mut dyn CallbackOps) -> Result<bool> {
    Ok(class_extends_via_callback(class_name, "java/lang/RuntimeException", ops)?
        || class_extends_via_callback(class_name, "java/lang/Error", ops)?)
}

fn take_uncaught_java_exception_slot(class_name: &str) -> Slot {
    take_uncaught_java_exception_ref(class_name).map_or(Slot::Reference(None), |exception_ref| {
        Slot::Reference(Some(exception_ref))
    })
}

fn privileged_action_class(args: &[Slot], heap: &duke_gc::Heap) -> Result<(u64, String)> {
    let action_ref = extract_ref_arg(args, 0)?;
    let action_class = heap.get(action_ref)?.class_name.clone();
    Ok((action_ref, action_class))
}

fn invoke_privileged_action(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    wrap_checked_exception: bool,
) -> Result<Option<Slot>> {
    let (action_ref, action_class) = privileged_action_class(args, heap)?;
    let action_args = vec![Slot::Reference(Some(action_ref))];
    match ops.invoke(
        heap,
        out,
        &action_class,
        "run",
        PRIVILEGED_ACTION_RUN_DESCRIPTOR,
        action_args,
    ) {
        Ok(result) => Ok(result),
        Err(Error::JavaException { class_name })
            if wrap_checked_exception && !is_unchecked_exception_class(&class_name, ops)? =>
        {
            let cause = take_uncaught_java_exception_slot(&class_name);
            push_pending_java_exception_cause(PRIVILEGED_ACTION_EXCEPTION, cause);
            Err(Error::JavaException {
                class_name: PRIVILEGED_ACTION_EXCEPTION.to_string(),
            })
        }
        Err(err) => Err(err),
    }
}



fn canonical_digest_algorithm(algorithm: &str) -> Option<&'static str> {
    let compact: String = algorithm
        .bytes()
        .filter(|byte| !matches!(byte, b'-' | b'_' | b' ' | b'\t' | b'\r' | b'\n'))
        .map(|byte| char::from(byte.to_ascii_uppercase()))
        .collect();
    match compact.as_str() {
        "SHA256" => Some("SHA-256"),
        "SHA1" => Some("SHA-1"),
        "MD5" => Some("MD5"),
        _ => None,
    }
}

fn require_digest_algorithm(algorithm: &str) -> Result<&'static str> {
    canonical_digest_algorithm(algorithm).ok_or_else(|| Error::JavaException {
        class_name: "java/security/NoSuchAlgorithmException".to_string(),
    })
}

const fn is_duke_provider_name(name: &str) -> bool {
    name.eq_ignore_ascii_case(DUKE_SECURITY_PROVIDER)
}

const fn is_message_digest_service_type(service_type: &str) -> bool {
    service_type.eq_ignore_ascii_case(MESSAGE_DIGEST_SERVICE_TYPE)
}

fn compute_message_digest(algorithm: &str, bytes: &[u8]) -> Result<Vec<u8>> {
    use sha1::Digest as _;
    let digest = match require_digest_algorithm(algorithm)? {
        "SHA-256" => sha2::Sha256::digest(bytes).to_vec(),
        "SHA-1" => sha1::Sha1::digest(bytes).to_vec(),
        "MD5" => md5::Md5::digest(bytes).to_vec(),
        _ => unreachable!("canonical digest algorithm must be supported"),
    };
    Ok(digest)
}

fn message_digest_algorithm(heap: &duke_gc::Heap, digest_ref: u64) -> Result<String> {
    let (string_algorithm, field_algorithm) = {
        let digest = heap.get(digest_ref)?;
        (
            digest.string_value.clone(),
            digest.fields.get(MESSAGE_DIGEST_ALGORITHM_FIELD).copied(),
        )
    };
    if let Some(algorithm) = string_algorithm {
        return Ok(algorithm);
    }
    match field_algorithm {
        Some(Slot::Reference(Some(algorithm_ref))) => string_value_from_ref(heap, algorithm_ref),
        _ => Ok(String::new()),
    }
}

fn message_digest_buffer(heap: &duke_gc::Heap, digest_ref: u64) -> Result<Vec<u8>> {
    match heap
        .get(digest_ref)?
        .fields
        .get(MESSAGE_DIGEST_BUFFER_FIELD)
        .copied()
    {
        Some(Slot::Reference(Some(buffer_ref))) => byte_array_from_ref(heap, buffer_ref),
        _ => Ok(Vec::new()),
    }
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
        digest
            .fields
            .resize(MESSAGE_DIGEST_BUFFER_FIELD + 1, Slot::Reference(None));
    }
    digest.fields[MESSAGE_DIGEST_BUFFER_FIELD] = buffer_slot;
    Ok(())
}

fn append_message_digest_buffer(
    heap: &mut duke_gc::Heap,
    digest_ref: u64,
    bytes: &[u8],
) -> Result<()> {
    let mut buffer = message_digest_buffer(heap, digest_ref)?;
    buffer.extend_from_slice(bytes);
    write_message_digest_buffer(heap, digest_ref, &buffer)
}

fn allocate_duke_provider(heap: &mut duke_gc::Heap) -> u64 {
    let provider_ref = heap.allocate("java/security/Provider".to_string(), 0);
    if let Ok(provider) = heap.get_mut(provider_ref) {
        provider.string_value = Some(DUKE_SECURITY_PROVIDER.to_string());
    }
    provider_ref
}

fn allocate_provider_service(
    heap: &mut duke_gc::Heap,
    provider_ref: u64,
    service_type: &str,
    algorithm: &str,
) -> u64 {
    let type_ref = heap.allocate_string(service_type.to_string());
    let algorithm_ref = heap.allocate_string(algorithm.to_string());
    let service_ref = heap.allocate("java/security/Provider$Service".to_string(), 3);
    if let Ok(service) = heap.get_mut(service_ref) {
        service.fields[PROVIDER_SERVICE_PROVIDER_FIELD] = Slot::Reference(Some(provider_ref));
        service.fields[PROVIDER_SERVICE_TYPE_FIELD] = Slot::Reference(Some(type_ref));
        service.fields[PROVIDER_SERVICE_ALGORITHM_FIELD] = Slot::Reference(Some(algorithm_ref));
        service.string_value = Some(format!("{service_type}:{algorithm}"));
    }
    service_ref
}

fn allocate_algorithm_set(
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    algorithms: &[&str],
) -> Result<u64> {
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    for algorithm in algorithms {
        let algorithm_ref = heap.allocate_string((*algorithm).to_string());
        native_hashset_add(
            &[
                Slot::Reference(Some(set_ref)),
                Slot::Reference(Some(algorithm_ref)),
            ],
            heap,
            out,
            control,
        )?;
    }
    Ok(set_ref)
}





















const UUID_MSB_FIELD: usize = 0;
const UUID_LSB_FIELD: usize = 1;

fn uuid_bits_from_object(obj: &duke_gc::HeapObject) -> Option<(i64, i64)> {
    match (
        obj.fields.get(UUID_MSB_FIELD),
        obj.fields.get(UUID_LSB_FIELD),
    ) {
        (Some(Slot::Long(msb)), Some(Slot::Long(lsb))) => Some((*msb, *lsb)),
        _ => None,
    }
}

fn uuid_bits_from_ref(heap: &duke_gc::Heap, uuid_ref: u64) -> Result<(i64, i64)> {
    uuid_bits_from_object(heap.get(uuid_ref)?).ok_or(Error::TypeMismatch {
        expected: "UUID fields",
        got: "other",
    })
}

fn write_uuid_bits(heap: &mut duke_gc::Heap, uuid_ref: u64, msb: i64, lsb: i64) -> Result<()> {
    let uuid = heap.get_mut(uuid_ref)?;
    if uuid.fields.len() <= UUID_LSB_FIELD {
        uuid.fields.resize(UUID_LSB_FIELD + 1, Slot::Long(0));
    }
    uuid.fields[UUID_MSB_FIELD] = Slot::Long(msb);
    uuid.fields[UUID_LSB_FIELD] = Slot::Long(lsb);
    Ok(())
}

fn allocate_uuid(heap: &mut duke_gc::Heap, msb: i64, lsb: i64) -> Result<u64> {
    let uuid_ref = heap.allocate("java/util/UUID".to_string(), 2);
    write_uuid_bits(heap, uuid_ref, msb, lsb)?;
    Ok(uuid_ref)
}

fn uuid_bits_from_bytes(bytes: &[u8; 16]) -> (i64, i64) {
    let mut msb_bytes = [0_u8; 8];
    let mut lsb_bytes = [0_u8; 8];
    msb_bytes.copy_from_slice(&bytes[..8]);
    lsb_bytes.copy_from_slice(&bytes[8..]);
    (i64::from_be_bytes(msb_bytes), i64::from_be_bytes(lsb_bytes))
}

fn uuid_invalid_format() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    }
}

fn uuid_hex_nibble(byte: u8) -> Option<u64> {
    match byte {
        b'0'..=b'9' => Some(u64::from(byte - b'0')),
        b'a'..=b'f' => Some(u64::from(byte - b'a' + 10)),
        b'A'..=b'F' => Some(u64::from(byte - b'A' + 10)),
        _ => None,
    }
}

fn parse_uuid_hex_u64(bytes: &[u8]) -> Result<u64> {
    let mut value = 0_u64;
    for byte in bytes {
        let nibble = uuid_hex_nibble(*byte).ok_or_else(uuid_invalid_format)?;
        value = (value << 4) | nibble;
    }
    Ok(value)
}

fn parse_uuid_string(text: &str) -> Result<(i64, i64)> {
    let bytes = text.as_bytes();
    if bytes.len() != 36
        || bytes[8] != b'-'
        || bytes[13] != b'-'
        || bytes[18] != b'-'
        || bytes[23] != b'-'
    {
        return Err(uuid_invalid_format());
    }
    let msb = (parse_uuid_hex_u64(&bytes[0..8])? << 32)
        | (parse_uuid_hex_u64(&bytes[9..13])? << 16)
        | parse_uuid_hex_u64(&bytes[14..18])?;
    let lsb = (parse_uuid_hex_u64(&bytes[19..23])? << 48)
        | parse_uuid_hex_u64(&bytes[24..36])?;
    Ok((msb.cast_signed(), lsb.cast_signed()))
}

fn uuid_to_string(msb: i64, lsb: i64) -> String {
    let msb = msb.cast_unsigned();
    let lsb = lsb.cast_unsigned();
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (msb >> 32) & 0xffff_ffff,
        (msb >> 16) & 0xffff,
        msb & 0xffff,
        (lsb >> 48) & 0xffff,
        lsb & 0xffff_ffff_ffff
    )
}














// ---------------------------------------------------------------------------
// java.util.regex.Pattern / Matcher
// Pattern: string_value = regex string.
// Matcher: fields[0]=Pattern ref, fields[1]=input ref, fields[2]=pos,
//          fields[3]=match_start (-1=no match), fields[4]=match_end;
//          string_value = last matched text.
// ---------------------------------------------------------------------------

const PATTERN_UNIX_LINES: i32 = 1;
const PATTERN_CASE_INSENSITIVE: i32 = 2;
const PATTERN_COMMENTS: i32 = 4;
const PATTERN_MULTILINE: i32 = 8;
const PATTERN_LITERAL: i32 = 16;
const PATTERN_DOTALL: i32 = 32;
const PATTERN_UNICODE_CASE: i32 = 64;
const PATTERN_CANON_EQ: i32 = 128;
const PATTERN_UNICODE_CHARACTER_CLASS: i32 = 256;
const PATTERN_SUPPORTED_FLAGS: i32 = PATTERN_UNIX_LINES
    | PATTERN_CASE_INSENSITIVE
    | PATTERN_COMMENTS
    | PATTERN_MULTILINE
    | PATTERN_LITERAL
    | PATTERN_DOTALL
    | PATTERN_UNICODE_CASE
    | PATTERN_CANON_EQ
    | PATTERN_UNICODE_CHARACTER_CLASS;

const PATTERN_FLAGS_FIELD: usize = 0;

const MATCHER_PATTERN_FIELD: usize = 0;
const MATCHER_INPUT_FIELD: usize = 1;
const MATCHER_POS_FIELD: usize = 2;
const MATCHER_MATCH_START_FIELD: usize = 3;
const MATCHER_MATCH_END_FIELD: usize = 4;
const MATCHER_APPEND_POS_FIELD: usize = 5;
const MATCHER_FIELD_COUNT: usize = 6;

#[derive(Clone, Copy)]
enum MatcherGroup<'a> {
    Index(usize),
    Name(&'a str),
}

fn regex_java_exception(class_name: &str, message: impl Into<String>) -> Error {
    push_pending_java_exception_message(class_name, message.into());
    Error::JavaException {
        class_name: class_name.to_string(),
    }
}

fn regex_pattern_syntax_error(message: impl Into<String>) -> Error {
    regex_java_exception("java/util/regex/PatternSyntaxException", message)
}

fn regex_illegal_argument(message: impl Into<String>) -> Error {
    regex_java_exception("java/lang/IllegalArgumentException", message)
}

fn regex_illegal_state(message: impl Into<String>) -> Error {
    regex_java_exception("java/lang/IllegalStateException", message)
}

fn regex_index_out_of_bounds(message: impl Into<String>) -> Error {
    regex_java_exception("java/lang/IndexOutOfBoundsException", message)
}

fn validate_pattern_flags(flags: i32) -> Result<()> {
    if flags & !PATTERN_SUPPORTED_FLAGS != 0 {
        return Err(regex_illegal_argument(format!("Unknown regex flags: {flags}")));
    }
    if flags & PATTERN_CANON_EQ != 0 {
        return Err(regex_pattern_syntax_error(
            "CANON_EQ is not supported by Duke's regex engine",
        ));
    }
    Ok(())
}

fn translate_java_named_groups(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    let mut chars = pattern.chars();
    while let Some(ch) = chars.next() {
        if ch == '(' {
            let mut probe = chars.clone();
            if probe.next() == Some('?') && probe.next() == Some('<') {
                match probe.next() {
                    Some('=' | '!') | None => out.push(ch),
                    Some(_) => {
                        out.push_str("(?P<");
                        chars.next();
                        chars.next();
                    }
                }
            } else {
                out.push(ch);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn copy_group_name(chars: &[char], start: usize, out: &mut String) -> Option<usize> {
    let mut idx = start;
    while idx < chars.len() {
        let ch = chars[idx];
        out.push(ch);
        idx += 1;
        if ch == '>' {
            return Some(idx);
        }
    }
    None
}

fn expand_ascii_case_insensitive(pattern: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::with_capacity(pattern.len());
    let mut idx = 0;
    let mut escaped = false;
    let mut in_class = false;
    while idx < chars.len() {
        let ch = chars[idx];
        if escaped {
            out.push(ch);
            escaped = false;
            idx += 1;
            continue;
        }
        if ch == '\\' {
            out.push(ch);
            escaped = true;
            idx += 1;
            continue;
        }
        if !in_class && ch == '(' && chars.get(idx + 1) == Some(&'?') {
            if chars.get(idx + 2) == Some(&'P') && chars.get(idx + 3) == Some(&'<') {
                out.push_str("(?P<");
                if let Some(next_idx) = copy_group_name(&chars, idx + 4, &mut out) {
                    idx = next_idx;
                    continue;
                }
            } else if chars.get(idx + 2) == Some(&'<')
                && !matches!(chars.get(idx + 3), Some('=' | '!') | None)
            {
                out.push_str("(?<");
                if let Some(next_idx) = copy_group_name(&chars, idx + 3, &mut out) {
                    idx = next_idx;
                    continue;
                }
            }
        }
        match ch {
            '[' => {
                in_class = true;
                out.push(ch);
            }
            ']' if in_class => {
                in_class = false;
                out.push(ch);
            }
            _ if !in_class && ch.is_ascii_alphabetic() => {
                out.push('[');
                out.push(ch.to_ascii_lowercase());
                out.push(ch.to_ascii_uppercase());
                out.push(']');
            }
            _ => out.push(ch),
        }
        idx += 1;
    }
    out
}

/// Unicode block table: normalized Java `In…` name → inclusive codepoint range.
/// Seeded with the commons-lang3 blocker plus a few neighbours; extend as more
/// libraries demand blocks. `Character.UnicodeBlock.forName` normalizes names by
/// stripping spaces/hyphens/underscores and uppercasing, which `normalize_block_name`
/// mirrors, so every key here is already normalized.
const UNICODE_BLOCKS: &[(&str, u32, u32)] = &[
    ("COMBININGDIACRITICALMARKS", 0x0300, 0x036F),
    ("BASICLATIN", 0x0000, 0x007F),
    ("LATIN1SUPPLEMENT", 0x0080, 0x00FF),
    ("GREEKANDCOPTIC", 0x0370, 0x03FF),
    ("CYRILLIC", 0x0400, 0x04FF),
];

/// Normalize a Java Unicode block name the way `Character.UnicodeBlock.forName`
/// does: drop spaces, hyphens and underscores, then uppercase.
fn normalize_block_name(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_uppercase)
        .collect()
}

/// Translate one `\p{name}` / `\P{name}` property token into Rust regex syntax.
/// `negated` is true for `\P`; `in_class` is true when the token sits inside an
/// existing `[...]` class. Returns `Err` (a `PatternSyntaxException`) for an
/// unknown block name, matching real Java's `Pattern.compile`.
fn translate_one_property(name: &str, negated: bool, in_class: bool) -> Result<String> {
    if let Some(block) = name.strip_prefix("In") {
        // Unicode block: Rust has no block support, so expand to a codepoint range.
        let norm = normalize_block_name(block);
        let (start, end) = UNICODE_BLOCKS
            .iter()
            .find(|(key, _, _)| *key == norm)
            .map(|(_, start, end)| (*start, *end))
            .ok_or_else(|| {
                regex_pattern_syntax_error(format!("Unknown character block name {{{block}}}"))
            })?;
        let range = format!("\\x{{{start:04X}}}-\\x{{{end:04X}}}");
        if negated {
            // `[^...]` works both at top level and nested inside another class.
            Ok(format!("[^{range}]"))
        } else if in_class {
            Ok(range)
        } else {
            Ok(format!("[{range}]"))
        }
    } else if let Some(script) = name.strip_prefix("Is") {
        // Script or binary property: strip Java's `Is` and let Rust validate.
        let sigil = if negated { 'P' } else { 'p' };
        Ok(format!("\\{sigil}{{{script}}}"))
    } else {
        // General category / POSIX / anything else: Rust uses the same names.
        let sigil = if negated { 'P' } else { 'p' };
        Ok(format!("\\{sigil}{{{name}}}"))
    }
}

/// Rewrite Java `\p{…}` / `\P{…}` property classes into Rust-regex-compatible
/// syntax, translating Unicode blocks (`\p{InXxx}`) to explicit codepoint ranges
/// and stripping the `Is` script prefix. Escaping- and class-aware. Unknown
/// block names return `Err`, matching Java's `PatternSyntaxException`.
fn translate_property_classes(pattern: &str) -> Result<String> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::with_capacity(pattern.len());
    let mut i = 0;
    let mut class_depth: usize = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' {
            let next = chars.get(i + 1).copied();
            if matches!(next, Some('p' | 'P')) && chars.get(i + 2) == Some(&'{') {
                let sigil = next.expect("checked by matches!");
                let mut j = i + 3;
                let mut name = String::new();
                while j < chars.len() && chars[j] != '}' {
                    name.push(chars[j]);
                    j += 1;
                }
                if j >= chars.len() {
                    // Unterminated `{`; leave verbatim and let the regex builder report it.
                    out.push(c);
                    out.push(sigil);
                    out.push('{');
                    out.push_str(&name);
                    break;
                }
                let translated = translate_one_property(&name, sigil == 'P', class_depth > 0)?;
                out.push_str(&translated);
                i = j + 1;
                continue;
            }
            // Ordinary escape (including `\\`): copy the pair verbatim so an
            // escaped backslash before `p` is not mistaken for a property token.
            out.push(c);
            if let Some(n) = next {
                out.push(n);
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        match c {
            '[' => {
                class_depth += 1;
                out.push(c);
            }
            ']' if class_depth > 0 => {
                class_depth -= 1;
                out.push(c);
            }
            _ => out.push(c),
        }
        i += 1;
    }
    Ok(out)
}

/// Helper: compile a regex from a pattern string.
/// Returns `Err` with `JavaException` on bad pattern.
fn compile_java_regex(pattern: &str) -> Result<regex::Regex> {
    compile_java_regex_with_flags(pattern, 0)
}

fn compile_java_regex_with_flags(pattern: &str, flags: i32) -> Result<regex::Regex> {
    validate_pattern_flags(flags)?;
    let mut source = if flags & PATTERN_LITERAL != 0 {
        regex::escape(pattern)
    } else {
        let named = translate_java_named_groups(pattern);
        translate_property_classes(&named)?
    };
    let ascii_case_insensitive =
        flags & PATTERN_CASE_INSENSITIVE != 0 && flags & PATTERN_UNICODE_CASE == 0;
    if ascii_case_insensitive {
        source = expand_ascii_case_insensitive(&source);
    }

    let mut builder = regex::RegexBuilder::new(&source);
    builder
        .case_insensitive(flags & PATTERN_CASE_INSENSITIVE != 0 && !ascii_case_insensitive)
        .multi_line(flags & PATTERN_MULTILINE != 0)
        .dot_matches_new_line(flags & PATTERN_DOTALL != 0)
        .ignore_whitespace(flags & PATTERN_COMMENTS != 0)
        .unicode(true);
    builder.build().map_err(|e| regex_pattern_syntax_error(e.to_string()))
}

fn pattern_text_and_flags(heap: &duke_gc::Heap, pat_ref: u64) -> Result<(String, i32)> {
    let pat = heap.get(pat_ref)?;
    let pattern_str = pat.string_value.clone().unwrap_or_default();
    let flags = match pat.fields.get(PATTERN_FLAGS_FIELD).copied() {
        Some(Slot::Int(flags)) => flags,
        _ => 0,
    };
    Ok((pattern_str, flags))
}

fn allocate_pattern(heap: &mut duke_gc::Heap, pattern_str: String, flags: i32) -> Result<u64> {
    let pat_ref = heap.allocate("java/util/regex/Pattern".to_string(), 1);
    let pat = heap.get_mut(pat_ref)?;
    pat.fields[PATTERN_FLAGS_FIELD] = Slot::Int(flags);
    pat.string_value = Some(pattern_str);
    Ok(pat_ref)
}





fn regex_split_parts(re: &regex::Regex, input: &str, limit: i32) -> Vec<String> {
    if limit > 0 {
        return re
            .splitn(input, usize::try_from(limit).unwrap_or(usize::MAX))
            .map(str::to_string)
            .collect();
    }
    let mut parts: Vec<String> = re.split(input).map(str::to_string).collect();
    if limit == 0 {
        while parts.last().is_some_and(String::is_empty) {
            parts.pop();
        }
    }
    parts
}

fn alloc_string_array_from_parts(heap: &mut duke_gc::Heap, parts: &[String]) -> Result<u64> {
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), parts.len());
    for (idx, part) in parts.iter().enumerate() {
        let str_ref = heap.allocate_string(part.clone());
        heap.get_mut(arr_ref)?.fields[idx] = Slot::Reference(Some(str_ref));
    }
    Ok(arr_ref)
}

fn pattern_split_impl(args: &[Slot], heap: &mut duke_gc::Heap, limit: i32) -> Result<Option<Slot>> {
    let pat_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = charsequence_chars(heap, input_ref)?.unwrap_or_default();
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let parts = regex_split_parts(&re, &input, limit);
    let arr_ref = alloc_string_array_from_parts(heap, &parts)?;
    Ok(Some(Slot::Reference(Some(arr_ref))))
}



fn matcher_pattern_input(heap: &duke_gc::Heap, m_ref: u64) -> Result<Option<(u64, u64)>> {
    let fields = &heap.get(m_ref)?.fields;
    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
        return Ok(None);
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
        return Ok(None);
    };
    Ok(Some((pat_ref, input_ref)))
}

fn matcher_pattern_input_text(
    heap: &duke_gc::Heap,
    m_ref: u64,
) -> Result<Option<(String, i32, String)>> {
    let Some((pat_ref, input_ref)) = matcher_pattern_input(heap, m_ref)? else {
        return Ok(None);
    };
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = charsequence_chars(heap, input_ref)?.unwrap_or_default();
    Ok(Some((pattern_str, flags, input)))
}

fn matcher_field_int(heap: &duke_gc::Heap, m_ref: u64, field: usize, default: i32) -> Result<i32> {
    Ok(match heap.get(m_ref)?.fields.get(field).copied() {
        Some(Slot::Int(value)) => value,
        _ => default,
    })
}

fn set_matcher_no_match(heap: &mut duke_gc::Heap, m_ref: u64) -> Result<()> {
    let matcher = heap.get_mut(m_ref)?;
    matcher.fields[MATCHER_MATCH_START_FIELD] = Slot::Int(-1);
    matcher.fields[MATCHER_MATCH_END_FIELD] = Slot::Int(0);
    matcher.string_value = None;
    Ok(())
}

fn reset_matcher_fields(heap: &mut duke_gc::Heap, m_ref: u64) -> Result<()> {
    let matcher = heap.get_mut(m_ref)?;
    matcher.fields[MATCHER_POS_FIELD] = Slot::Int(0);
    matcher.fields[MATCHER_MATCH_START_FIELD] = Slot::Int(-1);
    matcher.fields[MATCHER_MATCH_END_FIELD] = Slot::Int(0);
    matcher.fields[MATCHER_APPEND_POS_FIELD] = Slot::Int(0);
    matcher.string_value = None;
    Ok(())
}

fn next_find_pos(input: &str, start: usize, end: usize) -> usize {
    if start != end || end >= input.len() {
        return end;
    }
    let mut safe_end = end;
    while safe_end < input.len() && !input.is_char_boundary(safe_end) {
        safe_end += 1;
    }
    input[safe_end..]
        .chars()
        .next()
        .map_or(safe_end, |ch| safe_end + ch.len_utf8())
}

fn store_matcher_match(
    heap: &mut duke_gc::Heap,
    m_ref: u64,
    input: &str,
    start: usize,
    end: usize,
) -> Result<()> {
    let next_pos = next_find_pos(input, start, end);
    let start_i32 = i32::try_from(start).unwrap_or(i32::MAX);
    let end_i32 = i32::try_from(end).unwrap_or(i32::MAX);
    let next_i32 = i32::try_from(next_pos).unwrap_or(i32::MAX);
    let matcher = heap.get_mut(m_ref)?;
    matcher.fields[MATCHER_POS_FIELD] = Slot::Int(next_i32);
    matcher.fields[MATCHER_MATCH_START_FIELD] = Slot::Int(start_i32);
    matcher.fields[MATCHER_MATCH_END_FIELD] = Slot::Int(end_i32);
    matcher.string_value = Some(input[start..end].to_string());
    Ok(())
}

fn last_match_bounds(heap: &duke_gc::Heap, m_ref: u64) -> Result<(usize, usize)> {
    let start = matcher_field_int(heap, m_ref, MATCHER_MATCH_START_FIELD, -1)?;
    let end = matcher_field_int(heap, m_ref, MATCHER_MATCH_END_FIELD, 0)?;
    if start < 0 {
        return Err(regex_illegal_state("No match available"));
    }
    Ok((
        usize::try_from(start).unwrap_or(0),
        usize::try_from(end.max(0)).unwrap_or(0),
    ))
}

/// Convert a UTF-8 byte offset within `input` to a UTF-16 code-unit offset.
/// The `regex` crate yields byte offsets, but every match index Java's `Matcher`
/// exposes (`start()`/`end()`) is a UTF-16 code-unit offset. They coincide for
/// ASCII but diverge for multi-byte characters (e.g. combining marks matched by
/// `\p{InCombiningDiacriticalMarks}`), so the Java-visible getters must convert.
fn byte_to_utf16_index(input: &str, byte_idx: usize) -> usize {
    let clamped = byte_idx.min(input.len());
    input[..clamped].chars().map(char::len_utf16).sum()
}

/// Like `matcher_group_bounds`, but returns bounds as UTF-16 code-unit offsets so
/// they match Java's `Matcher.start()`/`end()` semantics. Used only by the
/// Java-visible index getters; internal consumers keep byte offsets.
fn matcher_group_bounds_java(
    heap: &duke_gc::Heap,
    m_ref: u64,
    group: MatcherGroup<'_>,
) -> Result<Option<(usize, usize)>> {
    let Some((start, end)) = matcher_group_bounds(heap, m_ref, group)? else {
        return Ok(None);
    };
    let Some((_, _, input)) = matcher_pattern_input_text(heap, m_ref)? else {
        return Ok(Some((start, end)));
    };
    Ok(Some((
        byte_to_utf16_index(&input, start),
        byte_to_utf16_index(&input, end),
    )))
}

fn matcher_group_bounds(
    heap: &duke_gc::Heap,
    m_ref: u64,
    group: MatcherGroup<'_>,
) -> Result<Option<(usize, usize)>> {
    let (match_start, match_end) = last_match_bounds(heap, m_ref)?;
    let Some((pattern_str, flags, input)) = matcher_pattern_input_text(heap, m_ref)? else {
        return Err(regex_illegal_state("No match available"));
    };
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let Some(caps) = re.captures_at(&input, match_start) else {
        return Err(regex_illegal_state("No match available"));
    };
    let Some(whole) = caps.get(0) else {
        return Err(regex_illegal_state("No match available"));
    };
    if whole.start() != match_start || whole.end() != match_end {
        return Err(regex_illegal_state("No match available"));
    }
    match group {
        MatcherGroup::Index(index) => {
            if index >= caps.len() {
                return Err(regex_index_out_of_bounds(format!("No group {index}")));
            }
            Ok(caps.get(index).map(|m| (m.start(), m.end())))
        }
        MatcherGroup::Name(name) => {
            if !re.capture_names().flatten().any(|candidate| candidate == name) {
                return Err(regex_illegal_argument(format!(
                    "No group with name <{name}>"
                )));
            }
            Ok(caps.name(name).map(|m| (m.start(), m.end())))
        }
    }
}

fn matcher_group_text(
    heap: &duke_gc::Heap,
    m_ref: u64,
    group: MatcherGroup<'_>,
) -> Result<Option<String>> {
    let Some((_, _, input)) = matcher_pattern_input_text(heap, m_ref)? else {
        return Err(regex_illegal_state("No match available"));
    };
    Ok(matcher_group_bounds(heap, m_ref, group)?.map(|(start, end)| input[start..end].to_string()))
}













fn append_to_string_builder(heap: &mut duke_gc::Heap, builder_ref: u64, text: &str) -> Result<()> {
    let builder = heap.get_mut(builder_ref)?;
    builder
        .string_value
        .get_or_insert_with(String::new)
        .push_str(text);
    Ok(())
}








// ---------------------------------------------------------------------------
// Optional extensions (callback-based)
// ---------------------------------------------------------------------------

/// Native: `Optional.map(Function)Optional` — maps value if present.
pub(crate) fn native_optional_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
    let result_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if matches!(value, Slot::Reference(None)) {
        // empty — propagate empty
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    }
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mapped = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, value],
    )?;
    heap.get_mut(result_ref)?.fields[0] = mapped.unwrap_or(Slot::Reference(None));
    Ok(Some(Slot::Reference(Some(result_ref))))
}





// ---------------------------------------------------------------------------
// HashMap extensions: compute, merge
// ---------------------------------------------------------------------------

/// Helper: find key index in `HashMap` fields (`fields[0]`=size, `fields[1,3,5..]`=keys, `fields[2,4,6..]`=vals).
fn hashmap_find_key(fields: &[Slot], key: Slot, heap: &duke_gc::Heap) -> Option<usize> {
    let size = match fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return None,
    };
    for i in 0..size {
        let key_slot = fields.get(1 + i * 2)?;
        if slots_equal(key_slot, &key, heap) {
            return Some(1 + i * 2);
        }
    }
    None
}



// ---------------------------------------------------------------------------
// java.lang.StringBuffer — mutable string, delegates to StringBuilder internals
// (string_value field used as buffer, same as StringBuilder)
// ---------------------------------------------------------------------------






// ---------------------------------------------------------------------------
// java.util.StringJoiner
// fields[0] = Reference(delimiter), fields[1] = Reference(prefix), fields[2] = Reference(suffix)
// fields[3] = Reference(emptyValue), fields[4..] = added elements
// instance_field_count = 4 (slots 0-3 pre-allocated)
// ---------------------------------------------------------------------------





/// Helper: read a string from a slot (returns "" for null).
///
/// `CharSequence`-polymorphic: a real-layout `java/lang/String` is decoded from
/// slot 0 via [`charsequence_chars`]; other char backings use their
/// `string_value` buffer.
fn slot_to_string(slot: Slot, heap: &duke_gc::Heap) -> String {
    match slot {
        Slot::Reference(Some(r)) => charsequence_chars(heap, r).ok().flatten().unwrap_or_default(),
        _ => String::new(),
    }
}



// ---------------------------------------------------------------------------
// HashMap natives
// ---------------------------------------------------------------------------

fn uses_first_field_value_equality(class_name: &str) -> bool {
    matches!(
        class_name,
        "java/lang/Boolean"
            | "java/lang/Byte"
            | "java/lang/Character"
            | "java/lang/Double"
            | "java/lang/Float"
            | "java/lang/Integer"
            | "java/lang/Long"
            | "java/lang/Short"
    )
}

/// Semantic equality for `HashMap` keys: reference identity by default, with value
/// semantics for strings, class mirrors, and boxed primitive wrappers.
fn slots_equal(a: &Slot, b: &Slot, heap: &duke_gc::Heap) -> bool {
    match (a, b) {
        (Slot::Reference(None), Slot::Reference(None)) => true,
        (Slot::Reference(Some(ra)), Slot::Reference(Some(rb))) => {
            if ra == rb {
                return true;
            }
            let Ok(oa) = heap.get(*ra) else { return false };
            let Ok(ob) = heap.get(*rb) else { return false };
            if oa.class_name != ob.class_name {
                return false;
            }
            match oa.class_name.as_str() {
                // Real-layout String content lives in slot 0, so compare the
                // decoded chars rather than the `string_value` side-channel.
                "java/lang/String" => {
                    read_string_bytes(heap, *ra).ok() == read_string_bytes(heap, *rb).ok()
                }
                "java/lang/Class" => oa.string_value == ob.string_value,
                "java/util/UUID" => uuid_bits_from_object(oa) == uuid_bits_from_object(ob),
                class_name if uses_first_field_value_equality(class_name) => {
                    oa.fields.first() == ob.fields.first()
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// Native: `HashMap.<init>()V` — initialises size counter at fields\[0\] to 0.
fn find_hashmap_entry_index(fields: &[Slot], key: &Slot, heap: &duke_gc::Heap) -> Option<usize> {
    (1..fields.len())
        .step_by(2)
        .find(|&i| i + 1 < fields.len() && slots_equal(&fields[i], key, heap))
}









// ---------------------------------------------------------------------------
// java.util.Properties natives
// ---------------------------------------------------------------------------

const PROPERTIES_SIZE_FIELD: usize = 0;
const PROPERTIES_DEFAULTS_FIELD: usize = 1;
const PROPERTIES_ENTRIES_START: usize = 2;
const PROPERTIES_ENUM_INDEX_FIELD: usize = 0;
const PROPERTIES_ENUM_COUNT_FIELD: usize = 1;
const PROPERTIES_ENUM_NAMES_START: usize = 2;
const PROPERTIES_STORE_TIMESTAMP: &str = "1970-01-01T00:00:00Z";

fn properties_illegal_argument() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    }
}

fn properties_io_exception() -> Error {
    Error::JavaException {
        class_name: "java/io/IOException".to_string(),
    }
}

fn properties_stream_id_from_slot(slot: Slot, heap: &duke_gc::Heap) -> Result<i32> {
    let Slot::Reference(Some(stream_ref)) = slot else {
        return Err(Error::NullPointerException);
    };
    match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(properties_io_exception()),
    }
}

fn ensure_properties_layout(heap: &mut duke_gc::Heap, props_ref: u64) -> Result<()> {
    let props = heap.get_mut(props_ref)?;
    if props.fields.len() < PROPERTIES_ENTRIES_START {
        props
            .fields
            .resize(PROPERTIES_ENTRIES_START, Slot::Reference(None));
    }
    Ok(())
}

fn init_properties_with_defaults(
    heap: &mut duke_gc::Heap,
    props_ref: u64,
    defaults: Slot,
) -> Result<()> {
    ensure_properties_layout(heap, props_ref)?;
    let props = heap.get_mut(props_ref)?;
    props.fields[PROPERTIES_SIZE_FIELD] = Slot::Int(0);
    props.fields[PROPERTIES_DEFAULTS_FIELD] = defaults;
    props.fields.truncate(PROPERTIES_ENTRIES_START);
    Ok(())
}

fn properties_entry_index(fields: &[Slot], key: &Slot, heap: &duke_gc::Heap) -> Option<usize> {
    (PROPERTIES_ENTRIES_START..fields.len())
        .step_by(2)
        .find(|&idx| idx + 1 < fields.len() && slots_equal(&fields[idx], key, heap))
}

fn slot_is_java_string(heap: &duke_gc::Heap, slot: Slot) -> bool {
    let Slot::Reference(Some(string_ref)) = slot else {
        return false;
    };
    heap.get(string_ref).is_ok_and(|obj| {
        // A real-layout String carries its content in slot 0 (`value:[B`);
        // detect it there instead of via the `string_value` side-channel.
        obj.class_name == "java/lang/String"
            && matches!(obj.fields.first(), Some(Slot::Reference(Some(_))))
    })
}

fn string_from_slot(heap: &duke_gc::Heap, slot: Slot) -> Option<String> {
    let Slot::Reference(Some(string_ref)) = slot else {
        return None;
    };
    charsequence_chars(heap, string_ref).ok().flatten()
}

fn properties_local_entries(heap: &duke_gc::Heap, props_ref: u64) -> Result<Vec<(Slot, Slot)>> {
    let fields = heap.get(props_ref)?.fields.clone();
    let mut entries = Vec::new();
    let mut idx = PROPERTIES_ENTRIES_START;
    while idx + 1 < fields.len() {
        entries.push((fields[idx], fields[idx + 1]));
        idx += 2;
    }
    Ok(entries)
}

fn properties_local_string_entries(
    heap: &duke_gc::Heap,
    props_ref: u64,
) -> Result<Vec<(String, String)>> {
    let mut entries = Vec::new();
    for (key_slot, value_slot) in properties_local_entries(heap, props_ref)? {
        let Some(key) = string_from_slot(heap, key_slot) else {
            continue;
        };
        let Some(value) = string_from_slot(heap, value_slot) else {
            continue;
        };
        entries.push((key, value));
    }
    Ok(entries)
}

fn properties_put_slots(
    heap: &mut duke_gc::Heap,
    props_ref: u64,
    key: Slot,
    value: Slot,
) -> Result<Slot> {
    ensure_properties_layout(heap, props_ref)?;
    let entry_idx = {
        let fields = &heap.get(props_ref)?.fields;
        properties_entry_index(fields, &key, heap)
    };

    if let Some(idx) = entry_idx {
        let old = heap.get(props_ref)?.fields[idx + 1];
        heap.get_mut(props_ref)?.fields[idx + 1] = value;
        return Ok(old);
    }

    let props = heap.get_mut(props_ref)?;
    match props.fields.get_mut(PROPERTIES_SIZE_FIELD) {
        Some(Slot::Int(size)) => *size += 1,
        _ => props.fields[PROPERTIES_SIZE_FIELD] = Slot::Int(1),
    }
    props.fields.push(key);
    props.fields.push(value);
    Ok(Slot::Reference(None))
}

fn properties_put_string_pair(
    heap: &mut duke_gc::Heap,
    props_ref: u64,
    key: String,
    value: String,
) -> Result<()> {
    let key_slot = Slot::Reference(Some(heap.allocate_string(key)));
    let value_slot = Slot::Reference(Some(heap.allocate_string(value)));
    properties_put_slots(heap, props_ref, key_slot, value_slot)?;
    Ok(())
}

fn properties_get_property_slot(
    heap: &duke_gc::Heap,
    props_ref: u64,
    key: Slot,
) -> Result<Slot> {
    let fields = heap.get(props_ref)?.fields.clone();
    if let Some(idx) = properties_entry_index(&fields, &key, heap) {
        let value = fields[idx + 1];
        if slot_is_java_string(heap, value) {
            return Ok(value);
        }
    }

    match fields.get(PROPERTIES_DEFAULTS_FIELD).copied() {
        Some(Slot::Reference(Some(defaults_ref))) => {
            properties_get_property_slot(heap, defaults_ref, key)
        }
        _ => Ok(Slot::Reference(None)),
    }
}

fn properties_push_unique_name(heap: &duke_gc::Heap, names: &mut Vec<Slot>, key: Slot) {
    if !slot_is_java_string(heap, key) {
        return;
    }
    if names.iter().any(|existing| slots_equal(existing, &key, heap)) {
        return;
    }
    names.push(key);
}

fn properties_collect_name_slots(
    heap: &duke_gc::Heap,
    props_ref: u64,
    names: &mut Vec<Slot>,
) -> Result<()> {
    let fields = heap.get(props_ref)?.fields.clone();
    if let Some(Slot::Reference(Some(defaults_ref))) = fields.get(PROPERTIES_DEFAULTS_FIELD) {
        properties_collect_name_slots(heap, *defaults_ref, names)?;
    }

    let mut idx = PROPERTIES_ENTRIES_START;
    while idx + 1 < fields.len() {
        let key = fields[idx];
        let value = fields[idx + 1];
        if slot_is_java_string(heap, value) {
            properties_push_unique_name(heap, names, key);
        }
        idx += 2;
    }
    Ok(())
}

const fn is_properties_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\u{000c}')
}

fn split_properties_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if matches!(chars.peek(), Some('\n')) {
                    chars.next();
                }
                lines.push(std::mem::take(&mut current));
            }
            '\n' => lines.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    lines.push(current);
    lines
}

fn has_odd_trailing_backslashes(line: &str) -> bool {
    let mut count = 0usize;
    for ch in line.chars().rev() {
        if ch != '\\' {
            break;
        }
        count += 1;
    }
    count % 2 == 1
}

fn logical_properties_lines(text: &str) -> Vec<String> {
    let mut logical = Vec::new();
    let mut pending = String::new();
    let mut continuing = false;

    for line in split_properties_lines(text) {
        let mut piece = if continuing {
            line.trim_start_matches(is_properties_whitespace).to_string()
        } else {
            line
        };

        if continuing && piece.is_empty() {
            continue;
        }

        if has_odd_trailing_backslashes(&piece) {
            piece.pop();
            pending.push_str(&piece);
            continuing = true;
        } else {
            pending.push_str(&piece);
            logical.push(std::mem::take(&mut pending));
            continuing = false;
        }
    }

    if continuing || !pending.is_empty() {
        logical.push(pending);
    }
    logical
}

fn hex_value(ch: char) -> Option<u32> {
    match ch {
        '0'..='9' => Some(u32::from(ch) - u32::from('0')),
        'a'..='f' => Some(u32::from(ch) - u32::from('a') + 10),
        'A'..='F' => Some(u32::from(ch) - u32::from('A') + 10),
        _ => None,
    }
}

fn unescape_property_text(raw: &str) -> Result<String> {
    let mut result = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        let Some(escaped) = chars.next() else {
            result.push('\\');
            break;
        };

        match escaped {
            'n' => result.push('\n'),
            'r' => result.push('\r'),
            't' => result.push('\t'),
            'f' => result.push('\u{000c}'),
            'u' => {
                let mut code = 0_u32;
                for _ in 0..4 {
                    let Some(hex) = chars.next().and_then(hex_value) else {
                        return Err(properties_illegal_argument());
                    };
                    code = (code << 4) | hex;
                }
                let Some(decoded) = char::from_u32(code) else {
                    return Err(properties_illegal_argument());
                };
                result.push(decoded);
            }
            other => result.push(other),
        }
    }
    Ok(result)
}

fn parse_property_logical_line(line: &str) -> Result<Option<(String, String)>> {
    let chars: Vec<char> = line.chars().collect();
    let mut idx = 0usize;
    while idx < chars.len() && is_properties_whitespace(chars[idx]) {
        idx += 1;
    }
    if idx >= chars.len() || matches!(chars[idx], '#' | '!') {
        return Ok(None);
    }

    let key_start = idx;
    let mut escaped = false;
    while idx < chars.len() {
        let ch = chars[idx];
        if escaped {
            escaped = false;
            idx += 1;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            idx += 1;
            continue;
        }
        if matches!(ch, '=' | ':') || is_properties_whitespace(ch) {
            break;
        }
        idx += 1;
    }

    let key_end = idx;
    let value_start = if idx < chars.len() {
        if is_properties_whitespace(chars[idx]) {
            while idx < chars.len() && is_properties_whitespace(chars[idx]) {
                idx += 1;
            }
            if idx < chars.len() && matches!(chars[idx], '=' | ':') {
                idx += 1;
            }
        } else {
            idx += 1;
        }
        while idx < chars.len() && is_properties_whitespace(chars[idx]) {
            idx += 1;
        }
        idx
    } else {
        chars.len()
    };

    let raw_key: String = chars[key_start..key_end].iter().collect();
    let raw_value: String = chars[value_start..].iter().collect();
    Ok(Some((
        unescape_property_text(&raw_key)?,
        unescape_property_text(&raw_value)?,
    )))
}

fn parse_properties_bytes(bytes: &[u8]) -> Result<Vec<(String, String)>> {
    let text: String = bytes.iter().map(|byte| char::from(*byte)).collect();
    let mut entries = Vec::new();
    for line in logical_properties_lines(&text) {
        if let Some((key, value)) = parse_property_logical_line(&line)? {
            entries.push((key, value));
        }
    }
    Ok(entries)
}

fn push_u16_escape(out: &mut String, unit: u16) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push('\\');
    out.push('u');
    for shift in [12, 8, 4, 0] {
        let idx = usize::from((unit >> shift) & 0x000f);
        out.push(char::from(HEX[idx]));
    }
}

fn push_unicode_escape(out: &mut String, ch: char) {
    let mut encoded = [0_u16; 2];
    for unit in ch.encode_utf16(&mut encoded).iter().copied() {
        push_u16_escape(out, unit);
    }
}

fn push_escaped_key_char(out: &mut String, ch: char) {
    match ch {
        '\\' => out.push_str("\\\\"),
        '=' => out.push_str("\\="),
        ':' => out.push_str("\\:"),
        ' ' => out.push_str("\\ "),
        '#' => out.push_str("\\#"),
        '!' => out.push_str("\\!"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        '\u{000c}' => out.push_str("\\f"),
        _ if !ch.is_ascii() || u32::from(ch) < 0x20 || u32::from(ch) > 0x7e => {
            push_unicode_escape(out, ch);
        }
        _ => out.push(ch),
    }
}

fn escape_property_key(key: &str) -> String {
    let mut result = String::new();
    for ch in key.chars() {
        push_escaped_key_char(&mut result, ch);
    }
    result
}

fn escape_property_value(value: &str) -> String {
    let mut result = String::new();
    for (idx, ch) in value.chars().enumerate() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{000c}' => result.push_str("\\f"),
            ' ' if idx == 0 => result.push_str("\\ "),
            _ if !ch.is_ascii() || u32::from(ch) < 0x20 || u32::from(ch) > 0x7e => {
                push_unicode_escape(&mut result, ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

fn escape_property_comment(comment: &str) -> String {
    let mut result = String::new();
    for ch in comment.chars() {
        match ch {
            '\n' | '\r' => {}
            _ if !ch.is_ascii() || u32::from(ch) < 0x20 || u32::from(ch) > 0x7e => {
                push_unicode_escape(&mut result, ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

fn append_store_comment(out: &mut String, comment: &str) {
    for line in split_properties_lines(comment) {
        if line.is_empty() {
            out.push_str("#\n");
        } else {
            out.push_str("# ");
            out.push_str(&escape_property_comment(&line));
            out.push('\n');
        }
    }
}

fn render_properties_store(heap: &duke_gc::Heap, props_ref: u64, comment: Option<&str>) -> Result<String> {
    let mut output = String::new();
    if let Some(comment_text) = comment {
        append_store_comment(&mut output, comment_text);
    }
    output.push_str("# ");
    output.push_str(PROPERTIES_STORE_TIMESTAMP);
    output.push('\n');

    for (key, value) in properties_local_string_entries(heap, props_ref)? {
        output.push_str(&escape_property_key(&key));
        output.push('=');
        output.push_str(&escape_property_value(&value));
        output.push('\n');
    }
    Ok(output)
}

fn write_iso_8859_1_ascii(heap: &mut duke_gc::Heap, file_id: i32, text: &str) -> Result<()> {
    for byte in text.bytes() {
        heap.write_host_file_byte(file_id, i32::from(byte))?;
    }
    Ok(())
}





















// ---------------------------------------------------------------------------
// ConcurrentHashMap natives
// ---------------------------------------------------------------------------

fn concurrent_hashmap_lock(
    heap: &mut duke_gc::Heap,
    this_ref: u64,
) -> Result<std::sync::Arc<std::sync::Mutex<()>>> {
    let obj = heap.get_mut(this_ref)?;
    if let Some(payload) = &obj.atomic_payload {
        return match payload {
            duke_gc::AtomicPayload::ConcurrentMapLock(lock) => Ok(std::sync::Arc::clone(lock)),
            duke_gc::AtomicPayload::Int(_)
            | duke_gc::AtomicPayload::Long(_)
            | duke_gc::AtomicPayload::Bool(_)
            | duke_gc::AtomicPayload::Reference(_)
            | duke_gc::AtomicPayload::ReentrantLock(_)
            | duke_gc::AtomicPayload::Condition(_)
            | duke_gc::AtomicPayload::ReadWriteLock(_)
            | duke_gc::AtomicPayload::ReadWriteLockView { .. }
            | duke_gc::AtomicPayload::Executor(_)
            | duke_gc::AtomicPayload::CountDownLatch(_)
            | duke_gc::AtomicPayload::Semaphore(_)
            | duke_gc::AtomicPayload::CyclicBarrier(_) => Err(Error::TypeMismatch {
                expected: "concurrent map lock",
                got: "other",
            }),
        };
    }

    let lock = std::sync::Arc::new(std::sync::Mutex::new(()));
    obj.atomic_payload = Some(duke_gc::AtomicPayload::ConcurrentMapLock(
        std::sync::Arc::clone(&lock),
    ));
    Ok(lock)
}

fn concurrent_hashmap_guard(
    lock: &std::sync::Arc<std::sync::Mutex<()>>,
) -> std::sync::MutexGuard<'_, ()> {
    lock.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

const fn require_chm_non_null(slot: Slot) -> Result<Slot> {
    if matches!(slot, Slot::Reference(None)) {
        Err(Error::NullPointerException)
    } else {
        Ok(slot)
    }
}

fn chm_non_null_arg(args: &[Slot], idx: usize) -> Result<Slot> {
    require_chm_non_null(extract_slot_arg(args, idx))
}

fn chm_entry_snapshot(heap: &duke_gc::Heap, map_ref: u64) -> Result<Vec<(Slot, Slot)>> {
    let fields = heap.get(map_ref)?.fields.clone();
    let mut entries = Vec::with_capacity(fields.len().saturating_sub(1) / 2);
    let mut i = 1usize;
    while i + 1 < fields.len() {
        entries.push((fields[i], fields[i + 1]));
        i += 2;
    }
    Ok(entries)
}

























// ---------------------------------------------------------------------------
// HashSet natives
// ---------------------------------------------------------------------------


fn find_hashset_entry_index(
    fields: &[Slot],
    element: &Slot,
    heap: &duke_gc::Heap,
) -> Option<usize> {
    fields
        .iter()
        .skip(1)
        .position(|field| slots_equal(field, element, heap))
        .map(|idx| idx + 1)
}
















const PROCESS_ID_FIELD: usize = 0;
const PROCESS_STDIN_FIELD: usize = 1;
const PROCESS_STDOUT_FIELD: usize = 2;
const PROCESS_STDERR_FIELD: usize = 3;

fn string_array_from_slot(slot: Slot, heap: &duke_gc::Heap) -> Result<Vec<String>> {
    let Slot::Reference(Some(array_ref)) = slot else {
        return Err(Error::NullPointerException);
    };
    let elements = heap.get(array_ref)?.fields.clone();
    elements
        .into_iter()
        .map(|element| match element {
            Slot::Reference(Some(string_ref)) => string_value_from_ref(heap, string_ref),
            _ => Err(Error::NullPointerException),
        })
        .collect()
}

fn optional_file_path_from_slot(
    slot: Slot,
    heap: &duke_gc::Heap,
) -> Result<Option<std::path::PathBuf>> {
    match slot {
        Slot::Reference(Some(file_ref)) => Ok(Some(file_path_from_ref(file_ref, heap)?)),
        Slot::Reference(None) => Ok(None),
        _ => Err(Error::NullPointerException),
    }
}

fn allocate_process_impl(
    heap: &mut duke_gc::Heap,
    ids: duke_gc::SpawnedProcessIds,
) -> Result<Option<Slot>> {
    let process_ref = heap.allocate("java/lang/ProcessImpl".to_string(), 4);
    let process_obj = heap.get_mut(process_ref)?;
    process_obj.fields[PROCESS_ID_FIELD] = Slot::Int(ids.process_id);
    process_obj.fields[PROCESS_STDIN_FIELD] = Slot::Int(ids.stdin_id);
    process_obj.fields[PROCESS_STDOUT_FIELD] = Slot::Int(ids.stdout_id);
    process_obj.fields[PROCESS_STDERR_FIELD] = Slot::Int(ids.stderr_id);
    Ok(Some(Slot::Reference(Some(process_ref))))
}

fn spawn_process_impl(
    heap: &mut duke_gc::Heap,
    command: &[String],
    cwd: Option<&std::path::Path>,
) -> Result<Option<Slot>> {
    let ids = heap.spawn_host_process(command, cwd)?;
    allocate_process_impl(heap, ids)
}

fn process_field_id_from_this(
    args: &[Slot],
    heap: &duke_gc::Heap,
    field_idx: usize,
) -> Result<i32> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.get(field_idx) {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(Error::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

fn allocate_process_stream(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    handle_id: i32,
) -> Result<Option<Slot>> {
    let stream_ref = heap.allocate(class_name.to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(handle_id);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}














// ---------------------------------------------------------------------------
// GC root gathering
// ---------------------------------------------------------------------------

thread_local! {
    /// Shadow stack of pointers to every [`ExecutionState`] whose `run_execution`
    /// is currently active on the Rust call stack, ordered outermost-first with
    /// the innermost/current state last. Entries are pushed on entry to
    /// `run_execution` (via [`RootProviderGuard`]) and popped on return.
    ///
    /// This exists because class initialization re-enters the interpreter with a
    /// FRESH `ExecutionState` (`prepare_execution_state` → nested `run_execution`)
    /// that shares the heap but has no link to the caller's frame. Without this,
    /// a GC fired inside a nested `<clinit>` would only see the innermost state's
    /// frame and reclaim objects the suspended caller still holds in its locals.
    static GC_ROOT_PROVIDERS: std::cell::RefCell<Vec<*mut ExecutionState>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// RAII guard that registers the currently-executing [`ExecutionState`] as a GC
/// root provider for the duration of a `run_execution` call, so that a GC fired
/// inside a NESTED `run_execution` also scans this (now suspended) state's frame
/// + call stack + interned constants.
struct RootProviderGuard;

impl RootProviderGuard {
    /// Register `state` as an active root provider until the returned guard is
    /// dropped.
    ///
    /// # Invariant / soundness
    /// `state` must point to an `ExecutionState` that stays live and unmoved for
    /// the whole lifetime of the returned guard. This holds by construction: the
    /// only caller passes the `&mut ExecutionState` argument of the enclosing
    /// `run_execution`, which owns the state for strictly longer than the guard.
    /// While a nested `run_execution` runs, this state is suspended on the Rust
    /// stack and its `&mut` borrows are inactive, so a GC in the nested call may
    /// read it in [`gather_roots`] and patch it in [`patch_forwarded_slots`]
    /// through the pointer without aliasing a live borrow.
    fn push(state: *mut ExecutionState) -> Self {
        GC_ROOT_PROVIDERS.with(|s| s.borrow_mut().push(state));
        Self
    }
}

impl Drop for RootProviderGuard {
    fn drop(&mut self) {
        GC_ROOT_PROVIDERS.with(|s| {
            s.borrow_mut().pop();
        });
    }
}

// ---------------------------------------------------------------------------
// Native local root handles (JNI-local-reference model)
// ---------------------------------------------------------------------------

/// A raw pointer to a heap reference that a native method is holding across a
/// re-entrant callback (`CallbackOps::invoke`) and that must therefore be
/// treated as a GC root *and* forwarded in place on every collection that fires
/// while the native is suspended.
///
/// This is the sound replacement for the one-shot post-invoke
/// `patch_forwarded_*` idiom: because [`patch_forwarded_slots`] runs immediately
/// after *every* `heap.collect` while that collection's one-hop forward map is
/// still valid, a pinned handle is re-forwarded from its CURRENT value against
/// the CURRENT map at each collect (A→B, then B→C, …). No forward chain is ever
/// chased and no stale from-address is ever re-keyed against a later map, so the
/// handle stays correct under ANY number of collections — exactly the JNI local
/// reference contract.
#[derive(Clone, Copy)]
enum HandlePtr {
    /// A bare object reference stored in one of the native's stack locals.
    Ref(*mut u64),
    /// A single [`Slot`] stored in one of the native's stack locals.
    Slot(*mut Slot),
    /// A contiguous run of [`Slot`]s (e.g. the backing buffer of a snapshot
    /// `Vec<Slot>`), described by base pointer and length.
    Slots(*mut Slot, usize),
}

thread_local! {
    /// Stack of native-held GC roots — "local handles", analogous to JNI local
    /// references. Each entry points at a live reference/slot owned by a native
    /// method that is currently suspended inside `CallbackOps::invoke`.
    ///
    /// Entries are pushed via [`NativeRootScope`] and popped (truncated) when the
    /// owning scope is dropped, so the stack is empty whenever no native is
    /// holding a heap reference across a callback. [`gather_roots`] reads every
    /// entry (keeping the target object alive) and [`patch_forwarded_slots`]
    /// rewrites every entry in place after each collection (keeping the native's
    /// held reference pointed at the object's new location).
    static NATIVE_ROOT_HANDLES: std::cell::RefCell<Vec<HandlePtr>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// RAII scope that lets a native method register the heap references/slots it
/// holds across a callback as GC local handles.
///
/// # Safety contract
/// Every pointer registered via [`pin_ref`](Self::pin_ref) /
/// [`pin_slot`](Self::pin_slot) / [`pin_slots`](Self::pin_slots) must remain
/// valid and point at storage that is neither moved nor reallocated for the
/// whole lifetime of the scope. This holds by construction for the intended
/// callers: they pin their own stack locals (`&mut u64` / `&mut Slot`) or the
/// backing buffer of a snapshot `Vec<Slot>` that is not pushed to, popped from,
/// or reassigned while the scope is alive. While a callback runs, the native is
/// suspended on the Rust stack and its `&mut` borrows of those locals are
/// inactive, so the collector may read them in [`gather_roots`] and write them
/// in [`patch_forwarded_slots`] through the raw pointers without aliasing a live
/// borrow — the same rationale as [`RootProviderGuard`]'s suspended frames.
pub(crate) struct NativeRootScope {
    /// Stack length captured at construction; `Drop` truncates back to it.
    base: usize,
}

impl NativeRootScope {
    /// Open a new handle scope. Handles registered on it are automatically
    /// removed when the returned guard is dropped.
    pub(crate) fn new() -> Self {
        let base = NATIVE_ROOT_HANDLES.with(|s| s.borrow().len());
        Self { base }
    }

    // The pin methods register into the shared thread-local rather than into
    // `self`, but take `&mut self` deliberately: a pin is only valid for the
    // lifetime of THIS uniquely-borrowed guard, and `&mut self` ties each
    // registration to the guard that will pop it on drop.

    /// Pin a bare object reference held in a native stack local.
    #[allow(clippy::needless_pass_by_ref_mut, clippy::unused_self)]
    pub(crate) fn pin_ref(&mut self, reference: &mut u64) {
        let ptr = std::ptr::from_mut(reference);
        NATIVE_ROOT_HANDLES.with(|s| s.borrow_mut().push(HandlePtr::Ref(ptr)));
    }

    /// Pin a single [`Slot`] held in a native stack local.
    #[allow(clippy::needless_pass_by_ref_mut, clippy::unused_self)]
    pub(crate) fn pin_slot(&mut self, slot: &mut Slot) {
        let ptr = std::ptr::from_mut(slot);
        NATIVE_ROOT_HANDLES.with(|s| s.borrow_mut().push(HandlePtr::Slot(ptr)));
    }

    /// Pin a contiguous slice of [`Slot`]s (e.g. a snapshot `Vec<Slot>`'s
    /// buffer). A no-op for an empty slice.
    #[allow(clippy::needless_pass_by_ref_mut, clippy::unused_self)]
    pub(crate) fn pin_slots(&mut self, slots: &mut [Slot]) {
        if slots.is_empty() {
            return;
        }
        let len = slots.len();
        let ptr = slots.as_mut_ptr();
        NATIVE_ROOT_HANDLES.with(|s| s.borrow_mut().push(HandlePtr::Slots(ptr, len)));
    }
}

impl Drop for NativeRootScope {
    fn drop(&mut self) {
        NATIVE_ROOT_HANDLES.with(|s| s.borrow_mut().truncate(self.base));
    }
}

/// Append the CURRENT target of every registered native handle to `roots` so the
/// collector keeps those objects alive. Mirror of the parent-frame walk in
/// [`gather_roots`].
fn extend_roots_with_native_handles(roots: &mut Vec<Slot>) {
    NATIVE_ROOT_HANDLES.with(|handles| {
        for handle in handles.borrow().iter() {
            // SAFETY: see `NativeRootScope`'s safety contract. Each pointer
            // targets a live stack local / snapshot buffer owned by a native
            // that is suspended in a callback below us on the Rust stack, so the
            // storage is valid and not concurrently borrowed while we read it.
            unsafe {
                match *handle {
                    HandlePtr::Ref(ptr) => roots.push(Slot::Reference(Some(*ptr))),
                    HandlePtr::Slot(ptr) => roots.push(*ptr),
                    HandlePtr::Slots(ptr, len) => {
                        for i in 0..len {
                            roots.push(*ptr.add(i));
                        }
                    }
                }
            }
        }
    });
}

/// Apply GC forwarding to every registered native handle in place after a
/// collection, so each native's held reference tracks its object's new location.
/// Mirror of the parent-frame patch walk in [`patch_forwarded_slots`].
fn forward_native_handles(heap: &duke_gc::Heap) {
    NATIVE_ROOT_HANDLES.with(|handles| {
        for handle in handles.borrow().iter() {
            // SAFETY: see `NativeRootScope`'s safety contract. Each pointer
            // targets a live stack local / snapshot buffer owned by a native
            // that is suspended in a callback below us on the Rust stack, so the
            // storage is valid and not concurrently borrowed while we write it.
            unsafe {
                match *handle {
                    HandlePtr::Ref(ptr) => {
                        let mut slot = Slot::Reference(Some(*ptr));
                        heap.apply_forward(&mut slot);
                        if let Some(new_ref) = slot.as_reference() {
                            *ptr = new_ref;
                        }
                    }
                    HandlePtr::Slot(ptr) => heap.apply_forward(&mut *ptr),
                    HandlePtr::Slots(ptr, len) => {
                        for i in 0..len {
                            heap.apply_forward(&mut *ptr.add(i));
                        }
                    }
                }
            }
        }
    });
}

/// Invoke `f` for every PARENT active `ExecutionState` — that is, every entry on
/// the shadow stack below the innermost/current one.
///
/// The current (topmost) state is deliberately skipped: [`gather_roots`] and
/// [`patch_forwarded_slots`] already scan it directly through their `frame` /
/// `call_stack` arguments. Walking it again here would double-count roots and,
/// worse, apply forwarding to the same slots twice. When the shadow stack is
/// empty (e.g. a direct unit-test call to `gather_roots`) this is a no-op.
fn for_each_parent_root_provider(mut f: impl FnMut(*mut ExecutionState)) {
    GC_ROOT_PROVIDERS.with(|s| {
        let providers = s.borrow();
        if let Some((_current, parents)) = providers.split_last() {
            for &p in parents {
                f(p);
            }
        }
    });
}

/// Collect all live Slot values from the interpreter's current execution state.
/// The GC uses these as the root set for reachability analysis.
fn gather_roots(
    frame: &duke_runtime::Frame,
    call_stack: &[CallFrame],
    registry: &ClassRegistry,
    string_intern: &std::collections::HashMap<(u8, String), u64>,
) -> Vec<duke_runtime::Slot> {
    let mut roots = Vec::new();
    roots.extend(frame.slots());
    for cf in call_stack {
        roots.extend(cf.frame.slots());
    }
    // Outer/parent execution states suspended on the Rust call stack (e.g. the
    // caller of a class `<clinit>` that re-entered the interpreter via a nested
    // `run_execution`). Their frames live on the Rust stack and are otherwise
    // invisible to the collector, so a GC fired inside the nested execution
    // would reclaim objects the caller still holds in its locals. Treat every
    // parent's frame + call stack + interned constants as live roots.
    for_each_parent_root_provider(|state_ptr| {
        // SAFETY: see `RootProviderGuard::push`. Parent states are suspended on
        // the Rust stack and not concurrently mutated while we read them.
        let state: &ExecutionState = unsafe { &*state_ptr };
        roots.extend(state.frame.slots());
        for cf in &state.call_stack {
            roots.extend(cf.frame.slots());
        }
        for &r in state.string_intern.values() {
            roots.push(duke_runtime::Slot::Reference(Some(r)));
        }
    });
    for ctx in registry.all_classes() {
        roots.extend(ctx.static_fields.iter().copied());
    }
    // Interned string/class-constant references (populated by `ldc`) must be
    // kept alive across collections. They are cached by the interpreter and
    // re-served on every `ldc` of the same constant, so if the collector
    // reclaimed one the cache would hand back a dangling reference (Java's
    // string constant pool never GCs literals).
    for &r in string_intern.values() {
        roots.push(duke_runtime::Slot::Reference(Some(r)));
    }
    // Native local handles: heap references/slots that a native method is
    // holding across a re-entrant callback. Their storage lives in the native's
    // Rust stack frame (or a snapshot buffer) and is otherwise invisible to the
    // collector, so without this a GC fired inside the callback would reclaim
    // objects the suspended native still needs.
    extend_roots_with_native_handles(&mut roots);
    roots
}

/// Apply GC forwarding pointers to all live interpreter slots after a minor
/// collection. Must be called immediately after `heap.collect()` returns so
/// that stale young-gen references are updated to their new locations.
fn patch_forwarded_slots(
    frame: &mut duke_runtime::Frame,
    call_stack: &mut [CallFrame],
    registry: &mut ClassRegistry,
    heap: &duke_gc::Heap,
    string_intern: &mut std::collections::HashMap<(u8, String), u64>,
) {
    for slot in frame.slots_mut() {
        heap.apply_forward(slot);
    }
    for cf in call_stack.iter_mut() {
        for slot in cf.frame.slots_mut() {
            heap.apply_forward(slot);
        }
    }
    // Parent execution states suspended on the Rust stack — mirror of the
    // parent-walk in `gather_roots`. Their live references were part of the root
    // set, so the collector relocated them and their slots must have forwarding
    // applied or they would dangle into stale young-gen indices.
    for_each_parent_root_provider(|state_ptr| {
        // SAFETY: see `RootProviderGuard::push`. Parent states are suspended on
        // the Rust stack and not concurrently mutated while we patch them.
        let state: &mut ExecutionState = unsafe { &mut *state_ptr };
        for slot in state.frame.slots_mut() {
            heap.apply_forward(slot);
        }
        for cf in &mut state.call_stack {
            for slot in cf.frame.slots_mut() {
                heap.apply_forward(slot);
            }
        }
        for r in state.string_intern.values_mut() {
            let mut slot = duke_runtime::Slot::Reference(Some(*r));
            heap.apply_forward(&mut slot);
            if let Some(new_r) = slot.as_reference() {
                *r = new_r;
            }
        }
    });
    for ctx in registry.all_classes_mut() {
        for slot in &mut ctx.static_fields {
            heap.apply_forward(slot);
        }
    }
    // Relocate the interned-constant cache so subsequent `ldc` hits return the
    // constant's new address rather than a stale pre-collection reference. This
    // was the root cause of the `String.format("\\u%04x", …)` `InvalidRef` in
    // gson's `JsonWriter.<clinit>`: the collector moved the interned "\u%04x"
    // literal but the cache kept handing out its old slot on the next loop turn.
    for r in string_intern.values_mut() {
        let mut slot = duke_runtime::Slot::Reference(Some(*r));
        heap.apply_forward(&mut slot);
        if let Some(new_r) = slot.as_reference() {
            *r = new_r;
        }
    }
    // Native local handles — mirror of the parent-frame patch walk above. Each
    // pinned reference/slot is forwarded in place against THIS collection's
    // one-hop map while it is still valid, so a native holding a reference
    // across a callback tracks the object across any number of collections.
    forward_native_handles(heap);
}

// ---------------------------------------------------------------------------
// Phase 48: Stream.generate/iterate/concat/empty
// ---------------------------------------------------------------------------



/// Native: `Stream.concat(Stream, Stream)Stream` — concatenates two eager streams.
pub(crate) fn native_stream_concat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a_ref = extract_ref_arg(args, 0)?;
    let b_ref = extract_ref_arg(args, 1)?;
    let a_size = match heap.get(a_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let b_size = match heap.get(b_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let a_elems: Vec<Slot> = heap.get(a_ref)?.fields[1..=a_size].to_vec();
    let b_elems: Vec<Slot> = heap.get(b_ref)?.fields[1..=b_size].to_vec();
    let total = a_size + b_size;
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(total).unwrap_or(0));
    heap.get_mut(out_ref)?.fields.extend(a_elems.into_iter().chain(b_elems));
    Ok(Some(Slot::Reference(Some(out_ref))))
}


// ---------------------------------------------------------------------------
// Phase 46: Stream.takeWhile/dropWhile (Java 9)
// ---------------------------------------------------------------------------



// ---------------------------------------------------------------------------
// Phase 45: ArrayList.forEach, Stream.sorted(Comparator), Arrays.toString,
//           HashMap.replace, Collections.swap/unmodifiableMap,
//           Collectors.partitioningBy, IntStream.sorted
// ---------------------------------------------------------------------------











// ---------------------------------------------------------------------------
// Phase 49: Comparator.thenComparing, Predicate combinators, Function combinators,
//           Stream.mapToLong, Stream.mapToDouble
// ---------------------------------------------------------------------------



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




/// Allocates a `duke/util/LongStream` with `fields[0]=Int(size), fields[1..n]=Long(value)`.
fn make_long_stream(heap: &mut duke_gc::Heap, values: Vec<i64>) -> u64 {
    let r = heap.allocate("duke/util/LongStream".to_string(), 1);
    if let Ok(obj) = heap.get_mut(r) {
        obj.fields[0] = Slot::Int(i32::try_from(values.len()).unwrap_or(0));
        for v in values {
            obj.fields.push(Slot::Long(v));
        }
    }
    r
}



/// Allocates a `duke/util/DoubleStream` with `fields[0]=Int(size), fields[1..n]=Double(value)`.
fn make_double_stream(heap: &mut duke_gc::Heap, values: Vec<f64>) -> u64 {
    let r = heap.allocate("duke/util/DoubleStream".to_string(), 1);
    if let Ok(obj) = heap.get_mut(r) {
        obj.fields[0] = Slot::Int(i32::try_from(values.len()).unwrap_or(0));
        for v in values {
            obj.fields.push(Slot::Double(v));
        }
    }
    r
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




// ---- LongStream terminal ops ----










// ---- LongStream intermediate ops ----




// ---- LongStream.mapToInt / mapToDouble ----


// ---- DoubleStream static factories ----



// ---- DoubleStream terminal ops ----







// ---- DoubleStream intermediate ops ----



// ---- IntStream.asLongStream / asDoubleStream ----



// ---- OptionalLong ----



// ---- Collectors.summingInt / averagingInt ----



// ---------------------------------------------------------------------------
// Phase 53: IntStream/LongStream terminal ops, Comparator.comparingLong,
//           Optional.or / ifPresentOrElse, Collectors.toUnmodifiable*
// ---------------------------------------------------------------------------












// ---------------------------------------------------------------------------
// Phase 58: Comparator.comparingDouble, Map.copyOf/entry/ofEntries,
//           Collections.singletonMap/singletonSet/unmodifiableSet
// ---------------------------------------------------------------------------


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

/// Native: `Collections.singletonMap(K,V)Map` — returns a single-entry unmodifiable map.
pub(crate) fn native_collections_singleton_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_map_of(args, heap, out, control)
}

/// Native: `Collections.singleton(E)Set` — returns a single-element unmodifiable set.
pub(crate) fn native_collections_singleton_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_set_of_factory(args, heap, out, control)
}

/// Native: `Collections.unmodifiableSet(Set)Set` — returns a view of the set (same backing object).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // Wrap the source set in a UnmodifiableSet (same field layout as HashSet).
    let src_ref = extract_ref_arg(args, 0)?;
    let src_fields = heap.get(src_ref)?.fields.clone();
    let class_name = heap.get(src_ref)?.class_name.clone();
    let n = src_fields.len();
    let wrapper_ref = heap.allocate("java/util/UnmodifiableSet".to_string(), n);
    // Copy field layout from source set
    let _ = class_name;
    for (i, f) in src_fields.into_iter().enumerate() {
        heap.get_mut(wrapper_ref)?.fields[i] = f;
    }
    Ok(Some(Slot::Reference(Some(wrapper_ref))))
}

// ---------------------------------------------------------------------------
// Phase 59: Stream.flatMapToInt/Long/Double, Collectors.toMap (3-arg),
//           forEach on HashSet/TreeSet/TreeMap/LinkedList/LinkedHashMap/PriorityQueue
// ---------------------------------------------------------------------------

/// Native: `Stream.flatMapToInt(Function<T,IntStream>)IntStream`
pub(crate) fn native_stream_flat_map_to_int(
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
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result: Vec<i32> = Vec::new();
    for elem in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, elem],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(int_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}

/// Native: `Stream.flatMapToLong(Function<T,LongStream>)LongStream`
pub(crate) fn native_stream_flat_map_to_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result: Vec<i64> = Vec::new();
    for elem in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, elem],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(long_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}

/// Native: `Stream.flatMapToDouble(Function<T,DoubleStream>)DoubleStream`
pub(crate) fn native_stream_flat_map_to_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = stream_elements(heap, stream_ref)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result: Vec<f64> = Vec::new();
    for elem in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, elem],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(double_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, result,
    )))))
}

/// Native: `Collectors.toMap(keyFn, valFn, mergeFn)Collector` — stores three functions.
pub(crate) fn native_collectors_to_map_merge(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let key_fn = extract_slot_arg(args, 0);
    let val_fn = extract_slot_arg(args, 1);
    let merge_fn = extract_slot_arg(args, 2);
    let r = heap.allocate("duke/util/ToMapMergeCollector".to_string(), 3);
    heap.get_mut(r)?.fields[0] = key_fn;
    heap.get_mut(r)?.fields[1] = val_fn;
    heap.get_mut(r)?.fields[2] = merge_fn;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `HashSet.forEach(Consumer)V`
pub(crate) fn native_hashset_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(consumer_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(None);
    };
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
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

/// Native: `TreeSet.forEach(Consumer)V`
pub(crate) fn native_treeset_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    // TreeSet uses same layout as HashSet: fields[0]=size, fields[1..size]=elements
    native_hashset_for_each(args, heap, out, control, ops)
}

/// Native: `TreeMap.forEach(BiConsumer)V`
pub(crate) fn native_treemap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    // TreeMap uses same layout as HashMap: fields[0]=size, fields[1,2]=k0/v0 ...
    native_hashmap_for_each(args, heap, out, control, ops)
}

/// Native: `LinkedHashMap.forEach(BiConsumer)V`
pub(crate) fn native_linkedhashmap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_hashmap_for_each(args, heap, out, control, ops)
}

/// Native: `LinkedList.forEach(Consumer)V`
pub(crate) fn native_linked_list_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_arraylist_for_each(args, heap, out, control, ops)
}

/// Native: `PriorityQueue.forEach(Consumer)V`
pub(crate) fn native_priorityqueue_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_arraylist_for_each(args, heap, out, control, ops)
}

// ---------------------------------------------------------------------------
// Phase 60: IntStream/LongStream/DoubleStream takeWhile/dropWhile,
//           Integer/Long/Double compare/max/min,
//           TreeMap.keySet/values/getOrDefault, TreeSet.stream
// ---------------------------------------------------------------------------

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

/// Native: `TreeMap.keySet()Set` — delegates to `HashMap` keySet (same field layout).
pub(crate) fn native_treemap_key_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_hashmap_key_set(args, heap, out, control)
}

/// Native: `TreeMap.values()Collection` — delegates to `HashMap` values (same field layout).
pub(crate) fn native_treemap_values(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_hashmap_values(args, heap, out, control)
}

/// Native: `TreeMap.getOrDefault(Object,Object)Object` — looks up key; returns default if absent.
pub(crate) fn native_treemap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let default_val = extract_slot_arg(args, 2);
    let found_idx = {
        let fields = &heap.get(this_ref)?.fields;
        find_hashmap_entry_index(fields, &key, heap)
    };
    Ok(Some(match found_idx {
        Some(i) => heap.get(this_ref)?.fields[i + 1],
        None => default_val,
    }))
}

/// Native: `TreeSet.stream()Stream` — wraps sorted elements into a `duke/util/Stream`.
pub(crate) fn native_treeset_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // TreeSet layout: fields[0]=size, fields[1..=size]=elements (same as HashSet)
    native_hashset_stream(args, heap, out, control)
}





// ---------------------------------------------------------------------------
// Phase 54: IntStream/LongStream/DoubleStream limit/skip,
//           IntStream/LongStream flatMap, Collectors.mapping
// ---------------------------------------------------------------------------











// ---------------------------------------------------------------------------
// Phase 55: Complete DoubleStream, LongStream gaps, Collectors.summingLong/averagingDouble
// ---------------------------------------------------------------------------

















// ---------------------------------------------------------------------------
// Phase 56: Collectors.minBy/maxBy, summingDouble, averagingLong,
//           toUnmodifiableMap, collectingAndThen
// ---------------------------------------------------------------------------

/// Native: `Collectors.minBy(Comparator)Collector` — returns a `MinByCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_min_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let cmp_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/MinByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = cmp_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.maxBy(Comparator)Collector` — returns a `MaxByCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_max_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let cmp_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/MaxByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = cmp_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.summingDouble(ToDoubleFunction)Collector` — returns a `SummingDoubleCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_summing_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/SummingDoubleCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.averagingLong(ToLongFunction)Collector` — returns an `AveragingLongCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_averaging_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/AveragingLongCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.toUnmodifiableMap(keyFn, valueFn)Collector` — same sentinel as `ToMapCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_unmodifiable_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let key_fn_slot = extract_slot_arg(args, 0);
    let val_fn_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ToUnmodifiableMapCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = key_fn_slot;
    heap.get_mut(r)?.fields[1] = val_fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.collectingAndThen(downstream, finisher)Collector` — returns a
/// `CollectingAndThenCollector` with `fields[0]`=downstream, `fields[1]`=finisher.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_collecting_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let downstream_slot = extract_slot_arg(args, 0);
    let finisher_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/CollectingAndThenCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = downstream_slot;
    heap.get_mut(r)?.fields[1] = finisher_slot;
    Ok(Some(Slot::Reference(Some(r))))
}


// ---------------------------------------------------------------------------
// Phase 57: Collectors.reducing, Stream.iterate predicate, Optional.stream,
//           ArrayDeque completion
// ---------------------------------------------------------------------------

/// Native: `Collectors.reducing(BinaryOperator)` — returns a
/// `ReducingNoIdentityCollector` with `fields[0]`=op.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_reducing_no_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let op_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/ReducingNoIdentityCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = op_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.reducing(T, BinaryOperator)` — returns a
/// `ReducingCollector` with `fields[0]`=identity, `fields[1]`=op.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_reducing_with_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let identity_slot = extract_slot_arg(args, 0);
    let op_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ReducingCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = identity_slot;
    heap.get_mut(r)?.fields[1] = op_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
















// ---------------------------------------------------------------------------
// Phase 62: java.time (LocalDate, LocalDateTime, Instant, Duration, Period)
// ---------------------------------------------------------------------------

/// Convert (year, month, day) to a proleptic Gregorian epoch day count.
/// Day 0 = 1970-01-01.  Howard Hinnant's branchless algorithm.
#[allow(
    clippy::cast_lossless,         // i32/u32 → i64 widening casts
    clippy::cast_possible_truncation, // result fits i32 for any valid Gregorian date
    clippy::missing_const_for_fn   // i64::from not const-stable yet
)]
fn ymd_to_epoch_days(year: i32, month: u32, day: u32) -> i32 {
    let (y, m, d) = (year as i64, month as i64, day as i64);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400); // year of era [0, 399]
    let day_of_year = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + day_of_year; // day of era [0, 146096]
    (era * 146_097 + doe - 719_468) as i32
}

/// Convert a proleptic Gregorian epoch day to (year, month, day).
#[allow(
    clippy::cast_lossless,            // i32 → i64 widening cast
    clippy::cast_possible_truncation, // y fits i32 for valid dates
    clippy::cast_sign_loss,           // m/d are [1,12]/[1,31], sign-safe u32
    clippy::missing_const_for_fn      // i64::from not const-stable yet
)]
fn epoch_days_to_ymd(epoch_days: i32) -> (i32, u32, u32) {
    let z = epoch_days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097); // [0, 146096]
    let yoe = (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let day_of_year = day_of_era - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * day_of_year + 2) / 153; // [0, 11]
    let d = day_of_year - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

// ---- LocalDate layout: fields[0] = Slot::Int(epoch_days) ----



















fn localdate_compare_impl(heap: &duke_gc::Heap, this_ref: u64, other_ref: u64) -> Result<i32> {
    let other = heap.get(other_ref)?;
    if other.class_name != "java/time/LocalDate" {
        return Err(class_cast_error());
    }
    let this_epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let other_epoch = match other.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(ordering_to_int(this_epoch.cmp(&other_epoch)))
}





/// Helper: days in a given month of a given year (handles leap years).
const fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30, // months 4,6,9,11 + any invalid input
    }
}

const NANOS_PER_SECOND_I128: i128 = 1_000_000_000;
const NANOS_PER_MILLI_I128: i128 = 1_000_000;
const SECONDS_PER_DAY_I64: i64 = 86_400;

fn clamp_i128_to_i64(value: i128) -> i64 {
    match i64::try_from(value) {
        Ok(value) => value,
        Err(_) if value.is_negative() => i64::MIN,
        Err(_) => i64::MAX,
    }
}

fn clamp_i64_to_i32(value: i64) -> i32 {
    match i32::try_from(value) {
        Ok(value) => value,
        Err(_) if value.is_negative() => i32::MIN,
        Err(_) => i32::MAX,
    }
}

fn normalize_seconds_nanos(total_nanos: i128) -> (i64, i32) {
    let seconds = clamp_i128_to_i64(total_nanos.div_euclid(NANOS_PER_SECOND_I128));
    let nanos = i32::try_from(total_nanos.rem_euclid(NANOS_PER_SECOND_I128)).unwrap_or(0);
    (seconds, nanos)
}

fn shift_year_month_day(year: i32, month: u32, day: u32, delta_months: i64) -> (i32, u32, u32) {
    let total_months = i64::from(year) * 12 + (i64::from(month) - 1) + delta_months;
    let new_year = clamp_i64_to_i32(total_months.div_euclid(12));
    let new_month = u32::try_from(total_months.rem_euclid(12) + 1).unwrap_or(1);
    let new_day = day.min(days_in_month(new_year, new_month));
    (new_year, new_month, new_day)
}

fn parse_fraction_to_nanos(fraction: &str) -> Option<i32> {
    if fraction.is_empty() || fraction.len() > 9 || !fraction.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let digits: i32 = fraction.parse().ok()?;
    let scale = 10_i32.pow(u32::try_from(9 - fraction.len()).ok()?);
    Some(digits.saturating_mul(scale))
}

fn parse_iso_local_date_components(text: &str) -> Option<(i32, u32, u32)> {
    let mut parts = text.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some()
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
    {
        return None;
    }
    Some((year, month, day))
}

fn parse_iso_time_components(text: &str) -> Option<(i32, i32, i32, i32)> {
    let (clock_part, nanos) = match text.split_once('.') {
        Some((clock, fraction)) => (clock, parse_fraction_to_nanos(fraction)?),
        None => (text, 0),
    };
    let mut parts = clock_part.split(':');
    let hour: i32 = parts.next()?.parse().ok()?;
    let minute: i32 = parts.next()?.parse().ok()?;
    let second: i32 = parts.next()?.parse().ok()?;
    if parts.next().is_some()
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }
    Some((hour, minute, second, nanos))
}

fn parse_iso_localdatetime_components(text: &str) -> Option<(i32, u32, u32, i32, i32, i32, i32)> {
    let (date_part, time_part) = text.split_once('T')?;
    let (year, month, day) = parse_iso_local_date_components(date_part)?;
    let (hour, minute, second, nanos) = parse_iso_time_components(time_part)?;
    Some((year, month, day, hour, minute, second, nanos))
}

fn parse_iso_instant_components(text: &str) -> Option<(i64, i32)> {
    let body = text.strip_suffix('Z')?;
    let (year, month, day, hour, minute, second, nanos) = parse_iso_localdatetime_components(body)?;
    let epoch_day = i64::from(ymd_to_epoch_days(year, month, day));
    let epoch_seconds =
        epoch_day * SECONDS_PER_DAY_I64 + i64::from(hour * 3600 + minute * 60 + second);
    Some((epoch_seconds, nanos))
}

fn epoch_seconds_to_datetime_parts(epoch_seconds: i64) -> (i32, u32, u32, i32, i32, i32) {
    let epoch_days = clamp_i64_to_i32(epoch_seconds.div_euclid(SECONDS_PER_DAY_I64));
    let second_of_day = epoch_seconds.rem_euclid(SECONDS_PER_DAY_I64);
    let hour = i32::try_from(second_of_day / 3600).unwrap_or(0);
    let minute = i32::try_from((second_of_day % 3600) / 60).unwrap_or(0);
    let second = i32::try_from(second_of_day % 60).unwrap_or(0);
    let (year, month, day) = epoch_days_to_ymd(epoch_days);
    (year, month, day, hour, minute, second)
}

fn date_time_parse_error(input: &str) -> Error {
    push_pending_java_exception_message(
        "java/time/format/DateTimeParseException",
        format!("Text '{input}' could not be parsed"),
    );
    Error::JavaException {
        class_name: "java/time/format/DateTimeParseException".to_string(),
    }
}

fn class_cast_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/ClassCastException".to_string(),
    }
}

fn duration_parts_from_ref(heap: &duke_gc::Heap, duration_ref: u64) -> Result<(i64, i32)> {
    let obj = heap.get(duration_ref)?;
    let seconds = match obj.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let nanos = match obj.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok((seconds, nanos))
}

fn duration_total_nanos_from_ref(heap: &duke_gc::Heap, duration_ref: u64) -> Result<i128> {
    let (seconds, nanos) = duration_parts_from_ref(heap, duration_ref)?;
    Ok(i128::from(seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos))
}

fn allocate_duration_from_total_nanos(heap: &mut duke_gc::Heap, total_nanos: i128) -> Result<u64> {
    let (seconds, nanos) = normalize_seconds_nanos(total_nanos);
    let duration_ref = heap.allocate("java/time/Duration".to_string(), 2);
    heap.get_mut(duration_ref)?.fields[0] = Slot::Long(seconds);
    heap.get_mut(duration_ref)?.fields[1] = Slot::Int(nanos);
    Ok(duration_ref)
}

fn instant_parts_from_ref(heap: &duke_gc::Heap, instant_ref: u64) -> Result<(i64, i32)> {
    let obj = heap.get(instant_ref)?;
    let seconds = match obj.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let nanos = match obj.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok((seconds, nanos))
}

fn instant_total_nanos_from_ref(heap: &duke_gc::Heap, instant_ref: u64) -> Result<i128> {
    let (seconds, nanos) = instant_parts_from_ref(heap, instant_ref)?;
    Ok(i128::from(seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos))
}

fn allocate_instant_from_total_nanos(heap: &mut duke_gc::Heap, total_nanos: i128) -> Result<u64> {
    let (seconds, nanos) = normalize_seconds_nanos(total_nanos);
    let instant_ref = heap.allocate("java/time/Instant".to_string(), 2);
    heap.get_mut(instant_ref)?.fields[0] = Slot::Long(seconds);
    heap.get_mut(instant_ref)?.fields[1] = Slot::Int(nanos);
    Ok(instant_ref)
}

fn localdatetime_components_from_ref(
    heap: &duke_gc::Heap,
    localdatetime_ref: u64,
) -> Result<(i32, i32, i32, i32, i32)> {
    let obj = heap.get(localdatetime_ref)?;
    let epoch_day = match obj.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let hour = match obj.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let minute = match obj.fields.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let second = match obj.fields.get(3) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let nanos = match obj.fields.get(4) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok((epoch_day, hour, minute, second, nanos))
}

fn localdatetime_total_nanos_from_ref(
    heap: &duke_gc::Heap,
    localdatetime_ref: u64,
) -> Result<i128> {
    let (epoch_day, hour, minute, second, nanos) =
        localdatetime_components_from_ref(heap, localdatetime_ref)?;
    let epoch_seconds =
        i64::from(epoch_day) * SECONDS_PER_DAY_I64 + i64::from(hour * 3600 + minute * 60 + second);
    Ok(i128::from(epoch_seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos))
}

fn allocate_localdate(heap: &mut duke_gc::Heap, epoch_day: i32) -> Result<u64> {
    let localdate_ref = heap.allocate("java/time/LocalDate".to_string(), 1);
    heap.get_mut(localdate_ref)?.fields[0] = Slot::Int(epoch_day);
    Ok(localdate_ref)
}

fn allocate_localdatetime(
    heap: &mut duke_gc::Heap,
    epoch_day: i32,
    hour: i32,
    minute: i32,
    second: i32,
    nanos: i32,
) -> Result<u64> {
    let localdatetime_ref = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    heap.get_mut(localdatetime_ref)?.fields[0] = Slot::Int(epoch_day);
    heap.get_mut(localdatetime_ref)?.fields[1] = Slot::Int(hour);
    heap.get_mut(localdatetime_ref)?.fields[2] = Slot::Int(minute);
    heap.get_mut(localdatetime_ref)?.fields[3] = Slot::Int(second);
    heap.get_mut(localdatetime_ref)?.fields[4] = Slot::Int(nanos);
    Ok(localdatetime_ref)
}

// ---- Duration layout: fields[0]=Slot::Long(seconds), fields[1]=Slot::Int(nanos_adj) ----




















fn duration_compare_impl(heap: &duke_gc::Heap, this_ref: u64, other_ref: u64) -> Result<i32> {
    let other = heap.get(other_ref)?;
    if other.class_name != "java/time/Duration" {
        return Err(class_cast_error());
    }
    let this_total = duration_total_nanos_from_ref(heap, this_ref)?;
    let other_total = duration_total_nanos_from_ref(heap, other_ref)?;
    Ok(ordering_to_int(this_total.cmp(&other_total)))
}





// ---- Period layout: fields[0]=years(Int), fields[1]=months(Int), fields[2]=days(Int) ----










// ---- Instant layout: fields[0]=Slot::Long(epoch_seconds), fields[1]=Slot::Int(nanos_adj) ----















fn instant_compare_impl(heap: &duke_gc::Heap, this_ref: u64, other_ref: u64) -> Result<i32> {
    let other = heap.get(other_ref)?;
    if other.class_name != "java/time/Instant" {
        return Err(class_cast_error());
    }
    let this_parts = instant_parts_from_ref(heap, this_ref)?;
    let other_parts = instant_parts_from_ref(heap, other_ref)?;
    Ok(ordering_to_int(this_parts.cmp(&other_parts)))
}





// ---------------------------------------------------------------------------
// Phase 63: java.time.LocalDateTime
// Layout: fields[0]=epoch_days(Int), fields[1]=hour(Int),
//         fields[2]=minute(Int), fields[3]=second(Int), fields[4]=nano(Int)
// ---------------------------------------------------------------------------




















fn localdatetime_compare_impl(heap: &duke_gc::Heap, this_ref: u64, other_ref: u64) -> Result<i32> {
    let other = heap.get(other_ref)?;
    if other.class_name != "java/time/LocalDateTime" {
        return Err(class_cast_error());
    }
    let this_parts = localdatetime_components_from_ref(heap, this_ref)?;
    let other_parts = localdatetime_components_from_ref(heap, other_ref)?;
    Ok(ordering_to_int(this_parts.cmp(&other_parts)))
}





// ---------------------------------------------------------------------------
// Phase 64: String.indent, StringBuilder.setCharAt, Collections.disjoint,
//           HashMap.computeIfPresent
// ---------------------------------------------------------------------------





// ---------------------------------------------------------------------------
// Phase 117: Map.remove(k,v), Collections.emptyList (immutable), Stream.concat,
//            Map.replace, Integer.sum, IntStream.mapToObj, Optional.map,
//            UnmodifiableSet
// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// jdk/internal/misc/Unsafe — object-field / array CAS natives
// ---------------------------------------------------------------------------
//
// Duke models just enough of `Unsafe` to run real `java.util.concurrent`
// bytecode (notably `ConcurrentHashMap`) under `DUKE_REAL_JDK=1`. Duke has no
// byte offsets: an object's instance fields and an array's elements both live in
// one flat positional `HeapObject::fields` vector. The "offset" produced by
// `objectFieldOffset` and consumed by the get/put/CAS natives below is simply
// the positional slot index into `fields` — the exact same index used by
// `Getfield`/`Putfield` (via `field_slot_idx`) and by `Aaload`/`Aastore` (the
// raw element index). Because `arrayBaseOffset` returns 0 and `arrayIndexScale`
// returns 1, the shift/scale arithmetic real bytecode performs
// (`(long)i << ASHIFT + ABASE`, with `ASHIFT == 0`) collapses to `offset == i`,
// so array access indexes `fields[i]` directly and is consistent with object
// field access. This encoding is self-contained: no code outside these natives
// interprets the value, so a plain slot index as `i64` is sufficient.

/// Read the flat `fields` slot addressed by an `Unsafe` offset, bounds-checked.
fn unsafe_field_slot(heap: &duke_gc::Heap, obj_ref: u64, offset: i64) -> Result<Slot> {
    let idx = usize::try_from(offset).map_err(|_| Error::NullPointerException)?;
    heap.get(obj_ref)?
        .fields
        .get(idx)
        .copied()
        .ok_or(Error::NullPointerException)
}











#[cfg(test)]
mod havoc_thread_join_itself {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn test_join_java_thread_itself() {
        let runtime = Arc::new(Mutex::new(CompletionRuntime::default()));
        let runtime_clone = runtime.clone();

        let (tx, rx) = std::sync::mpsc::channel();
        let (tx_panic, rx_panic) = std::sync::mpsc::channel();

        let handle = thread::spawn(move || -> Result<()> {
            rx.recv().unwrap();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = join_java_thread(&runtime_clone, 0);
            }));
            if let Err(e) = result {
                if let Some(s) = e.downcast_ref::<&str>() {
                    tx_panic.send((*s).to_string()).unwrap();
                } else if let Some(s) = e.downcast_ref::<String>() {
                    tx_panic.send(s.clone()).unwrap();
                }
            }
            Ok(())
        });

        {
            let mut rt = runtime.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            rt.handles.insert(0, handle);
            let mut record = crate::threading::ThreadRecord::new(123, 0);
            record.finished = false;
            rt.threads.register(record);
        }

        tx.send(()).unwrap();

        // It should NOT panic, but rather return Ok(())
        let res = rx_panic.recv_timeout(std::time::Duration::from_millis(50));
        assert!(res.is_err(), "Expected no panic, but received one!");
    }
}

#[cfg(test)]
mod havoc_string_indent_overflow {
    use super::*;
    use std::io::sink;
    use duke_gc::Heap;
    use duke_runtime::Slot;

    #[test]

    fn test_string_indent_overflow() {
        let mut heap = Heap::new();
        let r = heap.allocate_string("hello\nworld".to_string());

        let args = vec![Slot::Reference(Some(r)), Slot::Int(i32::MIN)];
        let mut control = NativeControl::default();

        let _ = native_string_indent(&args, &mut heap, &mut sink(), &mut control);
    }
}

#[cfg(test)]
mod tests_zip_coverage {
    use super::*;

    #[test]
    fn zip_registry_error_coverage() {
        // ID 999 doesn't exist
        let err = zip_entry_count(999).unwrap_err();
        assert!(matches!(err, Error::JavaException { .. }));

        let err = zip_get_entry_info(999, "test").unwrap_err();
        assert!(matches!(err, Error::JavaException { .. }));

        let err = zip_read_entry(999, "test").unwrap_err();
        assert!(matches!(err, Error::JavaException { .. }));

        // Removing non-existent shouldn't panic
        zip_close(999);
    }
}

#[cfg(test)]
mod tests_zip_open_coverage {
    use super::*;

    #[test]
    fn zip_open_io_error() {
        let err = zip_open(std::path::Path::new("/does/not/exist/ever/zip.zip")).unwrap_err();
        assert!(matches!(err, Error::JavaException { ref class_name } if class_name == "java/io/FileNotFoundException"));
    }

    #[test]
    fn zip_open_format_error() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("bad_zip_format.zip");
        std::fs::write(&path, b"not a zip file").unwrap();
        let err = zip_open(&path).unwrap_err();
        assert!(matches!(err, Error::JavaException { ref class_name } if class_name == "java/util/zip/ZipException"));
        std::fs::remove_file(&path).unwrap();
    }







#[cfg(test)]
mod havoc_coverage_tests {
    use super::*;

    #[test]
    fn test_system_property_value_fallback() {
        assert!(system_property_value_fallback("file.separator").is_some());
        assert!(system_property_value_fallback("path.separator").is_some());
        assert!(system_property_value_fallback("line.separator").is_some());
        assert!(system_property_value_fallback("os.name").is_some());
        assert!(system_property_value_fallback("unknown.property").is_none());
        assert!(system_property_value_fallback("java.version").is_some());
        assert!(system_property_value_fallback("user.dir").is_some());
    }
}

}

#[cfg(test)]
mod havoc_string_repeat_oom {
    use super::*;
    use std::io::sink;
    use duke_gc::Heap;
    use duke_runtime::Slot;

    #[test]
    fn test_string_repeat_oom_trigger() {
        let mut heap = Heap::new();
        let r = heap.allocate_string("12345678901234567890".to_string());

        let args = vec![Slot::Reference(Some(r)), Slot::Int(i32::MAX)];
        let mut control = NativeControl::default();

        let result = native_string_repeat(&args, &mut heap, &mut sink(), &mut control);
        let err = result.unwrap_err();
        assert!(matches!(err, Error::JavaException { ref class_name } if class_name == "java/lang/OutOfMemoryError"));
    }
}

#[cfg(test)]
mod sentry_tests {
    use super::*;

    #[test]
    fn test_zip_functions_error_cases() {
        let invalid_id = -999;

        let count_err = zip_entry_count(invalid_id).unwrap_err();
        assert!(matches!(count_err, Error::JavaException { ref class_name } if class_name == "java/io/IOException"));

        let info_err = zip_get_entry_info(invalid_id, "test").unwrap_err();
        assert!(matches!(info_err, Error::JavaException { ref class_name } if class_name == "java/io/IOException"));

        let read_err = zip_read_entry(invalid_id, "test").unwrap_err();
        assert!(matches!(read_err, Error::JavaException { ref class_name } if class_name == "java/io/IOException"));

        // This shouldn't panic
        zip_close(invalid_id);
    }
}

#[cfg(test)]
mod havoc_string_indent_overflow_positive {
    use super::*;
    use std::io::sink;
    use duke_gc::Heap;
    use duke_runtime::Slot;

    #[test]
    fn test_string_indent_overflow_trigger() {
        let mut heap = Heap::new();
        let r = heap.allocate_string("hello\nworld".to_string());

        let args = vec![Slot::Reference(Some(r)), Slot::Int(i32::MAX)];
        let mut control = NativeControl::default();

        let result = native_string_indent(&args, &mut heap, &mut sink(), &mut control);
        let err = result.unwrap_err();
        assert!(matches!(err, Error::JavaException { ref class_name } if class_name == "java/lang/OutOfMemoryError"));
    }
}

#[cfg(test)]
mod float_double_bit_natives_tests {
    use super::*;
    use std::io::sink;
    use duke_gc::Heap;
    use duke_runtime::Slot;

    #[allow(clippy::type_complexity)]
    fn call(
        f: fn(&[Slot], &mut Heap, &mut dyn Write, &mut NativeControl) -> Result<Option<Slot>>,
        args: &[Slot],
    ) -> Slot {
        let mut heap = Heap::new();
        let mut control = NativeControl::default();
        f(args, &mut heap, &mut sink(), &mut control)
            .expect("native ok")
            .expect("native returns a value")
    }

    #[test]
    fn float_to_raw_int_bits_matches_java() {
        // 1.0f == 0x3f800000
        let out = call(native_float_float_to_raw_int_bits, &[Slot::Float(1.0)]);
        assert_eq!(out, Slot::Int(0x3f80_0000));
    }

    #[test]
    fn int_bits_to_float_round_trips() {
        let out = call(native_float_int_bits_to_float, &[Slot::Int(0x3f80_0000)]);
        assert_eq!(out, Slot::Float(1.0));
    }

    #[test]
    fn float_to_int_bits_canonicalizes_nan() {
        let out = call(native_float_float_to_int_bits, &[Slot::Float(f32::NAN)]);
        assert_eq!(out, Slot::Int(0x7fc0_0000));
    }

    #[test]
    fn double_to_raw_long_bits_matches_java() {
        // 1.0 == 0x3ff0000000000000
        let out = call(native_double_double_to_raw_long_bits, &[Slot::Double(1.0)]);
        assert_eq!(out, Slot::Long(0x3ff0_0000_0000_0000));
    }

    #[test]
    fn long_bits_to_double_round_trips() {
        let out =
            call(native_double_long_bits_to_double, &[Slot::Long(0x3ff0_0000_0000_0000)]);
        assert_eq!(out, Slot::Double(1.0));
    }

    #[test]
    fn double_to_long_bits_canonicalizes_nan() {
        let out = call(native_double_double_to_long_bits, &[Slot::Double(f64::NAN)]);
        assert_eq!(out, Slot::Long(0x7ff8_0000_0000_0000_u64.cast_signed()));
    }
}

#[cfg(test)]
mod string_format_boxed_hex_tests {
    use super::*;
    use std::io::sink;
    use duke_gc::Heap;
    use duke_runtime::Slot;

    // Regression for gson's JsonWriter.<clinit>: `String.format("\\u%04x",
    // Integer.valueOf(i))` must zero-pad the boxed int to width 4 and keep the
    // literal "\u" prefix intact — e.g. i=31 -> "".
    #[test]
    fn formats_boxed_integer_with_backslash_u_prefix() {
        let mut heap = Heap::new();
        let mut control = NativeControl::default();

        let fmt_ref = heap.allocate_string("\\u%04x".to_string());
        // Boxed Integer(31), stored in a one-element Object[] varargs array.
        let boxed = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(boxed).unwrap().fields[0] = Slot::Int(31);
        let arr = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
        heap.get_mut(arr).unwrap().fields[0] = Slot::Reference(Some(boxed));

        let args = vec![Slot::Reference(Some(fmt_ref)), Slot::Reference(Some(arr))];
        let out = native_string_format(&args, &mut heap, &mut sink(), &mut control)
            .expect("format ok")
            .expect("format returns a string");
        let Slot::Reference(Some(r)) = out else {
            panic!("expected string reference, got {out:?}");
        };
        assert_eq!(
            heap.get(r).unwrap().string_value.as_deref(),
            Some("\\u001f")
        );
    }
}

#[cfg(test)]
mod intern_cache_gc_patch_tests {
    use super::*;
    use duke_gc::Heap;
    use duke_runtime::{Frame, Slot};

    // Root cause of the gson InvalidRef: after the collector relocates an
    // interned constant, the interpreter's `string_intern` cache must be patched
    // to the new address, otherwise the next `ldc` of the same constant serves a
    // dangling reference. `patch_forwarded_slots` is responsible for that.
    #[test]
    fn patch_forwarded_slots_relocates_intern_cache_entries() {
        let mut heap = Heap::new();
        // Allocate an interned string and register it in the cache. Keep it live
        // via a stack slot so the collector relocates (rather than reclaims) it.
        let interned = heap.allocate_string("\\u%04x".to_string());
        let mut string_intern: std::collections::HashMap<(u8, String), u64> =
            std::collections::HashMap::new();
        string_intern.insert((0, "\\u%04x".to_string()), interned);

        let mut frame = Frame::new(4, 4, vec![Slot::Reference(Some(interned))])
            .expect("frame");
        let mut registry = ClassRegistry::new();
        let mut call_stack: Vec<CallFrame> = Vec::new();

        let roots = gather_roots(&frame, &call_stack, &registry, &string_intern);
        heap.collect(&roots);
        patch_forwarded_slots(
            &mut frame,
            &mut call_stack,
            &mut registry,
            &heap,
            &mut string_intern,
        );

        // The cache entry must now resolve to a live object with the same payload.
        let patched = string_intern
            .get(&(0, "\\u%04x".to_string()))
            .copied()
            .expect("cache entry survives");
        assert_eq!(
            heap.get(patched)
                .expect("patched ref is live")
                .string_value
                .as_deref(),
            Some("\\u%04x")
        );
    }
}

#[cfg(test)]
mod read_string_bytes_tests {
    use super::*;
    use duke_gc::Heap;

    #[test]
    fn round_trips_latin1_ascii_string() {
        let mut heap = Heap::new();
        let s = heap.allocate_string("Hello, World!".to_string());
        // coder 0 (Latin-1) for pure ASCII.
        assert_eq!(heap.get(s).unwrap().fields[1], Slot::Int(0));
        assert_eq!(read_string_bytes(&heap, s).unwrap(), "Hello, World!");
    }

    #[test]
    fn round_trips_latin1_high_byte_string() {
        // 'é' (U+00E9) stays Latin-1 (coder 0) but is stored as a signed Java byte.
        let mut heap = Heap::new();
        let s = heap.allocate_string("café".to_string());
        assert_eq!(heap.get(s).unwrap().fields[1], Slot::Int(0));
        assert_eq!(read_string_bytes(&heap, s).unwrap(), "café");
    }

    #[test]
    fn round_trips_utf16_string_with_supplementary_and_bmp() {
        // CJK (BMP, non-Latin-1) + emoji (supplementary → surrogate pair): both
        // force the UTF-16 little-endian (coder 1) path.
        let mut heap = Heap::new();
        let value = "中文🚀ok";
        let s = heap.allocate_string(value.to_string());
        assert_eq!(
            heap.get(s).unwrap().fields[1],
            Slot::Int(1),
            "non-Latin-1 content must use coder 1"
        );
        assert_eq!(read_string_bytes(&heap, s).unwrap(), value);
    }

    #[test]
    fn empty_string_decodes_to_empty_not_null() {
        let mut heap = Heap::new();
        let s = heap.allocate_string(String::new());
        assert_eq!(read_string_bytes(&heap, s).unwrap(), "");
    }

    #[test]
    fn round_trips_interned_literal_via_central_helper() {
        // Mirrors the ldc/intern mint path: allocate_string is what backs an
        // interned constant. The read flows through the rerouted central helper.
        let mut heap = Heap::new();
        let interned = heap.allocate_string("\\u%04x".to_string());
        assert_eq!(string_value_from_ref(&heap, interned).unwrap(), "\\u%04x");
        assert_eq!(read_string_bytes(&heap, interned).unwrap(), "\\u%04x");
    }

    #[test]
    fn null_value_slot_maps_to_npe() {
        // A String receiver whose value:[B slot was never populated (the null-src
        // copy-constructor case) must surface NullPointerException, matching the
        // pre-reroute string_value_from_ref behavior.
        let mut heap = Heap::new();
        let s = heap.allocate("java/lang/String".to_string(), 4);
        assert!(matches!(
            read_string_bytes(&heap, s),
            Err(Error::NullPointerException)
        ));
    }

    #[test]
    fn survives_minor_gc_and_promotion() {
        // The value:[B is reachable only through slot 0; after minor GCs move the
        // String (and promote it to old gen), read_string_bytes must still decode
        // from the forwarded backing array.
        let mut heap = Heap::new();
        let value = "héllo中🚀"; // Latin-1 + BMP + supplementary → UTF-16 path
        let s = heap.allocate_string(value.to_string());

        let mut root = Slot::Reference(Some(s));
        for _ in 0..8 {
            heap.minor_collect_prepare(&[root]);
            heap.apply_forward(&mut root);
            heap.minor_collect_finish();
        }
        let moved = root.as_reference().expect("String survives the collections");
        assert_eq!(read_string_bytes(&heap, moved).unwrap(), value);
    }

    /// Drive the compact-strings ctor `String.<init>([BB)V`
    /// ([`native_string_init_bytes_coder`]) exactly as the JDK boot does: take an
    /// already-encoded `(value:[B, coder)` pair off a source String and assign it
    /// to a fresh receiver. The receiver must decode back to the source value for
    /// both the Latin-1 (`coder == 0`) and UTF-16 (`coder == 1`) paths.
    fn assert_compact_ctor_round_trips(value: &str, expected_coder: i32) {
        let mut heap = Heap::new();
        // Source String supplies a well-formed (value:[B, coder) pair.
        let src = heap.allocate_string(value.to_string());
        let bytes_ref = match heap.get(src).unwrap().fields[0] {
            Slot::Reference(Some(r)) => r,
            other => panic!("source value:[B slot must be a live ref, got {other:?}"),
        };
        let coder = match heap.get(src).unwrap().fields[1] {
            Slot::Int(c) => c,
            other => panic!("source coder slot must be Int, got {other:?}"),
        };
        assert_eq!(coder, expected_coder, "coder for {value:?}");

        // Fresh, empty 4-slot receiver — the `new java/lang/String` shape before
        // `<init>` runs.
        let this = heap.allocate("java/lang/String".to_string(), 4);
        let args = [
            Slot::Reference(Some(this)),
            Slot::Reference(Some(bytes_ref)),
            Slot::Int(coder),
        ];
        let ret = native_string_init_bytes_coder(
            &args,
            &mut heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
        )
        .expect("compact-strings ctor must succeed");
        assert!(ret.is_none(), "a void <init> returns no value");

        assert_eq!(
            heap.get(this).unwrap().fields[1],
            Slot::Int(coder),
            "receiver coder must match the supplied coder"
        );
        assert_eq!(
            read_string_bytes(&heap, this).unwrap(),
            value,
            "receiver must decode back to the source value"
        );
    }

    #[test]
    fn compact_ctor_round_trips_latin1() {
        assert_compact_ctor_round_trips("café", 0);
    }

    #[test]
    fn compact_ctor_round_trips_utf16() {
        assert_compact_ctor_round_trips("中文🚀ok", 1);
    }
}

#[cfg(test)]
mod native_helper_tests {
    use super::*;

    #[test]
    fn should_return_error_when_extract_ref_arg_receives_int() {
        let args = vec![Slot::Int(42)];
        let res = extract_ref_arg(&args, 0);
        assert!(matches!(res, Err(Error::NullPointerException)));
    }

    #[test]
    fn should_return_error_when_extract_ref_arg_out_of_bounds() {
        let args = vec![];
        let res = extract_ref_arg(&args, 0);
        assert!(matches!(res, Err(Error::NullPointerException)));
    }

    #[test]
    fn should_return_error_when_extract_io_fd_receives_no_fields() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/lang/Object".to_string(), 0);
        let res = extract_io_fd(&heap, obj_ref);
        assert!(matches!(res, Err(Error::JavaException { .. })));
    }

    #[test]
    fn should_return_error_when_extract_io_fd_at_receives_no_fields() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/lang/Object".to_string(), 0);
        let res = extract_io_fd_at(&heap, obj_ref, 10);
        assert!(matches!(res, Err(Error::JavaException { .. })));
    }

    #[test]
    fn should_return_error_when_extract_int_arg_receives_ref() {
        let args = vec![Slot::Reference(None)];
        let res = extract_int_arg(&args, 0);
        assert!(matches!(res, Err(Error::TypeMismatch { .. })));
    }

    #[test]
    fn should_return_error_when_extract_int_arg_out_of_bounds() {
        let args = vec![];
        let res = extract_int_arg(&args, 0);
        assert!(matches!(res, Err(Error::TypeMismatch { .. })));
    }

    #[test]
    fn should_extract_null_for_out_of_bounds_field_arg() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/lang/Object".to_string(), 0);
        let res = extract_field_arg(&heap, obj_ref, 5).unwrap();
        assert_eq!(res, Slot::Reference(None));
    }

    #[test]
    fn should_extract_null_for_empty_first_field_arg() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/lang/Object".to_string(), 0);
        let res = extract_first_field_arg(&heap, obj_ref).unwrap();
        assert_eq!(res, Slot::Reference(None));
    }

    #[test]
    fn should_extract_null_for_out_of_bounds_slot_arg() {
        let args = vec![];
        let res = extract_slot_arg(&args, 0);
        assert_eq!(res, Slot::Reference(None));
    }

    #[test]
    fn should_init_reentrant_read_write_lock() {
        let mut heap = duke_gc::Heap::new();
        let lock_ref = heap.allocate("java/util/concurrent/locks/ReentrantReadWriteLock".to_string(), 2);
        let args = vec![Slot::Reference(Some(lock_ref))];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        let res = native_reentrant_read_write_lock_init(&args, &mut heap, &mut out, &mut control).unwrap();
        assert_eq!(res, None);
        let lock_obj = heap.get(lock_ref).unwrap();
        assert!(matches!(lock_obj.atomic_payload, Some(duke_gc::AtomicPayload::ReadWriteLock(_))));
        assert!(matches!(lock_obj.fields[0], Slot::Reference(Some(_))));
        assert!(matches!(lock_obj.fields[1], Slot::Reference(Some(_))));
    }

    #[test]
    fn should_return_existing_read_lock_if_present() {
        let mut heap = duke_gc::Heap::new();
        let lock_ref = heap.allocate("java/util/concurrent/locks/ReentrantReadWriteLock".to_string(), 2);
        let read_ref = heap.allocate("java/util/concurrent/locks/ReentrantReadWriteLock$ReadLock".to_string(), 0);
        heap.get_mut(lock_ref).unwrap().fields[0] = Slot::Reference(Some(read_ref));
        let args = vec![Slot::Reference(Some(lock_ref))];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        let res = native_reentrant_read_write_lock_read_lock(&args, &mut heap, &mut out, &mut control).unwrap();
        assert_eq!(res, Some(Slot::Reference(Some(read_ref))));
    }

    #[test]
    fn should_create_new_read_lock_if_missing() {
        let mut heap = duke_gc::Heap::new();
        let lock_ref = heap.allocate("java/util/concurrent/locks/ReentrantReadWriteLock".to_string(), 1);
        let state = std::sync::Arc::new(std::sync::Mutex::new(duke_gc::ReadWriteLockState::default()));
        heap.get_mut(lock_ref).unwrap().atomic_payload = Some(duke_gc::AtomicPayload::ReadWriteLock(state));
        let args = vec![Slot::Reference(Some(lock_ref))];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        let res = native_reentrant_read_write_lock_read_lock(&args, &mut heap, &mut out, &mut control).unwrap();
        assert!(matches!(res, Some(Slot::Reference(Some(_)))));
    }

    #[test]
    fn should_return_existing_write_lock_if_present() {
        let mut heap = duke_gc::Heap::new();
        let lock_ref = heap.allocate("java/util/concurrent/locks/ReentrantReadWriteLock".to_string(), 2);
        let write_ref = heap.allocate("java/util/concurrent/locks/ReentrantReadWriteLock$WriteLock".to_string(), 0);
        heap.get_mut(lock_ref).unwrap().fields[1] = Slot::Reference(Some(write_ref));
        let args = vec![Slot::Reference(Some(lock_ref))];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        let res = native_reentrant_read_write_lock_write_lock(&args, &mut heap, &mut out, &mut control).unwrap();
        assert_eq!(res, Some(Slot::Reference(Some(write_ref))));
    }

    #[test]
    fn should_create_new_write_lock_if_missing() {
        let mut heap = duke_gc::Heap::new();
        let lock_ref = heap.allocate("java/util/concurrent/locks/ReentrantReadWriteLock".to_string(), 2);
        let state = std::sync::Arc::new(std::sync::Mutex::new(duke_gc::ReadWriteLockState::default()));
        heap.get_mut(lock_ref).unwrap().atomic_payload = Some(duke_gc::AtomicPayload::ReadWriteLock(state));
        let args = vec![Slot::Reference(Some(lock_ref))];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        let res = native_reentrant_read_write_lock_write_lock(&args, &mut heap, &mut out, &mut control).unwrap();
        assert!(matches!(res, Some(Slot::Reference(Some(_)))));
    }


    #[test]
    fn should_return_none_when_priorityqueue_peek_size_zero() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/util/PriorityQueue".to_string(), 1);
        heap.get_mut(obj_ref).unwrap().fields[0] = Slot::Int(0);

        let args = vec![Slot::Reference(Some(obj_ref))];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        let res = native_priorityqueue_peek(&args, &mut heap, &mut out, &mut control).unwrap();
        assert_eq!(res, Some(Slot::Reference(None)));
    }

    #[test]
    fn should_return_none_when_priorityqueue_peek_no_fields() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/util/PriorityQueue".to_string(), 0);

        let args = vec![Slot::Reference(Some(obj_ref))];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        let res = native_priorityqueue_peek(&args, &mut heap, &mut out, &mut control).unwrap();
        assert_eq!(res, Some(Slot::Reference(None)));
    }

    #[test]
    fn should_return_element_when_priorityqueue_peek_size_not_zero() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/util/PriorityQueue".to_string(), 2);
        heap.get_mut(obj_ref).unwrap().fields[0] = Slot::Int(1);
        heap.get_mut(obj_ref).unwrap().fields[1] = Slot::Int(42);

        let args = vec![Slot::Reference(Some(obj_ref))];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        let res = native_priorityqueue_peek(&args, &mut heap, &mut out, &mut control).unwrap();
        assert_eq!(res, Some(Slot::Int(42)));
    }

}

#[cfg(test)]
mod tests_sentry {

    #[test]
    fn test_atomic_helpers_error_paths() {
        let mut heap = duke_gc::Heap::new();
        let this_ref = heap.allocate("java/lang/Object".to_string(), 0);

        let err_i32 = super::with_atomic_i32(&heap, this_ref, |_| ()).unwrap_err();
        assert!(matches!(err_i32, crate::Error::InvalidRef { address: _ }));

        let err_i64 = super::with_atomic_i64(&heap, this_ref, |_| ()).unwrap_err();
        assert!(matches!(err_i64, crate::Error::InvalidRef { address: _ }));

        let err_bool = super::with_atomic_bool(&heap, this_ref, |_| ()).unwrap_err();
        assert!(matches!(err_bool, crate::Error::InvalidRef { address: _ }));
    }

}

#[cfg(test)]
mod havoc_matcher_bounds_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_next_find_pos_panics_on_invalid_boundary(
            s in ".*",
            end in 0..1000usize
        ) {
            let start = end;
            if start <= s.len() && !s.is_char_boundary(end) {
                // This should no longer trigger a panic.
                let _ = next_find_pos(&s, start, end);
            }
        }
    }
}

#[cfg(test)]
mod havoc_string_tests {
    use super::*;
    #[test]
    fn test_string_indexof_from_panic() {
        let mut heap = duke_gc::Heap::new();
        let this_ref = heap.allocate_string("h\u{1f4a9}world".to_string());
        let target_ref = heap.allocate_string("world".to_string());
        let args = [
            Slot::Reference(Some(this_ref)),
            Slot::Reference(Some(target_ref)),
            Slot::Int(2), // 2 is inside \u{1f4a9} which spans bytes 1..5
        ];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        let _ = native_string_indexof_from(&args, &mut heap, &mut out, &mut control);
    }

    #[test]
    fn test_string_last_indexof_from_panic() {
        let mut heap = duke_gc::Heap::new();
        let this_ref = heap.allocate_string("h\u{1f4a9}world".to_string());
        let target_ref = heap.allocate_string("o".to_string()); // len = 1
        let args = [
            Slot::Reference(Some(this_ref)),
            Slot::Reference(Some(target_ref)),
            Slot::Int(2), // 2 is inside \u{1f4a9} which spans bytes 1..5
        ];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        // from + sub_str.len() = 2 + 1 = 3. 3 is inside \u{1f4a9} (bytes 1..5)
        let _ = native_string_last_indexof_from(&args, &mut heap, &mut out, &mut control);
    }
}

#[cfg(test)]
mod unsafe_object_field_tests {
    use super::*;

    /// Minimal `CallbackOps` returning a fixed slot index for `instance_field_slot`,
    /// and recording the class/field names it was asked about.
    struct FixedSlotOps {
        slot: usize,
        seen: Option<(String, String)>,
    }

    impl CallbackOps for FixedSlotOps {
        fn invoke(
            &mut self,
            _heap: &mut duke_gc::Heap,
            _output: &mut dyn Write,
            _class: &str,
            _method: &str,
            _descriptor: &str,
            _args: Vec<Slot>,
        ) -> Result<Option<Slot>> {
            Ok(None)
        }

        fn ensure_loaded(&mut self, _class: &str) -> Result<()> {
            Ok(())
        }

        fn inspect_class(&mut self, _class: &str) -> Result<ReflectedClassInfo> {
            Err(Error::ClassNotFound {
                name: String::new(),
            })
        }

        fn instance_field_slot(&mut self, class: &str, field_name: &str) -> Result<usize> {
            self.seen = Some((class.to_string(), field_name.to_string()));
            Ok(self.slot)
        }
    }

    fn ctrl() -> NativeControl {
        NativeControl::default()
    }

    #[test]
    fn object_field_offset_returns_positional_slot_index() {
        let mut heap = duke_gc::Heap::new();
        // A `java/lang/Class` mirror is a string-backed object holding the class key.
        let class_ref = heap.allocate("java/lang/Class".to_string(), 0);
        heap.get_mut(class_ref).unwrap().string_value =
            Some("java/util/concurrent/ConcurrentHashMap".to_string());
        let name_ref = heap.allocate_string("baseCount".to_string());
        let args = [
            Slot::Reference(Some(0)), // this (Unsafe) — ignored
            Slot::Reference(Some(class_ref)),
            Slot::Reference(Some(name_ref)),
        ];
        let mut out = Vec::new();
        let mut ops = FixedSlotOps {
            slot: 5,
            seen: None,
        };
        let result =
            native_unsafe_object_field_offset(&args, &mut heap, &mut out, &mut ctrl(), &mut ops)
                .unwrap();
        assert_eq!(result, Some(Slot::Long(5)));
        assert_eq!(
            ops.seen,
            Some((
                "java/util/concurrent/ConcurrentHashMap".to_string(),
                "baseCount".to_string()
            ))
        );
    }

    #[test]
    fn array_base_offset_is_zero_and_index_scale_is_one() {
        let mut heap = duke_gc::Heap::new();
        let class_ref = heap.allocate("java/lang/Class".to_string(), 0);
        let args = [Slot::Reference(Some(class_ref))];
        let mut out = Vec::new();
        assert_eq!(
            native_unsafe_array_base_offset(&args, &mut heap, &mut out, &mut ctrl()).unwrap(),
            Some(Slot::Int(0))
        );
        assert_eq!(
            native_unsafe_array_index_scale(&args, &mut heap, &mut out, &mut ctrl()).unwrap(),
            Some(Slot::Int(1))
        );
    }

    #[test]
    fn get_unsafe_returns_nonnull_unsafe_reference() {
        let mut heap = duke_gc::Heap::new();
        let mut out = Vec::new();
        let result = native_unsafe_get_unsafe(&[], &mut heap, &mut out, &mut ctrl()).unwrap();
        let Some(Slot::Reference(Some(r))) = result else {
            panic!("expected a non-null Unsafe reference, got {result:?}");
        };
        assert_eq!(heap.get(r).unwrap().class_name, "jdk/internal/misc/Unsafe");
    }

    #[test]
    fn compare_and_set_reference_swaps_on_identity_match() {
        let mut heap = duke_gc::Heap::new();
        let holder = heap.allocate("Holder".to_string(), 2);
        let a = heap.allocate("A".to_string(), 0);
        let b = heap.allocate("B".to_string(), 0);
        // fields[1] currently holds `a`.
        heap.get_mut(holder).unwrap().fields[1] = Slot::Reference(Some(a));
        let args = [
            Slot::Reference(Some(0)), // this
            Slot::Reference(Some(holder)),
            Slot::Long(1), // offset == slot index
            Slot::Reference(Some(a)),
            Slot::Reference(Some(b)),
        ];
        let mut out = Vec::new();
        let result =
            native_unsafe_compare_and_set_reference(&args, &mut heap, &mut out, &mut ctrl())
                .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert_eq!(
            heap.get(holder).unwrap().fields[1],
            Slot::Reference(Some(b))
        );
    }

    #[test]
    fn compare_and_set_reference_noops_on_mismatch() {
        let mut heap = duke_gc::Heap::new();
        let holder = heap.allocate("Holder".to_string(), 2);
        let a = heap.allocate("A".to_string(), 0);
        let b = heap.allocate("B".to_string(), 0);
        let c = heap.allocate("C".to_string(), 0);
        heap.get_mut(holder).unwrap().fields[1] = Slot::Reference(Some(a));
        let args = [
            Slot::Reference(Some(0)),
            Slot::Reference(Some(holder)),
            Slot::Long(1),
            Slot::Reference(Some(c)), // expected `c` but field holds `a`
            Slot::Reference(Some(b)),
        ];
        let mut out = Vec::new();
        let result =
            native_unsafe_compare_and_set_reference(&args, &mut heap, &mut out, &mut ctrl())
                .unwrap();
        assert_eq!(result, Some(Slot::Int(0)));
        // Unchanged.
        assert_eq!(
            heap.get(holder).unwrap().fields[1],
            Slot::Reference(Some(a))
        );
    }

    #[test]
    fn compare_and_set_int_and_long_swap_and_noop() {
        let mut heap = duke_gc::Heap::new();
        let holder = heap.allocate("Holder".to_string(), 2);
        heap.get_mut(holder).unwrap().fields[0] = Slot::Int(7);
        heap.get_mut(holder).unwrap().fields[1] = Slot::Long(100);
        let mut out = Vec::new();

        // int: match → swap
        let ok_args = [
            Slot::Reference(Some(0)),
            Slot::Reference(Some(holder)),
            Slot::Long(0),
            Slot::Int(7),
            Slot::Int(9),
        ];
        assert_eq!(
            native_unsafe_compare_and_set_int(&ok_args, &mut heap, &mut out, &mut ctrl()).unwrap(),
            Some(Slot::Int(1))
        );
        assert_eq!(heap.get(holder).unwrap().fields[0], Slot::Int(9));

        // int: mismatch → no-op
        let bad_args = [
            Slot::Reference(Some(0)),
            Slot::Reference(Some(holder)),
            Slot::Long(0),
            Slot::Int(7), // stale expected
            Slot::Int(11),
        ];
        assert_eq!(
            native_unsafe_compare_and_set_int(&bad_args, &mut heap, &mut out, &mut ctrl()).unwrap(),
            Some(Slot::Int(0))
        );
        assert_eq!(heap.get(holder).unwrap().fields[0], Slot::Int(9));

        // long: match → swap
        let long_args = [
            Slot::Reference(Some(0)),
            Slot::Reference(Some(holder)),
            Slot::Long(1),
            Slot::Long(100),
            Slot::Long(250),
        ];
        assert_eq!(
            native_unsafe_compare_and_set_long(&long_args, &mut heap, &mut out, &mut ctrl())
                .unwrap(),
            Some(Slot::Int(1))
        );
        assert_eq!(heap.get(holder).unwrap().fields[1], Slot::Long(250));
    }

    #[test]
    fn get_and_put_reference_round_trip() {
        let mut heap = duke_gc::Heap::new();
        let holder = heap.allocate("Holder".to_string(), 3);
        let v = heap.allocate("V".to_string(), 0);
        heap.get_mut(holder).unwrap().fields[2] = Slot::Reference(None);
        let mut out = Vec::new();

        let put_args = [
            Slot::Reference(Some(0)),
            Slot::Reference(Some(holder)),
            Slot::Long(2),
            Slot::Reference(Some(v)),
        ];
        assert_eq!(
            native_unsafe_put_reference(&put_args, &mut heap, &mut out, &mut ctrl()).unwrap(),
            None
        );

        let get_args = [
            Slot::Reference(Some(0)),
            Slot::Reference(Some(holder)),
            Slot::Long(2),
        ];
        assert_eq!(
            native_unsafe_get_reference(&get_args, &mut heap, &mut out, &mut ctrl()).unwrap(),
            Some(Slot::Reference(Some(v)))
        );
    }

    #[test]
    fn get_reference_acquire_reads_array_element_by_index() {
        // Under arrayBaseOffset==0 / arrayIndexScale==1, an Unsafe offset equals the
        // array element index into the backing `fields` vector.
        let mut heap = duke_gc::Heap::new();
        let node = heap.allocate("Node".to_string(), 0);
        let array = heap.allocate("[LNode;".to_string(), 4);
        heap.get_mut(array).unwrap().fields[3] = Slot::Reference(Some(node));
        let args = [
            Slot::Reference(Some(0)),
            Slot::Reference(Some(array)),
            Slot::Long(3),
        ];
        let mut out = Vec::new();
        assert_eq!(
            native_unsafe_get_reference(&args, &mut heap, &mut out, &mut ctrl()).unwrap(),
            Some(Slot::Reference(Some(node)))
        );
    }

    #[test]
    fn get_and_add_int_returns_old_and_writes_sum() {
        let mut heap = duke_gc::Heap::new();
        let holder = heap.allocate("Holder".to_string(), 1);
        heap.get_mut(holder).unwrap().fields[0] = Slot::Int(10);
        let args = [
            Slot::Reference(Some(0)),
            Slot::Reference(Some(holder)),
            Slot::Long(0),
            Slot::Int(5),
        ];
        let mut out = Vec::new();
        assert_eq!(
            native_unsafe_get_and_add_int(&args, &mut heap, &mut out, &mut ctrl()).unwrap(),
            Some(Slot::Int(10))
        );
        assert_eq!(heap.get(holder).unwrap().fields[0], Slot::Int(15));
    }

    #[test]
    fn out_of_bounds_offset_is_reported_as_npe() {
        let mut heap = duke_gc::Heap::new();
        let holder = heap.allocate("Holder".to_string(), 1);
        let args = [
            Slot::Reference(Some(0)),
            Slot::Reference(Some(holder)),
            Slot::Long(99),
        ];
        let mut out = Vec::new();
        let err =
            native_unsafe_get_reference(&args, &mut heap, &mut out, &mut ctrl()).unwrap_err();
        assert!(matches!(err, Error::NullPointerException));
    }
}

#[cfg(test)]
mod tests_char_titlecase {
    use super::*;

    /// Golden `(codepoint, Character.toTitleCase(codepoint))` pairs captured
    /// from `OpenJDK` 21 (`javac`/`java` 21.0.10). titlecase equals simple
    /// uppercase everywhere except the Latin digraph titlecase characters, and
    /// characters with no simple uppercase (`ß`, `ﬀ`) map to themselves.
    const TITLECASE_GOLDENS: &[(u32, u32)] = &[
        (0x0061, 0x0041), // 'a' -> 'A'
        (0x007A, 0x005A), // 'z' -> 'Z'
        (0x0041, 0x0041), // 'A' -> 'A'
        (0x0020, 0x0020), // space unchanged
        (0x0039, 0x0039), // '9' unchanged
        (0x00DF, 0x00DF), // 'ß' has no simple uppercase -> itself
        (0x01C4, 0x01C5), // 'Ǆ' -> titlecase 'ǅ'
        (0x01C5, 0x01C5), // 'ǅ' already titlecase
        (0x01C6, 0x01C5), // 'ǆ' -> titlecase 'ǅ'
        (0x01C7, 0x01C8),
        (0x01C8, 0x01C8),
        (0x01C9, 0x01C8),
        (0x01CA, 0x01CB),
        (0x01CB, 0x01CB),
        (0x01CC, 0x01CB),
        (0x01F1, 0x01F2),
        (0x01F2, 0x01F2),
        (0x01F3, 0x01F2),
        (0x0068, 0x0048), // 'h' -> 'H' (commons-lang3 capitalize path)
        (0x0048, 0x0048), // 'H' unchanged
        (0x00E9, 0x00C9), // 'é' -> 'É'
        (0x00C9, 0x00C9), // 'É' unchanged
        (0x4E2D, 0x4E2D), // CJK unchanged
        (0x1F600, 0x1F600), // emoji (supplementary) unchanged
        (0x0130, 0x0130), // 'İ' unchanged
        (0x0131, 0x0049), // 'ı' dotless i -> 'I'
        (0x1E9E, 0x1E9E), // 'ẞ' capital sharp s unchanged
        (0xFB00, 0xFB00), // 'ﬀ' ligature has no simple uppercase -> itself
    ];

    #[test]
    fn to_titlecase_matches_java21_goldens() {
        let mut heap = duke_gc::Heap::new();
        let mut out = Vec::new();
        for &(cp, expected) in TITLECASE_GOLDENS {
            let result = native_char_to_titlecase(
                &[Slot::Int(cp.cast_signed())],
                &mut heap,
                &mut out,
                &mut NativeControl::default(),
            )
            .unwrap();
            assert_eq!(
                result,
                Some(Slot::Int(expected.cast_signed())),
                "toTitleCase(0x{cp:04X}) expected 0x{expected:04X}",
            );
        }
    }
}

#[cfg(test)]
mod lambda_reflection_tests {
    use super::*;
    use crate::registry::LambdaInfo;

    fn empty_loader() -> duke_loader::DirectoryLoader {
        // Points at the workspace root; it will never resolve a `$$Lambda$N`
        // classfile, so `inspect_reflected_class` falls to the registered
        // synthetic `ClassContext`.
        duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap(),
        )
    }

    fn function_lambda() -> LambdaInfo {
        LambdaInfo {
            impl_class: "Foo".to_string(),
            impl_method: "lambda$0".to_string(),
            impl_desc: "(Ljava/lang/Object;)Ljava/lang/Object;".to_string(),
            impl_kind: 6,
            sam_method: "apply".to_string(),
            sam_desc: "(Ljava/lang/Object;)Ljava/lang/Object;".to_string(),
            sam_interface: "java/util/function/Function".to_string(),
            captured_count: 0,
        }
    }

    #[test]
    fn synthesized_lambda_sam_returns_sam_for_lambda_only() {
        let mut registry = ClassRegistry::new();
        let name = registry.register_lambda(function_lambda());

        let sam = synthesized_lambda_sam(&registry, &name).expect("lambda SAM synthesized");
        assert_eq!(sam.name, "apply");
        assert_eq!(sam.descriptor, "(Ljava/lang/Object;)Ljava/lang/Object;");
        assert!(sam.is_public, "the SAM is a public method");
        assert!(!sam.is_static, "the SAM is a concrete instance method");
        assert!(sam.signature.is_none());

        // Non-lambda classes get no synthesized method.
        assert!(synthesized_lambda_sam(&registry, "java/lang/Object").is_none());
    }

    #[test]
    fn inspect_reflected_class_lists_the_lambda_sam() {
        // `inspect_reflected_class` is the shared source of the method list for
        // both `getDeclaredMethods` (reads `.methods` directly) and `getMethods`
        // (walks classes via `collect_public_reflected_methods`, reading the same
        // per-class `.methods`). The lambda ClassContext has EMPTY methods (so the
        // Missing-fallback SAM dispatch stays intact), so the SAM must come from
        // the synthesized entry.
        let mut registry = ClassRegistry::new();
        let name = registry.register_lambda(function_lambda());
        let loader = empty_loader();

        let info = inspect_reflected_class(&mut registry, &loader, &name)
            .expect("reflect the synthetic lambda proxy");
        let names: Vec<&str> = info.methods.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["apply"],
            "getDeclaredMethods must enumerate exactly the lambda SAM"
        );
        let sam = &info.methods[0];
        assert_eq!(sam.descriptor, "(Ljava/lang/Object;)Ljava/lang/Object;");
        assert!(sam.is_public && !sam.is_static);
        // The dispatch side-channel is untouched: ClassContext.methods stays empty.
        assert!(
            registry.get(&name).expect("lambda ctx").methods.is_empty(),
            "the synthesized SAM must NOT be added to ClassContext.methods"
        );
    }
}
