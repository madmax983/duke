fn optional_string_slot(heap: &mut duke_gc::Heap, value: Option<&str>) -> Slot {
    value.map_or(Slot::Reference(None), |s| string_slot(heap, s))
}
/// Native: `Optional.empty()Optional` — returns an Optional with no value.
pub(crate) fn native_optional_empty(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(None);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Optional.of(T)Optional` — wraps value; throws NPE if null.
pub(crate) fn native_optional_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_slot_arg(args, 0);
    if matches!(val, Slot::Reference(None)) {
        return Err(Error::NullPointerException);
    }
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r)?.fields[0] = val;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Optional.ofNullable(T)Optional` — wraps value or empty if null.
pub(crate) fn native_optional_of_nullable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_slot_arg(args, 0);
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r)?.fields[0] = val;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Optional.get()T` — returns value or throws `NoSuchElementException`.
pub(crate) fn native_optional_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_first_field_arg(heap, this_ref)?;
    if matches!(val, Slot::Reference(None)) {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(val))
}
/// Native: `Optional.isPresent()Z` — true if a value is present.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let present = !matches!(
        heap.get(this_ref) ?.fields.first(), Some(Slot::Reference(None)) | None
    );
    Ok(Some(Slot::Int(i32::from(present))))
}
/// Native: `Optional.isEmpty()Z` — true if no value is present (Java 11+).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let empty = matches!(
        heap.get(this_ref) ?.fields.first(), Some(Slot::Reference(None)) | None
    );
    Ok(Some(Slot::Int(i32::from(empty))))
}
/// Native: `Optional.orElse(T)T` — returns value if present, else the argument.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_first_field_arg(heap, this_ref)?;
    let result = if matches!(val, Slot::Reference(None)) {
        extract_slot_arg(args, 1)
    } else {
        val
    };
    Ok(Some(result))
}
/// Native: `Optional.orElseThrow()T` — returns value or throws `NoSuchElementException`.
pub(crate) fn native_optional_or_else_throw(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_optional_get(args, heap, out, control)
}
/// Native: `OptionalInt.getAsInt()I`
pub(crate) fn native_optional_int_get_as_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let v = match heap.get(r)?.fields.first().copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}
/// Native: `OptionalInt.isPresent()Z`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_int_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r) ?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}
/// Native: `OptionalInt.orElse(int)I` — returns value if present, else the default.
pub(crate) fn native_optional_int_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r) ?.fields.get(1), Some(Slot::Int(1)));
    if present {
        Ok(Some(heap.get(r)?.fields.first().copied().unwrap_or(Slot::Int(0))))
    } else {
        Ok(Some(args.get(1).copied().unwrap_or(Slot::Int(0))))
    }
}
/// Native: `OptionalInt.of(int)OptionalInt` — creates a present `OptionalInt`.
pub(crate) fn native_optional_int_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let value = args.first().copied().unwrap_or(Slot::Int(0));
    let r = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    heap.get_mut(r)?.fields[0] = value;
    heap.get_mut(r)?.fields[1] = Slot::Int(1);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `OptionalInt.empty()OptionalInt` — creates an empty `OptionalInt`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_int_empty(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    heap.get_mut(r)?.fields[1] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `OptionalLong.orElse(long)J`
pub(crate) fn native_optional_long_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r) ?.fields.get(1), Some(Slot::Int(1)));
    if present {
        Ok(Some(heap.get(r)?.fields.first().copied().unwrap_or(Slot::Long(0))))
    } else {
        Ok(Some(args.get(1).copied().unwrap_or(Slot::Long(0))))
    }
}
/// Native: `OptionalDouble.orElse(double)D`
pub(crate) fn native_optional_double_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r) ?.fields.get(1), Some(Slot::Int(1)));
    if present {
        Ok(Some(heap.get(r)?.fields.first().copied().unwrap_or(Slot::Double(0.0))))
    } else {
        Ok(Some(args.get(1).copied().unwrap_or(Slot::Double(0.0))))
    }
}
/// Native: `OptionalDouble.getAsDouble()D`
pub(crate) fn native_optional_double_get_as_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let v = match heap.get(r)?.fields.first().copied() {
        Some(Slot::Double(d)) => d,
        _ => 0.0,
    };
    Ok(Some(Slot::Double(v)))
}
/// Native: `Optional.map(Function)Optional` — maps value if present.
pub(crate) fn native_optional_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
    let result_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if matches!(value, Slot::Reference(None)) {
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    }
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mapped = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, value],
        )?;
    heap.get_mut(result_ref)?.fields[0] = mapped.unwrap_or(Slot::Reference(None));
    Ok(Some(Slot::Reference(Some(result_ref))))
}
/// Native: `Optional.filter(Predicate)Optional` — keeps value only if predicate passes.
pub(crate) fn native_optional_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
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
    let test_result = ops
        .invoke(
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
/// Native: `Optional.flatMap(Function)Optional` — maps value to Optional if present, flattens.
pub(crate) fn native_optional_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
    if matches!(value, Slot::Reference(None)) {
        let empty_ref = heap.allocate("java/util/Optional".to_string(), 1);
        heap.get_mut(empty_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(empty_ref))));
    }
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        let empty_ref = heap.allocate("java/util/Optional".to_string(), 1);
        heap.get_mut(empty_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(empty_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let result = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, value],
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
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
    if let (Slot::Reference(Some(_)), Slot::Reference(Some(consumer_ref))) = (
        value,
        consumer_slot,
    ) {
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
/// Native: `Optional.orElseGet(Supplier)Object` — calls supplier if empty.
pub(crate) fn native_optional_or_else_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let supplier_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
    if !matches!(value, Slot::Reference(None)) {
        return Ok(Some(value));
    }
    let Slot::Reference(Some(supplier_ref)) = supplier_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let supplier_class = heap.get(supplier_ref)?.class_name.clone();
    let result = ops
        .invoke(
            heap,
            out,
            &supplier_class,
            "get",
            "()Ljava/lang/Object;",
            vec![supplier_slot],
        )?;
    Ok(Some(result.unwrap_or(Slot::Reference(None))))
}
fn optional_file_path_from_slot(
    slot: Slot,
    heap: &duke_gc::Heap,
) -> Result<Option<std::path::PathBuf>> {
    match slot {
        Slot::Reference(Some(file_ref)) => Ok(Some(file_path_from_ref(file_ref, heap)?)),
        Slot::Reference(None) => Ok(None),
        _ => Err(Error::NullPointerException),
    }
}
/// Native: `OptionalLong.getAsLong()J`
pub(crate) fn native_optional_long_get_as_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r) ?.fields.get(1), Some(Slot::Int(1)));
    if !present {
        return Err(Error::MethodNotFound {
            name: "OptionalLong.getAsLong on empty".to_string(),
            descriptor: String::new(),
        });
    }
    Ok(Some(heap.get(r)?.fields.first().copied().unwrap_or(Slot::Long(0))))
}
/// Native: `OptionalLong.isPresent()Z`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_long_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r) ?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}
/// Native: `Optional.or(Supplier<Optional>)Optional` (Java 9) —
/// returns this Optional if present; otherwise invokes supplier and returns its result.
pub(crate) fn native_optional_or(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let supplier_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
    if !matches!(value, Slot::Reference(None)) {
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    }
    let Slot::Reference(Some(supplier_ref)) = supplier_slot else {
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    };
    let supplier_class = heap.get(supplier_ref)?.class_name.clone();
    let result = ops
        .invoke(
            heap,
            out,
            &supplier_class,
            "get",
            "()Ljava/lang/Object;",
            vec![supplier_slot],
        )?;
    Ok(Some(result.unwrap_or(Slot::Reference(None))))
}
/// Native: `Optional.ifPresentOrElse(Consumer, Runnable)V` (Java 9) —
/// if value present invokes consumer, otherwise invokes runnable.
pub(crate) fn native_optional_if_present_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let runnable_slot = extract_slot_arg(args, 2);
    let value = extract_first_field_arg(heap, opt_ref)?;
    if matches!(value, Slot::Reference(None)) {
        let Slot::Reference(Some(runnable_ref)) = runnable_slot else {
            return Ok(None);
        };
        let runnable_class = heap.get(runnable_ref)?.class_name.clone();
        ops.invoke(heap, out, &runnable_class, "run", "()V", vec![runnable_slot])?;
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
/// Native: `OptionalDouble.isPresent()Z`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_double_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r) ?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}
/// Native: `Optional.stream()Stream` — returns a stream of 0 or 1 elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = extract_first_field_arg(heap, this_ref)?;
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    match value {
        Slot::Reference(None) => {
            heap.get_mut(out_ref)?.fields[0] = Slot::Int(0);
        }
        v => {
            heap.get_mut(out_ref)?.fields[0] = Slot::Int(1);
            heap.get_mut(out_ref)?.fields.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}
