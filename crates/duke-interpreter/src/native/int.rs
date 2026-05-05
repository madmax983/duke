/// Extract int elements from an `IntStream` heap object.
fn int_stream_elems(heap: &duke_gc::Heap, ref_: u64) -> Vec<i32> {
    let size = match heap.get(ref_).ok().and_then(|o| o.fields.first().copied()) {
        Some(Slot::Int(n)) => usize::try_from(n).unwrap_or(0),
        _ => 0,
    };
    heap.get(ref_)
        .ok()
        .map(|o| {
            o.fields[1..=size]
                .iter()
                .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
                .collect()
        })
        .unwrap_or_default()
}
/// Native: `IntStream.range(int,int)IntStream` — half-open range [start, end).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let start = match args.first().copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    let end = match args.get(1).copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    let values: Vec<i32> = (start..end).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}
/// Native: `IntStream.rangeClosed(int,int)IntStream` — inclusive range [start, end].
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_range_closed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let start = match args.first().copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    let end = match args.get(1).copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    let values: Vec<i32> = (start..=end).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}
/// Native: `IntStream.of(int...)IntStream` — from an int array argument.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let values: Vec<i32> = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
        .collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}
/// Native: `IntStream.iterate(seed, UnaryOperator)IntStream` — generates up to 4096 elements.
pub(crate) fn native_int_stream_iterate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    const MAX: usize = 4096;
    let seed = match args.first().copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![seed])))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut values = Vec::with_capacity(MAX);
    let mut cur = seed;
    for _ in 0..MAX {
        values.push(cur);
        let next = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(I)I",
                vec![fn_slot, Slot::Int(cur)],
            )?
            .unwrap_or(Slot::Int(cur));
        match next {
            Slot::Int(n) => cur = n,
            _ => break,
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}
/// Native: `IntStream.count()J`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_count(
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
/// Native: `IntStream.sum()I`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_sum(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let sum: i32 = int_stream_elems(heap, r)
        .iter()
        .copied()
        .fold(0_i32, i32::wrapping_add);
    Ok(Some(Slot::Int(sum)))
}
/// Native: `IntStream.min()OptionalInt`
pub(crate) fn native_int_stream_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let opt_ref = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    if let Some(&v) = elems.iter().min() {
        heap.get_mut(opt_ref)?.fields[0] = Slot::Int(v);
        heap.get_mut(opt_ref)?.fields[1] = Slot::Int(1);
    } else {
        heap.get_mut(opt_ref)?.fields[1] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(opt_ref))))
}
/// Native: `IntStream.max()OptionalInt`
pub(crate) fn native_int_stream_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let opt_ref = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    if let Some(&v) = elems.iter().max() {
        heap.get_mut(opt_ref)?.fields[0] = Slot::Int(v);
        heap.get_mut(opt_ref)?.fields[1] = Slot::Int(1);
    } else {
        heap.get_mut(opt_ref)?.fields[1] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(opt_ref))))
}
/// Native: `IntStream.average()OptionalDouble`
pub(crate) fn native_int_stream_average(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let opt_ref = heap.allocate("duke/util/OptionalDouble".to_string(), 2);
    if elems.is_empty() {
        heap.get_mut(opt_ref)?.fields[1] = Slot::Int(0);
    } else {
        let sum: i64 = elems.iter().map(|&n| i64::from(n)).sum();
        #[allow(clippy::cast_precision_loss)]
        let avg = sum as f64 / elems.len() as f64;
        heap.get_mut(opt_ref)?.fields[0] = Slot::Double(avg);
        heap.get_mut(opt_ref)?.fields[1] = Slot::Int(1);
    }
    Ok(Some(Slot::Reference(Some(opt_ref))))
}
/// Native: `IntStream.toArray()int[]`
pub(crate) fn native_int_stream_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let arr_ref = heap.allocate("[I".to_string(), elems.len());
    for (i, v) in elems.into_iter().enumerate() {
        heap.get_mut(arr_ref)?.fields[i] = Slot::Int(v);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}
/// Native: `IntStream.filter(IntPredicate)IntStream`
pub(crate) fn native_int_stream_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(I)Z",
                vec![pred_slot, Slot::Int(v)],
            )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, kept)))))
}
/// Native: `IntStream.peek(IntConsumer)IntStream` — calls consumer for each element, returns same stream.
pub(crate) fn native_int_stream_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let elems = int_stream_elems(heap, r);
    if let Slot::Reference(Some(fn_ref)) = fn_slot {
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        for &v in &elems {
            ops.invoke(
                heap,
                out,
                &fn_class,
                "accept",
                "(I)V",
                vec![fn_slot, Slot::Int(v)],
            )?;
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, elems)))))
}
/// Native: `IntStream.map(IntUnaryOperator)IntStream`
pub(crate) fn native_int_stream_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(I)I",
                vec![fn_slot, Slot::Int(v)],
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
/// Native: `IntStream.forEach(IntConsumer)V`
pub(crate) fn native_int_stream_for_each(
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
    let elems = int_stream_elems(heap, r);
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for v in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(I)V",
            vec![consumer_slot, Slot::Int(v)],
        )?;
    }
    Ok(None)
}
/// Native: `IntStream.boxed()Stream` — wraps each int into `java/lang/Integer`.
pub(crate) fn native_int_stream_boxed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for v in elems {
        let boxed_ref = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(v);
        heap.get_mut(stream_ref)?.fields.push(Slot::Reference(Some(boxed_ref)));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
/// Native: `IntStream.mapToObj(IntFunction)Stream` — maps ints to objects.
pub(crate) fn native_int_stream_map_to_obj(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut mapped = Vec::new();
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(I)Ljava/lang/Object;",
                vec![fn_slot, Slot::Int(v)],
            )?;
        mapped.push(result.unwrap_or(Slot::Reference(None)));
    }
    let new_size = i32::try_from(mapped.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(new_size);
    for elem in mapped {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
/// Native: `IntStream.distinct()IntStream` — removes duplicate int values.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_distinct(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let mut seen: Vec<i32> = Vec::new();
    for v in elems {
        if !seen.contains(&v) {
            seen.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, seen)))))
}
/// Native: `IntStream.reduce(int, IntBinaryOperator)I` — fold with identity via callback.
pub(crate) fn native_int_stream_reduce_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let identity = match args.get(1) {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    let fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(identity)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let values = int_stream_elems(heap, stream_ref);
    let mut acc = identity;
    for v in values {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(II)I",
                vec![fn_slot, Slot::Int(acc), Slot::Int(v)],
            )?
            .unwrap_or(Slot::Int(0));
        acc = match result {
            Slot::Int(n) => n,
            _ => 0,
        };
    }
    Ok(Some(Slot::Int(acc)))
}
/// Native: `IntStream.reduce(IntBinaryOperator)OptionalInt` — fold without identity.
pub(crate) fn native_int_stream_reduce_optional(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        let r = make_optional_int(heap, None);
        return Ok(Some(Slot::Reference(Some(r))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let values = int_stream_elems(heap, stream_ref);
    if values.is_empty() {
        return Ok(Some(Slot::Reference(Some(make_optional_int(heap, None)))));
    }
    let mut acc = values[0];
    for &v in &values[1..] {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(II)I",
                vec![fn_slot, Slot::Int(acc), Slot::Int(v)],
            )?
            .unwrap_or(Slot::Int(0));
        acc = match result {
            Slot::Int(n) => n,
            _ => 0,
        };
    }
    Ok(Some(Slot::Reference(Some(make_optional_int(heap, Some(acc))))))
}
/// Native: `IntStream.sorted()IntStream` — returns a new sorted `IntStream`.
pub(crate) fn native_int_stream_sorted(
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
    let mut vals: Vec<i32> = heap
        .get(stream_ref)?
        .fields[1..=size]
        .iter()
        .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
        .collect();
    vals.sort_unstable();
    let new_stream = heap.allocate("duke/util/IntStream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(
        i32::try_from(vals.len()).unwrap_or(0),
    );
    for v in vals {
        heap.get_mut(new_stream)?.fields.push(Slot::Int(v));
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}
/// Native: `IntStream.asLongStream()LongStream` — widens each int to long.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_as_long_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let values: Vec<i64> = int_stream_elems(heap, r)
        .into_iter()
        .map(i64::from)
        .collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}
/// Native: `IntStream.asDoubleStream()DoubleStream` — widens each int to double.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_as_double_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let values: Vec<f64> = int_stream_elems(heap, r)
        .into_iter()
        .map(f64::from)
        .collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, values)))))
}
/// Native: `IntStream.findFirst()OptionalInt`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_find_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let opt = make_optional_int(heap, elems.into_iter().next());
    Ok(Some(Slot::Reference(Some(opt))))
}
/// Native: `IntStream.anyMatch(IntPredicate)Z`
pub(crate) fn native_int_stream_any_match(
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
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(I)Z",
                vec![pred_slot, Slot::Int(v)],
            )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}
/// Native: `IntStream.allMatch(IntPredicate)Z`
pub(crate) fn native_int_stream_all_match(
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
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(I)Z",
                vec![pred_slot, Slot::Int(v)],
            )?;
        if !matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}
/// Native: `IntStream.noneMatch(IntPredicate)Z`
pub(crate) fn native_int_stream_none_match(
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
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(I)Z",
                vec![pred_slot, Slot::Int(v)],
            )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}
/// Native: `IntStream.mapToLong(IntToLongFunction)LongStream`
pub(crate) fn native_int_stream_map_to_long(
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
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(I)J",
                vec![fn_slot, Slot::Int(v)],
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
/// Native: `IntStream.takeWhile(IntPredicate)IntStream` — keeps prefix while predicate holds.
pub(crate) fn native_int_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1) else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
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
                "(I)Z",
                vec![pred_slot, Slot::Int(v)],
            )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        } else {
            break;
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, kept)))))
}
/// Native: `IntStream.dropWhile(IntPredicate)IntStream` — drops prefix while predicate holds.
pub(crate) fn native_int_stream_drop_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1) else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
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
                    "(I)Z",
                    vec![pred_slot, Slot::Int(v)],
                )?;
            if !matches!(result, Some(Slot::Int(n)) if n != 0) {
                dropping = false;
                kept.push(v);
            }
        } else {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, kept)))))
}
/// Native: `IntStream.limit(long)IntStream` — truncate to at most n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_limit(
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
    let elems: Vec<i32> = int_stream_elems(heap, r).into_iter().take(n).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, elems)))))
}
/// Native: `IntStream.skip(long)IntStream` — skip first n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_skip(
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
    let elems: Vec<i32> = int_stream_elems(heap, r).into_iter().skip(n).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, elems)))))
}
/// Native: `IntStream.flatMap(IntFunction<IntStream>)IntStream` — map each int to an `IntStream` and concatenate.
pub(crate) fn native_int_stream_flat_map(
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
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let sub = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(I)Ljava/lang/Object;",
                vec![fn_slot, Slot::Int(v)],
            )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            let sub_elems = int_stream_elems(heap, sub_ref);
            result.extend(sub_elems);
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}
