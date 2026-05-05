fn concurrent_hashmap_lock(
    heap: &mut duke_gc::Heap,
    this_ref: u64,
) -> Result<std::sync::Arc<std::sync::Mutex<()>>> {
    let obj = heap.get_mut(this_ref)?;
    if let Some(payload) = &obj.atomic_payload {
        return match payload {
            duke_gc::AtomicPayload::ConcurrentMapLock(lock) => {
                Ok(std::sync::Arc::clone(lock))
            }
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
            | duke_gc::AtomicPayload::CyclicBarrier(_) => {
                Err(Error::TypeMismatch {
                    expected: "concurrent map lock",
                    got: "other",
                })
            }
        };
    }
    let lock = std::sync::Arc::new(std::sync::Mutex::new(()));
    obj.atomic_payload = Some(
        duke_gc::AtomicPayload::ConcurrentMapLock(std::sync::Arc::clone(&lock)),
    );
    Ok(lock)
}
fn concurrent_hashmap_guard(
    lock: &std::sync::Arc<std::sync::Mutex<()>>,
) -> std::sync::MutexGuard<'_, ()> {
    lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
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
        &[Slot::Reference(Some(this_ref)), Slot::Reference(Some(source_ref))],
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
    native_hashmap_contains_key(
        &[Slot::Reference(Some(this_ref)), key],
        heap,
        out,
        control,
    )
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
    native_hashmap_contains_value(
        &[Slot::Reference(Some(this_ref)), value],
        heap,
        out,
        control,
    )
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
    let computed = ops
        .invoke(
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
    let new_value = ops
        .invoke(
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
    let new_value = ops
        .invoke(
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
    let merged = ops
        .invoke(
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
