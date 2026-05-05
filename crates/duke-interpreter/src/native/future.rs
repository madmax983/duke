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
    Ok(Some(Slot::Int(i32::from(future_state(heap, future_ref)? == FUTURE_CANCELLED))))
}
pub(crate) fn native_future_is_done(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let future_ref = extract_ref_arg(args, 0)?;
    Ok(
        Some(
            Slot::Int(
                i32::from(
                    matches!(
                        future_state(heap, future_ref) ?, FUTURE_DONE | FUTURE_CANCELLED
                        | FUTURE_FAILED
                    ),
                ),
            ),
        ),
    )
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
        FUTURE_CANCELLED => {
            Err(Error::JavaException {
                class_name: "java/util/concurrent/CancellationException".to_string(),
            })
        }
        FUTURE_FAILED => {
            let cause = extract_field_arg(heap, future_ref, FUTURE_EXCEPTION_FIELD)?;
            push_pending_java_exception_cause(
                "java/util/concurrent/ExecutionException",
                cause,
            );
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
                let deadline = match extract_field_arg(
                    heap,
                    future_ref,
                    FUTURE_WAIT_DEADLINE_FIELD,
                )? {
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
                    heap.write_field(
                        future_ref,
                        FUTURE_WAIT_DEADLINE_FIELD,
                        Slot::Long(0),
                    )?;
                    return Err(Error::JavaException {
                        class_name: "java/util/concurrent/TimeoutException".to_string(),
                    });
                }
            }
            request_native_retry(control);
            Ok(None)
        }
        _ => {
            Err(Error::InvalidRef {
                address: future_ref,
            })
        }
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
