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
    thread.fields[THREAD_INTERRUPTED_SLOT] = Slot::Int(
        i32::from(current_host_thread_is_interrupted()),
    );
    thread.fields[THREAD_HOST_KEY_SLOT] = Slot::Int(
        java_host_key_for_current_host().unwrap_or(-1),
    );
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
    control
        .request(NativeThreadAction::Start {
            thread_ref,
        });
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
        control
            .request(NativeThreadAction::Join {
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
    control.request(NativeThreadAction::Sleep(std::time::Duration::from_millis(millis)));
    Ok(None)
}
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
        heap.get(thread_ref) ?.fields.get(THREAD_INTERRUPTED_SLOT),
        Some(Slot::Int(value)) if * value != 0
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
    Ok(Some(Slot::Int(i32::from(field_interrupted || host_interrupted))))
}
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_thread_interrupted(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Int(i32::from(take_current_host_thread_interrupted()))))
}
