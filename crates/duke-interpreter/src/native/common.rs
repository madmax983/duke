
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
