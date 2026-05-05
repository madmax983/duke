/// Native: `Arrays.stream(int[])IntStream` — wraps an int array into an `IntStream`.
pub(crate) fn native_arrays_stream_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let arr_obj = heap.get(arr_ref)?;
    let values: Vec<i32> = arr_obj
        .fields
        .iter()
        .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
        .collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}
/// Native: `Arrays.stream(int[], int, int)IntStream` — wraps a subrange as an `IntStream`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_arrays_stream_int_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let from = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let to = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let arr_obj = heap.get(arr_ref)?;
    let values: Vec<i32> = arr_obj
        .fields
        .get(from..to.min(arr_obj.fields.len()))
        .unwrap_or(&[])
        .iter()
        .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
        .collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}
/// Native: `Arrays.stream(Object[])Stream` — wraps a reference array as an eager `Stream`.
pub(crate) fn native_arrays_stream_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let elems: Vec<Slot> = heap.get(arr_ref)?.fields.clone();
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for elem in elems {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
/// Native: `Arrays.sort(Object[])V` — natural order sort using `compareTo`.
#[allow(clippy::too_many_lines)]
pub(crate) fn native_arrays_sort_objects(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let len = heap.get(arr_ref)?.fields.len();
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
/// Native: `Arrays.fill(int[], int)` — fills all elements with val.
pub(crate) fn native_arrays_fill_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
/// Native: `Arrays.fill(Object[], Object)` — fills all elements with val.
pub(crate) fn native_arrays_fill_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let val = extract_slot_arg(args, 1);
    let obj = heap.get_mut(arr_ref)?;
    for slot in &mut obj.fields {
        *slot = val;
    }
    Ok(None)
}
/// Native: `Arrays.copyOf(int[], int)` — copies to new int[] of given length.
pub(crate) fn native_arrays_copyof_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let new_len = match args.get(1) {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => {
            return Err(Error::NegativeArraySize {
                size: *n,
            });
        }
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
/// Native: `Arrays.copyOf(Object[], int)` — copies to new Object[] of given length.
pub(crate) fn native_arrays_copyof_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let new_len = match args.get(1) {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => {
            return Err(Error::NegativeArraySize {
                size: *n,
            });
        }
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
/// Native: `Arrays.sort(int[])` — sorts fields in place.
pub(crate) fn native_arrays_sort_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(arr_ref)?;
    obj.fields
        .sort_by(|a, b| match (a, b) {
            (Slot::Int(x), Slot::Int(y)) => x.cmp(y),
            _ => std::cmp::Ordering::Equal,
        });
    Ok(None)
}
/// Native: `Arrays.equals(int[], int[])boolean` — element-wise equality.
pub(crate) fn native_arrays_equals_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a_ref = extract_ref_arg(args, 0)?;
    let b_ref = extract_ref_arg(args, 1)?;
    let a_len = heap.get(a_ref)?.fields.len();
    let b_len = heap.get(b_ref)?.fields.len();
    if a_len != b_len {
        return Ok(Some(Slot::Int(0)));
    }
    for i in 0..a_len {
        let x = heap.get(a_ref)?.fields[i];
        let y = heap.get(b_ref)?.fields[i];
        match (x, y) {
            (Slot::Int(a), Slot::Int(b)) if a == b => {}
            _ => return Ok(Some(Slot::Int(0))),
        }
    }
    Ok(Some(Slot::Int(1)))
}
/// Native: `Arrays.copyOfRange(int[], int, int)int[]` — slice of int array, zero-padded.
pub(crate) fn native_arrays_copy_of_range_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
        heap.get_mut(dst_ref)?.fields[i] = src_fields
            .get(from + i)
            .copied()
            .unwrap_or(Slot::Int(0));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}
/// Native: `Arrays.copyOfRange(Object[], int, int)Object[]` — slice of reference array.
pub(crate) fn native_arrays_copy_of_range_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
/// Native: `Arrays.asList(Object[])List` — wraps a reference array as an `ArrayList`.
pub(crate) fn native_arrays_as_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let arr_len = heap.get(arr_ref)?.fields.len();
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    for i in 0..arr_len {
        let elem = extract_field_arg(heap, arr_ref, i)?;
        native_arraylist_add(
            &[Slot::Reference(Some(list_ref)), elem],
            heap,
            out,
            control,
        )?;
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}
/// Native: `Arrays.toString(int[])String` — formats as `[1, 2, 3]`.
pub(crate) fn native_arrays_to_string_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    use std::fmt::Write as FmtWrite;
    let arr_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(arr_ref)?.fields.clone();
    let mut result = String::with_capacity(fields.len() * 4 + 2);
    result.push('[');
    for (i, s) in fields.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        match s {
            Slot::Int(n) => {
                let _ = write!(result, "{n}");
            }
            _ => result.push('0'),
        }
    }
    result.push(']');
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Arrays.toString(Object[])String` — formats as `[a, b, c]`.
pub(crate) fn native_arrays_to_string_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    use std::fmt::Write as FmtWrite;
    let arr_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(arr_ref)?.fields.clone();
    let mut result = String::with_capacity(fields.len() * 8 + 2);
    result.push('[');
    for (i, s) in fields.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        match s {
            Slot::Reference(Some(r)) => {
                let part = heap
                    .get(*r)
                    .ok()
                    .and_then(|o| o.string_value.clone())
                    .unwrap_or_else(|| "null".to_string());
                result.push_str(&part);
            }
            Slot::Reference(None) => result.push_str("null"),
            Slot::Int(n) => {
                let _ = write!(result, "{n}");
            }
            _ => result.push('?'),
        }
    }
    result.push(']');
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}
