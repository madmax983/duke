fn condition_state(
    heap: &duke_gc::Heap,
    this_ref: u64,
) -> Result<std::sync::Arc<std::sync::Mutex<duke_gc::ConditionState>>> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Condition(state)) => {
            Ok(std::sync::Arc::clone(state))
        }
        _ => Err(atomic_payload_error(this_ref)),
    }
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
            let result = timeout_nanos
                .map(|_| {
                    let remaining = if timed_out {
                        0
                    } else {
                        deadline
                            .map_or(
                                0,
                                |deadline| {
                                    i64::try_from(
                                            deadline.saturating_duration_since(now).as_nanos(),
                                        )
                                        .unwrap_or(i64::MAX)
                                },
                            )
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
    let deadline = timeout_nanos
        .and_then(|nanos| {
            u64::try_from(nanos.max(0))
                .ok()
                .and_then(|nanos| {
                    now.checked_add(std::time::Duration::from_nanos(nanos))
                })
        });
    let immediate_timeout = timeout_nanos.is_some_and(|nanos| nanos <= 0);
    condition_guard
        .waiters
        .push(duke_gc::ConditionWaiter {
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
