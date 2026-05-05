fn throwable_field_slot(
    heap: &duke_gc::Heap,
    throwable_ref: u64,
    index: usize,
) -> Result<Slot> {
    Ok(
        heap
            .get(throwable_ref)?
            .fields
            .get(index)
            .copied()
            .unwrap_or(Slot::Reference(None)),
    )
}
fn throwable_header(heap: &duke_gc::Heap, throwable_ref: u64) -> Result<String> {
    let obj = heap.get(throwable_ref)?;
    let class_name = obj.class_name.replace('/', ".");
    Ok(
        match &obj.string_value {
            Some(msg) => format!("{class_name}: {msg}"),
            None => class_name,
        },
    )
}
fn throwable_trace_string(heap: &duke_gc::Heap, throwable_ref: u64) -> Result<String> {
    let mut out = String::new();
    let mut visited = std::collections::HashSet::new();
    append_throwable_trace(heap, throwable_ref, "", "", &mut out, &mut visited)?;
    Ok(out)
}
/// `Throwable.addSuppressed(Throwable suppressed)V`
///
/// Signature: `args[0]` = this (Throwable), `args[1]` = suppressed (Throwable)
pub(crate) fn native_throwable_add_suppressed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _stdout: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let suppressed = args.get(1).copied().unwrap_or(Slot::Reference(None));
    if suppressed.as_reference().is_none() {
        return Ok(None);
    }
    let existing = throwable_field_slot(heap, this_ref, THROWABLE_SUPPRESSED_FIELD)?;
    let mut elements = existing
        .as_reference()
        .map_or_else(
            Vec::new,
            |array_ref| {
                heap.get(array_ref).map(|obj| obj.fields.clone()).unwrap_or_default()
            },
        );
    elements.push(suppressed);
    let array_ref = allocate_slot_array(heap, THROWABLE_ARRAY_CLASS, &elements)?;
    set_object_field(
        heap,
        this_ref,
        THROWABLE_SUPPRESSED_FIELD,
        Slot::Reference(Some(array_ref)),
    )?;
    Ok(None)
}
/// Native: `Throwable.<init>(String)V` — stores detail message in `string_value`.
/// Native: `Throwable.<init>()V` - captures the construction stack trace.
pub(crate) fn native_throwable_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}
pub(crate) fn native_throwable_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let msg = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = msg;
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}
/// Native: `Throwable.<init>(String, Throwable)V` — stores message + cause.
pub(crate) fn native_throwable_init_string_cause(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let msg = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = msg;
    if let Some(&cause_slot) = args.get(2) && let Ok(obj) = heap.get_mut(this_ref)
        && !obj.fields.is_empty()
    {
        obj.fields[THROWABLE_CAUSE_FIELD] = cause_slot;
    }
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}
/// Native: `Throwable.<init>(Throwable)V` - stores only the cause.
pub(crate) fn native_throwable_init_cause(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    if let Some(&cause_slot) = args.get(1) && let Ok(obj) = heap.get_mut(this_ref)
        && !obj.fields.is_empty()
    {
        obj.fields[THROWABLE_CAUSE_FIELD] = cause_slot;
    }
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}
/// Native: `Throwable.getCause()Throwable` — returns the stored cause.
#[allow(clippy::unnecessary_wraps)]
/// Native: `Throwable.fillInStackTrace()Throwable`.
pub(crate) fn native_throwable_fill_in_stack_trace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(Some(Slot::Reference(Some(this_ref))))
}
pub(crate) fn native_throwable_get_cause(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let cause = throwable_field_slot(heap, *r, THROWABLE_CAUSE_FIELD)?;
            Ok(Some(cause))
        }
        _ => Ok(Some(Slot::Reference(None))),
    }
}
/// Native: `Throwable.getMessage()String` — returns the stored detail message.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_throwable_get_message(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let msg = heap.get(*r)?.string_value.clone();
            let slot = msg
                .map_or(
                    Slot::Reference(None),
                    |s| { Slot::Reference(Some(heap.allocate_string(s))) },
                );
            Ok(Some(slot))
        }
        _ => Ok(Some(Slot::Reference(None))),
    }
}
/// Native: `Throwable.toString()String` — returns `"ClassName: message"` or just class name.
pub(crate) fn native_throwable_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = throwable_header(heap, this_ref)?;
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Throwable.getStackTrace()StackTraceElement[]`.
pub(crate) fn native_throwable_get_stack_trace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let slot = throwable_field_slot(heap, this_ref, THROWABLE_STACK_TRACE_FIELD)?;
    Ok(Some(clone_reference_array(heap, slot, STACK_TRACE_ARRAY_CLASS)?))
}
/// Native: `Throwable.setStackTrace(StackTraceElement[])V`.
pub(crate) fn native_throwable_set_stack_trace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let new_trace = clone_reference_array(
        heap,
        args.get(1).copied().unwrap_or(Slot::Reference(None)),
        STACK_TRACE_ARRAY_CLASS,
    )?;
    set_object_field(heap, this_ref, THROWABLE_STACK_TRACE_FIELD, new_trace)?;
    Ok(None)
}
/// Native: `Throwable.getSuppressed()Throwable[]`.
pub(crate) fn native_throwable_get_suppressed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let slot = throwable_field_slot(heap, this_ref, THROWABLE_SUPPRESSED_FIELD)?;
    Ok(Some(clone_reference_array(heap, slot, THROWABLE_ARRAY_CLASS)?))
}
/// Native: `Throwable.printStackTrace()V`.
pub(crate) fn native_throwable_print_stack_trace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = throwable_trace_string(heap, this_ref)?;
    write!(out, "{text}").ok();
    Ok(None)
}
/// Native: `Throwable.printStackTrace(PrintStream)V`.
pub(crate) fn native_throwable_print_stack_trace_print_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = throwable_trace_string(heap, this_ref)?;
    write_to_print_stream_or_output(args, heap, out, &text)?;
    Ok(None)
}
