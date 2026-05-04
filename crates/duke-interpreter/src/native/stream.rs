// ---- Stream natives ----
// duke/util/Stream: fields[0]=Int(size), fields[1..]=element refs

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

/// Native: `ArrayList.stream()` — wrap `ArrayList` elements into a `Stream`.
pub(crate) fn native_arraylist_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    let elems: Vec<Slot> =
        heap.get(list_ref)?.fields[1..=usize::try_from(size).unwrap_or(0)].to_vec();
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(size);
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
        let result = ops.invoke(
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
        let result = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![Slot::Reference(Some(fn_ref)), elem],
        )?;
        // Box primitive results so stream elements are always References (Java type-erasure)
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
    let elems: Vec<Slot> =
        heap.get(stream_ref)?.fields[1..=usize::try_from(size).unwrap_or(0)].to_vec();

    // Dispatch on collector type.
    let collector_class = match extract_ref_arg(args, 1) {
        Ok(r) => heap.get(r)?.class_name.clone(),
        Err(_) => "duke/util/ToListCollector".to_string(),
    };

    if collector_class == "duke/util/JoiningCollector" {
        let collector_ref = extract_ref_arg(args, 1)?;
        let read_str_field = |heap: &duke_gc::Heap, idx: usize| -> String {
            match heap
                .get(collector_ref)
                .ok()
                .and_then(|o| o.fields.get(idx).copied())
            {
                Some(Slot::Reference(Some(dr))) => heap
                    .get(dr)
                    .ok()
                    .and_then(|o| o.string_value.clone())
                    .unwrap_or_default(),
                _ => String::new(),
            }
        };
        let delim = read_str_field(heap, 0);
        let prefix = read_str_field(heap, 1);
        let suffix = read_str_field(heap, 2);

        // ⚡ Bolt: Eliminate intermediate Vec<String> allocation, format! macro overhead,
        // and .join() by appending directly to a single String buffer.
        // Havoc: prevent OOM from capacity overflow
        let extra = elems.len().saturating_mul(10usize.saturating_add(delim.len()));
        let cap = prefix.len().checked_add(suffix.len()).and_then(|x| x.checked_add(extra));
        let max_size = 1024 * 1024 * 128; // 128 MB max string size

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
        // collect() returns Object; box the Long so bytecode can checkcast/invokevirtual it.
        let boxed = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Long(i64::from(size));
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/GroupingByCollector" {
        // GroupingByCollector: fields[0] = key function slot
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // Build a HashMap: key → ArrayList of values
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
            // find existing bucket or create new list
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
                // Append elem to existing list
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
                // New key — create list with one element
                let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
                heap.get_mut(list_ref)?.fields[0] = Slot::Int(1);
                heap.get_mut(list_ref)?.fields.push(elem);
                heap.get_mut(map_ref)?.fields.push(key);
                heap.get_mut(map_ref)?
                    .fields
                    .push(Slot::Reference(Some(list_ref)));
                heap.get_mut(map_ref)?.fields[0] =
                    Slot::Int(i32::try_from(size_n + 1).unwrap_or(i32::MAX));
            }
        }
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/ToSetCollector" {
        // Collect into HashSet (deduplicates).
        let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
        heap.get_mut(set_ref)?.fields[0] = Slot::Int(0);
        for elem in elems {
            // Check for duplicate before inserting
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
        // Collect into HashMap using key/val extractor functions.
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
            // Box primitives so the map stores References (Object contract).
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
        // Same as ToMapCollector but produces an UnmodifiableMap.
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
        // Collect into HashMap with merge function for duplicate keys.
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
                // Duplicate key — apply merge function: merge(existing, new)
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
        // Collect into a Map<Boolean, List> partitioned by predicate.
        let collector_ref = extract_ref_arg(args, 1)?;
        let pred_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(pred_ref)) = pred_slot else {
            return Err(Error::NullPointerException);
        };
        let pred_class = heap.get(pred_ref)?.class_name.clone();
        // Create two lists and the result map.
        let true_list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(true_list)?.fields[0] = Slot::Int(0);
        let false_list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(false_list)?.fields[0] = Slot::Int(0);
        for elem in elems {
            let result = ops.invoke(
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
        // Build HashMap: Boolean(1)→trueList, Boolean(0)→falseList
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(2);
        let bool_true = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(bool_true)?.fields[0] = Slot::Int(1);
        let bool_false = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(bool_false)?.fields[0] = Slot::Int(0);
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(bool_true)));
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(true_list)));
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(bool_false)));
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(false_list)));
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/PartitioningByDownstreamCollector" {
        // partitioningBy(pred, downstream): partition then apply downstream to each group.
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
            let result = ops.invoke(
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
        // Apply downstream collector to each partition by building a mini stream.
        let apply_downstream = |elems_sub: Vec<Slot>,
                                heap: &mut duke_gc::Heap,
                                downstream: Slot|
         -> Result<Option<Slot>> {
            let n = elems_sub.len();
            let downstream_class = match downstream {
                Slot::Reference(Some(r)) => heap.get(r)?.class_name.clone(),
                _ => return Ok(Some(Slot::Reference(None))),
            };
            if downstream_class == "duke/util/CountingCollector" {
                let boxed = heap.allocate("java/lang/Long".to_string(), 1);
                #[allow(clippy::cast_possible_wrap)] // n is a collection count; fits i64
                let count_long = Slot::Long(n as i64);
                heap.get_mut(boxed)?.fields[0] = count_long;
                return Ok(Some(Slot::Reference(Some(boxed))));
            }
            // Generic: build a mini stream and collect into a list.
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
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(bool_true)));
        heap.get_mut(map_ref)?
            .fields
            .push(true_result.unwrap_or(Slot::Reference(None)));
        heap.get_mut(map_ref)?
            .fields
            .push(Slot::Reference(Some(bool_false)));
        heap.get_mut(map_ref)?
            .fields
            .push(false_result.unwrap_or(Slot::Reference(None)));
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/SummingIntCollector" {
        // Sum via applyAsInt(elem) for each element.
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i32;
        for elem in elems {
            let result = ops.invoke(
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
        // Return boxed Integer
        let boxed = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Int(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingIntCollector" {
        // Average via applyAsInt(elem) for each element.
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        let mut count = 0_usize;
        for elem in elems {
            let result = ops.invoke(
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
        let avg = if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        };
        // Return boxed Double
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/SummarizingIntCollector" {
        // IntSummaryStatistics via applyAsInt(elem) for each element.
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
            let result = ops.invoke(
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
        // fields: [0]=count(J) [1]=sum(J) [2]=min(I) [3]=max(I)
        let stats = heap.allocate("java/util/IntSummaryStatistics".to_string(), 4);
        heap.get_mut(stats)?.fields[0] = Slot::Long(count);
        heap.get_mut(stats)?.fields[1] = Slot::Long(sum);
        heap.get_mut(stats)?.fields[2] = Slot::Int(min);
        heap.get_mut(stats)?.fields[3] = Slot::Int(max);
        Ok(Some(Slot::Reference(Some(stats))))
    } else if collector_class == "duke/util/SummingLongCollector" {
        // Sum via applyAsLong(elem) for each element.
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        for elem in elems {
            let result = ops.invoke(
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
        // Return boxed Long
        let boxed = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Long(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingDoubleCollector" {
        // Average via applyAsDouble(elem) for each element.
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0.0_f64;
        let mut count = 0_usize;
        for elem in elems {
            let result = ops.invoke(
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
        // Return boxed Double
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/MappingCollector" {
        // MappingCollector: fields[0]=mapper fn, fields[1]=downstream collector
        let collector_ref = extract_ref_arg(args, 1)?;
        let mapper_slot = extract_first_field_arg(heap, collector_ref)?;
        let downstream_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(mapper_ref)) = mapper_slot else {
            return Err(Error::NullPointerException);
        };
        let mapper_class = heap.get(mapper_ref)?.class_name.clone();
        // Map each element through the mapper function
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
        // Build a temporary stream from mapped elements and collect with downstream
        let mapped_size = i32::try_from(mapped_elems.len()).unwrap_or(0);
        let tmp_stream = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(tmp_stream)?.fields[0] = Slot::Int(mapped_size);
        for elem in mapped_elems {
            heap.get_mut(tmp_stream)?.fields.push(elem);
        }
        let tmp_args = vec![Slot::Reference(Some(tmp_stream)), downstream_slot];
        native_stream_collect(&tmp_args, heap, out, control, ops)
    } else if collector_class == "duke/util/GroupingBy2Collector" {
        // groupingBy(keyFn, downstream): fields[0]=keyFn, fields[1]=downstream collector
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let downstream_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // First pass: group raw elements by key into HashMap<key, ArrayList<elem>>
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
            // Box primitive keys so HashMap.get(boxed) can match them via slots_equal
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
                heap.get_mut(raw_map)?
                    .fields
                    .push(Slot::Reference(Some(list_ref)));
                heap.get_mut(raw_map)?.fields[0] =
                    Slot::Int(i32::try_from(size_n + 1).unwrap_or(i32::MAX));
            }
        }
        // Second pass: apply downstream collector to each group's ArrayList
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
            let group_elems: Vec<Slot> =
                heap.get(list_ref)?.fields[1..=usize::try_from(group_size).unwrap_or(0)].to_vec();
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
        // minBy(comparator): fields[0] = comparator
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
                        vec![cmp_slot, elem, *cur],
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
        // maxBy(comparator): fields[0] = comparator
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
                        vec![cmp_slot, elem, *cur],
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
        // summingDouble: fields[0] = ToDoubleFunction
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0.0_f64;
        for elem in elems {
            let result = ops.invoke(
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
        // averagingLong: fields[0] = ToLongFunction
        let collector_ref = extract_ref_arg(args, 1)?;
        let fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut sum = 0_i64;
        let mut count = 0_usize;
        for elem in elems {
            let result = ops.invoke(
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
        let avg = if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        };
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/ReducingNoIdentityCollector" {
        // reducing(BinaryOperator) → Optional<T>
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
        // reducing(identity, BinaryOperator) → T
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
        // reducing(identity, mapper, BinaryOperator) → U
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
        // collectingAndThen(downstream, finisher): fields[0]=downstream, fields[1]=finisher
        let collector_ref = extract_ref_arg(args, 1)?;
        let downstream_slot = extract_first_field_arg(heap, collector_ref)?;
        let finisher_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(finisher_ref)) = finisher_slot else {
            return Err(Error::NullPointerException);
        };
        let finisher_class = heap.get(finisher_ref)?.class_name.clone();
        // First collect with downstream
        let tmp_args = vec![Slot::Reference(Some(stream_ref)), downstream_slot];
        let intermediate = native_stream_collect(&tmp_args, heap, out, control, ops)?
            .unwrap_or(Slot::Reference(None));
        // Then apply finisher
        let result = ops.invoke(
            heap,
            out,
            &finisher_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![finisher_slot, intermediate],
        )?;
        Ok(result.or(Some(Slot::Reference(None))))
    } else if collector_class == "duke/util/ToUnmodifiableListCollector" {
        // toUnmodifiableList(): collect into UnmodifiableList (mutations throw).
        let list_ref = heap.allocate("java/util/UnmodifiableList".to_string(), 1);
        heap.get_mut(list_ref)?.fields[0] = Slot::Int(size);
        for elem in elems {
            heap.get_mut(list_ref)?.fields.push(elem);
        }
        Ok(Some(Slot::Reference(Some(list_ref))))
    } else {
        // ToListCollector (default): collect into ArrayList.
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
    // Insertion sort via compareTo callbacks (stable, O(n²) — fine for test sizes).
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
            let cmp = ops.invoke(
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
        let result = ops.invoke(
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
        let result = ops.invoke(
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
        let result = ops.invoke(
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
        let result = ops.invoke(
            heap,
            out,
            &op_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![Slot::Reference(Some(op_ref)), acc, elem],
        )?;
        acc = result.unwrap_or(Slot::Reference(None));
    }
    // Box primitive accumulator before storing in Optional (Java generics always hold References)
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
    // Autobox: if the identity was a Reference (stream of boxed type) but the accumulator
    // impl returned a raw primitive (e.g. Integer::sum returns int), re-box the result so
    // that the caller can apply intValue() / longValue() as expected.
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

/// Helper: allocate a `JoiningCollector` with 3 fields: delimiter, prefix, suffix.
fn make_joining_collector(
    heap: &mut duke_gc::Heap,
    delimiter: &str,
    prefix: &str,
    suffix: &str,
) -> u64 {
    let collector_ref = heap.allocate("duke/util/JoiningCollector".to_string(), 3);
    let delim_ref = heap.allocate_string(delimiter.to_string());
    let prefix_ref = heap.allocate_string(prefix.to_string());
    let suffix_ref = heap.allocate_string(suffix.to_string());
    let obj = heap.get_mut(collector_ref).expect("just allocated");
    obj.fields[0] = Slot::Reference(Some(delim_ref));
    obj.fields[1] = Slot::Reference(Some(prefix_ref));
    obj.fields[2] = Slot::Reference(Some(suffix_ref));
    collector_ref
}

/// Native: `Collectors.joining(delim)Collector` — returns a joining collector with delimiter.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_joining(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let delim = match args.first() {
        Some(Slot::Reference(Some(r))) => heap
            .get(*r)
            .ok()
            .and_then(|o| o.string_value.clone())
            .unwrap_or_default(),
        _ => String::new(),
    };
    let collector_ref = make_joining_collector(heap, &delim, "", "");
    Ok(Some(Slot::Reference(Some(collector_ref))))
}

/// Native: `Collectors.joining()Collector` — no-arg version (empty delimiter).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_joining_no_arg(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let collector_ref = make_joining_collector(heap, "", "", "");
    Ok(Some(Slot::Reference(Some(collector_ref))))
}

/// Native: `Collectors.joining(delim, prefix, suffix)Collector` — full 3-arg joining collector.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_joining_full(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let read_str = |heap: &duke_gc::Heap, idx: usize| -> String {
        match args.get(idx) {
            Some(Slot::Reference(Some(r))) => heap
                .get(*r)
                .ok()
                .and_then(|o| o.string_value.clone())
                .unwrap_or_default(),
            _ => String::new(),
        }
    };
    let delim = read_str(heap, 0);
    let prefix = read_str(heap, 1);
    let suffix = read_str(heap, 2);
    let collector_ref = make_joining_collector(heap, &delim, &prefix, &suffix);
    Ok(Some(Slot::Reference(Some(collector_ref))))
}

/// Native: `Collectors.toList()Collector` — returns a sentinel collector object.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let collector_ref = heap.allocate("duke/util/ToListCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(collector_ref))))
}

/// Native: `Collectors.counting()Collector` — returns a counting collector sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_counting(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/CountingCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.groupingBy(Function)Collector` — returns a grouping-by collector.
/// Stores `fn_slot` in `fields[0]`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_grouping_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/GroupingByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
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
            vec![consumer_slot, *elem],
        )?;
    }
    // Return a new stream with same elements (consumer may have GC'd things)
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(elems.len()).unwrap_or(0));
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
    // Arrays use fields directly (no length header); arraylength returns fields.len().
    let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 0);
    for elem in elems {
        heap.get_mut(arr_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

// ---------------------------------------------------------------------------
// Stream.limit / Stream.skip / Stream.flatMap
// ---------------------------------------------------------------------------

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

    // Lazy generators: materialise N elements on limit().
    if class_name == "duke/util/GeneratorStream" {
        let supplier_slot = extract_first_field_arg(heap, stream_ref)?;
        let Slot::Reference(Some(sup_ref)) = supplier_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let sup_class = heap.get(sup_ref)?.class_name.clone();
        let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(max_size).unwrap_or(0));
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
        // fields[0] = current seed, fields[1] = UnaryOperator fn
        let seed = extract_first_field_arg(heap, stream_ref)?;
        let fn_slot = extract_field_arg(heap, stream_ref, 1)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(max_size).unwrap_or(0));
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
        let inner = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, elem],
        )?;
        if let Some(Slot::Reference(Some(inner_ref))) = inner {
            // inner should be a duke/util/Stream — flatten its elements
            let inner_size = match heap.get(inner_ref)?.fields.first() {
                Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                _ => 0,
            };
            let inner_elems: Vec<Slot> = heap.get(inner_ref)?.fields[1..=inner_size].to_vec();
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

// ---------------------------------------------------------------------------
// IntStream — duke/util/IntStream
// fields[0]=Int(size), fields[1..n]=Int(value) elements (unboxed ints)
// ---------------------------------------------------------------------------

/// Build an `IntStream` heap object from a `Vec<i32>`.
fn make_int_stream(heap: &mut duke_gc::Heap, values: Vec<i32>) -> u64 {
    let n = i32::try_from(values.len()).unwrap_or(0);
    let r = heap.allocate("duke/util/IntStream".to_string(), 1);
    heap.get_mut(r).expect("fresh").fields[0] = Slot::Int(n);
    for v in values {
        heap.get_mut(r).expect("fresh").fields.push(Slot::Int(v));
    }
    r
}

/// Create an `OptionalInt` heap object. `None` = empty, `Some(v)` = present.
fn make_optional_int(heap: &mut duke_gc::Heap, value: Option<i32>) -> u64 {
    let r = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    if let Some(v) = value {
        heap.get_mut(r).expect("fresh").fields[0] = Slot::Int(v);
        heap.get_mut(r).expect("fresh").fields[1] = Slot::Int(1);
    } else {
        heap.get_mut(r).expect("fresh").fields[1] = Slot::Int(0);
    }
    r
}

/// Create a `java/util/Optional` heap object. `None` = empty, `Some(slot)` = present.
fn make_optional(heap: &mut duke_gc::Heap, value: Option<Slot>) -> u64 {
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r).expect("fresh").fields[0] = value.unwrap_or(Slot::Reference(None));
    r
}

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
    // args[0] is the int[] array ref
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
        return Ok(Some(Slot::Reference(Some(make_int_stream(
            heap,
            vec![seed],
        )))));
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
    // OptionalInt: fields[0]=Int(value), fields[1]=Int(1=present/0=empty)
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
    // OptionalDouble: fields[0]=Double(value), fields[1]=Int(1=present/0=empty)
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
        let result = ops.invoke(
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
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(I)I",
            vec![fn_slot, Slot::Int(v)],
        )?;
        result.push(match r {
            Some(Slot::Int(n)) => n,
            _ => 0,
        });
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
        heap.get_mut(stream_ref)?
            .fields
            .push(Slot::Reference(Some(boxed_ref)));
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
        let result = ops.invoke(
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

/// Native: `String.<init>(String)V` — copy constructor: copies `string_value` from source.
pub(crate) fn native_string_init_copy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_val = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = src_val;
    Ok(None)
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
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
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
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    if present {
        Ok(Some(
            heap.get(r)?.fields.first().copied().unwrap_or(Slot::Int(0)),
        ))
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
    heap.get_mut(r)?.fields[1] = Slot::Int(1); // present = true
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
    heap.get_mut(r)?.fields[1] = Slot::Int(0); // present = false
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

/// Native: `OptionalDouble.orElse(double)D`
pub(crate) fn native_optional_double_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
