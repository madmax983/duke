pub(crate) fn native_short_parseshort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(
        args,
        heap,
        i32::from(i16::MIN),
        i32::from(i16::MAX),
    )?;
    Ok(Some(Slot::Int(val)))
}
pub(crate) fn native_short_parseshort_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i16::MIN),
        i32::from(i16::MAX),
    )?;
    Ok(Some(Slot::Int(val)))
}
pub(crate) fn native_short_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_short_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(
        args,
        heap,
        i32::from(i16::MIN),
        i32::from(i16::MAX),
    )?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_short_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i16::MIN),
        i32::from(i16::MAX),
    )?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_short_shortvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
pub(crate) fn native_short_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let short_val = |s: &Slot| -> Result<i32> {
        match s {
            Slot::Reference(Some(r)) => {
                match heap.get(*r)?.fields.first() {
                    Some(Slot::Int(n)) => Ok(*n),
                    _ => Err(Error::InvalidRef { address: *r }),
                }
            }
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => short_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = short_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
