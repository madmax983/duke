/// Native: `Integer.parseInt(String)` — parses string to int.
pub(crate) fn native_integer_parseint(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(args, heap, i32::MIN, i32::MAX)?;
    Ok(Some(Slot::Int(val)))
}
/// Native: `Integer.parseInt(String,int)` — parses string to int with radix.
pub(crate) fn native_integer_parseint_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::MIN,
        i32::MAX,
    )?;
    Ok(Some(Slot::Int(val)))
}
/// Native: `Integer.valueOf(int)` — boxes int into Integer object.
pub(crate) fn native_integer_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.valueOf(String)` — parses and boxes int.
pub(crate) fn native_integer_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(args, heap, i32::MIN, i32::MAX)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.valueOf(String,int)` — parses and boxes int with radix.
pub(crate) fn native_integer_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::MIN,
        i32::MAX,
    )?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.decode(String)` — parses prefixed string and boxes int.
pub(crate) fn native_integer_decode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i32_decode_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.intValue()` — unboxes Integer to int.
pub(crate) fn native_integer_intvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
/// Native: `Integer.toString(int)` — static, converts int to String.
pub(crate) fn native_integer_tostring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.toHexString(int)` — unsigned lowercase hex string.
pub(crate) fn native_integer_tohexstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let r = heap.allocate_string(format!("{val:x}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.toOctalString(int)` — unsigned octal string.
pub(crate) fn native_integer_tooctalstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let r = heap.allocate_string(format!("{val:o}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.toBinaryString(int)` — unsigned binary string.
pub(crate) fn native_integer_tobinarystring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let r = heap.allocate_string(format!("{val:b}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.toUnsignedLong(int)` — widen via unsigned 32-bit interpretation.
pub(crate) fn native_integer_tounsignedlong_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    Ok(Some(Slot::Long(i64::from(val))))
}
/// Native: `Integer.compareUnsigned(int,int)` — compares ints as unsigned 32-bit values.
pub(crate) fn native_integer_compareunsigned_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let b = u32::from_ne_bytes(extract_int_arg(args, 1)?.to_ne_bytes());
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Integer.compareTo(Object)` — compares two boxed Integers.
pub(crate) fn native_integer_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let int_val = |s: &Slot| -> Result<i32> {
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
        Some(s) => int_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = int_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Integer.bitCount(int)` — count number of set bits (popcount).
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
pub(crate) fn native_integer_bitcount(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.count_ones() as i32)))
}
/// Native: `Integer.numberOfLeadingZeros(int)` — count leading zero bits.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
pub(crate) fn native_integer_leading_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.leading_zeros() as i32)))
}
/// Native: `Integer.numberOfTrailingZeros(int)` — count trailing zero bits.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
pub(crate) fn native_integer_trailing_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.trailing_zeros() as i32)))
}
/// Native: `Integer.highestOneBit(int)` — return value with only the highest set bit.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_integer_highest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    let result = if v == 0 { 0u32 } else { 1u32 << v.ilog2() };
    Ok(Some(Slot::Int(result as i32)))
}
/// Native: `Integer.lowestOneBit(int)` — return value with only the lowest set bit.
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn native_integer_lowest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(v & v.wrapping_neg())))
}
/// Native: `Integer.reverse(int)` — reverse bit order.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_integer_reverse(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.reverse_bits() as i32)))
}
/// Native: `Integer.reverseBytes(int)` — reverse byte order (swap endianness).
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_integer_reverse_bytes(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.swap_bytes() as i32)))
}
/// Native: `Integer.signum(int)` — returns -1, 0, or 1.
pub(crate) fn native_integer_signum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(v.signum())))
}
/// Native: `Integer.compare(int, int)` — static two-value comparison.
pub(crate) fn native_integer_compare_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Integer.sum(int, int)` — static addition (functional interface target).
pub(crate) fn native_integer_sum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.wrapping_add(b))))
}
/// Native: `Integer.max(int, int)` — static max (functional interface target).
pub(crate) fn native_integer_max_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.max(b))))
}
/// Native: `Integer.min(int, int)` — static min (functional interface target).
pub(crate) fn native_integer_min_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.min(b))))
}
/// Native: `Integer.compare(int,int)int` — returns negative/zero/positive.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_integer_compare(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(a.cmp(&b) as i32)))
}
/// Native: `Integer.max(int,int)int`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_integer_max(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(a.max(b))))
}
/// Native: `Integer.min(int,int)int`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_integer_min(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(a.min(b))))
}
