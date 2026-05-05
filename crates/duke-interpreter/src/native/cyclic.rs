fn cyclic_barrier_state(
    heap: &duke_gc::Heap,
    this_ref: u64,
) -> Result<std::sync::Arc<std::sync::Mutex<duke_gc::CyclicBarrierState>>> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::CyclicBarrier(state)) => {
            Ok(std::sync::Arc::clone(state))
        }
        _ => Err(atomic_payload_error(this_ref)),
    }
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
    barrier.lock().unwrap_or_else(std::sync::PoisonError::into_inner).break_generation();
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
            if guard.waiters[waiter_idx].deadline.is_some_and(|deadline| now >= deadline)
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
            guard
                .waiters
                .push(duke_gc::CyclicBarrierWaiter {
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
        let action_result = ops
            .invoke(
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
    barrier.lock().unwrap_or_else(std::sync::PoisonError::into_inner).trip_generation();
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
        let guard = barrier.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
    barrier.lock().unwrap_or_else(std::sync::PoisonError::into_inner).reset();
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
        let guard = barrier.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        (guard.parties, guard.count)
    };
    let string_ref = heap
        .allocate_string(
            format!(
                "java.util.concurrent.CyclicBarrier[Parties = {parties}, Count = {count}]"
            ),
        );
    Ok(Some(Slot::Reference(Some(string_ref))))
}
