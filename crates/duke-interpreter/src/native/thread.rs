pub(crate) fn native_thread_current_thread(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = heap.allocate("java/lang/Thread".to_string(), 4);
    let thread = heap.get_mut(thread_ref)?;
    thread.fields[THREAD_TARGET_SLOT] = Slot::Reference(None);
    thread.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    thread.fields[THREAD_INTERRUPTED_SLOT] = Slot::Int(i32::from(current_host_thread_is_interrupted()));
    thread.fields[THREAD_HOST_KEY_SLOT] = Slot::Int(java_host_key_for_current_host().unwrap_or(-1));
    Ok(Some(Slot::Reference(Some(thread_ref))))
}

pub(crate) fn native_thread_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let this = heap.get_mut(this_ref)?;
    this.fields[THREAD_TARGET_SLOT] = Slot::Reference(None);
    this.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    this.fields[THREAD_INTERRUPTED_SLOT] = Slot::Int(0);
    this.fields[THREAD_HOST_KEY_SLOT] = Slot::Int(-1);
    Ok(None)
}

pub(crate) fn native_thread_init_runnable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let this = heap.get_mut(this_ref)?;
    this.fields[THREAD_TARGET_SLOT] = target;
    this.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    this.fields[THREAD_INTERRUPTED_SLOT] = Slot::Int(0);
    this.fields[THREAD_HOST_KEY_SLOT] = Slot::Int(-1);
    Ok(None)
}

pub(crate) fn native_thread_start(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    control.request(NativeThreadAction::Start { thread_ref });
    Ok(None)
}

pub(crate) fn native_thread_join(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let thread = heap.get(thread_ref)?;
    let Some(Slot::Int(thread_id)) = thread.fields.get(THREAD_ID_SLOT) else {
        return Ok(None);
    };
    if *thread_id >= 0 {
        control.request(NativeThreadAction::Join {
            thread_id: *thread_id,
        });
    }
    Ok(None)
}

pub(crate) fn native_thread_sleep(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let millis = match args.first() {
        Some(Slot::Long(value)) => *value,
        _ => {
            return Err(Error::TypeMismatch {
                expected: "long",
                got: "other",
            });
        }
    };
    let millis = u64::try_from(millis.max(0)).unwrap_or(0);
    control.request(NativeThreadAction::Sleep(std::time::Duration::from_millis(
        millis,
    )));
    Ok(None)
}

const EXECUTOR_SHUTDOWN_FIELD: usize = 0;
const EXECUTOR_AWAIT_DEADLINE_FIELD: usize = 1;

const FUTURE_STATE_FIELD: usize = 0;
const FUTURE_RESULT_FIELD: usize = 1;
const FUTURE_EXCEPTION_FIELD: usize = 2;
const FUTURE_WAIT_DEADLINE_FIELD: usize = 3;
const FUTURE_TASK_FIELD: usize = 4;

const FUTURE_PENDING: i32 = 0;
const FUTURE_RUNNING: i32 = 1;
const FUTURE_DONE: i32 = 2;
const FUTURE_CANCELLED: i32 = 3;
const FUTURE_FAILED: i32 = 4;

const TIMEUNIT_NANOS_FIELD: usize = 2;

fn executor_shared(
    heap: &duke_gc::Heap,
    executor_ref: u64,
) -> Result<std::sync::Arc<duke_gc::ExecutorShared>> {
    match heap.get(executor_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Executor(state)) => Ok(std::sync::Arc::clone(state)),
        _ => Err(atomic_payload_error(executor_ref)),
    }
}

fn allocate_executor(heap: &mut duke_gc::Heap, max_workers: usize) -> Result<Slot> {
    let executor_ref = heap.allocate("duke/util/concurrent/DukeExecutorService".to_string(), 2);
    {
        let executor = heap.get_mut(executor_ref)?;
        executor.fields[EXECUTOR_SHUTDOWN_FIELD] = Slot::Int(0);
        executor.fields[EXECUTOR_AWAIT_DEADLINE_FIELD] = Slot::Long(0);
        executor.atomic_payload = Some(duke_gc::AtomicPayload::executor(max_workers));
    }
    Ok(Slot::Reference(Some(executor_ref)))
}

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

fn init_future(
    heap: &mut duke_gc::Heap,
    result: Slot,
    task: Slot,
) -> Result<u64> {
    let future_ref = heap.allocate("duke/util/concurrent/DukeFuture".to_string(), 5);
    {
        let future = heap.get_mut(future_ref)?;
        future.fields[FUTURE_STATE_FIELD] = Slot::Int(FUTURE_PENDING);
        future.fields[FUTURE_RESULT_FIELD] = result;
        future.fields[FUTURE_EXCEPTION_FIELD] = Slot::Reference(None);
        future.fields[FUTURE_WAIT_DEADLINE_FIELD] = Slot::Long(0);
        future.fields[FUTURE_TASK_FIELD] = task;
    }
    heap.remember_reference_write(future_ref, result);
    heap.remember_reference_write(future_ref, task);
    Ok(future_ref)
}

fn executor_submit_common(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    control: &mut NativeControl,
    kind: duke_gc::ExecutorTaskKind,
    preset_result: Slot,
) -> Result<u64> {
    let executor_ref = extract_ref_arg(args, 0)?;
    let task_ref = extract_ref_arg(args, 1)?;
    let executor = executor_shared(heap, executor_ref)?;
    let is_shutdown = {
        let guard = executor
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.shutdown
    };
    if is_shutdown {
        return Err(Error::JavaException {
            class_name: "java/util/concurrent/RejectedExecutionException".to_string(),
        });
    }

    let future_ref = init_future(
        heap,
        preset_result,
        Slot::Reference(Some(task_ref)),
    )?;
    control.request(NativeThreadAction::ExecutorSubmit {
        executor_ref,
        future_ref,
        task_ref,
        kind,
    });
    Ok(future_ref)
}

pub(crate) fn native_executor_submit_runnable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let future_ref = executor_submit_common(
        args,
        heap,
        control,
        duke_gc::ExecutorTaskKind::Runnable,
        Slot::Reference(None),
    )?;
    Ok(Some(Slot::Reference(Some(future_ref))))
}

pub(crate) fn native_executor_submit_runnable_result(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let result = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let future_ref = executor_submit_common(
        args,
        heap,
        control,
        duke_gc::ExecutorTaskKind::Runnable,
        result,
    )?;
    Ok(Some(Slot::Reference(Some(future_ref))))
}

pub(crate) fn native_executor_submit_callable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let future_ref = executor_submit_common(
        args,
        heap,
        control,
        duke_gc::ExecutorTaskKind::Callable,
        Slot::Reference(None),
    )?;
    Ok(Some(Slot::Reference(Some(future_ref))))
}

pub(crate) fn native_executor_execute(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let _future_ref = executor_submit_common(
        args,
        heap,
        control,
        duke_gc::ExecutorTaskKind::Runnable,
        Slot::Reference(None),
    )?;
    Ok(None)
}

pub(crate) fn native_executor_shutdown(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let executor_ref = extract_ref_arg(args, 0)?;
    let executor = executor_shared(heap, executor_ref)?;
    {
        let mut guard = executor
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.shutdown = true;
        guard.refresh_terminated();
    }
    executor.available.notify_all();
    heap.write_field(executor_ref, EXECUTOR_SHUTDOWN_FIELD, Slot::Int(1))?;
    Ok(None)
}

fn executor_is_shutdown(heap: &duke_gc::Heap, executor_ref: u64) -> Result<bool> {
    let executor = executor_shared(heap, executor_ref)?;
    let guard = executor
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(guard.shutdown)
}

pub(crate) fn native_executor_is_shutdown(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let executor_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(executor_is_shutdown(
        heap,
        executor_ref,
    )?))))
}

fn executor_is_terminated(heap: &duke_gc::Heap, executor_ref: u64) -> Result<bool> {
    let executor = executor_shared(heap, executor_ref)?;
    let mut guard = executor
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.refresh_terminated();
    Ok(guard.terminated)
}

pub(crate) fn native_executor_is_terminated(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let executor_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(executor_is_terminated(
        heap,
        executor_ref,
    )?))))
}

fn timeunit_nanos_per_unit(heap: &duke_gc::Heap, unit_ref: u64) -> Result<i64> {
    match heap.get(unit_ref)?.fields.get(TIMEUNIT_NANOS_FIELD) {
        Some(Slot::Long(nanos)) => Ok(*nanos),
        _ => Err(Error::TypeMismatch {
            expected: "TimeUnit",
            got: "other",
        }),
    }
}

fn saturating_mul_i64(lhs: i64, rhs: i64) -> i64 {
    let value = i128::from(lhs).saturating_mul(i128::from(rhs));
    let clamped = value.clamp(i128::from(i64::MIN), i128::from(i64::MAX));
    match i64::try_from(clamped) {
        Ok(value) => value,
        Err(_) if clamped < 0 => i64::MIN,
        Err(_) => i64::MAX,
    }
}

fn timeout_nanos(timeout: i64, unit_ref: u64, heap: &duke_gc::Heap) -> Result<i64> {
    Ok(saturating_mul_i64(
        timeout.max(0),
        timeunit_nanos_per_unit(heap, unit_ref)?,
    ))
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

pub(crate) fn native_executor_await_termination(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let executor_ref = extract_ref_arg(args, 0)?;
    if executor_is_terminated(heap, executor_ref)? {
        heap.write_field(executor_ref, EXECUTOR_AWAIT_DEADLINE_FIELD, Slot::Long(0))?;
        return Ok(Some(Slot::Int(1)));
    }

    let timeout = extract_long_arg(args, 1)?;
    let unit_ref = extract_ref_arg(args, 2)?;
    let nanos = timeout_nanos(timeout, unit_ref, heap)?;
    if nanos <= 0 {
        return Ok(Some(Slot::Int(0)));
    }

    let now = monotonic_nano_time_now();
    let deadline = match extract_field_arg(heap, executor_ref, EXECUTOR_AWAIT_DEADLINE_FIELD)? {
        Slot::Long(value) if value > 0 => value,
        _ => {
            let deadline = now.saturating_add(nanos);
            heap.write_field(
                executor_ref,
                EXECUTOR_AWAIT_DEADLINE_FIELD,
                Slot::Long(deadline),
            )?;
            deadline
        }
    };
    if now >= deadline {
        heap.write_field(executor_ref, EXECUTOR_AWAIT_DEADLINE_FIELD, Slot::Long(0))?;
        return Ok(Some(Slot::Int(0)));
    }
    request_native_retry(control);
    Ok(None)
}

fn future_state(heap: &duke_gc::Heap, future_ref: u64) -> Result<i32> {
    match heap.get(future_ref)?.fields.get(FUTURE_STATE_FIELD) {
        Some(Slot::Int(state)) => Ok(*state),
        _ => Ok(FUTURE_PENDING),
    }
}

pub(crate) fn native_future_cancel(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let future_ref = extract_ref_arg(args, 0)?;
    if future_state(heap, future_ref)? == FUTURE_PENDING {
        heap.write_field(future_ref, FUTURE_STATE_FIELD, Slot::Int(FUTURE_CANCELLED))?;
        return Ok(Some(Slot::Int(1)));
    }
    Ok(Some(Slot::Int(0)))
}

pub(crate) fn native_future_is_cancelled(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let future_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(
        future_state(heap, future_ref)? == FUTURE_CANCELLED,
    ))))
}

pub(crate) fn native_future_is_done(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let future_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(matches!(
        future_state(heap, future_ref)?,
        FUTURE_DONE | FUTURE_CANCELLED | FUTURE_FAILED
    )))))
}

fn future_get_common(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    control: &mut NativeControl,
    timeout: Option<i64>,
) -> Result<Option<Slot>> {
    let future_ref = extract_ref_arg(args, 0)?;
    match future_state(heap, future_ref)? {
        FUTURE_DONE => {
            heap.write_field(future_ref, FUTURE_WAIT_DEADLINE_FIELD, Slot::Long(0))?;
            Ok(Some(extract_field_arg(heap, future_ref, FUTURE_RESULT_FIELD)?))
        }
        FUTURE_CANCELLED => Err(Error::JavaException {
            class_name: "java/util/concurrent/CancellationException".to_string(),
        }),
        FUTURE_FAILED => {
            let cause = extract_field_arg(heap, future_ref, FUTURE_EXCEPTION_FIELD)?;
            push_pending_java_exception_cause("java/util/concurrent/ExecutionException", cause);
            Err(Error::JavaException {
                class_name: "java/util/concurrent/ExecutionException".to_string(),
            })
        }
        FUTURE_PENDING | FUTURE_RUNNING => {
            if let Some(nanos) = timeout {
                if nanos <= 0 {
                    return Err(Error::JavaException {
                        class_name: "java/util/concurrent/TimeoutException".to_string(),
                    });
                }
                let now = monotonic_nano_time_now();
                let deadline = match extract_field_arg(heap, future_ref, FUTURE_WAIT_DEADLINE_FIELD)?
                {
                    Slot::Long(value) if value > 0 => value,
                    _ => {
                        let deadline = now.saturating_add(nanos);
                        heap.write_field(
                            future_ref,
                            FUTURE_WAIT_DEADLINE_FIELD,
                            Slot::Long(deadline),
                        )?;
                        deadline
                    }
                };
                if now >= deadline {
                    heap.write_field(future_ref, FUTURE_WAIT_DEADLINE_FIELD, Slot::Long(0))?;
                    return Err(Error::JavaException {
                        class_name: "java/util/concurrent/TimeoutException".to_string(),
                    });
                }
            }
            request_native_retry(control);
            Ok(None)
        }
        _ => Err(Error::InvalidRef {
            address: future_ref,
        }),
    }
}

pub(crate) fn native_future_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    future_get_common(args, heap, control, None)
}

pub(crate) fn native_future_get_timeout(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let timeout = extract_long_arg(args, 1)?;
    let unit_ref = extract_ref_arg(args, 2)?;
    let nanos = timeout_nanos(timeout, unit_ref, heap)?;
    future_get_common(args, heap, control, Some(nanos))
}

// java.util.concurrent.atomic natives.
pub(crate) fn native_thread_interrupt(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let host_key = match heap.get(thread_ref)?.fields.get(THREAD_HOST_KEY_SLOT) {
        Some(Slot::Int(host_key)) => *host_key,
        _ => -1,
    };
    heap.write_field(thread_ref, THREAD_INTERRUPTED_SLOT, Slot::Int(1))?;
    if let Some(host_thread_id) = host_thread_for_java_thread(host_key) {
        interrupt_host_thread(host_thread_id);
    }
    Ok(None)
}

pub(crate) fn native_thread_is_interrupted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let field_interrupted = matches!(
        heap.get(thread_ref)?.fields.get(THREAD_INTERRUPTED_SLOT),
        Some(Slot::Int(value)) if *value != 0
    );
    let host_key = match heap.get(thread_ref)?.fields.get(THREAD_HOST_KEY_SLOT) {
        Some(Slot::Int(host_key)) => *host_key,
        _ => -1,
    };
    let host_interrupted = host_thread_for_java_thread(host_key)
        .is_some_and(|host_thread_id| {
            interrupted_host_threads()
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .contains(&host_thread_id)
        });
    Ok(Some(Slot::Int(i32::from(
        field_interrupted || host_interrupted,
    ))))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_thread_interrupted(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Int(i32::from(
        take_current_host_thread_interrupted(),
    ))))
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
    let mut guard = latch
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

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
        if guard.waiters[waiter_idx]
            .deadline
            .is_some_and(|deadline| now >= deadline)
        {
            guard.waiters.remove(waiter_idx);
            return Ok(Some(Slot::Int(0)));
        }
    } else {
        guard.waiters.push(duke_gc::CountDownLatchWaiter {
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
