fn semaphore_state(
    heap: &duke_gc::Heap,
    this_ref: u64,
) -> Result<std::sync::Arc<std::sync::Mutex<duke_gc::SemaphoreState>>> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Semaphore(state)) => {
            Ok(std::sync::Arc::clone(state))
        }
        _ => Err(atomic_payload_error(this_ref)),
    }
}
pub(crate) fn native_semaphore_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let permits = extract_int_arg(args, 1)?;
    heap.get_mut(this_ref)?.atomic_payload = Some(
        duke_gc::AtomicPayload::semaphore(permits, false),
    );
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
    heap.get_mut(this_ref)?.atomic_payload = Some(
        duke_gc::AtomicPayload::semaphore(permits, fair),
    );
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
    if let Some(idx) = waiters.iter().position(|waiter| waiter.thread_id == thread_id) {
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
    state.waiters.front().is_none_or(|waiter| waiter.thread_id == thread_id)
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
    let mut guard = semaphore.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
        if guard.waiters[waiter_idx].deadline.is_some_and(|deadline| now >= deadline) {
            guard.waiters.remove(waiter_idx);
            return Ok(Some(Slot::Int(0)));
        }
    } else {
        guard
            .waiters
            .push_back(duke_gc::SemaphoreWaiter {
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
        &mut semaphore.lock().unwrap_or_else(std::sync::PoisonError::into_inner),
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
fn semaphore_release_common(
    args: &[Slot],
    heap: &duke_gc::Heap,
    permits: i32,
) -> Result<()> {
    semaphore_validate_permits(permits)?;
    let this_ref = extract_ref_arg(args, 0)?;
    let semaphore = semaphore_state(heap, this_ref)?;
    let mut guard = semaphore.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
    let mut guard = semaphore.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
    let fair = semaphore.lock().unwrap_or_else(std::sync::PoisonError::into_inner).fair;
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
    let string_ref = heap
        .allocate_string(format!("java.util.concurrent.Semaphore[Permits = {permits}]"));
    Ok(Some(Slot::Reference(Some(string_ref))))
}
