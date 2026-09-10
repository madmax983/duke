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
/// `Executors.newVirtualThreadPerTaskExecutor()` — one (virtual) thread per task.
pub(crate) fn native_executors_new_virtual_thread_per_task_executor(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // Virtual threads are cheap; model as an unbounded pool.
    Ok(Some(allocate_executor(heap, 256)?))
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
        Slot::Reference(Some(reference)) => heap_object_to_string_ref(heap, reference)?,
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
/// Native: `ReentrantReadWriteLock.init()`
#[allow(clippy::unnecessary_wraps)]
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
/// Native: `ReentrantReadWriteLock.readLock()`
#[allow(clippy::unnecessary_wraps)]
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
/// Native: `ReentrantReadWriteLock.writeLock()`
#[allow(clippy::unnecessary_wraps)]
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
pub(crate) fn native_base64_get_encoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let encoder_ref =
        allocate_base64_coder(heap, "java/util/Base64$Encoder", Base64Variant::Standard)?;
    Ok(Some(Slot::Reference(Some(encoder_ref))))
}
pub(crate) fn native_base64_get_mime_encoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let encoder_ref =
        allocate_base64_coder(heap, "java/util/Base64$Encoder", Base64Variant::Mime)?;
    Ok(Some(Slot::Reference(Some(encoder_ref))))
}
pub(crate) fn native_base64_get_url_encoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let encoder_ref = allocate_base64_coder(heap, "java/util/Base64$Encoder", Base64Variant::Url)?;
    Ok(Some(Slot::Reference(Some(encoder_ref))))
}
pub(crate) fn native_base64_get_decoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let decoder_ref =
        allocate_base64_coder(heap, "java/util/Base64$Decoder", Base64Variant::Standard)?;
    Ok(Some(Slot::Reference(Some(decoder_ref))))
}
pub(crate) fn native_base64_get_mime_decoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let decoder_ref =
        allocate_base64_coder(heap, "java/util/Base64$Decoder", Base64Variant::Mime)?;
    Ok(Some(Slot::Reference(Some(decoder_ref))))
}
pub(crate) fn native_base64_get_url_decoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let decoder_ref = allocate_base64_coder(heap, "java/util/Base64$Decoder", Base64Variant::Url)?;
    Ok(Some(Slot::Reference(Some(decoder_ref))))
}
pub(crate) fn native_base64_encoder_encode_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let variant = base64_variant_arg(args, heap)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    let string_ref = heap.allocate_string(encode_base64(&input, variant));
    Ok(Some(Slot::Reference(Some(string_ref))))
}
pub(crate) fn native_base64_encoder_encode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let variant = base64_variant_arg(args, heap)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    let encoded = encode_base64(&input, variant);
    let array_ref = alloc_byte_array(heap, &encoded.into_bytes());
    Ok(Some(Slot::Reference(Some(array_ref))))
}
pub(crate) fn native_base64_decoder_decode_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let variant = base64_variant_arg(args, heap)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = string_value_from_ref(heap, input_ref)?;
    let decoded = decode_base64(input.as_bytes(), variant)?;
    let array_ref = alloc_byte_array(heap, &decoded);
    Ok(Some(Slot::Reference(Some(array_ref))))
}
pub(crate) fn native_base64_decoder_decode_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let variant = base64_variant_arg(args, heap)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    let decoded = decode_base64(&input, variant)?;
    let array_ref = alloc_byte_array(heap, &decoded);
    Ok(Some(Slot::Reference(Some(array_ref))))
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
    let mut this_ref = extract_ref_arg(args, 0)?;
    let mut key = chm_non_null_arg(args, 1)?;
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

    // Pin `this_ref`/`key` across the mapping function: the lock is dropped for
    // the callback, so both must survive and be forwarded in place on every
    // collection it triggers before we re-lock and re-read the map below.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut this_ref);
    scope.pin_slot(&mut key);
    let computed = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, key],
    )?;
    drop(scope);
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
    let mut this_ref = extract_ref_arg(args, 0)?;
    let mut key = chm_non_null_arg(args, 1)?;
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

    // Pin `this_ref`/`key` across the remapping function so both survive and are
    // forwarded in place on every collection it triggers before we re-lock.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut this_ref);
    scope.pin_slot(&mut key);
    let new_value = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, key, old_value],
    )?;
    drop(scope);
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
    let mut this_ref = extract_ref_arg(args, 0)?;
    let mut key = chm_non_null_arg(args, 1)?;
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

    // Pin `this_ref`/`key` across the remapping function so both survive and are
    // forwarded in place on every collection it triggers before we re-lock.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut this_ref);
    scope.pin_slot(&mut key);
    let new_value = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, key, old_value],
    )?;
    drop(scope);
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
    let mut this_ref = extract_ref_arg(args, 0)?;
    let mut key = chm_non_null_arg(args, 1)?;
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

    // Pin `this_ref`/`key` across the remapping function so the read-modify-write
    // stays linearizable: both are forwarded in place on every collection the
    // callback triggers, so the write-back below always lands on the right entry.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut this_ref);
    scope.pin_slot(&mut key);
    let merged = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, old_value, value],
    )?;
    drop(scope);
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
    let mut consumer_ref = extract_ref_arg(args, 1)?;
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let entries = {
        let _guard = concurrent_hashmap_guard(&lock);
        chm_entry_snapshot(heap, this_ref)?
    };
    // Flatten the (key, value) snapshot into a single interleaved
    // [k0,v0,k1,v1,...] buffer so the whole run of not-yet-visited slots can be
    // pinned as one GC handle. Empty map -> empty buffer -> pin_slots is a no-op
    // and the loop runs zero times.
    let mut entries_flat: Vec<Slot> = Vec::with_capacity(entries.len() * 2);
    for (key, value) in entries {
        entries_flat.push(key);
        entries_flat.push(value);
    }
    // Pin the consumer and the snapshot buffer across the callback loop: each
    // collection the consumer triggers keeps them alive (gather_roots) and
    // forwards them in place (patch_forwarded_slots), so the not-yet-visited
    // pairs stay valid across ANY number of collections.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut consumer_ref);
    scope.pin_slots(&mut entries_flat);
    // Index access (not `.iter()`) is deliberate: iterating by reference would
    // hold a live `&[Slot]` borrow of the pinned buffer across `ops.invoke`,
    // which the collector writes through the pin handle.
    let pair_count = entries_flat.len() / 2;
    for i in 0..pair_count {
        let key = entries_flat[i * 2];
        let value = entries_flat[i * 2 + 1];
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;Ljava/lang/Object;)V",
            vec![Slot::Reference(Some(consumer_ref)), key, value],
        )?;
    }
    drop(scope);
    Ok(None)
}

// ---------------------------------------------------------------------------
// java/util/concurrent/CopyOnWriteArrayList
//
// Synthetic thread-unsafe stand-in for `CopyOnWriteArrayList`. Mirrors the
// `java/util/ArrayList` storage convention exactly (fields[0] = size as
// `Slot::Int`, fields[1..] = elements), which lets the shared list iterator
// (`duke/util/ArrayListIterator`, via `native_arraylist_iterator`) operate on
// a CoWAL instance without modification. Single-threaded execution means the
// copy-on-write snapshot semantics collapse to plain in-place mutation.
// ---------------------------------------------------------------------------

/// Native: `CopyOnWriteArrayList.<init>()V` — initializes with size = 0.
pub(crate) fn native_cowal_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `CopyOnWriteArrayList.add(Object)Z` — appends element, returns true.
pub(crate) fn native_cowal_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(Error::NullPointerException),
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1)))
}

/// Native: `CopyOnWriteArrayList.addIfAbsent(Object)Z` — appends the element and
/// returns true only if no equal element is already present; otherwise leaves
/// the list unchanged and returns false. Uses the same equality convention as
/// `ArrayList.contains`.
pub(crate) fn native_cowal_add_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    for slot in heap.get(this_ref)?.fields.iter().skip(1) {
        if slots_equal(slot, &element, heap) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(Error::NullPointerException),
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1)))
}

/// Native: `CopyOnWriteArrayList.get(I)Object` — returns element at index.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_cowal_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)? as usize;
    let obj = heap.get(this_ref)?;
    obj.fields.get(idx + 1).map_or_else(
        || {
            Err(Error::JavaException {
                class_name: "java/lang/IndexOutOfBoundsException".to_string(),
            })
        },
        |slot| Ok(Some(*slot)),
    )
}

/// Native: `CopyOnWriteArrayList.size()I`
pub(crate) fn native_cowal_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(sz)) => Ok(Some(Slot::Int(*sz))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `CopyOnWriteArrayList.contains(Object)Z`
pub(crate) fn native_cowal_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let mut found = false;
    for slot in heap.get(this_ref)?.fields.iter().skip(1) {
        if slots_equal(slot, &target, heap) {
            found = true;
            break;
        }
    }
    Ok(Some(Slot::Int(i32::from(found))))
}

/// Native: `CopyOnWriteArrayList.isEmpty()Z` — returns true if size is 0.
pub(crate) fn native_cowal_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let is_empty = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz == 0,
        _ => true,
    };
    Ok(Some(Slot::Int(i32::from(is_empty))))
}
// ---------------------------------------------------------------------------
// java/util/concurrent/LinkedBlockingQueue
//
// Synthetic FIFO queue backed by the ArrayList storage convention
// (fields[0] = size as `Slot::Int`, fields[1..] = elements, front at index 1).
// Single-threaded execution collapses the blocking semantics: `put`/`offer`
// always succeed (unbounded), and `take`/`poll` return the head or null when
// empty. The optional bounded-capacity constructor argument is accepted and
// ignored (treated as effectively unbounded).
// ---------------------------------------------------------------------------

/// Native: `LinkedBlockingQueue.<init>()V` — initializes an empty queue.
pub(crate) fn native_lbq_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `LinkedBlockingQueue.<init>(I)V` — capacity ignored; empty queue.
pub(crate) fn native_lbq_init_capacity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Shared tail-append used by add/offer/put: appends `element`, bumps size,
/// returns the new size for callers that need it.
fn lbq_enqueue(heap: &mut duke_gc::Heap, this_ref: u64, element: Slot) -> Result<()> {
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(Error::NullPointerException),
    }
    obj.fields.push(element);
    Ok(())
}

/// Shared head-removal used by poll/take/remove: removes and returns the front
/// element, or `None` when the queue is empty.
fn lbq_dequeue(heap: &mut duke_gc::Heap, this_ref: u64) -> Result<Option<Slot>> {
    let obj = heap.get_mut(this_ref)?;
    let size = match obj.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    if size <= 0 || obj.fields.len() < 2 {
        return Ok(None);
    }
    let front = obj.fields.remove(1);
    obj.fields[0] = Slot::Int(size - 1);
    Ok(Some(front))
}

/// Native: `LinkedBlockingQueue.add(Object)Z` — appends, returns true.
pub(crate) fn native_lbq_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    lbq_enqueue(heap, this_ref, element)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `LinkedBlockingQueue.offer(Object)Z` — appends, returns true (unbounded).
pub(crate) fn native_lbq_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    lbq_enqueue(heap, this_ref, element)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `LinkedBlockingQueue.put(Object)V` — appends (never blocks; unbounded).
pub(crate) fn native_lbq_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    lbq_enqueue(heap, this_ref, element)?;
    Ok(None)
}

/// Native: `LinkedBlockingQueue.poll()Object` — removes/returns head, or null.
pub(crate) fn native_lbq_poll(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(
        lbq_dequeue(heap, this_ref)?.unwrap_or(Slot::Reference(None)),
    ))
}

/// Native: `LinkedBlockingQueue.take()Object` — removes/returns head. In a
/// single-threaded VM there is no producer to wait for, so an empty queue
/// yields null rather than blocking forever.
pub(crate) fn native_lbq_take(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(
        lbq_dequeue(heap, this_ref)?.unwrap_or(Slot::Reference(None)),
    ))
}

/// Native: `LinkedBlockingQueue.peek()Object` — returns head without removing, or null.
pub(crate) fn native_lbq_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let front = match obj.fields.first() {
        Some(Slot::Int(sz)) if *sz > 0 => obj.fields.get(1).copied(),
        _ => None,
    };
    Ok(Some(front.unwrap_or(Slot::Reference(None))))
}

/// Native: `LinkedBlockingQueue.size()I`
pub(crate) fn native_lbq_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(sz)) => Ok(Some(Slot::Int(*sz))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `LinkedBlockingQueue.isEmpty()Z`
pub(crate) fn native_lbq_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let is_empty = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz == 0,
        _ => true,
    };
    Ok(Some(Slot::Int(i32::from(is_empty))))
}

/// Native: `LinkedBlockingQueue.remainingCapacity()I` — reports `Integer.MAX_VALUE`
/// (this synthetic queue is effectively unbounded).
pub(crate) fn native_lbq_remaining_capacity(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::MAX)))
}

/// Shared drain: removes up to `max` elements from the queue and appends each to
/// the target collection via its `add(Object)Z`; returns the number transferred.
fn lbq_drain_into(
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    mut this_ref: u64,
    mut target_ref: u64,
    max: i32,
) -> Result<Option<Slot>> {
    let target_class = heap.get(target_ref)?.class_name.clone();
    // Pin the queue and the target across the whole drain loop. Each `add`
    // callback may trigger GC, and both refs are re-dereferenced afterwards on
    // the next iteration (lbq_dequeue reads `this_ref`, and `target_ref` is
    // re-passed into the callback). The pin forwards them in place on every
    // collection, so both stay valid across ANY number of GCs.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut this_ref);
    scope.pin_ref(&mut target_ref);
    let mut count: i32 = 0;
    while count < max {
        let Some(elem) = lbq_dequeue(heap, this_ref)? else {
            break;
        };
        ops.invoke(
            heap,
            output,
            &target_class,
            "add",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(target_ref)), elem],
        )?;
        count += 1;
    }
    drop(scope);
    Ok(Some(Slot::Int(count)))
}

/// Native: `LinkedBlockingQueue.drainTo(Collection)I` — drains every available
/// element into the target collection, returning the count transferred.
pub(crate) fn native_lbq_drain_to(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target_ref = extract_ref_arg(args, 1)?;
    lbq_drain_into(heap, output, ops, this_ref, target_ref, i32::MAX)
}

/// Native: `LinkedBlockingQueue.drainTo(Collection, int)I` — drains up to
/// `maxElements` into the target collection, returning the count transferred.
pub(crate) fn native_lbq_drain_to_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target_ref = extract_ref_arg(args, 1)?;
    let max = extract_int_arg(args, 2)?;
    lbq_drain_into(heap, output, ops, this_ref, target_ref, max)
}

/// Native: `LinkedBlockingQueue.clear()V` — removes all elements.
pub(crate) fn native_lbq_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields.truncate(1);
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}
