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
