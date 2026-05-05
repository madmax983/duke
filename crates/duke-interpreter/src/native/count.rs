fn count_down_latch_state(
    heap: &duke_gc::Heap,
    this_ref: u64,
) -> Result<std::sync::Arc<std::sync::Mutex<duke_gc::CountDownLatchState>>> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::CountDownLatch(state)) => {
            Ok(std::sync::Arc::clone(state))
        }
        _ => Err(atomic_payload_error(this_ref)),
    }
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
    heap.get_mut(this_ref)?.atomic_payload = Some(
        duke_gc::AtomicPayload::count_down_latch(count),
    );
    Ok(None)
}
fn count_down_latch_await_common(
    args: &[Slot],
    heap: &duke_gc::Heap,
    control: &mut NativeControl,
    timeout_nanos: Option<i64>,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let latch = count_down_latch_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let now = std::time::Instant::now();
    let interrupted = take_current_host_thread_interrupted();
    let mut guard = latch.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if interrupted {
        guard.waiters.retain(|waiter| waiter.thread_id != thread_id);
        return Err(interrupted_exception_error());
    }
    if guard.count <= 0 {
        guard.waiters.retain(|waiter| waiter.thread_id != thread_id);
        return Ok(timeout_nanos.map(|_| Slot::Int(1)));
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
            .push(duke_gc::CountDownLatchWaiter {
                thread_id,
                deadline: timeout_nanos.and_then(deadline_from_now),
            });
    }
    drop(guard);
    request_native_retry(control);
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
    let mut guard = latch.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
    let count = latch.lock().unwrap_or_else(std::sync::PoisonError::into_inner).count;
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
    let string_ref = heap
        .allocate_string(
            format!("java.util.concurrent.CountDownLatch[Count = {count}]"),
        );
    Ok(Some(Slot::Reference(Some(string_ref))))
}
