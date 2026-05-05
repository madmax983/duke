/// Native: `StringBuffer.<init>()V` — empty buffer.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stringbuffer_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.string_value = Some(String::new());
    Ok(None)
}
/// Native: `StringBuffer.<init>(Ljava/lang/String;)V` — init with string.
pub(crate) fn native_stringbuffer_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = match args.get(1).copied() {
        Some(Slot::Reference(Some(r))) => {
            heap.get(r)?.string_value.clone().unwrap_or_default()
        }
        _ => String::new(),
    };
    heap.get_mut(this_ref)?.string_value = Some(s);
    Ok(None)
}
/// Native: `StringBuffer.append(...)StringBuffer` — append any type; returns `this`.
pub(crate) fn native_stringbuffer_append(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let frag = match extract_slot_arg(args, 1) {
        Slot::Reference(Some(r)) => {
            heap.get(r)?.string_value.clone().unwrap_or_else(|| "null".to_string())
        }
        Slot::Reference(None) => "null".to_string(),
        Slot::Int(n) => n.to_string(),
        Slot::Long(n) => n.to_string(),
        Slot::Double(d) => format!("{d}"),
        Slot::Float(f) => format!("{f}"),
        Slot::ReturnAddress(_) => String::new(),
    };
    let buf = heap.get_mut(this_ref)?.string_value.get_or_insert_with(String::new);
    buf.push_str(&frag);
    Ok(Some(Slot::Reference(Some(this_ref))))
}
/// Native: `StringBuffer.toString()String`.
pub(crate) fn native_stringbuffer_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `StringBuffer.length()I`.
pub(crate) fn native_stringbuffer_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let len = heap.get(this_ref)?.string_value.as_deref().unwrap_or("").len();
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}
