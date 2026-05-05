/// Native: `Stream.of(Object[])Stream` — create stream from varargs array.
pub(crate) fn native_stream_of(
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
/// Native: `Stream.count()J` — returns the number of elements as a long.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let n = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => i64::from(*n),
        _ => 0,
    };
    Ok(Some(Slot::Long(n)))
}
/// Native: `Stream.filter(Predicate)Stream` — keeps elements where `predicate.test()` returns true.
pub(crate) fn native_stream_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept: Vec<Slot> = Vec::new();
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![Slot::Reference(Some(pred_ref)), elem],
            )?;
        if matches!(result, Some(Slot::Int(1))) {
            kept.push(elem);
        }
    }
    let new_size = i32::try_from(kept.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in kept {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}
/// Native: `Stream.map(Function)Stream` — transforms each element via `function.apply()`.
pub(crate) fn native_stream_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut mapped: Vec<Slot> = Vec::with_capacity(elems.len());
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
                vec![Slot::Reference(Some(fn_ref)), elem],
            )?;
        let boxed = box_primitive_slot(result.unwrap_or(Slot::Reference(None)), heap);
        mapped.push(boxed);
    }
    let new_size = i32::try_from(mapped.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in mapped {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}
/// Native: `Stream.forEach(Consumer)V` — calls `consumer.accept()` on each element.
pub(crate) fn native_stream_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(None);
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for elem in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![Slot::Reference(Some(consumer_ref)), elem],
        )?;
    }
    Ok(None)
}
/// Native: `Stream.collect(Collector)Object` — collects to list (only toList collector supported).
#[allow(
    clippy::too_many_lines,
    clippy::only_used_in_recursion,
    clippy::cognitive_complexity
)]
pub(crate) fn native_stream_collect(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    let elems: Vec<Slot> = heap
        .get(stream_ref)?
        .fields[1..=usize::try_from(size).unwrap_or(0)]
        .to_vec();
    let collector_class = match extract_ref_arg(args, 1) {
        Ok(r) => heap.get(r)?.class_name.clone(),
        Err(_) => "duke/util/ToListCollector".to_string(),
    };
    if collector_class == "duke/util/JoiningCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let read_str_field = |heap: &duke_gc::Heap, idx: usize| -> String {
            match heap.get(collector_ref).ok().and_then(|o| o.fields.get(idx).copied()) {
                Some(Slot::Reference(Some(dr))) => {
                    heap.get(dr)
                        .ok()
                        .and_then(|o| o.string_value.clone())
                        .unwrap_or_default()
                }
                _ => String::new(),
            }
        };
        let delim = read_str_field(heap, 0);
        let prefix = read_str_field(heap, 1);
        let suffix = read_str_field(heap, 2);
        let extra = elems.len().saturating_mul(10usize.saturating_add(delim.len()));
        let cap = prefix
            .len()
            .checked_add(suffix.len())
            .and_then(|x| x.checked_add(extra));
        let max_size = 1024 * 1024 * 128;
        if cap.is_none_or(|c| c > max_size) {
            return Err(Error::JavaException {
                class_name: "java/lang/OutOfMemoryError".to_string(),
            });
        }
        let mut joined = String::with_capacity(cap.unwrap());
        joined.push_str(&prefix);
        let mut first = true;
        for s in &elems {
            #[allow(clippy::collapsible_if)]
            if let Slot::Reference(Some(r)) = s {
                #[allow(clippy::collapsible_if)]
                if let Ok(obj) = heap.get(*r) {
                    if let Some(s_val) = &obj.string_value {
                        if !first {
                            joined.push_str(&delim);
                        }
                        joined.push_str(s_val);
                        first = false;
                    }
                }
            }
        }
        joined.push_str(&suffix);
        let result_ref = heap.allocate_string(joined);
        Ok(Some(Slot::Reference(Some(result_ref))))
    } else if collector_class == "duke/util/CountingCollector" {
        let boxed = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Long(i64::from(size));
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/GroupingByCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let key = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![fn_slot, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let fields = heap.get(map_ref)?.fields.clone();
            let size_n = match fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let mut found_ki = None;
            for i in 0..size_n {
                let ki = 1 + i * 2;
                if fields.get(ki).is_some_and(|k| slots_equal(k, &key, heap)) {
                    found_ki = Some(ki);
                    break;
                }
            }
            if let Some(ki) = found_ki {
                let list_slot = extract_field_arg(heap, map_ref, ki + 1)?;
                if let Slot::Reference(Some(list_ref)) = list_slot {
                    let list_size = match heap.get(list_ref)?.fields.first() {
                        Some(Slot::Int(n)) => *n,
                        _ => 0,
                    };
                    heap.get_mut(list_ref)?.fields.push(elem);
                    heap.get_mut(list_ref)?.fields[0] = Slot::Int(list_size + 1);
                }
            } else {
                let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
                heap.get_mut(list_ref)?.fields[0] = Slot::Int(1);
                heap.get_mut(list_ref)?.fields.push(elem);
                heap.get_mut(map_ref)?.fields.push(key);
                heap.get_mut(map_ref)?.fields.push(Slot::Reference(Some(list_ref)));
                heap.get_mut(map_ref)?.fields[0] = Slot::Int(
                    i32::try_from(size_n + 1).unwrap_or(i32::MAX),
                );
            }
        }
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/ToSetCollector" {
        let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
        heap.get_mut(set_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let set_fields = heap.get(set_ref)?.fields.clone();
            let set_size = match set_fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let already = set_fields[1..=set_size]
                .iter()
                .any(|s| slots_equal(s, &elem, heap));
            if !already {
                let cur_size = match heap.get(set_ref)?.fields.first() {
                    Some(Slot::Int(n)) => *n,
                    _ => 0,
                };
                heap.get_mut(set_ref)?.fields.push(elem);
                heap.get_mut(set_ref)?.fields[0] = Slot::Int(cur_size + 1);
            }
        }
        Ok(Some(Slot::Reference(Some(set_ref))))
    } else if collector_class == "duke/util/ToMapCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let key_fn = extract_first_field_arg(heap, collector_ref)?;
        let val_fn = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(key_ref)) = key_fn else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(val_ref)) = val_fn else {
            return Err(Error::NullPointerException);
        };
        let key_class = heap.get(key_ref)?.class_name.clone();
        let val_class = heap.get(val_ref)?.class_name.clone();
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let k_raw = ops
                .invoke(
                    heap,
                    out,
                    &key_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![key_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let v_raw = ops
                .invoke(
                    heap,
                    out,
                    &val_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![val_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let k = box_primitive_slot(k_raw, heap);
            let v = box_primitive_slot(v_raw, heap);
            let cur_size = match heap.get(map_ref)?.fields.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            heap.get_mut(map_ref)?.fields.push(k);
            heap.get_mut(map_ref)?.fields.push(v);
            heap.get_mut(map_ref)?.fields[0] = Slot::Int(cur_size + 1);
        }
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/ToUnmodifiableMapCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let key_fn = extract_first_field_arg(heap, collector_ref)?;
        let val_fn = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(key_ref)) = key_fn else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(val_ref)) = val_fn else {
            return Err(Error::NullPointerException);
        };
        let key_class = heap.get(key_ref)?.class_name.clone();
        let val_class = heap.get(val_ref)?.class_name.clone();
        let map_ref = heap.allocate("java/util/UnmodifiableMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let k_raw = ops
                .invoke(
                    heap,
                    out,
                    &key_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![key_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let v_raw = ops
                .invoke(
                    heap,
                    out,
                    &val_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![val_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let k = box_primitive_slot(k_raw, heap);
            let v = box_primitive_slot(v_raw, heap);
            let cur_size = match heap.get(map_ref)?.fields.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            heap.get_mut(map_ref)?.fields.push(k);
            heap.get_mut(map_ref)?.fields.push(v);
            heap.get_mut(map_ref)?.fields[0] = Slot::Int(cur_size + 1);
        }
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/ToMapMergeCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let key_fn = extract_first_field_arg(heap, collector_ref)?;
        let val_fn = extract_field_arg(heap, collector_ref, 1)?;
        let merge_fn = extract_field_arg(heap, collector_ref, 2)?;
        let Slot::Reference(Some(key_ref)) = key_fn else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(val_ref)) = val_fn else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(merge_ref)) = merge_fn else {
            return Err(Error::NullPointerException);
        };
        let key_class = heap.get(key_ref)?.class_name.clone();
        let val_class = heap.get(val_ref)?.class_name.clone();
        let merge_class = heap.get(merge_ref)?.class_name.clone();
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let k = ops
                .invoke(
                    heap,
                    out,
                    &key_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![key_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let v = ops
                .invoke(
                    heap,
                    out,
                    &val_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![val_fn, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let fields = heap.get(map_ref)?.fields.clone();
            if let Some(i) = find_hashmap_entry_index(&fields, &k, heap) {
                let existing = fields[i + 1];
                let merged = ops
                    .invoke(
                        heap,
                        out,
                        &merge_class,
                        "apply",
                        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                        vec![merge_fn, existing, v],
                    )?
                    .unwrap_or(Slot::Reference(None));
                heap.get_mut(map_ref)?.fields[i + 1] = merged;
            } else {
                let cur_size = match heap.get(map_ref)?.fields.first() {
                    Some(Slot::Int(n)) => *n,
                    _ => 0,
                };
                heap.get_mut(map_ref)?.fields.push(k);
                heap.get_mut(map_ref)?.fields.push(v);
                heap.get_mut(map_ref)?.fields[0] = Slot::Int(cur_size + 1);
            }
        }
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/PartitioningByCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let pred_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(pred_ref)) = pred_slot else {
            return Err(Error::NullPointerException);
        };
        let pred_class = heap.get(pred_ref)?.class_name.clone();
        let true_list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(true_list)?.fields[0] = Slot::Int(0);
        let false_list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(false_list)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let result = ops
                .invoke(
                    heap,
                    out,
                    &pred_class,
                    "test",
                    "(Ljava/lang/Object;)Z",
                    vec![Slot::Reference(Some(pred_ref)), elem],
                )?;
            let is_true = matches!(result, Some(Slot::Int(n)) if n != 0);
            let target = if is_true { true_list } else { false_list };
            let cur_size = match heap.get(target)?.fields.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            heap.get_mut(target)?.fields.push(elem);
            heap.get_mut(target)?.fields[0] = Slot::Int(cur_size + 1);
        }
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(2);
        let bool_true = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(bool_true)?.fields[0] = Slot::Int(1);
        let bool_false = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(bool_false)?.fields[0] = Slot::Int(0);
        heap.get_mut(map_ref)?.fields.push(Slot::Reference(Some(bool_true)));
        heap.get_mut(map_ref)?.fields.push(Slot::Reference(Some(true_list)));
        heap.get_mut(map_ref)?.fields.push(Slot::Reference(Some(bool_false)));
        heap.get_mut(map_ref)?.fields.push(Slot::Reference(Some(false_list)));
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/PartitioningByDownstreamCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let pred_slot = extract_first_field_arg(heap, collector_ref)?;
        let downstream_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(pred_ref)) = pred_slot else {
            return Err(Error::NullPointerException);
        };
        let pred_class = heap.get(pred_ref)?.class_name.clone();
        let mut true_elems: Vec<Slot> = Vec::new();
        let mut false_elems: Vec<Slot> = Vec::new();
        for elem in elems {
            let result = ops
                .invoke(
                    heap,
                    out,
                    &pred_class,
                    "test",
                    "(Ljava/lang/Object;)Z",
                    vec![Slot::Reference(Some(pred_ref)), elem],
                )?;
            if matches!(result, Some(Slot::Int(n)) if n != 0) {
                true_elems.push(elem);
            } else {
                false_elems.push(elem);
            }
        }
        let apply_downstream = |
            elems_sub: Vec<Slot>,
            heap: &mut duke_gc::Heap,
            downstream: Slot,
        | -> Result<Option<Slot>> {
            let n = elems_sub.len();
            let downstream_class = match downstream {
                Slot::Reference(Some(r)) => heap.get(r)?.class_name.clone(),
                _ => return Ok(Some(Slot::Reference(None))),
            };
            if downstream_class == "duke/util/CountingCollector" {
                let boxed = heap.allocate("java/lang/Long".to_string(), 1);
                #[allow(clippy::cast_possible_wrap)]
                let count_long = Slot::Long(n as i64);
                heap.get_mut(boxed)?.fields[0] = count_long;
                return Ok(Some(Slot::Reference(Some(boxed))));
            }
            let stream_ref = heap.allocate("duke/util/Stream".to_string(), n);
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let n_i32 = n as i32;
            heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n_i32);
            for (i, e) in elems_sub.into_iter().enumerate() {
                heap.get_mut(stream_ref)?.fields[1 + i] = e;
            }
            let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
            heap.get_mut(list_ref)?.fields[0] = Slot::Int(0);
            for i in 1..=n {
                let e = heap.get(stream_ref)?.fields[i];
                let cur = match heap.get(list_ref)?.fields.first() {
                    Some(Slot::Int(x)) => *x,
                    _ => 0,
                };
                heap.get_mut(list_ref)?.fields.push(e);
                heap.get_mut(list_ref)?.fields[0] = Slot::Int(cur + 1);
            }
            Ok(Some(Slot::Reference(Some(list_ref))))
        };
        let true_result = apply_downstream(true_elems, heap, downstream_slot)?;
        let false_result = apply_downstream(false_elems, heap, downstream_slot)?;
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(2);
        let bool_true = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(bool_true)?.fields[0] = Slot::Int(1);
        let bool_false = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(bool_false)?.fields[0] = Slot::Int(0);
        heap.get_mut(map_ref)?.fields.push(Slot::Reference(Some(bool_true)));
        heap.get_mut(map_ref)?.fields.push(true_result.unwrap_or(Slot::Reference(None)));
        heap.get_mut(map_ref)?.fields.push(Slot::Reference(Some(bool_false)));
        heap.get_mut(map_ref)?
            .fields
            .push(false_result.unwrap_or(Slot::Reference(None)));
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/SummingIntCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i32;
        for elem in elems {
            let result = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "applyAsInt",
                    "(Ljava/lang/Object;)I",
                    vec![fn_slot, elem],
                )?;
            if let Some(Slot::Int(n)) = result {
                sum = sum.wrapping_add(n);
            }
        }
        let boxed = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Int(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingIntCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        let mut count = 0_usize;
        for elem in elems {
            let result = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "applyAsInt",
                    "(Ljava/lang/Object;)I",
                    vec![fn_slot, elem],
                )?;
            if let Some(Slot::Int(n)) = result {
                sum += i64::from(n);
                count += 1;
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let avg = if count == 0 { 0.0 } else { sum as f64 / count as f64 };
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/SummarizingIntCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        let mut min = i32::MAX;
        let mut max = i32::MIN;
        let mut count = 0_i64;
        for elem in elems {
            let result = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "applyAsInt",
                    "(Ljava/lang/Object;)I",
                    vec![fn_slot, elem],
                )?;
            if let Some(Slot::Int(n)) = result {
                sum += i64::from(n);
                if n < min {
                    min = n;
                }
                if n > max {
                    max = n;
                }
                count += 1;
            }
        }
        if count == 0 {
            min = 0;
            max = 0;
        }
        let stats = heap.allocate("java/util/IntSummaryStatistics".to_string(), 4);
        heap.get_mut(stats)?.fields[0] = Slot::Long(count);
        heap.get_mut(stats)?.fields[1] = Slot::Long(sum);
        heap.get_mut(stats)?.fields[2] = Slot::Int(min);
        heap.get_mut(stats)?.fields[3] = Slot::Int(max);
        Ok(Some(Slot::Reference(Some(stats))))
    } else if collector_class == "duke/util/SummingLongCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        for elem in elems {
            let result = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "applyAsLong",
                    "(Ljava/lang/Object;)J",
                    vec![fn_slot, elem],
                )?;
            match result {
                Some(Slot::Long(n)) => sum = sum.wrapping_add(n),
                Some(Slot::Int(n)) => sum = sum.wrapping_add(i64::from(n)),
                _ => {}
            }
        }
        let boxed = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Long(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingDoubleCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0.0_f64;
        let mut count = 0_usize;
        for elem in elems {
            let result = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "applyAsDouble",
                    "(Ljava/lang/Object;)D",
                    vec![fn_slot, elem],
                )?;
            match result {
                Some(Slot::Double(d)) => {
                    sum += d;
                    count += 1;
                }
                Some(Slot::Float(f)) => {
                    sum += f64::from(f);
                    count += 1;
                }
                Some(Slot::Int(n)) => {
                    sum += f64::from(n);
                    count += 1;
                }
                _ => {}
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let avg = if count == 0 { 0.0 } else { sum / count as f64 };
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/MappingCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let mapper_slot = extract_first_field_arg(heap, collector_ref)?;
        let downstream_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(mapper_ref)) = mapper_slot else {
            return Err(Error::NullPointerException);
        };
        let mapper_class = heap.get(mapper_ref)?.class_name.clone();
        let mut mapped_elems = Vec::with_capacity(elems.len());
        for elem in elems {
            let mapped = ops
                .invoke(
                    heap,
                    out,
                    &mapper_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![mapper_slot, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            mapped_elems.push(mapped);
        }
        let mapped_size = i32::try_from(mapped_elems.len()).unwrap_or(0);
        let tmp_stream = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(tmp_stream)?.fields[0] = Slot::Int(mapped_size);
        for elem in mapped_elems {
            heap.get_mut(tmp_stream)?.fields.push(elem);
        }
        let tmp_args = vec![Slot::Reference(Some(tmp_stream)), downstream_slot];
        native_stream_collect(&tmp_args, heap, out, control, ops)
    } else if collector_class == "duke/util/GroupingBy2Collector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let downstream_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let raw_map = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(raw_map)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let key_raw = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![fn_slot, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            let key = box_primitive_slot(key_raw, heap);
            let fields = heap.get(raw_map)?.fields.clone();
            let size_n = match fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let mut found_ki = None;
            for i in 0..size_n {
                let ki = 1 + i * 2;
                if fields.get(ki).is_some_and(|k| slots_equal(k, &key, heap)) {
                    found_ki = Some(ki);
                    break;
                }
            }
            if let Some(ki) = found_ki {
                let list_slot = extract_field_arg(heap, raw_map, ki + 1)?;
                if let Slot::Reference(Some(list_ref)) = list_slot {
                    let list_size = match heap.get(list_ref)?.fields.first() {
                        Some(Slot::Int(n)) => *n,
                        _ => 0,
                    };
                    heap.get_mut(list_ref)?.fields.push(elem);
                    heap.get_mut(list_ref)?.fields[0] = Slot::Int(list_size + 1);
                }
            } else {
                let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
                heap.get_mut(list_ref)?.fields[0] = Slot::Int(1);
                heap.get_mut(list_ref)?.fields.push(elem);
                heap.get_mut(raw_map)?.fields.push(key);
                heap.get_mut(raw_map)?.fields.push(Slot::Reference(Some(list_ref)));
                heap.get_mut(raw_map)?.fields[0] = Slot::Int(
                    i32::try_from(size_n + 1).unwrap_or(i32::MAX),
                );
            }
        }
        let result_map = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(result_map)?.fields[0] = Slot::Int(0);
        let raw_fields = heap.get(raw_map)?.fields.clone();
        let group_count = match raw_fields.first() {
            Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
            _ => 0,
        };
        for i in 0..group_count {
            let key = raw_fields
                .get(1 + i * 2)
                .copied()
                .unwrap_or(Slot::Reference(None));
            let list_slot = raw_fields
                .get(2 + i * 2)
                .copied()
                .unwrap_or(Slot::Reference(None));
            let Slot::Reference(Some(list_ref)) = list_slot else {
                continue;
            };
            let group_size_field = heap
                .get(list_ref)?
                .fields
                .first()
                .copied()
                .unwrap_or(Slot::Int(0));
            let group_size = match group_size_field {
                Slot::Int(n) => n,
                _ => 0,
            };
            let tmp_stream = heap.allocate("duke/util/Stream".to_string(), 1);
            heap.get_mut(tmp_stream)?.fields[0] = group_size_field;
            let group_elems: Vec<Slot> = heap
                .get(list_ref)?
                .fields[1..=usize::try_from(group_size).unwrap_or(0)]
                .to_vec();
            for e in group_elems {
                heap.get_mut(tmp_stream)?.fields.push(e);
            }
            let tmp_args = vec![Slot::Reference(Some(tmp_stream)), downstream_slot];
            let collected = native_stream_collect(&tmp_args, heap, out, control, ops)?
                .unwrap_or(Slot::Reference(None));
            let cur_result_size = match heap.get(result_map)?.fields.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            heap.get_mut(result_map)?.fields.push(key);
            heap.get_mut(result_map)?.fields.push(collected);
            heap.get_mut(result_map)?.fields[0] = Slot::Int(cur_result_size + 1);
        }
        Ok(Some(Slot::Reference(Some(result_map))))
    } else if collector_class == "duke/util/MinByCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let cmp_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(cmp_ref)) = cmp_slot else {
            let r = make_optional(heap, None);
            return Ok(Some(Slot::Reference(Some(r))));
        };
        let cmp_class = heap.get(cmp_ref)?.class_name.clone();
        let mut min: Option<Slot> = None;
        for elem in elems {
            let is_less = if let Some(ref cur) = min {
                let result = ops
                    .invoke(
                        heap,
                        out,
                        &cmp_class,
                        "compare",
                        "(Ljava/lang/Object;Ljava/lang/Object;)I",
                        vec![cmp_slot, elem, * cur],
                    )?
                    .unwrap_or(Slot::Int(0));
                matches!(result, Slot::Int(n) if n < 0)
            } else {
                true
            };
            if is_less {
                min = Some(elem);
            }
        }
        let r = make_optional(heap, min);
        Ok(Some(Slot::Reference(Some(r))))
    } else if collector_class == "duke/util/MaxByCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let cmp_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(cmp_ref)) = cmp_slot else {
            let r = make_optional(heap, None);
            return Ok(Some(Slot::Reference(Some(r))));
        };
        let cmp_class = heap.get(cmp_ref)?.class_name.clone();
        let mut max: Option<Slot> = None;
        for elem in elems {
            let is_greater = if let Some(ref cur) = max {
                let result = ops
                    .invoke(
                        heap,
                        out,
                        &cmp_class,
                        "compare",
                        "(Ljava/lang/Object;Ljava/lang/Object;)I",
                        vec![cmp_slot, elem, * cur],
                    )?
                    .unwrap_or(Slot::Int(0));
                matches!(result, Slot::Int(n) if n > 0)
            } else {
                true
            };
            if is_greater {
                max = Some(elem);
            }
        }
        let r = make_optional(heap, max);
        Ok(Some(Slot::Reference(Some(r))))
    } else if collector_class == "duke/util/SummingDoubleCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0.0_f64;
        for elem in elems {
            let result = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "applyAsDouble",
                    "(Ljava/lang/Object;)D",
                    vec![fn_slot, elem],
                )?;
            match result {
                Some(Slot::Double(d)) => sum += d,
                Some(Slot::Float(f)) => sum += f64::from(f),
                Some(Slot::Int(n)) => sum += f64::from(n),
                _ => {}
            }
        }
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingLongCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        let mut count = 0_usize;
        for elem in elems {
            let result = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "applyAsLong",
                    "(Ljava/lang/Object;)J",
                    vec![fn_slot, elem],
                )?;
            match result {
                Some(Slot::Long(n)) => {
                    sum = sum.wrapping_add(n);
                    count += 1;
                }
                Some(Slot::Int(n)) => {
                    sum = sum.wrapping_add(i64::from(n));
                    count += 1;
                }
                _ => {}
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let avg = if count == 0 { 0.0 } else { sum as f64 / count as f64 };
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/ReducingNoIdentityCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let op_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(bop_ref)) = op_slot else {
            return Err(Error::NullPointerException);
        };
        let op_class = heap.get(bop_ref)?.class_name.clone();
        let result = if elems.is_empty() {
            None
        } else {
            let mut acc = elems[0];
            for elem in elems.into_iter().skip(1) {
                acc = ops
                    .invoke(
                        heap,
                        out,
                        &op_class,
                        "apply",
                        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                        vec![op_slot, acc, elem],
                    )?
                    .unwrap_or(Slot::Reference(None));
            }
            Some(acc)
        };
        let opt_ref = make_optional(heap, result);
        Ok(Some(Slot::Reference(Some(opt_ref))))
    } else if collector_class == "duke/util/ReducingCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let identity_slot = extract_first_field_arg(heap, collector_ref)?;
        let op_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(bop_ref)) = op_slot else {
            return Err(Error::NullPointerException);
        };
        let op_class = heap.get(bop_ref)?.class_name.clone();
        let mut acc = identity_slot;
        for elem in elems {
            acc = ops
                .invoke(
                    heap,
                    out,
                    &op_class,
                    "apply",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![op_slot, acc, elem],
                )?
                .unwrap_or(Slot::Reference(None));
        }
        Ok(Some(acc))
    } else if collector_class == "duke/util/ReducingMappingCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let identity_slot = extract_first_field_arg(heap, collector_ref)?;
        let mapper_slot = extract_field_arg(heap, collector_ref, 1)?;
        let op_slot = extract_field_arg(heap, collector_ref, 2)?;
        let Slot::Reference(Some(mapper_ref)) = mapper_slot else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(bop_ref)) = op_slot else {
            return Err(Error::NullPointerException);
        };
        let mapper_class = heap.get(mapper_ref)?.class_name.clone();
        let op_class = heap.get(bop_ref)?.class_name.clone();
        let mut acc = identity_slot;
        for elem in elems {
            let mapped = ops
                .invoke(
                    heap,
                    out,
                    &mapper_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![mapper_slot, elem],
                )?
                .unwrap_or(Slot::Reference(None));
            acc = ops
                .invoke(
                    heap,
                    out,
                    &op_class,
                    "apply",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![op_slot, acc, mapped],
                )?
                .unwrap_or(Slot::Reference(None));
        }
        Ok(Some(acc))
    } else if collector_class == "duke/util/CollectingAndThenCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let downstream_slot = extract_first_field_arg(heap, collector_ref)?;
        let finisher_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(finisher_ref)) = finisher_slot else {
            return Err(Error::NullPointerException);
        };
        let finisher_class = heap.get(finisher_ref)?.class_name.clone();
        let tmp_args = vec![Slot::Reference(Some(stream_ref)), downstream_slot];
        let intermediate = native_stream_collect(&tmp_args, heap, out, control, ops)?
            .unwrap_or(Slot::Reference(None));
        let result = ops
            .invoke(
                heap,
                out,
                &finisher_class,
                "apply",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
                vec![finisher_slot, intermediate],
            )?;
        Ok(result.or(Some(Slot::Reference(None))))
    } else if collector_class == "duke/util/ToUnmodifiableListCollector" {
        let list_ref = heap.allocate("java/util/UnmodifiableList".to_string(), 1);
        heap.get_mut(list_ref)?.fields[0] = Slot::Int(size);
        for elem in elems {
            heap.get_mut(list_ref)?.fields.push(elem);
        }
        Ok(Some(Slot::Reference(Some(list_ref))))
    } else {
        let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(list_ref)?.fields[0] = Slot::Int(size);
        for elem in elems {
            heap.get_mut(list_ref)?.fields.push(elem);
        }
        Ok(Some(Slot::Reference(Some(list_ref))))
    }
}
/// Native: `Stream.distinct()Stream` — removes duplicate elements (by `slots_equal`).
pub(crate) fn native_stream_distinct(
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
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut seen: Vec<Slot> = Vec::new();
    for elem in elems {
        if !seen.iter().any(|s| slots_equal(s, &elem, heap)) {
            seen.push(elem);
        }
    }
    let new_size = i32::try_from(seen.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in seen {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}
/// Native: `Stream.sorted()Stream` — sorts elements by natural order via `compareTo`.
pub(crate) fn native_stream_sorted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    for i in 1..elems.len() {
        let mut j = i;
        while j > 0 {
            let Slot::Reference(Some(a)) = elems[j - 1] else {
                break;
            };
            let Slot::Reference(Some(b)) = elems[j] else {
                break;
            };
            let class_a = heap.get(a)?.class_name.clone();
            let cmp = ops
                .invoke(
                    heap,
                    out,
                    &class_a,
                    "compareTo",
                    "(Ljava/lang/Object;)I",
                    vec![Slot::Reference(Some(a)), Slot::Reference(Some(b))],
                )?;
            if matches!(cmp, Some(Slot::Int(n)) if n > 0) {
                elems.swap(j - 1, j);
                j -= 1;
            } else {
                break;
            }
        }
    }
    let new_size = i32::try_from(elems.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in elems {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}
/// Native: `Stream.anyMatch(Predicate)Z` — true if any element satisfies predicate.
pub(crate) fn native_stream_any_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![Slot::Reference(Some(pred_ref)), elem],
            )?;
        if matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}
/// Native: `Stream.allMatch(Predicate)Z` — true if all elements satisfy predicate.
pub(crate) fn native_stream_all_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![Slot::Reference(Some(pred_ref)), elem],
            )?;
        if !matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}
/// Native: `Stream.noneMatch(Predicate)Z` — true if no element satisfies predicate.
pub(crate) fn native_stream_none_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![Slot::Reference(Some(pred_ref)), elem],
            )?;
        if matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}
/// Native: `Stream.findFirst()Optional` — returns Optional of first element, or empty.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_find_first(
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
    let opt_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if size > 0 {
        let first = heap.get(stream_ref)?.fields[1];
        heap.get_mut(opt_ref)?.fields[0] = first;
    } else {
        heap.get_mut(opt_ref)?.fields[0] = Slot::Reference(None);
    }
    Ok(Some(Slot::Reference(Some(opt_ref))))
}
/// Native: `Stream.reduce(BinaryOperator)Optional` — folds elements left via binary op.
pub(crate) fn native_stream_reduce(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let op_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(op_ref)) = op_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let op_class = heap.get(op_ref)?.class_name.clone();
    let reduce_result_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if elems.is_empty() {
        heap.get_mut(reduce_result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(reduce_result_ref))));
    }
    let mut acc = elems[0];
    for elem in elems.into_iter().skip(1) {
        let result = ops
            .invoke(
                heap,
                out,
                &op_class,
                "apply",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                vec![Slot::Reference(Some(op_ref)), acc, elem],
            )?;
        acc = result.unwrap_or(Slot::Reference(None));
    }
    let acc_boxed = box_primitive_slot(acc, heap);
    heap.get_mut(reduce_result_ref)?.fields[0] = acc_boxed;
    Ok(Some(Slot::Reference(Some(reduce_result_ref))))
}
/// Native: `Stream.reduce(identity, BinaryOperator)Object` — fold with initial value.
pub(crate) fn native_stream_reduce_with_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let identity = extract_slot_arg(args, 1);
    let fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(identity));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut acc = identity;
    for elem in elems {
        acc = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                vec![fn_slot, acc, elem],
            )?
            .unwrap_or(Slot::Reference(None));
    }
    let acc = match (identity, acc) {
        (Slot::Reference(_), Slot::Int(v)) => {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Int(v);
            Slot::Reference(Some(r))
        }
        (Slot::Reference(_), Slot::Long(v)) => {
            let r = heap.allocate("java/lang/Long".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Long(v);
            Slot::Reference(Some(r))
        }
        (Slot::Reference(_), Slot::Double(v)) => {
            let r = heap.allocate("java/lang/Double".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Double(v);
            Slot::Reference(Some(r))
        }
        _ => acc,
    };
    Ok(Some(acc))
}
/// Native: `Stream.toList()List` — terminal op returning an unmodifiable list (same as collect).
pub(crate) fn native_stream_to_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_stream_collect(args, heap, out, control, ops)
}
/// Native: `Stream.peek(Consumer)Stream` — side-effect each element, returns same stream.
pub(crate) fn native_stream_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(Some(Slot::Reference(Some(stream_ref))));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for elem in &elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, * elem],
        )?;
    }
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(
        i32::try_from(elems.len()).unwrap_or(0),
    );
    for elem in elems {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}
/// Native: `Stream.toArray()Object[]` — materializes stream into an Object array.
pub(crate) fn native_stream_to_array(
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
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 0);
    for elem in elems {
        heap.get_mut(arr_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}
/// Native: `Stream.limit(long)Stream` — keeps first N elements.
pub(crate) fn native_stream_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let max_size = match args.get(1).copied() {
        Some(Slot::Long(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        _ => 0,
    };
    let class_name = heap.get(stream_ref)?.class_name.clone();
    if class_name == "duke/util/GeneratorStream" {
        let supplier_slot = extract_first_field_arg(heap, stream_ref)?;
        let Slot::Reference(Some(sup_ref)) = supplier_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let sup_class = heap.get(sup_ref)?.class_name.clone();
        let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(out_ref)?.fields[0] = Slot::Int(
            i32::try_from(max_size).unwrap_or(0),
        );
        for _ in 0..max_size {
            let elem = ops
                .invoke(
                    heap,
                    out,
                    &sup_class,
                    "get",
                    "()Ljava/lang/Object;",
                    vec![Slot::Reference(Some(sup_ref))],
                )?
                .unwrap_or(Slot::Reference(None));
            heap.get_mut(out_ref)?.fields.push(elem);
        }
        return Ok(Some(Slot::Reference(Some(out_ref))));
    }
    if class_name == "duke/util/IteratorStream" {
        let seed = extract_first_field_arg(heap, stream_ref)?;
        let fn_slot = extract_field_arg(heap, stream_ref, 1)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(out_ref)?.fields[0] = Slot::Int(
            i32::try_from(max_size).unwrap_or(0),
        );
        let mut current = seed;
        for _ in 0..max_size {
            heap.get_mut(out_ref)?.fields.push(current);
            current = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![fn_slot, current],
                )?
                .unwrap_or(Slot::Reference(None));
        }
        return Ok(Some(Slot::Reference(Some(out_ref))));
    }
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let take = size.min(max_size);
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(take).unwrap_or(0));
    for elem in elems.into_iter().take(take) {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}
/// Native: `Stream.skip(long)Stream` — skips first N elements.
pub(crate) fn native_stream_skip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let skip_n = match args.get(1).copied() {
        Some(Slot::Long(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        _ => 0,
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let skipped: Vec<Slot> = elems.into_iter().skip(skip_n).collect();
    let new_size = i32::try_from(skipped.len()).unwrap_or(0);
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(new_size);
    for elem in skipped {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}
/// Native: `Stream.flatMap(Function)Stream` — maps each element to a Stream and flattens.
pub(crate) fn native_stream_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut flat: Vec<Slot> = Vec::with_capacity(elems.len());
    for elem in elems {
        let inner = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
                vec![fn_slot, elem],
            )?;
        if let Some(Slot::Reference(Some(inner_ref))) = inner {
            let inner_size = match heap.get(inner_ref)?.fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let inner_elems: Vec<Slot> = heap
                .get(inner_ref)?
                .fields[1..=inner_size]
                .to_vec();
            flat.extend(inner_elems);
        }
    }
    let new_size = i32::try_from(flat.len()).unwrap_or(0);
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(new_size);
    for elem in flat {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}
/// Native: `Stream.mapToInt(ToIntFunction)IntStream` — maps each element via `applyAsInt`.
pub(crate) fn native_stream_map_to_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut values = Vec::with_capacity(elems.len());
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsInt",
                "(Ljava/lang/Object;)I",
                vec![fn_slot, elem],
            )?
            .unwrap_or(Slot::Int(0));
        match result {
            Slot::Int(n) => values.push(n),
            Slot::Reference(Some(r)) => {
                let n = match heap.get(r)?.fields.first() {
                    Some(Slot::Int(v)) => *v,
                    _ => 0,
                };
                values.push(n);
            }
            _ => values.push(0),
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}
/// Native: `Stream.min(Comparator)Optional` — returns minimum element by comparator.
pub(crate) fn native_stream_min_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    stream_min_max_by_comparator(args, heap, out, ops, false)
}
/// Native: `Stream.max(Comparator)Optional` — returns maximum element by comparator.
pub(crate) fn native_stream_max_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    stream_min_max_by_comparator(args, heap, out, ops, true)
}
fn stream_min_max_by_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    want_max: bool,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let cmp_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(cmp_ref)) = cmp_slot else {
        let r = heap.allocate("java/util/Optional".to_string(), 1);
        heap.get_mut(r)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(r))));
    };
    let cmp_class = heap.get(cmp_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let opt_r = heap.allocate("java/util/Optional".to_string(), 1);
    if elems.is_empty() {
        heap.get_mut(opt_r)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(opt_r))));
    }
    let mut best = elems[0];
    for elem in elems.into_iter().skip(1) {
        let cmp_result = ops
            .invoke(
                heap,
                out,
                &cmp_class,
                "compare",
                "(Ljava/lang/Object;Ljava/lang/Object;)I",
                vec![cmp_slot, elem, best],
            )?
            .unwrap_or(Slot::Int(0));
        let cmp_val = match cmp_result {
            Slot::Int(n) => n,
            _ => 0,
        };
        if (want_max && cmp_val > 0) || (!want_max && cmp_val < 0) {
            best = elem;
        }
    }
    heap.get_mut(opt_r)?.fields[0] = best;
    Ok(Some(Slot::Reference(Some(opt_r))))
}
/// Native: `Stream.generate(Supplier)Stream` — returns a `duke/util/GeneratorStream` sentinel.
/// Materialised into a real Stream when `.limit(N)` is called.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_generate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let supplier = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/GeneratorStream".to_string(), 1);
    heap.get_mut(r)?.fields[0] = supplier;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Stream.iterate(seed, UnaryOperator)Stream` — returns a `duke/util/IteratorStream`.
/// Materialised into a real Stream when `.limit(N)` is called.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_iterate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let seed = extract_slot_arg(args, 0);
    let fn_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/IteratorStream".to_string(), 2);
    heap.get_mut(r)?.fields[0] = seed;
    heap.get_mut(r)?.fields[1] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Stream.concat(Stream, Stream)Stream` — concatenates two eager streams.
pub(crate) fn native_stream_concat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a_ref = extract_ref_arg(args, 0)?;
    let b_ref = extract_ref_arg(args, 1)?;
    let a_size = match heap.get(a_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let b_size = match heap.get(b_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let a_elems: Vec<Slot> = heap.get(a_ref)?.fields[1..=a_size].to_vec();
    let b_elems: Vec<Slot> = heap.get(b_ref)?.fields[1..=b_size].to_vec();
    let total = a_size + b_size;
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(total).unwrap_or(0));
    for elem in a_elems.into_iter().chain(b_elems) {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}
/// Native: `Stream.empty()Stream` — returns a zero-element stream.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_empty(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Stream.takeWhile(Predicate)Stream` — keeps prefix while predicate holds.
pub(crate) fn native_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1) else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept: Vec<Slot> = Vec::new();
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![Slot::Reference(Some(pred_ref)), elem],
            )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(elem);
        } else {
            break;
        }
    }
    let new_size = i32::try_from(kept.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in kept {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}
/// Native: `Stream.dropWhile(Predicate)Stream` — drops prefix while predicate holds, keeps rest.
pub(crate) fn native_stream_drop_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1) else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut dropping = true;
    let mut kept: Vec<Slot> = Vec::new();
    for elem in elems {
        if dropping {
            let result = ops
                .invoke(
                    heap,
                    out,
                    &pred_class,
                    "test",
                    "(Ljava/lang/Object;)Z",
                    vec![Slot::Reference(Some(pred_ref)), elem],
                )?;
            if matches!(result, Some(Slot::Int(n)) if n != 0) {
                continue;
            }
            dropping = false;
        }
        kept.push(elem);
    }
    let new_size = i32::try_from(kept.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in kept {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}
/// Native: `Stream.sorted(Comparator)Stream` — sorts stream elements using the given comparator.
pub(crate) fn native_stream_sorted_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let Slot::Reference(Some(comp_ref)) = extract_slot_arg(args, 1) else {
        return native_stream_sorted(args, heap, out, control, ops);
    };
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    for i in 1..elems.len() {
        let mut j = i;
        while j > 0 {
            let comp_class = heap.get(comp_ref)?.class_name.clone();
            let cmp = ops
                .invoke(
                    heap,
                    out,
                    &comp_class,
                    "compare",
                    "(Ljava/lang/Object;Ljava/lang/Object;)I",
                    vec![Slot::Reference(Some(comp_ref)), elems[j - 1], elems[j]],
                )?;
            if matches!(cmp, Some(Slot::Int(n)) if n > 0) {
                elems.swap(j - 1, j);
                j -= 1;
            } else {
                break;
            }
        }
    }
    let new_size = i32::try_from(elems.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in elems {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}
/// Native: `Stream.mapToLong(ToLongFunction)LongStream` — maps each element via `applyAsLong`.
pub(crate) fn native_stream_map_to_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut values = Vec::with_capacity(elems.len());
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(Ljava/lang/Object;)J",
                vec![fn_slot, elem],
            )?
            .unwrap_or(Slot::Long(0));
        let v = match result {
            Slot::Long(n) => n,
            Slot::Int(n) => i64::from(n),
            _ => 0,
        };
        values.push(v);
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}
/// Native: `Stream.mapToDouble(ToDoubleFunction)DoubleStream` — maps each element via `applyAsDouble`.
pub(crate) fn native_stream_map_to_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(heap, vec![])))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = stream_elements(heap, stream_ref)?;
    let mut values = Vec::with_capacity(elems.len());
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(Ljava/lang/Object;)D",
                vec![fn_slot, elem],
            )?
            .unwrap_or(Slot::Double(0.0));
        let v = match result {
            Slot::Double(d) => d,
            Slot::Float(f) => f64::from(f),
            Slot::Int(n) => f64::from(n),
            _ => 0.0,
        };
        values.push(v);
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, values)))))
}
/// Extracts stream payload slots from `fields[1..=size]`, safely handling empty streams and
/// malformed `size` headers.
fn stream_elements(heap: &duke_gc::Heap, stream_ref: u64) -> Result<Vec<Slot>> {
    let obj = heap.get(stream_ref)?;
    let declared_size = match obj.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let actual_size = declared_size.min(obj.fields.len().saturating_sub(1));
    if actual_size == 0 {
        return Ok(Vec::new());
    }
    Ok(obj.fields[1..=actual_size].to_vec())
}
/// Native: `Stream.flatMapToInt(Function<T,IntStream>)IntStream`
pub(crate) fn native_stream_flat_map_to_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result: Vec<i32> = Vec::new();
    for elem in elems {
        let sub = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
                vec![fn_slot, elem],
            )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(int_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}
/// Native: `Stream.flatMapToLong(Function<T,LongStream>)LongStream`
pub(crate) fn native_stream_flat_map_to_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result: Vec<i64> = Vec::new();
    for elem in elems {
        let sub = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
                vec![fn_slot, elem],
            )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(long_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}
/// Native: `Stream.flatMapToDouble(Function<T,DoubleStream>)DoubleStream`
pub(crate) fn native_stream_flat_map_to_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(heap, vec![])))));
    };
    let elems = stream_elements(heap, stream_ref)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result: Vec<f64> = Vec::new();
    for elem in elems {
        let sub = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
                vec![fn_slot, elem],
            )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(double_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, result)))))
}
/// Native: `Stream.iterate(seed, Predicate, UnaryOperator)Stream` — Java 9 3-arg form.
/// Eagerly materialises elements while predicate returns true, capped at 10,000.
pub(crate) fn native_stream_iterate_predicate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let seed = extract_slot_arg(args, 0);
    let pred_slot = extract_slot_arg(args, 1);
    let next_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Err(Error::NullPointerException);
    };
    let Slot::Reference(Some(next_ref)) = next_slot else {
        return Err(Error::NullPointerException);
    };
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let next_class = heap.get(next_ref)?.class_name.clone();
    let mut elems: Vec<Slot> = Vec::new();
    let mut current = seed;
    for _ in 0..10_000usize {
        let test = ops
            .invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![pred_slot, current],
            )?;
        match test {
            Some(Slot::Int(1)) => {}
            _ => break,
        }
        elems.push(current);
        current = ops
            .invoke(
                heap,
                out,
                &next_class,
                "apply",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
                vec![next_slot, current],
            )?
            .unwrap_or(Slot::Reference(None));
    }
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    let size = i32::try_from(elems.len()).unwrap_or(i32::MAX);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(size);
    for elem in elems {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}
