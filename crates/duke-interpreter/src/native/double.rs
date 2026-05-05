/// Native: `Double.parseDouble(String)` — parses string to double.
pub(crate) fn native_double_parsedouble(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f64 = s
        .trim()
        .parse()
        .map_err(|_| Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        })?;
    Ok(Some(Slot::Double(val)))
}
/// Native: `Double.valueOf(double)` — boxes double into Double object.
pub(crate) fn native_double_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_double_arg(args, 0)?;
    let r = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Double(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Double.doubleValue()` — unboxes Double to double.
pub(crate) fn native_double_doublevalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
/// Native: `Double.isNaN(D)Z` — returns 1 if value is NaN.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_isnan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Double(v)) => Ok(Some(Slot::Int(i32::from(v.is_nan())))),
        _ => Ok(Some(Slot::Int(0))),
    }
}
/// Native: `Double.compareTo(Object)` — compares two boxed Doubles.
pub(crate) fn native_double_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let double_val = |s: &Slot| -> Result<f64> {
        match s {
            Slot::Reference(Some(r)) => {
                match heap.get(*r)?.fields.first() {
                    Some(Slot::Double(n)) => Ok(*n),
                    _ => Err(Error::InvalidRef { address: *r }),
                }
            }
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => double_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = double_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.total_cmp(&b)))))
}
/// Native: `DoubleStream.sum()D` — sums all elements.
pub(crate) fn native_double_stream_sum(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let sum: f64 = stream_elements(heap, stream_ref)?
        .iter()
        .map(|s| match s {
            Slot::Double(d) => *d,
            Slot::Float(f) => f64::from(*f),
            Slot::Int(n) => f64::from(*n),
            _ => 0.0,
        })
        .sum();
    Ok(Some(Slot::Double(sum)))
}
/// Extract double elements from a `duke/util/DoubleStream`.
fn double_stream_elems(heap: &duke_gc::Heap, ref_: u64) -> Vec<f64> {
    stream_elements(heap, ref_)
        .map(|elems| {
            elems
                .into_iter()
                .filter_map(|s| { if let Slot::Double(d) = s { Some(d) } else { None } })
                .collect()
        })
        .unwrap_or_default()
}
/// Native: `DoubleStream.of(double[])DoubleStream` — from a double[] vararg array.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let values: Vec<f64> = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| { if let Slot::Double(d) = s { Some(*d) } else { None } })
        .collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, values)))))
}
/// Native: `DoubleStream.of(double)DoubleStream` — single-element factory.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_of_single(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = match args.first() {
        Some(Slot::Double(d)) => *d,
        Some(Slot::Float(f)) => f64::from(*f),
        _ => 0.0,
    };
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, vec![v])))))
}
/// Native: `DoubleStream.count()J`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_count(
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
/// Native: `DoubleStream.min()OptionalDouble`
pub(crate) fn native_double_stream_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let min = elems.iter().copied().reduce(f64::min);
    let opt_ref = make_optional_double_val(heap, min);
    Ok(Some(Slot::Reference(Some(opt_ref))))
}
/// Native: `DoubleStream.max()OptionalDouble`
pub(crate) fn native_double_stream_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let max = elems.iter().copied().reduce(f64::max);
    let opt_ref = make_optional_double_val(heap, max);
    Ok(Some(Slot::Reference(Some(opt_ref))))
}
/// Native: `DoubleStream.average()OptionalDouble`
pub(crate) fn native_double_stream_average(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let opt = if elems.is_empty() {
        None
    } else {
        #[allow(clippy::cast_precision_loss)]
        Some(elems.iter().sum::<f64>() / elems.len() as f64)
    };
    let opt_ref = make_optional_double_val(heap, opt);
    Ok(Some(Slot::Reference(Some(opt_ref))))
}
/// Native: `DoubleStream.toArray()double[]`
pub(crate) fn native_double_stream_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let arr_ref = heap.allocate("[D".to_string(), elems.len());
    for (i, v) in elems.into_iter().enumerate() {
        heap.get_mut(arr_ref)?.fields[i] = Slot::Double(v);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}
/// Native: `DoubleStream.sorted()DoubleStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_sorted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let mut elems = double_stream_elems(heap, r);
    elems.sort_by(f64::total_cmp);
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, elems)))))
}
/// Native: `DoubleStream.filter(DoublePredicate)DoubleStream`
pub(crate) fn native_double_stream_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(heap, vec![])))));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(D)Z",
                vec![pred_slot, Slot::Double(v)],
            )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, kept)))))
}
/// Native: `DoubleStream.map(DoubleUnaryOperator)DoubleStream`
pub(crate) fn native_double_stream_map(
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
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(D)D",
                vec![fn_slot, Slot::Double(v)],
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
/// Native: `DoubleStream.takeWhile(DoublePredicate)DoubleStream` — keeps prefix while predicate holds.
pub(crate) fn native_double_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1) else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(heap, vec![])))));
    };
    let elems = double_stream_elems(heap, r);
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
                "(D)Z",
                vec![pred_slot, Slot::Double(v)],
            )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        } else {
            break;
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, kept)))))
}
/// Native: `DoubleStream.dropWhile(DoublePredicate)DoubleStream` — drops prefix while predicate holds.
pub(crate) fn native_double_stream_drop_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1) else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(heap, vec![])))));
    };
    let elems = double_stream_elems(heap, r);
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
                    "(D)Z",
                    vec![pred_slot, Slot::Double(v)],
                )?;
            if !matches!(result, Some(Slot::Int(n)) if n != 0) {
                dropping = false;
                kept.push(v);
            }
        } else {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, kept)))))
}
/// Native: `Double.compare(double,double)int`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_compare(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    Ok(Some(Slot::Int(a.total_cmp(&b) as i32)))
}
/// Native: `Double.max(double,double)double`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_max(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    Ok(Some(Slot::Double(a.max(b))))
}
/// Native: `Double.min(double,double)double`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_min(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    Ok(Some(Slot::Double(a.min(b))))
}
/// Native: `DoubleStream.limit(long)DoubleStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_limit(
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
    let elems: Vec<f64> = double_stream_elems(heap, r).into_iter().take(n).collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, elems)))))
}
/// Native: `DoubleStream.skip(long)DoubleStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_skip(
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
    let elems: Vec<f64> = double_stream_elems(heap, r).into_iter().skip(n).collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, elems)))))
}
/// Native: `DoubleStream.forEach(DoubleConsumer)V`
pub(crate) fn native_double_stream_for_each(
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
    let elems = double_stream_elems(heap, r);
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for v in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(D)V",
            vec![consumer_slot, Slot::Double(v)],
        )?;
    }
    Ok(None)
}
/// Native: `DoubleStream.anyMatch(DoublePredicate)Z`
pub(crate) fn native_double_stream_any_match(
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
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(D)Z",
                vec![pred_slot, Slot::Double(v)],
            )?;
        if matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}
/// Native: `DoubleStream.allMatch(DoublePredicate)Z`
pub(crate) fn native_double_stream_all_match(
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
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(D)Z",
                vec![pred_slot, Slot::Double(v)],
            )?;
        if !matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}
/// Native: `DoubleStream.noneMatch(DoublePredicate)Z`
pub(crate) fn native_double_stream_none_match(
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
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(D)Z",
                vec![pred_slot, Slot::Double(v)],
            )?;
        if matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}
/// Native: `DoubleStream.findFirst()OptionalDouble`
pub(crate) fn native_double_stream_find_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let opt_ref = make_optional_double_val(heap, elems.into_iter().next());
    Ok(Some(Slot::Reference(Some(opt_ref))))
}
/// Native: `DoubleStream.reduce(double, DoubleBinaryOperator)D`
pub(crate) fn native_double_stream_reduce_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let identity = match args.get(1).copied() {
        Some(Slot::Double(d)) => d,
        _ => 0.0,
    };
    let fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Double(identity)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = double_stream_elems(heap, r);
    let mut acc = identity;
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(DD)D",
                vec![fn_slot, Slot::Double(acc), Slot::Double(v)],
            )?
            .unwrap_or(Slot::Double(0.0));
        acc = match result {
            Slot::Double(d) => d,
            Slot::Float(f) => f64::from(f),
            Slot::Int(n) => f64::from(n),
            _ => acc,
        };
    }
    Ok(Some(Slot::Double(acc)))
}
/// Native: `DoubleStream.reduce(DoubleBinaryOperator)OptionalDouble`
pub(crate) fn native_double_stream_reduce_optional(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        let opt_ref = make_optional_double_val(heap, None);
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = double_stream_elems(heap, r);
    if elems.is_empty() {
        return Ok(Some(Slot::Reference(Some(make_optional_double_val(heap, None)))));
    }
    let mut acc = elems[0];
    for &v in &elems[1..] {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(DD)D",
                vec![fn_slot, Slot::Double(acc), Slot::Double(v)],
            )?
            .unwrap_or(Slot::Double(0.0));
        acc = match result {
            Slot::Double(d) => d,
            Slot::Float(f) => f64::from(f),
            Slot::Int(n) => f64::from(n),
            _ => acc,
        };
    }
    Ok(Some(Slot::Reference(Some(make_optional_double_val(heap, Some(acc))))))
}
/// Native: `DoubleStream.flatMap(DoubleFunction<DoubleStream>)DoubleStream`
pub(crate) fn native_double_stream_flat_map(
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
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let sub = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(D)Ljava/lang/Object;",
                vec![fn_slot, Slot::Double(v)],
            )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(double_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, result)))))
}
/// Native: `DoubleStream.mapToInt(DoubleToIntFunction)IntStream`
pub(crate) fn native_double_stream_map_to_int(
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
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(D)I",
                vec![fn_slot, Slot::Double(v)],
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
/// Native: `DoubleStream.mapToLong(DoubleToLongFunction)LongStream`
pub(crate) fn native_double_stream_map_to_long(
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
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(D)J",
                vec![fn_slot, Slot::Double(v)],
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
/// Native: `DoubleStream.distinct()DoubleStream` — removes duplicate values.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_distinct(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<f64> = elems
        .into_iter()
        .filter(|&v| seen.insert(v.to_bits()))
        .collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, deduped)))))
}
/// Native: `DoubleStream.boxed()Stream` — boxes each double into `java/lang/Double`.
pub(crate) fn native_double_stream_boxed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for v in elems {
        let boxed_ref = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed_ref)?.fields[0] = Slot::Double(v);
        heap.get_mut(stream_ref)?.fields.push(Slot::Reference(Some(boxed_ref)));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
