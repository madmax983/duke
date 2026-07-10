/// Native: `Unsafe.getUnsafe()Ljdk/internal/misc/Unsafe;` — returns a reference
/// to a (stateless) synthetic `Unsafe` instance. Every Duke `Unsafe` native
/// ignores `this`, so a freshly allocated zero-field object is sufficient; real
/// bytecode caches the returned reference in its own static field.
#[allow(clippy::unnecessary_wraps)] // signature must match `NativeHandler`
pub(crate) fn native_unsafe_get_unsafe(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let unsafe_ref = heap.allocate("jdk/internal/misc/Unsafe".to_string(), 0);
    Ok(Some(Slot::Reference(Some(unsafe_ref))))
}
/// Native: `Unsafe.objectFieldOffset(Ljava/lang/Class;Ljava/lang/String;)J` —
/// returns the positional `fields` slot index of the named instance field.
pub(crate) fn native_unsafe_object_field_offset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 1)?;
    let field_name = extract_string_arg_value(args, 2, heap)?;
    let class_name = class_internal_name_from_ref(heap, class_ref)?;
    // The target class may only have been referenced via an `ldc` class constant
    // (mirror created) without its field layout being linked into the registry;
    // ensure it is loaded before resolving the positional field slot.
    ops.ensure_loaded(&class_name)?;
    let slot = ops.instance_field_slot(&class_name, &field_name)?;
    Ok(Some(Slot::Long(i64::try_from(slot).unwrap_or(0))))
}
/// Native: `Unsafe.arrayBaseOffset(Ljava/lang/Class;)I` — 0 under the positional
/// array model (element index == `fields` index).
pub(crate) fn native_unsafe_array_base_offset(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(0)))
}
/// Native: `Unsafe.arrayIndexScale(Ljava/lang/Class;)I` — 1 under the positional
/// array model. It is a power of two, satisfying the real-bytecode invariant.
pub(crate) fn native_unsafe_array_index_scale(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(1)))
}
/// Native: `Unsafe.compareAndSetReference(Ljava/lang/Object;JLjava/lang/Object;Ljava/lang/Object;)Z`.
/// Reference-identity CAS on `fields[offset]`: if it equals `expected`, store `x`
/// and return true; otherwise return false.
pub(crate) fn native_unsafe_compare_and_set_reference(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let obj_ref = extract_ref_arg(args, 1)?;
    let offset = extract_long_arg(args, 2)?;
    let expected = args.get(3).copied().unwrap_or(Slot::Reference(None));
    let x = args.get(4).copied().unwrap_or(Slot::Reference(None));
    let idx = usize::try_from(offset).map_err(|_| Error::NullPointerException)?;
    let current = unsafe_field_slot(heap, obj_ref, offset)?;
    if current.as_reference() == expected.as_reference() {
        heap.write_field(obj_ref, idx, x)?;
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}
/// Native: `Unsafe.compareAndSetInt(Ljava/lang/Object;JII)Z`.
pub(crate) fn native_unsafe_compare_and_set_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let obj_ref = extract_ref_arg(args, 1)?;
    let offset = extract_long_arg(args, 2)?;
    let expected = extract_int_arg(args, 3)?;
    let x = extract_int_arg(args, 4)?;
    let idx = usize::try_from(offset).map_err(|_| Error::NullPointerException)?;
    let current = unsafe_field_slot(heap, obj_ref, offset)?.as_int().unwrap_or(0);
    if current == expected {
        heap.write_field(obj_ref, idx, Slot::Int(x))?;
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}
/// Native: `Unsafe.compareAndSetLong(Ljava/lang/Object;JJJ)Z`.
pub(crate) fn native_unsafe_compare_and_set_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let obj_ref = extract_ref_arg(args, 1)?;
    let offset = extract_long_arg(args, 2)?;
    let expected = extract_long_arg(args, 3)?;
    let x = extract_long_arg(args, 4)?;
    let idx = usize::try_from(offset).map_err(|_| Error::NullPointerException)?;
    let current = unsafe_field_slot(heap, obj_ref, offset)?.as_long().unwrap_or(0);
    if current == expected {
        heap.write_field(obj_ref, idx, Slot::Long(x))?;
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}
/// Native: `Unsafe.getReferenceAcquire(Ljava/lang/Object;J)Ljava/lang/Object;`.
/// The single-threaded interpreter needs no memory ordering.
pub(crate) fn native_unsafe_get_reference(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let obj_ref = extract_ref_arg(args, 1)?;
    let offset = extract_long_arg(args, 2)?;
    Ok(Some(unsafe_field_slot(heap, obj_ref, offset)?))
}
/// Native: `Unsafe.putReferenceRelease(Ljava/lang/Object;JLjava/lang/Object;)V`.
pub(crate) fn native_unsafe_put_reference(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let obj_ref = extract_ref_arg(args, 1)?;
    let offset = extract_long_arg(args, 2)?;
    let x = args.get(3).copied().unwrap_or(Slot::Reference(None));
    let idx = usize::try_from(offset).map_err(|_| Error::NullPointerException)?;
    heap.write_field(obj_ref, idx, x)?;
    Ok(None)
}
/// Native: `Unsafe.getAndAddInt(Ljava/lang/Object;JI)I` — atomically adds `delta`
/// to `fields[offset]` and returns the previous value.
pub(crate) fn native_unsafe_get_and_add_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let obj_ref = extract_ref_arg(args, 1)?;
    let offset = extract_long_arg(args, 2)?;
    let delta = extract_int_arg(args, 3)?;
    let idx = usize::try_from(offset).map_err(|_| Error::NullPointerException)?;
    let old = unsafe_field_slot(heap, obj_ref, offset)?.as_int().unwrap_or(0);
    heap.write_field(obj_ref, idx, Slot::Int(old.wrapping_add(delta)))?;
    Ok(Some(Slot::Int(old)))
}