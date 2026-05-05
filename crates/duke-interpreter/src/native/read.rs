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
        return std::fs::read(path)
            .map_err(|_| Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
    }
    Err(Error::JavaException {
        class_name: "java/io/IOException".to_string(),
    })
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
        Some(duke_gc::AtomicPayload::ReadWriteLock(state)) => {
            Ok(std::sync::Arc::clone(state))
        }
        _ => Err(atomic_payload_error(this_ref)),
    }
}
fn read_write_view_state(
    heap: &duke_gc::Heap,
    this_ref: u64,
    expected_kind: duke_gc::ReadWriteLockViewKind,
) -> Result<std::sync::Arc<std::sync::Mutex<duke_gc::ReadWriteLockState>>> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(
            duke_gc::AtomicPayload::ReadWriteLockView { state, kind },
        ) if *kind == expected_kind => Ok(std::sync::Arc::clone(state)),
        _ => Err(atomic_payload_error(this_ref)),
    }
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
pub(crate) fn native_read_lock_lock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_view_state(
        heap,
        this_ref,
        duke_gc::ReadWriteLockViewKind::Read,
    )?;
    let thread_id = current_host_thread_id();
    let mut guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
    let state = read_write_view_state(
        heap,
        this_ref,
        duke_gc::ReadWriteLockViewKind::Read,
    )?;
    let thread_id = current_host_thread_id();
    let mut guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(Some(Slot::Int(i32::from(read_lock_try_acquire(&mut guard, thread_id)))))
}
pub(crate) fn native_read_lock_unlock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_view_state(
        heap,
        this_ref,
        duke_gc::ReadWriteLockViewKind::Read,
    )?;
    let thread_id = current_host_thread_id();
    let mut guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
