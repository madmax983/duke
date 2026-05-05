/// Native: `Long.bitCount(long)` — count number of set bits.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
pub(crate) fn native_long_bitcount(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Int(v.count_ones() as i32)))
}
/// Native: `Long.numberOfLeadingZeros(long)` — count leading zero bits.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
pub(crate) fn native_long_leading_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Int(v.leading_zeros() as i32)))
}
/// Native: `Long.numberOfTrailingZeros(long)` — count trailing zero bits.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
pub(crate) fn native_long_trailing_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Int(v.trailing_zeros() as i32)))
}
/// Native: `Long.highestOneBit(long)` — return value with only the highest set bit.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_long_highest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    let result = if v == 0 { 0u64 } else { 1u64 << v.ilog2() };
    Ok(Some(Slot::Long(result as i64)))
}
/// Native: `Long.lowestOneBit(long)` — return value with only the lowest set bit.
pub(crate) fn native_long_lowest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Long(v & v.wrapping_neg())))
}
/// Native: `Long.reverse(long)` — reverse bit order.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_long_reverse(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Long(v.reverse_bits() as i64)))
}
/// Native: `Long.reverseBytes(long)` — reverse byte order (swap endianness).
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_long_reverse_bytes(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Long(v.swap_bytes() as i64)))
}
/// Native: `Long.signum(long)` — returns -1, 0, or 1.
pub(crate) fn native_long_signum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Int(v.signum() as i32)))
}
/// Native: `Long.compare(long, long)` — static two-value comparison.
pub(crate) fn native_long_compare_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Long.sum(long, long)` — static addition (functional interface target).
pub(crate) fn native_long_sum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(a.wrapping_add(b))))
}
/// Native: `Long.parseLong(String)` — parses string to long.
pub(crate) fn native_long_parselong(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_from_string_arg(args, heap)?;
    Ok(Some(Slot::Long(val)))
}
/// Native: `Long.parseLong(String,int)` — parses string to long with radix.
pub(crate) fn native_long_parselong_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_from_string_and_radix_args(args, heap)?;
    Ok(Some(Slot::Long(val)))
}
/// Native: `Long.valueOf(long)` — boxes long into Long object.
pub(crate) fn native_long_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.valueOf(String)` — parses and boxes long.
pub(crate) fn native_long_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.valueOf(String,int)` — parses and boxes long with radix.
pub(crate) fn native_long_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_from_string_and_radix_args(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.decode(String)` — parses prefixed string and boxes long.
pub(crate) fn native_long_decode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_decode_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.longValue()` — unboxes Long to long.
pub(crate) fn native_long_longvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
/// Native: `Long.intValue()I` — returns the long value narrowed to int.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_intvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = match heap.get(this_ref)?.fields.first() {
        #[allow(clippy::cast_possible_truncation)]
        Some(Slot::Long(v)) => *v as i32,
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(val)))
}
/// Native: `Long.toString(long)` — static, converts long to String.
pub(crate) fn native_long_tostring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.toHexString(long)` — unsigned lowercase hex string.
pub(crate) fn native_long_tohexstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(format!("{val:x}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.toOctalString(long)` — unsigned octal string.
pub(crate) fn native_long_tooctalstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(format!("{val:o}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.toBinaryString(long)` — unsigned binary string.
pub(crate) fn native_long_tobinarystring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(format!("{val:b}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.compareUnsigned(long,long)` — compares longs as unsigned 64-bit values.
pub(crate) fn native_long_compareunsigned_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = u64::from_ne_bytes(extract_long_arg(args, 0)?.to_ne_bytes());
    let b = u64::from_ne_bytes(extract_long_arg(args, 1)?.to_ne_bytes());
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Long.compareTo(Object)` — compares two boxed Longs.
pub(crate) fn native_long_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let long_val = |s: &Slot| -> Result<i64> {
        match s {
            Slot::Reference(Some(r)) => {
                match heap.get(*r)?.fields.first() {
                    Some(Slot::Long(n)) => Ok(*n),
                    _ => Err(Error::InvalidRef { address: *r }),
                }
            }
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => long_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = long_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `LongStream.sum()J` — sums all elements.
pub(crate) fn native_long_stream_sum(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let sum: i64 = heap
        .get(stream_ref)?
        .fields[1..=size]
        .iter()
        .map(|s| match s {
            Slot::Long(n) => *n,
            Slot::Int(n) => i64::from(*n),
            _ => 0,
        })
        .sum();
    Ok(Some(Slot::Long(sum)))
}
/// Extract long elements from a `duke/util/LongStream`.
fn long_stream_elems(heap: &duke_gc::Heap, ref_: u64) -> Vec<i64> {
    stream_elements(heap, ref_)
        .map(|elems| {
            elems
                .into_iter()
                .filter_map(|s| { if let Slot::Long(n) = s { Some(n) } else { None } })
                .collect()
        })
        .unwrap_or_default()
}
/// Native: `LongStream.of(long[])LongStream` — from a long[] vararg array.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let values: Vec<i64> = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| { if let Slot::Long(n) = s { Some(*n) } else { None } })
        .collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}
/// Native: `LongStream.range(long,long)LongStream` — half-open range [start, end).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let start = match args.first().copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let end = match args.get(1).copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let values: Vec<i64> = (start..end).collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}
/// Native: `LongStream.rangeClosed(long,long)LongStream` — inclusive range [start, end].
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_range_closed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let start = match args.first().copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let end = match args.get(1).copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let values: Vec<i64> = (start..=end).collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}
/// Native: `LongStream.count()J`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match heap.get(r)?.fields.first() {
        Some(Slot::Int(n)) => i64::from(*n),
        _ => 0,
    };
    Ok(Some(Slot::Long(n)))
}
/// Native: `LongStream.min()OptionalLong`
pub(crate) fn native_long_stream_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let opt = make_optional_long(heap, elems.into_iter().min());
    Ok(Some(Slot::Reference(Some(opt))))
}
/// Native: `LongStream.max()OptionalLong`
pub(crate) fn native_long_stream_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let opt = make_optional_long(heap, elems.into_iter().max());
    Ok(Some(Slot::Reference(Some(opt))))
}
/// Native: `LongStream.average()OptionalDouble`
pub(crate) fn native_long_stream_average(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let opt = if elems.is_empty() {
        None
    } else {
        #[allow(clippy::cast_precision_loss)]
        Some(elems.iter().sum::<i64>() as f64 / elems.len() as f64)
    };
    let opt_ref = make_optional_double_val(heap, opt);
    Ok(Some(Slot::Reference(Some(opt_ref))))
}
/// Native: `LongStream.toArray()long[]`
pub(crate) fn native_long_stream_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let arr_ref = heap.allocate("[J".to_string(), elems.len());
    for (i, v) in elems.into_iter().enumerate() {
        heap.get_mut(arr_ref)?.fields[i] = Slot::Long(v);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}
/// Native: `LongStream.sorted()LongStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_sorted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let mut elems = long_stream_elems(heap, r);
    elems.sort_unstable();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, elems)))))
}
/// Native: `LongStream.distinct()LongStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_distinct(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let mut seen = std::collections::HashSet::new();
    let elems: Vec<i64> = long_stream_elems(heap, r)
        .into_iter()
        .filter(|v| seen.insert(*v))
        .collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, elems)))))
}
/// Native: `LongStream.reduce(long, LongBinaryOperator)long`
pub(crate) fn native_long_stream_reduce_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let identity = match args.get(1).copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Long(identity)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = long_stream_elems(heap, r);
    let mut acc = identity;
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(JJ)J",
                vec![fn_slot, Slot::Long(acc), Slot::Long(v)],
            )?
            .unwrap_or(Slot::Long(0));
        acc = match result {
            Slot::Long(n) => n,
            Slot::Int(n) => i64::from(n),
            _ => acc,
        };
    }
    Ok(Some(Slot::Long(acc)))
}
/// Native: `LongStream.boxed()Stream` — boxes each long into `java/lang/Long`.
pub(crate) fn native_long_stream_boxed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for v in elems {
        let boxed_ref = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(boxed_ref)?.fields[0] = Slot::Long(v);
        heap.get_mut(stream_ref)?.fields.push(Slot::Reference(Some(boxed_ref)));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
/// Native: `LongStream.filter(LongPredicate)LongStream`
pub(crate) fn native_long_stream_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(J)Z",
                vec![pred_slot, Slot::Long(v)],
            )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, kept)))))
}
/// Native: `LongStream.map(LongUnaryOperator)LongStream`
pub(crate) fn native_long_stream_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(J)J",
                vec![fn_slot, Slot::Long(v)],
            )?;
        result
            .push(
                match r {
                    Some(Slot::Long(n)) => n,
                    Some(Slot::Int(n)) => i64::from(n),
                    _ => 0,
                },
            );
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}
/// Native: `LongStream.forEach(LongConsumer)V`
pub(crate) fn native_long_stream_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(None);
    };
    let elems = long_stream_elems(heap, r);
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for v in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(J)V",
            vec![consumer_slot, Slot::Long(v)],
        )?;
    }
    Ok(None)
}
/// Native: `LongStream.mapToInt(LongToIntFunction)IntStream`
pub(crate) fn native_long_stream_map_to_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(J)I",
                vec![fn_slot, Slot::Long(v)],
            )?;
        result
            .push(
                match r {
                    Some(Slot::Int(n)) => n,
                    _ => 0,
                },
            );
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}
/// Native: `LongStream.findFirst()OptionalLong`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_find_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let opt = make_optional_long(heap, elems.into_iter().next());
    Ok(Some(Slot::Reference(Some(opt))))
}
/// Native: `LongStream.anyMatch(LongPredicate)Z`
pub(crate) fn native_long_stream_any_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(J)Z",
                vec![pred_slot, Slot::Long(v)],
            )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}
/// Native: `LongStream.allMatch(LongPredicate)Z`
pub(crate) fn native_long_stream_all_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(J)Z",
                vec![pred_slot, Slot::Long(v)],
            )?;
        if !matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}
/// Native: `LongStream.noneMatch(LongPredicate)Z`
pub(crate) fn native_long_stream_none_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(J)Z",
                vec![pred_slot, Slot::Long(v)],
            )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}
/// Native: `LongStream.takeWhile(LongPredicate)LongStream` — keeps prefix while predicate holds.
pub(crate) fn native_long_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1) else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(J)Z",
                vec![pred_slot, Slot::Long(v)],
            )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        } else {
            break;
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, kept)))))
}
/// Native: `LongStream.dropWhile(LongPredicate)LongStream` — drops prefix while predicate holds.
pub(crate) fn native_long_stream_drop_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1) else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut dropping = true;
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        if dropping {
            let result = ops
                .invoke(
                    heap,
                    out,
                    &pred_class,
                    "test",
                    "(J)Z",
                    vec![pred_slot, Slot::Long(v)],
                )?;
            if !matches!(result, Some(Slot::Int(n)) if n != 0) {
                dropping = false;
                kept.push(v);
            }
        } else {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, kept)))))
}
/// Native: `Long.compare(long,long)int`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_compare(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(Some(Slot::Int(a.cmp(&b) as i32)))
}
/// Native: `Long.max(long,long)long`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_max(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(Some(Slot::Long(a.max(b))))
}
/// Native: `Long.min(long,long)long`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_min(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(Some(Slot::Long(a.min(b))))
}
/// Native: `LongStream.limit(long)LongStream` — truncate to at most n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<i64> = long_stream_elems(heap, r).into_iter().take(n).collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, elems)))))
}
/// Native: `LongStream.skip(long)LongStream` — skip first n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_skip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<i64> = long_stream_elems(heap, r).into_iter().skip(n).collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, elems)))))
}
/// Native: `LongStream.flatMap(LongFunction<LongStream>)LongStream`
pub(crate) fn native_long_stream_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let sub = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(J)Ljava/lang/Object;",
                vec![fn_slot, Slot::Long(v)],
            )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            let sub_elems = long_stream_elems(heap, sub_ref);
            result.extend(sub_elems);
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}
/// Native: `LongStream.reduce(LongBinaryOperator)OptionalLong`
pub(crate) fn native_long_stream_reduce_optional(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        let opt_ref = make_optional_long(heap, None);
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = long_stream_elems(heap, r);
    if elems.is_empty() {
        return Ok(Some(Slot::Reference(Some(make_optional_long(heap, None)))));
    }
    let mut acc = elems[0];
    for &v in &elems[1..] {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(JJ)J",
                vec![fn_slot, Slot::Long(acc), Slot::Long(v)],
            )?
            .unwrap_or(Slot::Long(0));
        acc = match result {
            Slot::Long(n) => n,
            Slot::Int(n) => i64::from(n),
            _ => acc,
        };
    }
    Ok(Some(Slot::Reference(Some(make_optional_long(heap, Some(acc))))))
}
/// Native: `LongStream.mapToDouble(LongToDoubleFunction)DoubleStream`
pub(crate) fn native_long_stream_map_to_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(J)D",
                vec![fn_slot, Slot::Long(v)],
            )?;
        result
            .push(
                match r {
                    Some(Slot::Double(d)) => d,
                    Some(Slot::Float(f)) => f64::from(f),
                    Some(Slot::Int(n)) => f64::from(n),
                    _ => 0.0,
                },
            );
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, result)))))
}
