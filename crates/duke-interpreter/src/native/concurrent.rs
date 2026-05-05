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

pub(crate) fn native_concurrent_hashmap_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let _lock = concurrent_hashmap_lock(heap, this_ref)?;
    native_hashmap_init(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_init_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_concurrent_hashmap_init(args, heap, out, control)?;
    let this_ref = extract_ref_arg(args, 0)?;
    let source_ref = extract_ref_arg(args, 1)?;
    native_concurrent_hashmap_put_all(
        &[
            Slot::Reference(Some(this_ref)),
            Slot::Reference(Some(source_ref)),
        ],
        heap,
        out,
        control,
    )
}

pub(crate) fn native_concurrent_hashmap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_get(&[Slot::Reference(Some(this_ref)), key], heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let default = extract_slot_arg(args, 2);
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_get_or_default(
        &[Slot::Reference(Some(this_ref)), key, default],
        heap,
        out,
        control,
    )
}

pub(crate) fn native_concurrent_hashmap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_contains_key(&[Slot::Reference(Some(this_ref)), key], heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_contains_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = chm_non_null_arg(args, 1)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_contains_value(&[Slot::Reference(Some(this_ref)), value], heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_size(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_is_empty(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let value = chm_non_null_arg(args, 2)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_put(
        &[Slot::Reference(Some(this_ref)), key, value],
        heap,
        out,
        control,
    )
}

pub(crate) fn native_concurrent_hashmap_put_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let value = chm_non_null_arg(args, 2)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_put_if_absent(
        &[Slot::Reference(Some(this_ref)), key, value],
        heap,
        out,
        control,
    )
}

pub(crate) fn native_concurrent_hashmap_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_remove(&[Slot::Reference(Some(this_ref)), key], heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_remove_key_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let expected = chm_non_null_arg(args, 2)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_remove_key_value(
        &[Slot::Reference(Some(this_ref)), key, expected],
        heap,
        out,
        control,
    )
}

pub(crate) fn native_concurrent_hashmap_replace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let value = chm_non_null_arg(args, 2)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_replace(
        &[Slot::Reference(Some(this_ref)), key, value],
        heap,
        out,
        control,
    )
}

pub(crate) fn native_concurrent_hashmap_replace_key_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let expected = chm_non_null_arg(args, 2)?;
    let replacement = chm_non_null_arg(args, 3)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    let fields = heap.get(this_ref)?.fields.clone();
    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let actual = fields[i + 1];
        if slots_equal(&actual, &expected, heap) {
            heap.get_mut(this_ref)?.fields[i + 1] = replacement;
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

pub(crate) fn native_concurrent_hashmap_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_clear(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_put_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let source_ref = extract_ref_arg(args, 1)?;
    let entries = chm_entry_snapshot(heap, source_ref)?;
    for (key, value) in &entries {
        require_chm_non_null(*key)?;
        require_chm_non_null(*value)?;
    }

    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    for (key, value) in entries {
        native_hashmap_put(
            &[Slot::Reference(Some(this_ref)), key, value],
            heap,
            out,
            control,
        )?;
    }
    Ok(None)
}

pub(crate) fn native_concurrent_hashmap_key_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_key_set(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_values(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_values(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_entry_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_entry_set(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_compute_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_slot = Slot::Reference(Some(fn_ref));
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    {
        let _guard = concurrent_hashmap_guard(&lock);
        let fields = &heap.get(this_ref)?.fields;
        if let Some(i) = find_hashmap_entry_index(fields, &key, heap) {
            return Ok(Some(fields[i + 1]));
        }
    }

    let computed = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, key],
    )?;
    let Some(value) = computed else {
        return Ok(Some(Slot::Reference(None)));
    };
    if matches!(value, Slot::Reference(None)) {
        return Ok(Some(Slot::Reference(None)));
    }

    let _guard = concurrent_hashmap_guard(&lock);
    let fields = &heap.get(this_ref)?.fields;
    if let Some(i) = find_hashmap_entry_index(fields, &key, heap) {
        return Ok(Some(fields[i + 1]));
    }
    native_hashmap_put(
        &[Slot::Reference(Some(this_ref)), key, value],
        heap,
        out,
        &mut NativeControl::default(),
    )?;
    Ok(Some(value))
}

pub(crate) fn native_concurrent_hashmap_compute_if_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_slot = Slot::Reference(Some(fn_ref));
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let old_value = {
        let _guard = concurrent_hashmap_guard(&lock);
        let fields = &heap.get(this_ref)?.fields;
        match find_hashmap_entry_index(fields, &key, heap) {
            Some(i) => fields[i + 1],
            None => return Ok(Some(Slot::Reference(None))),
        }
    };

    let new_value = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, key, old_value],
    )?;
    let _guard = concurrent_hashmap_guard(&lock);
    match new_value {
        Some(value) if !matches!(value, Slot::Reference(None)) => {
            native_hashmap_put(
                &[Slot::Reference(Some(this_ref)), key, value],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            Ok(Some(value))
        }
        _ => {
            native_hashmap_remove(
                &[Slot::Reference(Some(this_ref)), key],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            Ok(Some(Slot::Reference(None)))
        }
    }
}

pub(crate) fn native_concurrent_hashmap_compute(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_slot = Slot::Reference(Some(fn_ref));
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let old_value = {
        let _guard = concurrent_hashmap_guard(&lock);
        let fields = &heap.get(this_ref)?.fields;
        find_hashmap_entry_index(fields, &key, heap)
            .map_or(Slot::Reference(None), |i| fields[i + 1])
    };

    let new_value = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, key, old_value],
    )?;
    let _guard = concurrent_hashmap_guard(&lock);
    match new_value {
        Some(value) if !matches!(value, Slot::Reference(None)) => {
            native_hashmap_put(
                &[Slot::Reference(Some(this_ref)), key, value],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            Ok(Some(value))
        }
        _ => {
            native_hashmap_remove(
                &[Slot::Reference(Some(this_ref)), key],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            Ok(Some(Slot::Reference(None)))
        }
    }
}

pub(crate) fn native_concurrent_hashmap_merge(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let value = chm_non_null_arg(args, 2)?;
    let fn_ref = extract_ref_arg(args, 3)?;
    let fn_slot = Slot::Reference(Some(fn_ref));
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let old_value = {
        let _guard = concurrent_hashmap_guard(&lock);
        let fields = &heap.get(this_ref)?.fields;
        if let Some(i) = find_hashmap_entry_index(fields, &key, heap) {
            Some(fields[i + 1])
        } else {
            native_hashmap_put(
                &[Slot::Reference(Some(this_ref)), key, value],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            return Ok(Some(value));
        }
    };
    let Some(old_value) = old_value else {
        return Ok(Some(value));
    };

    let merged = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, old_value, value],
    )?;
    let _guard = concurrent_hashmap_guard(&lock);
    match merged {
        Some(merged_value) if !matches!(merged_value, Slot::Reference(None)) => {
            let merged_value = box_primitive_slot(merged_value, heap);
            native_hashmap_put(
                &[Slot::Reference(Some(this_ref)), key, merged_value],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            Ok(Some(merged_value))
        }
        _ => {
            native_hashmap_remove(
                &[Slot::Reference(Some(this_ref)), key],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            Ok(Some(Slot::Reference(None)))
        }
    }
}

pub(crate) fn native_concurrent_hashmap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let consumer_ref = extract_ref_arg(args, 1)?;
    let consumer_slot = Slot::Reference(Some(consumer_ref));
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let entries = {
        let _guard = concurrent_hashmap_guard(&lock);
        chm_entry_snapshot(heap, this_ref)?
    };
    for (key, value) in entries {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;Ljava/lang/Object;)V",
            vec![consumer_slot, key, value],
        )?;
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// HashSet natives
// ---------------------------------------------------------------------------

pub(crate) fn native_set_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;

    match extract_slot_arg(args, 0) {
        Slot::Reference(Some(array_ref)) => {
            let elements = heap.get(array_ref)?.fields.clone();
            for element in elements {
                native_hashset_add(
                    &[Slot::Reference(Some(set_ref)), element],
                    heap,
                    out,
                    control,
                )?;
            }
        }
        Slot::Reference(None) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    }

    Ok(Some(Slot::Reference(Some(set_ref))))
}

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

pub(crate) fn native_hashset_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.is_empty() {
        obj.fields.push(Slot::Int(0));
    } else {
        obj.fields[0] = Slot::Int(0);
    }
    Ok(None)
}

pub(crate) fn native_hashset_init_from_collection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let mut this_ref = extract_ref_arg(args, 0)?;
    let collection_ref = extract_ref_arg(args, 1)?;
    native_hashset_init(&[Slot::Reference(Some(this_ref))], heap, output, control)?;

    let collection_class = heap.get(collection_ref)?.class_name.clone();
    let elements: Vec<Slot> = if collection_class == "java/util/HashSet" {
        heap.get(collection_ref)?
            .fields
            .iter()
            .skip(1)
            .copied()
            .collect()
    } else {
        let array_slot = ops.invoke(
            heap,
            output,
            &collection_class,
            "toArray",
            "()[Ljava/lang/Object;",
            vec![Slot::Reference(Some(collection_ref))],
        )?;
        let Some(Slot::Reference(Some(array_ref))) = array_slot else {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        };
        patch_forwarded_ref_if_needed(heap, &mut this_ref);
        heap.get(array_ref)?.fields.clone()
    };

    for element in elements {
        native_hashset_add(
            &[Slot::Reference(Some(this_ref)), element],
            heap,
            output,
            control,
        )?;
    }
    Ok(None)
}

/// Native: `HashSet.add(Object)Z` — adds element if not already present.
/// Returns 1 if added, 0 if element was already in the set.
pub(crate) fn native_hashset_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    // fields[0] = size, fields[1..] = elements
    if find_hashset_entry_index(&heap.get(this_ref)?.fields, &element, heap).is_some() {
        return Ok(Some(Slot::Int(0))); // duplicate
    }

    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(Error::NullPointerException),
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1)))
}

/// Native: `HashSet.contains(Object)Z` — returns 1 if element is present, 0 otherwise.
pub(crate) fn native_hashset_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let fields = &heap.get(this_ref)?.fields;

    if find_hashset_entry_index(fields, &element, heap).is_some() {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashSet.remove(Object)Z` — removes element if present, returns 1 if removed, 0 if absent.
/// Uses swap-remove (swaps target with last element) for O(1) deletion.
pub(crate) fn native_hashset_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let i_opt = find_hashset_entry_index(&heap.get(this_ref)?.fields, &element, heap);

    if let Some(i) = i_opt {
        let obj = heap.get_mut(this_ref)?;
        let last_idx = obj.fields.len() - 1;
        obj.fields.swap(i, last_idx);
        obj.fields.truncate(obj.fields.len() - 1);
        match obj.fields.first_mut() {
            Some(Slot::Int(sz)) => *sz -= 1,
            _ => return Err(Error::NullPointerException),
        }
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashSet.size()I` — returns element count from fields\[0\].
pub(crate) fn native_hashset_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Ok(Some(Slot::Int(*n))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `HashSet.isEmpty()Z` — returns 1 if size == 0, else 0.
pub(crate) fn native_hashset_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) => Ok(Some(Slot::Int(1))),
        Some(Slot::Int(_)) => Ok(Some(Slot::Int(0))),
        _ => Ok(Some(Slot::Int(1))),
    }
}

/// Native: `HashSet.iterator()Iterator` — creates a `HashSetIterator`.
pub(crate) fn native_hashset_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_ref = heap.allocate("duke/util/HashSetIterator".to_string(), 2);
    {
        let iter_obj = heap.get_mut(iter_ref)?;
        iter_obj.fields[0] = Slot::Reference(Some(this_ref));
        iter_obj.fields[1] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
}

/// Native: `HashSetIterator.<init>` — no-op; fields are set by `native_hashset_iterator`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_hashset_iter_init(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}

/// Native: `HashSetIterator.hasNext()Z`
pub(crate) fn native_hashset_iter_hasnext(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_obj = heap.get(this_ref)?;
    let set_ref = match iter_obj.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Int(0))),
    };
    let cursor = match iter_obj.fields.get(1) {
        Some(Slot::Int(i)) => *i,
        _ => 0,
    };
    let set_size = match heap.get(set_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(cursor < set_size))))
}

/// Native: `HashSetIterator.next()Object` — returns element at cursor, advances cursor.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_hashset_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (set_ref, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let sr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(Error::NullPointerException),
        };
        let c = match iter_obj.fields.get(1) {
            Some(Slot::Int(i)) => *i,
            _ => 0,
        };
        (sr, c)
    };
    let element = {
        let set_obj = heap.get(set_ref)?;
        match set_obj.fields.get(cursor as usize + 1) {
            Some(slot) => *slot,
            None => {
                return Err(Error::JavaException {
                    class_name: "java/util/NoSuchElementException".to_string(),
                });
            }
        }
    };
    heap.get_mut(this_ref)?.fields[1] = Slot::Int(cursor + 1);
    Ok(Some(element))
}
