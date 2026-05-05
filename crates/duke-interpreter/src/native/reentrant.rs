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
    heap.get_mut(this_ref)?.atomic_payload = Some(
        duke_gc::AtomicPayload::reentrant_lock(false),
    );
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
    heap.get_mut(this_ref)?.atomic_payload = Some(
        duke_gc::AtomicPayload::reentrant_lock(fair),
    );
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
    with_reentrant_lock_state(
        heap,
        this_ref,
        |state| {
            let mut guard = state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if !reentrant_lock_try_acquire(&mut guard, thread_id) {
                request_native_retry(control);
            }
            Ok(None)
        },
    )
}
pub(crate) fn native_reentrant_lock_try_lock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let thread_id = current_host_thread_id();
    with_reentrant_lock_state(
        heap,
        this_ref,
        |state| {
            let mut guard = state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            Ok(
                Some(
                    Slot::Int(
                        i32::from(reentrant_lock_try_acquire(&mut guard, thread_id)),
                    ),
                ),
            )
        },
    )
}
pub(crate) fn native_reentrant_lock_unlock(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let thread_id = current_host_thread_id();
    with_reentrant_lock_state(
        heap,
        this_ref,
        |state| {
            let mut guard = state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            reentrant_lock_release(&mut guard, thread_id)?;
            Ok(None)
        },
    )
}
pub(crate) fn native_reentrant_lock_new_condition(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock_state = with_reentrant_lock_state(
        heap,
        this_ref,
        |state| Ok(std::sync::Arc::clone(state)),
    )?;
    let condition_ref = heap
        .allocate("duke/util/concurrent/ConditionObject".to_string(), 0);
    heap.get_mut(condition_ref)?.atomic_payload = Some(
        duke_gc::AtomicPayload::condition(lock_state),
    );
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
    with_reentrant_lock_state(
        heap,
        this_ref,
        |state| {
            let guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let hold_count = if guard.owner == Some(thread_id) {
                guard.hold_count
            } else {
                0
            };
            Ok(Some(Slot::Int(hold_count)))
        },
    )
}
pub(crate) fn native_reentrant_lock_is_held_by_current_thread(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let thread_id = current_host_thread_id();
    with_reentrant_lock_state(
        heap,
        this_ref,
        |state| {
            let guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            Ok(Some(Slot::Int(i32::from(reentrant_lock_is_held_by(&guard, thread_id)))))
        },
    )
}
pub(crate) fn native_reentrant_lock_is_locked(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    with_reentrant_lock_state(
        heap,
        this_ref,
        |state| {
            let guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            Ok(Some(Slot::Int(i32::from(guard.owner.is_some() && guard.hold_count > 0))))
        },
    )
}
pub(crate) fn native_reentrant_lock_is_fair(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    with_reentrant_lock_state(
        heap,
        this_ref,
        |state| {
            let guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            Ok(Some(Slot::Int(i32::from(guard.fair))))
        },
    )
}
pub(crate) fn native_reentrant_read_write_lock_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = std::sync::Arc::new(
        std::sync::Mutex::new(duke_gc::ReadWriteLockState::default()),
    );
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
pub(crate) fn native_reentrant_read_write_lock_is_write_locked(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let state = read_write_lock_state(heap, this_ref)?;
    let guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
    let guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
    let guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let count = if guard.writer == Some(thread_id) { guard.write_hold_count } else { 0 };
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
    let guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(Some(Slot::Int(guard.readers.get(&thread_id).copied().unwrap_or_default())))
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
