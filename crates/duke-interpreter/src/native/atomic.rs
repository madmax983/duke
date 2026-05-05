const fn atomic_payload_error(this_ref: u64) -> Error {
    Error::InvalidRef {
        address: this_ref,
    }
}
fn atomic_bool_arg(args: &[Slot], idx: usize) -> Result<bool> {
    extract_int_arg(args, idx).map(|value| value != 0)
}
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
    Ok(
        Some(
            Slot::Int(
                with_atomic_i32(heap, this_ref, |cell| { cell.load(Ordering::SeqCst) })?,
            ),
        ),
    )
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
    let previous = with_atomic_i32(
        heap,
        this_ref,
        |cell| cell.swap(value, Ordering::SeqCst),
    )?;
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
    let exchanged = with_atomic_i32(
        heap,
        this_ref,
        |cell| {
            cell.compare_exchange(expected, update, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        },
    )?;
    Ok(Some(Slot::Int(i32::from(exchanged))))
}
pub(crate) fn native_atomic_integer_get_and_increment(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i32(
        heap,
        this_ref,
        |cell| cell.fetch_add(1, Ordering::SeqCst),
    )?;
    Ok(Some(Slot::Int(previous)))
}
pub(crate) fn native_atomic_integer_get_and_decrement(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i32(
        heap,
        this_ref,
        |cell| cell.fetch_sub(1, Ordering::SeqCst),
    )?;
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
    let previous = with_atomic_i32(
        heap,
        this_ref,
        |cell| cell.fetch_add(delta, Ordering::SeqCst),
    )?;
    Ok(Some(Slot::Int(previous)))
}
pub(crate) fn native_atomic_integer_increment_and_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i32(
        heap,
        this_ref,
        |cell| cell.fetch_add(1, Ordering::SeqCst),
    )?;
    Ok(Some(Slot::Int(previous.wrapping_add(1))))
}
pub(crate) fn native_atomic_integer_decrement_and_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i32(
        heap,
        this_ref,
        |cell| cell.fetch_sub(1, Ordering::SeqCst),
    )?;
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
    let previous = with_atomic_i32(
        heap,
        this_ref,
        |cell| cell.fetch_add(delta, Ordering::SeqCst),
    )?;
    Ok(Some(Slot::Int(previous.wrapping_add(delta))))
}
pub(crate) fn native_atomic_integer_long_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(
        Some(
            Slot::Long(
                i64::from(
                    with_atomic_i32(heap, this_ref, |cell| cell.load(Ordering::SeqCst))?,
                ),
            ),
        ),
    )
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
    Ok(
        Some(
            Slot::Long(
                with_atomic_i64(heap, this_ref, |cell| { cell.load(Ordering::SeqCst) })?,
            ),
        ),
    )
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
    let previous = with_atomic_i64(
        heap,
        this_ref,
        |cell| cell.swap(value, Ordering::SeqCst),
    )?;
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
    let exchanged = with_atomic_i64(
        heap,
        this_ref,
        |cell| {
            cell.compare_exchange(expected, update, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        },
    )?;
    Ok(Some(Slot::Int(i32::from(exchanged))))
}
pub(crate) fn native_atomic_long_get_and_increment(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i64(
        heap,
        this_ref,
        |cell| cell.fetch_add(1, Ordering::SeqCst),
    )?;
    Ok(Some(Slot::Long(previous)))
}
pub(crate) fn native_atomic_long_get_and_decrement(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i64(
        heap,
        this_ref,
        |cell| cell.fetch_sub(1, Ordering::SeqCst),
    )?;
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
    let previous = with_atomic_i64(
        heap,
        this_ref,
        |cell| cell.fetch_add(delta, Ordering::SeqCst),
    )?;
    Ok(Some(Slot::Long(previous)))
}
pub(crate) fn native_atomic_long_increment_and_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i64(
        heap,
        this_ref,
        |cell| cell.fetch_add(1, Ordering::SeqCst),
    )?;
    Ok(Some(Slot::Long(previous.wrapping_add(1))))
}
pub(crate) fn native_atomic_long_decrement_and_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let previous = with_atomic_i64(
        heap,
        this_ref,
        |cell| cell.fetch_sub(1, Ordering::SeqCst),
    )?;
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
    let previous = with_atomic_i64(
        heap,
        this_ref,
        |cell| cell.fetch_add(delta, Ordering::SeqCst),
    )?;
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
    heap.get_mut(this_ref)?.atomic_payload = Some(
        duke_gc::AtomicPayload::reference(Slot::Reference(None)),
    );
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
    heap.get_mut(this_ref)?.atomic_payload = Some(
        duke_gc::AtomicPayload::reference(value),
    );
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
    with_atomic_reference(
        heap,
        this_ref,
        |cell| {
            *cell.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = value;
            Ok(())
        },
    )?;
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
    let previous = with_atomic_reference(
        heap,
        this_ref,
        |cell| {
            let mut guard = cell
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let previous = *guard;
            *guard = value;
            drop(guard);
            Ok(previous)
        },
    )?;
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
    let exchanged = with_atomic_reference(
        heap,
        this_ref,
        |cell| {
            let mut guard = cell
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let exchanged = *guard == expected;
            if exchanged {
                *guard = update;
            }
            drop(guard);
            Ok(exchanged)
        },
    )?;
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
        Slot::Reference(Some(reference)) => {
            heap_object_to_string(heap.get(reference)?, reference)
        }
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
    let exchanged = with_atomic_bool(
        heap,
        this_ref,
        |cell| {
            cell.compare_exchange(expected, update, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        },
    )?;
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
    let previous = with_atomic_bool(
        heap,
        this_ref,
        |cell| cell.swap(value, Ordering::SeqCst),
    )?;
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
