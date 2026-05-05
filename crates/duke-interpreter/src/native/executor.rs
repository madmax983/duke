fn executor_shared(
    heap: &duke_gc::Heap,
    executor_ref: u64,
) -> Result<std::sync::Arc<duke_gc::ExecutorShared>> {
    match heap.get(executor_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Executor(state)) => Ok(std::sync::Arc::clone(state)),
        _ => Err(atomic_payload_error(executor_ref)),
    }
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
    let future_ref = init_future(heap, preset_result, Slot::Reference(Some(task_ref)))?;
    control
        .request(NativeThreadAction::ExecutorSubmit {
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
    let guard = executor.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(guard.shutdown)
}
pub(crate) fn native_executor_is_shutdown(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let executor_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(executor_is_shutdown(heap, executor_ref)?))))
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
    Ok(Some(Slot::Int(i32::from(executor_is_terminated(heap, executor_ref)?))))
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
    let deadline = match extract_field_arg(
        heap,
        executor_ref,
        EXECUTOR_AWAIT_DEADLINE_FIELD,
    )? {
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
const fn executor_task_signature(
    kind: duke_gc::ExecutorTaskKind,
) -> (&'static str, &'static str) {
    match kind {
        duke_gc::ExecutorTaskKind::Runnable => ("run", "()V"),
        duke_gc::ExecutorTaskKind::Callable => ("call", "()Ljava/lang/Object;"),
    }
}
