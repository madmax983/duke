fn with_atomic_i32<T>(
    heap: &duke_gc::Heap,
    this_ref: u64,
    f: impl FnOnce(&AtomicI32) -> T,
) -> Result<T> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Int(cell)) => Ok(f(cell)),
        _ => Err(atomic_payload_error(this_ref)),
    }
}
fn with_atomic_i64<T>(
    heap: &duke_gc::Heap,
    this_ref: u64,
    f: impl FnOnce(&std::sync::atomic::AtomicI64) -> T,
) -> Result<T> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Long(cell)) => Ok(f(cell)),
        _ => Err(atomic_payload_error(this_ref)),
    }
}
fn with_atomic_bool<T>(
    heap: &duke_gc::Heap,
    this_ref: u64,
    f: impl FnOnce(&std::sync::atomic::AtomicBool) -> T,
) -> Result<T> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Bool(cell)) => Ok(f(cell)),
        _ => Err(atomic_payload_error(this_ref)),
    }
}
fn with_atomic_reference<T>(
    heap: &duke_gc::Heap,
    this_ref: u64,
    f: impl FnOnce(&std::sync::Mutex<Slot>) -> Result<T>,
) -> Result<T> {
    match heap.get(this_ref)?.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::Reference(cell)) => f(cell),
        _ => Err(atomic_payload_error(this_ref)),
    }
}
fn with_reentrant_lock_state<T>(
    heap: &mut duke_gc::Heap,
    this_ref: u64,
    f: impl FnOnce(
        &std::sync::Arc<std::sync::Mutex<duke_gc::ReentrantLockState>>,
    ) -> Result<T>,
) -> Result<T> {
    let obj = heap.get_mut(this_ref)?;
    if obj.atomic_payload.is_none() {
        obj.atomic_payload = Some(duke_gc::AtomicPayload::reentrant_lock(false));
    }
    match obj.atomic_payload.as_ref() {
        Some(duke_gc::AtomicPayload::ReentrantLock(state)) => f(state),
        _ => Err(atomic_payload_error(this_ref)),
    }
}
