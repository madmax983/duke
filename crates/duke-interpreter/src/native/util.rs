/// Native: `OptionalDouble.isPresent()Z`
pub(crate) fn native_optional_double_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}

/// Native: `Optional.ifPresentOrElse(Consumer, Runnable)V` (Java 9) —
/// if value present invokes consumer, otherwise invokes runnable.
pub(crate) fn native_optional_if_present_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let runnable_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    if matches!(value, Slot::Reference(None)) {
        let Slot::Reference(Some(runnable_ref)) = runnable_slot else {
            return Ok(None);
        };
        let runnable_class = heap.get(runnable_ref)?.class_name.clone();
        ops.invoke(
            heap,
            out,
            &runnable_class,
            "run",
            "()V",
            vec![runnable_slot],
        )?;
    } else {
        let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
            return Ok(None);
        };
        let consumer_class = heap.get(consumer_ref)?.class_name.clone();
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, value],
        )?;
    }
    Ok(None)
}

/// Native: `Optional.or(Supplier<Optional>)Optional` (Java 9) —
/// returns this Optional if present; otherwise invokes supplier and returns its result.
pub(crate) fn native_optional_or(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let supplier_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    // If present (non-null value stored), return self
    if !matches!(value, Slot::Reference(None)) {
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    }
    let Slot::Reference(Some(supplier_ref)) = supplier_slot else {
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    };
    let supplier_class = heap.get(supplier_ref)?.class_name.clone();
    let result = ops.invoke(
        heap,
        out,
        &supplier_class,
        "get",
        "()Ljava/lang/Object;",
        vec![supplier_slot],
    )?;
    Ok(Some(result.unwrap_or(Slot::Reference(None))))
}

/// Native: `ComparingDoubleComparator.compare(O,O)I` — calls `fn.applyAsDouble(o)` for each.
pub(crate) fn native_comparing_double_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(Ljava/lang/Object;)D",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Double(0.0));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(Ljava/lang/Object;)D",
            vec![fn_slot, b],
        )?
        .unwrap_or(Slot::Double(0.0));
    let result = match (ka, kb) {
        (Slot::Double(da), Slot::Double(db)) => da.total_cmp(&db) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(result)))
}

/// Creates a `duke/util/ComparingDoubleComparator` with `fields[0] = fn_ref`.
pub(crate) fn native_comparator_comparing_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_ref = extract_ref_arg(args, 0)?;
    let r = heap.allocate("duke/util/ComparingDoubleComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(Some(fn_ref));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ComparingLongComparator.compare(O,O)I` — calls `fn.applyAsLong(o)` for each element.
pub(crate) fn native_comparing_long_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(Ljava/lang/Object;)J",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Long(0));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(Ljava/lang/Object;)J",
            vec![fn_slot, b],
        )?
        .unwrap_or(Slot::Long(0));
    let result = match (ka, kb) {
        (Slot::Long(la), Slot::Long(lb)) => la.cmp(&lb) as i32,
        (Slot::Int(ia), Slot::Int(ib)) => ia.cmp(&ib) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(result)))
}

/// Native: `Comparator.comparingLong(ToLongFunction)Comparator` — wraps key extractor.
/// Creates a `duke/util/ComparingLongComparator` with `fields[0] = fn_ref`.
pub(crate) fn native_comparator_comparing_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_ref = extract_ref_arg(args, 0)?;
    let r = heap.allocate("duke/util/ComparingLongComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(Some(fn_ref));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `OptionalLong.isPresent()Z`
pub(crate) fn native_optional_long_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}

/// Native: `OptionalLong.getAsLong()J`
pub(crate) fn native_optional_long_get_as_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    if !present {
        return Err(VmError::MethodNotFound {
            name: "OptionalLong.getAsLong on empty".to_string(),
            descriptor: String::new(),
        });
    }
    Ok(Some(
        heap.get(r)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Long(0)),
    ))
}

/// Native: `BiFunctionAndThen.apply(Object,Object)Object` — calls wrapped bifunction then after.
pub(crate) fn native_bifunction_and_then_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let bifunction = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let after = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(bf_ref)) = bifunction else {
        return Ok(Some(Slot::Reference(None)));
    };
    let bf_class = heap.get(bf_ref)?.class_name.clone();
    let mid = ops
        .invoke(
            heap,
            out,
            &bf_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![bifunction, a, b],
        )?
        .unwrap_or(Slot::Reference(None));
    Ok(Some(invoke_function_apply(after, mid, heap, out, ops)?))
}

/// Native: `BiFunction.andThen(Function)BiFunction` — returns `BiFunctionAndThen` proxy.
pub(crate) fn native_bifunction_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let bifunction = args.first().copied().unwrap_or(Slot::Reference(None));
    let after = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/BiFunctionAndThen".to_string(), 2);
    heap.get_mut(r)?.fields[0] = bifunction;
    heap.get_mut(r)?.fields[1] = after;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ComposeFunction.apply(O)O` — applies inner then outer.
pub(crate) fn native_compose_function_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let input = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let outer = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let inner = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let mid = invoke_function_apply(inner, input, heap, out, ops)?;
    invoke_function_apply(outer, mid, heap, out, ops).map(Some)
}

/// Native: `Function.compose(Function)Function` — `f.compose(g)` = `f(g(x))`.
pub(crate) fn native_function_compose(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let outer = args.first().copied().unwrap_or(Slot::Reference(None));
    let inner = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ComposeFunction".to_string(), 2);
    heap.get_mut(r)?.fields[0] = outer;
    heap.get_mut(r)?.fields[1] = inner;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `AndThenFunction.apply(O)O` — applies first then second.
pub(crate) fn native_and_then_function_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let input = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let first = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let second = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let mid = invoke_function_apply(first, input, heap, out, ops)?;
    invoke_function_apply(second, mid, heap, out, ops).map(Some)
}

/// Native: `Function.andThen(Function)Function` — `f.andThen(g)` = `g(f(x))`.
pub(crate) fn native_function_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let first = args.first().copied().unwrap_or(Slot::Reference(None));
    let second = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/AndThenFunction".to_string(), 2);
    heap.get_mut(r)?.fields[0] = first;
    heap.get_mut(r)?.fields[1] = second;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ThenComparingComparator.compare(O,O)I` — runs primary then secondary.
pub(crate) fn native_then_comparing_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let primary = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let secondary = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    // Invoke primary.compare(a, b)
    let result = invoke_comparator(primary, a, b, heap, out, ops)?;
    if result != 0 {
        return Ok(Some(Slot::Int(result)));
    }
    // Tie-break with secondary
    let result2 = invoke_comparator(secondary, a, b, heap, out, ops)?;
    Ok(Some(Slot::Int(result2)))
}

/// Native: `Comparator.thenComparing(Comparator)Comparator` — chains two comparators.
/// Stores primary in `fields[0]`, secondary in `fields[1]`.
pub(crate) fn native_comparator_then_comparing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let primary = args.first().copied().unwrap_or(Slot::Reference(None));
    let secondary = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ThenComparingComparator".to_string(), 2);
    heap.get_mut(r)?.fields[0] = primary;
    heap.get_mut(r)?.fields[1] = secondary;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Arrays.toString(Object[])String` — formats as `[a, b, c]`.
pub(crate) fn native_arrays_to_string_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(arr_ref)?.fields.clone();
    let mut parts: Vec<String> = Vec::with_capacity(fields.len());
    for s in &fields {
        let part = match s {
            Slot::Reference(Some(r)) => heap
                .get(*r)
                .ok()
                .and_then(|o| o.string_value.clone())
                .unwrap_or_else(|| "null".to_string()),
            Slot::Reference(None) => "null".to_string(),
            Slot::Int(n) => n.to_string(),
            _ => "?".to_string(),
        };
        parts.push(part);
    }
    let result = format!("[{}]", parts.join(", "));
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Arrays.toString(int[])String` — formats as `[1, 2, 3]`.
pub(crate) fn native_arrays_to_string_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(arr_ref)?.fields.clone();
    let parts: Vec<String> = fields
        .iter()
        .map(|s| match s {
            Slot::Int(n) => n.to_string(),
            _ => "0".to_string(),
        })
        .collect();
    let result = format!("[{}]", parts.join(", "));
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Optional.orElseGet(Supplier)Object` — calls supplier if empty.
pub(crate) fn native_optional_or_else_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let supplier_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    if !matches!(value, Slot::Reference(None)) {
        return Ok(Some(value));
    }
    let Slot::Reference(Some(supplier_ref)) = supplier_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let supplier_class = heap.get(supplier_ref)?.class_name.clone();
    let result = ops.invoke(
        heap,
        out,
        &supplier_class,
        "get",
        "()Ljava/lang/Object;",
        vec![supplier_slot],
    )?;
    Ok(Some(result.unwrap_or(Slot::Reference(None))))
}

/// Native: `Optional.ifPresent(Consumer)V` — invokes consumer if value is present.
pub(crate) fn native_optional_if_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    if let (Slot::Reference(Some(_)), Slot::Reference(Some(consumer_ref))) = (value, consumer_slot)
    {
        let consumer_class = heap.get(consumer_ref)?.class_name.clone();
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, value],
        )?;
    }
    Ok(None)
}

/// Native: `Optional.filter(Predicate)Optional` — keeps value only if predicate passes.
pub(crate) fn native_optional_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let pred_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let result_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if matches!(value, Slot::Reference(None)) {
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    }
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        heap.get_mut(result_ref)?.fields[0] = value;
        return Ok(Some(Slot::Reference(Some(result_ref))));
    };
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let test_result = ops.invoke(
        heap,
        out,
        &pred_class,
        "test",
        "(Ljava/lang/Object;)Z",
        vec![pred_slot, value],
    )?;
    let passes = matches!(test_result, Some(Slot::Int(n)) if n != 0);
    let stored = if passes { value } else { Slot::Reference(None) };
    heap.get_mut(result_ref)?.fields[0] = stored;
    Ok(Some(Slot::Reference(Some(result_ref))))
}

/// Native: `Random.nextBoolean()Z` — random boolean.
pub(crate) fn native_random_next_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, v) = random_step(heap, this_ref, 1)?;
    Ok(Some(Slot::Int(v)))
}

/// Native: `Random.nextFloat()F` — uniform [0.0, 1.0) as float.
pub(crate) fn native_random_next_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, bits) = random_step(heap, this_ref, 24)?;
    // next(24) is non-negative, safe to cast to f32
    let v = bits as f32 / (1u32 << 24) as f32;
    Ok(Some(Slot::Float(v)))
}

/// Native: `Random.nextDouble()D` — uniform [0.0, 1.0).
pub(crate) fn native_random_next_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, hi) = random_step(heap, this_ref, 26)?;
    let (_, lo) = random_step(heap, this_ref, 27)?;
    // Java spec: ((long)(next(26)) << 27) + next(27)) / (double)(1L << 53)
    let combined = (i64::from(hi) << 27) + i64::from(lo);
    let v = combined as f64 / (1u64 << 53) as f64;
    Ok(Some(Slot::Double(v)))
}

/// Native: `Random.nextLong()J` — 64-bit random long (two 32-bit calls).
pub(crate) fn native_random_next_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, hi) = random_step(heap, this_ref, 32)?;
    let (_, lo) = random_step(heap, this_ref, 32)?;
    let v = (i64::from(hi) << 32) + i64::from(lo);
    Ok(Some(Slot::Long(v)))
}

pub(crate) fn native_random_next_int_bound(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let bound = extract_int_arg(args, 1)?;
    if bound <= 0 {
        return Err(duke_runtime::VmError::JavaException {
            class_name: "java/lang/IllegalArgumentException".to_string(),
        });
    }
        let bound_u = bound as u32;
    // Java rejection-sampling to avoid modulo bias
    loop {
        let (_, bits) = random_step(heap, this_ref, 31)?;
                let bits_u = bits as u32; // bits from next(31) are always non-negative
        let val = bits_u % bound_u;
        if bits_u.wrapping_sub(val).wrapping_add(bound_u - 1) < u32::MAX {
                        return Ok(Some(Slot::Int(val as i32))); // val < bound <= i32::MAX
        }
    }
}

/// Native: `Random.nextInt()I` — full-range random int.
pub(crate) fn native_random_next_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, v) = random_step(heap, this_ref, 32)?;
    Ok(Some(Slot::Int(v)))
}

pub(crate) fn native_random_init_seed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let seed = match args.get(1).copied() {
        Some(Slot::Long(v)) => {  v as u64 },
        Some(Slot::Int(v)) => {  v as u64 },
        _ => 0,
    };
    let initial = (seed ^ RANDOM_MULTIPLIER) & RANDOM_MASK;
     { heap.get_mut(this_ref)?.fields[0] = Slot::Long(initial as i64); } // safe: < 2^48
    Ok(None)
}

/// Native: `Random.<init>()V` — seed from current time.
pub(crate) fn native_random_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let nanos = u64::from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos()),
    );
    let initial = (nanos ^ RANDOM_MULTIPLIER) & RANDOM_MASK;
     { heap.get_mut(this_ref)?.fields[0] = Slot::Long(initial as i64); } // safe: mask ensures < 2^48
    Ok(None)
}

/// Native: `ReversedComparator.compare(a, b)I` — inverts delegate comparison.
pub(crate) fn native_reversed_comparator_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let delegate = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(del_ref)) = delegate else {
        return Ok(Some(Slot::Int(0)));
    };
    let del_class = heap.get(del_ref)?.class_name.clone();
    let result = ops
        .invoke(
            heap,
            out,
            &del_class,
            "compare",
            "(Ljava/lang/Object;Ljava/lang/Object;)I",
            vec![delegate, a, b],
        )?
        .unwrap_or(Slot::Int(0));
    let cmp = match result {
        Slot::Int(n) => n,
        _ => 0,
    };
    Ok(Some(Slot::Int(-cmp)))
}

/// Native: `Comparator.reversed()Comparator` — wraps comparator to invert ordering.
pub(crate) fn native_comparator_reversed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let delegate = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ReversedComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = delegate;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Arrays.copyOfRange(Object[], int, int)Object[]` — slice of reference array.
pub(crate) fn native_arrays_copy_of_range_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let from = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let to = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let new_len = to.saturating_sub(from);
    let (src_class, src_fields) = {
        let obj = heap.get(src_ref)?;
        (obj.class_name.clone(), obj.fields.clone())
    };
    let dst_ref = heap.allocate(src_class, new_len);
    for i in 0..new_len {
        heap.get_mut(dst_ref)?.fields[i] = src_fields
            .get(from + i)
            .copied()
            .unwrap_or(Slot::Reference(None));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.copyOfRange(int[], int, int)int[]` — slice of int array, zero-padded.
pub(crate) fn native_arrays_copy_of_range_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let from = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let to = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let new_len = to.saturating_sub(from);
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[I".to_string(), new_len);
    for i in 0..new_len {
        heap.get_mut(dst_ref)?.fields[i] =
            src_fields.get(from + i).copied().unwrap_or(Slot::Int(0));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.equals(int[], int[])boolean` — element-wise equality.
pub(crate) fn native_arrays_equals_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a_ref = extract_ref_arg(args, 0)?;
    let b_ref = extract_ref_arg(args, 1)?;
    let a_fields = heap.get(a_ref)?.fields.clone();
    let b_fields = heap.get(b_ref)?.fields.clone();
    let equal = a_fields.len() == b_fields.len()
        && a_fields.iter().zip(&b_fields).all(|(x, y)| match (x, y) {
            (Slot::Int(a), Slot::Int(b)) => a == b,
            _ => false,
        });
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `Arrays.sort(int[])` — sorts fields in place.
pub(crate) fn native_arrays_sort_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(arr_ref)?;
    obj.fields.sort_by(|a, b| match (a, b) {
        (Slot::Int(x), Slot::Int(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    });
    Ok(None)
}

/// Native: `Arrays.copyOf(Object[], int)` — copies to new Object[] of given length.
pub(crate) fn native_arrays_copyof_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let new_len = match args.get(1) {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(VmError::NegativeArraySize { size: *n }),
        _ => 0,
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[Ljava/lang/Object;".to_string(), new_len);
    let dst = heap.get_mut(dst_ref)?;
    for i in 0..new_len {
        dst.fields[i] = src_fields.get(i).copied().unwrap_or(Slot::Reference(None));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.copyOf(int[], int)` — copies to new int[] of given length.
pub(crate) fn native_arrays_copyof_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let new_len = match args.get(1) {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(VmError::NegativeArraySize { size: *n }),
        _ => 0,
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[I".to_string(), new_len);
    let dst = heap.get_mut(dst_ref)?;
    for i in 0..new_len {
        dst.fields[i] = src_fields.get(i).copied().unwrap_or(Slot::Int(0));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.fill(Object[], Object)` — fills all elements with val.
pub(crate) fn native_arrays_fill_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let val = extract_slot_arg(args, 1);
    let obj = heap.get_mut(arr_ref)?;
    for slot in &mut obj.fields {
        *slot = val;
    }
    Ok(None)
}

/// Native: `Arrays.fill(int[], int)` — fills all elements with val.
pub(crate) fn native_arrays_fill_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let val = match args.get(1) {
        Some(Slot::Int(v)) => Slot::Int(*v),
        _ => Slot::Int(0),
    };
    let obj = heap.get_mut(arr_ref)?;
    for slot in &mut obj.fields {
        *slot = val;
    }
    Ok(None)
}

/// Native: `Arrays.sort(Object[])V` — natural order sort using `compareTo`.
pub(crate) fn native_arrays_sort_objects(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let len = heap.get(arr_ref)?.fields.len();
    // Insertion sort with compareTo callbacks
    for i in 1..len {
        let mut j = i;
        while j > 0 {
            let a = heap.get(arr_ref)?.fields[j - 1];
            let b = heap.get(arr_ref)?.fields[j];
            let cmp = compare_slots_natural(a, b, heap, out, ops)?;
            if cmp <= 0 {
                break;
            }
            heap.write_field(arr_ref, j - 1, b)?;
            heap.write_field(arr_ref, j, a)?;
            j -= 1;
        }
    }
    Ok(None)
}

/// Native: `Comparator.comparing compare(Object,Object)I` — compare via key extractor.
pub(crate) fn native_comparing_comparator_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let fn_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Reference(None));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, b],
        )?
        .unwrap_or(Slot::Reference(None));
    // Compare extracted keys via natural ordering (String, Integer, Long, or raw int).
    let cmp = compare_treemap_keys(ka, kb, heap) as i32;
    Ok(Some(Slot::Int(cmp)))
}

/// Native: `Comparator.comparing(Function)Comparator` — creates a comparator by key extractor.
/// Returns a `duke/util/ComparingComparator` with `fields[0]`=fn\_ref.
pub(crate) fn native_comparator_comparing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/ComparingComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Math.random()D` — returns a pseudo-random double in [0.0, 1.0).
pub(crate) fn native_math_random(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // Use a simple deterministic seed based on stack pointer heuristic
    // For a JVM interpreter we just use a fixed-seed LCG for reproducibility
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(12345);
    let old = SEED.load(Ordering::Relaxed);
    let new = old.wrapping_mul(25_214_903_917).wrapping_add(11) & 0x0000_FFFF_FFFF_FFFF;
    SEED.store(new, Ordering::Relaxed);
        let v = (new as f64) / (1_u64 << 48) as f64;
    Ok(Some(Slot::Double(v)))
}

/// Native: `ComparingIntComparator.compare(O,O)I` — calls `fn.applyAsInt(o)` for each element.
pub(crate) fn native_comparing_int_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let b = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(Ljava/lang/Object;)I",
            vec![Slot::Reference(Some(fn_ref)), a],
        )?
        .unwrap_or(Slot::Int(0));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(Ljava/lang/Object;)I",
            vec![Slot::Reference(Some(fn_ref)), b],
        )?
        .unwrap_or(Slot::Int(0));
    let result = match (ka, kb) {
        (Slot::Int(ia), Slot::Int(ib)) => ia.cmp(&ib) as i32,
        _ => 0,
    };
    let _ = control;
    Ok(Some(Slot::Int(result)))
}

/// Native: `Comparator.comparingInt(ToIntFunction)Comparator` — wraps key extractor.
/// Creates a `duke/util/ComparingIntComparator` with `fields[0] = fn_ref`.
pub(crate) fn native_comparator_comparing_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fn_ref = extract_ref_arg(args, 0)?;
    let r = heap.allocate("duke/util/ComparingIntComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(Some(fn_ref));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Comparator.reverseOrder()Comparator` — returns a singleton reverse comparator.
pub(crate) fn native_comparator_reverse_order(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("duke/util/ReverseOrderComparator".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Comparator.naturalOrder()Comparator` — returns a singleton synthetic comparator.
pub(crate) fn native_comparator_natural_order(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("duke/util/NaturalOrderComparator".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `OptionalDouble.getAsDouble()D`
pub(crate) fn native_optional_double_get_as_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let v = match heap.get(r)?.fields.first().copied() {
        Some(Slot::Double(d)) => d,
        _ => 0.0,
    };
    Ok(Some(Slot::Double(v)))
}

/// Native: `OptionalDouble.orElse(double)D`
pub(crate) fn native_optional_double_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    if present {
        Ok(Some(
            heap.get(r)?
                .fields
                .first()
                .copied()
                .unwrap_or(Slot::Double(0.0)),
        ))
    } else {
        Ok(Some(args.get(1).copied().unwrap_or(Slot::Double(0.0))))
    }
}

/// Native: `OptionalLong.orElse(long)J`
pub(crate) fn native_optional_long_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    if present {
        Ok(Some(
            heap.get(r)?
                .fields
                .first()
                .copied()
                .unwrap_or(Slot::Long(0)),
        ))
    } else {
        Ok(Some(args.get(1).copied().unwrap_or(Slot::Long(0))))
    }
}

/// Native: `OptionalInt.empty()OptionalInt` — creates an empty `OptionalInt`.
pub(crate) fn native_optional_int_empty(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    heap.get_mut(r)?.fields[1] = Slot::Int(0); // present = false
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `OptionalInt.of(int)OptionalInt` — creates a present `OptionalInt`.
pub(crate) fn native_optional_int_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let value = args.first().copied().unwrap_or(Slot::Int(0));
    let r = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    heap.get_mut(r)?.fields[0] = value;
    heap.get_mut(r)?.fields[1] = Slot::Int(1); // present = true
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `OptionalInt.orElse(int)I` — returns value if present, else the default.
pub(crate) fn native_optional_int_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    if present {
        Ok(Some(
            heap.get(r)?.fields.first().copied().unwrap_or(Slot::Int(0)),
        ))
    } else {
        Ok(Some(args.get(1).copied().unwrap_or(Slot::Int(0))))
    }
}

/// Native: `OptionalInt.isPresent()Z`
pub(crate) fn native_optional_int_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}

/// Native: `OptionalInt.getAsInt()I`
pub(crate) fn native_optional_int_get_as_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let v = match heap.get(r)?.fields.first().copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}

/// Native: `Optional.orElseThrow()T` — returns value or throws `NoSuchElementException`.
pub(crate) fn native_optional_or_else_throw(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_optional_get(args, heap, out, control)
}

/// Native: `Optional.orElse(T)T` — returns value if present, else the argument.
pub(crate) fn native_optional_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let result = if matches!(val, Slot::Reference(None)) {
        args.get(1).copied().unwrap_or(Slot::Reference(None))
    } else {
        val
    };
    Ok(Some(result))
}

/// Native: `Optional.isEmpty()Z` — true if no value is present (Java 11+).
pub(crate) fn native_optional_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let empty = matches!(
        heap.get(this_ref)?.fields.first(),
        Some(Slot::Reference(None)) | None
    );
    Ok(Some(Slot::Int(i32::from(empty))))
}

/// Native: `Optional.isPresent()Z` — true if a value is present.
pub(crate) fn native_optional_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let present = !matches!(
        heap.get(this_ref)?.fields.first(),
        Some(Slot::Reference(None)) | None
    );
    Ok(Some(Slot::Int(i32::from(present))))
}

/// Native: `Optional.get()T` — returns value or throws `NoSuchElementException`.
pub(crate) fn native_optional_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    if matches!(val, Slot::Reference(None)) {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(val))
}

/// Native: `Optional.ofNullable(T)Optional` — wraps value or empty if null.
pub(crate) fn native_optional_of_nullable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r)?.fields[0] = val;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Optional.of(T)Optional` — wraps value; throws NPE if null.
pub(crate) fn native_optional_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = args.first().copied().unwrap_or(Slot::Reference(None));
    if matches!(val, Slot::Reference(None)) {
        return Err(VmError::NullPointerException);
    }
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r)?.fields[0] = val;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Optional.empty()Optional` — returns an Optional with no value.
pub(crate) fn native_optional_empty(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(None);
    Ok(Some(Slot::Reference(Some(r))))
}
