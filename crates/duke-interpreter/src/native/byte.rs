const fn byte_from_slot(slot: Slot) -> Result<u8> {
    match slot {
        Slot::Int(value) => Ok(value.to_be_bytes()[3]),
        _ => {
            Err(Error::TypeMismatch {
                expected: "byte",
                got: "other",
            })
        }
    }
}
fn byte_array_window(
    heap: &duke_gc::Heap,
    array_ref: u64,
    offset: i32,
    length: i32,
) -> Result<Vec<u8>> {
    if offset < 0 || length < 0 {
        return Err(index_out_of_bounds_error());
    }
    let start = usize::try_from(offset).map_err(|_| index_out_of_bounds_error())?;
    let count = usize::try_from(length).map_err(|_| index_out_of_bounds_error())?;
    let end = start.checked_add(count).ok_or_else(index_out_of_bounds_error)?;
    let fields = &heap.get(array_ref)?.fields;
    if end > fields.len() {
        return Err(index_out_of_bounds_error());
    }
    fields[start..end].iter().copied().map(byte_from_slot).collect()
}
pub(crate) fn native_byte_array_output_stream_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.string_value = Some(String::new());
    Ok(None)
}
pub(crate) fn native_byte_array_output_stream_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let string_ref = heap.allocate_string(text);
    Ok(Some(Slot::Reference(Some(string_ref))))
}
pub(crate) fn native_byte_parsebyte(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(
        args,
        heap,
        i32::from(i8::MIN),
        i32::from(i8::MAX),
    )?;
    Ok(Some(Slot::Int(val)))
}
pub(crate) fn native_byte_parsebyte_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i8::MIN),
        i32::from(i8::MAX),
    )?;
    Ok(Some(Slot::Int(val)))
}
pub(crate) fn native_byte_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_byte_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(
        args,
        heap,
        i32::from(i8::MIN),
        i32::from(i8::MAX),
    )?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_byte_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i8::MIN),
        i32::from(i8::MAX),
    )?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_byte_bytevalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
pub(crate) fn native_byte_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let byte_val = |s: &Slot| -> Result<i32> {
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
        Some(s) => byte_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = byte_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
fn byte_array_from_ref(heap: &duke_gc::Heap, array_ref: u64) -> Result<Vec<u8>> {
    let arr = heap.get(array_ref)?;
    let mut bytes = Vec::with_capacity(arr.fields.len());
    for slot in &arr.fields {
        let Slot::Int(v) = slot else {
            return Err(Error::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        };
        bytes.push(v.to_le_bytes()[0]);
    }
    Ok(bytes)
}
