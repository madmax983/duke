/// Native: `Object.<init>()V` - root constructor is a no-op after null-checking `this`.
pub(crate) fn native_object_init(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(None)
}
/// Native: `Object.getClass()` — returns a lightweight `Class` object for the runtime type.
pub(crate) fn native_object_get_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let class_name = heap.get(this_ref)?.class_name.clone();
    let class_ref = allocate_class_object(heap, &class_name)?;
    Ok(Some(Slot::Reference(Some(class_ref))))
}
/// Native: `Object.equals(Object)` — default Java object identity comparison.
pub(crate) fn native_object_equals(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let equal = extract_ref_arg(args, 1) == Ok(this_ref);
    Ok(Some(Slot::Int(i32::from(equal))))
}
/// Native: `Object.hashCode()` — returns heap address as hash.
pub(crate) fn native_object_hashcode(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(Slot::Reference(Some(r))) => Ok(Some(Slot::Int(*r as i32))),
        _ => Err(Error::NullPointerException),
    }
}
/// Native: `Object.toString()` — delegates to `heap_object_to_string` so String,
/// boxed primitives, and opaque objects all produce the correct Java representation.
pub(crate) fn native_object_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap_object_to_string(heap.get(this_ref)?, this_ref);
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Object.clone()` — shallow-copies a heap object.
pub(crate) fn native_object_clone(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let new_ref = heap.clone_object(this_ref)?;
    Ok(Some(Slot::Reference(Some(new_ref))))
}
