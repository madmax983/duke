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
    heap.get_mut(stream_ref)?.fields.extend(elems);
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
    let Slot::Reference(Some(mut pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver and the not-yet-visited element snapshot: the
    // native holds them in Rust locals across every `ops.invoke`, and native
    // args live in a Copy `Vec<Slot>` the collector never scans, so a relocating
    // GC inside the callback would otherwise leave them stale. Record surviving
    // elements as indices into the pinned snapshot (rather than a separate ref
    // Vec that would itself go stale) and materialise them while still pinned.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut pred_ref);
    scope.pin_slots(&mut elems);
    let mut kept_idx: Vec<usize> = Vec::new();
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(pred_ref)), elem],
        )?;
        if matches!(result, Some(Slot::Int(1))) {
            kept_idx.push(i);
        }
    }
    let new_size = i32::try_from(kept_idx.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for &i in &kept_idx {
        let elem = elems[i];
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    drop(scope);
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
    let Slot::Reference(Some(mut fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    // Pin the callback receiver, the not-yet-visited element snapshot, and the
    // already-produced results: all are held in Rust locals across `ops.invoke`
    // and would otherwise go stale if the callback triggers a relocating GC.
    // `mapped` is pre-sized and index-assigned (never pushed), so its pinned
    // buffer never reallocates.
    let mut mapped: Vec<Slot> = vec![Slot::Reference(None); elems.len()];
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut fn_ref);
    scope.pin_slots(&mut elems);
    scope.pin_slots(&mut mapped);
    for i in 0..elems.len() {
        let elem = elems[i];
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
        mapped[i] = boxed;
    }
    let new_size = i32::try_from(mapped.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    #[allow(clippy::needless_range_loop)]
    for i in 0..mapped.len() {
        let elem = mapped[i];
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    drop(scope);
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
    let Slot::Reference(Some(mut consumer_ref)) = consumer_slot else {
        return Ok(None);
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    // Pin the callback receiver and the not-yet-visited element snapshot across
    // the per-element callback so a relocating GC inside the consumer cannot
    // leave the re-passed receiver or a later element stale.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut consumer_ref);
    scope.pin_slots(&mut elems);
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![Slot::Reference(Some(consumer_ref)), elem],
        )?;
    }
    drop(scope);
    Ok(None)
}

/// Shared `toMap`/`toUnmodifiableMap` body: applies `key_fn`/`val_fn` to every
/// element and assembles a `map_class` map of the boxed results.
///
/// GC-safety: references produced *inside* a callback and then stored into a heap
/// object's fields are only forwarded one-hop by `patch_forwarded_slots`, which
/// aliases under multiple relocating collections + address reuse. So instead of
/// pushing each key/value into the result map during the callback loop, the
/// boxed results accumulate in two pre-sized, *pinned* Rust buffers (the handle
/// stack is forwarded correctly on every collection), and the map is assembled
/// only after the last callback — when no further collection can run. The
/// receivers and the element snapshot are pinned across the callbacks too.
#[allow(clippy::too_many_arguments)]
fn collect_into_map(
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    key_fn: &mut Slot,
    val_fn: &mut Slot,
    elems: &mut [Slot],
    key_class: &str,
    val_class: &str,
    map_class: &str,
) -> Result<u64> {
    let n = elems.len();
    let mut keys_buf: Vec<Slot> = vec![Slot::Reference(None); n];
    let mut vals_buf: Vec<Slot> = vec![Slot::Reference(None); n];
    {
        let mut scope = NativeRootScope::new();
        scope.pin_slot(key_fn);
        scope.pin_slot(val_fn);
        scope.pin_slots(elems);
        scope.pin_slots(&mut keys_buf);
        scope.pin_slots(&mut vals_buf);
        for i in 0..n {
            let k_raw = ops
                .invoke(
                    heap,
                    out,
                    key_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![*key_fn, elems[i]],
                )?
                .unwrap_or(Slot::Reference(None));
            // Box and stash in the pinned buffer BEFORE the value callback so the
            // key survives the value callback's (and every later) collection.
            keys_buf[i] = box_primitive_slot(k_raw, heap);
            let v_raw = ops
                .invoke(
                    heap,
                    out,
                    val_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![*val_fn, elems[i]],
                )?
                .unwrap_or(Slot::Reference(None));
            vals_buf[i] = box_primitive_slot(v_raw, heap);
        }
        // Assemble the map after the last callback: no collection runs here, so a
        // single one-hop-free write of each pinned buffer entry is safe.
        let map_ref = heap.allocate(map_class.to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(i32::try_from(n).unwrap_or(i32::MAX));
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            heap.get_mut(map_ref)?.fields.push(keys_buf[i]);
            heap.get_mut(map_ref)?.fields.push(vals_buf[i]);
        }
        drop(scope);
        Ok(map_ref)
    }
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
    let mut elems: Vec<Slot> =
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
        let mut fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // Group by key WITHOUT touching the heap map during the callback loop:
        // per-element keys accumulate in a pre-sized, PINNED buffer (`keys_buf`),
        // and group membership is tracked as plain element-index lists (usize —
        // no heap refs, so GC-immune). Refs stored into a heap object mid-loop
        // are only forwarded one-hop and alias under multiple collections; the
        // pinned handle stack is forwarded correctly every collection. The map of
        // ArrayLists is assembled after the last callback, when no GC can run.
        let n = elems.len();
        let mut keys_buf: Vec<Slot> = vec![Slot::Reference(None); n];
        let mut group_keys: Vec<Slot> = vec![Slot::Reference(None); n];
        let mut group_members: Vec<Vec<usize>> = Vec::new();
        let mut group_count = 0usize;
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut fn_slot);
        scope.pin_slots(&mut elems);
        scope.pin_slots(&mut keys_buf);
        scope.pin_slots(&mut group_keys);
        #[allow(clippy::needless_range_loop)]
        for ei in 0..n {
            let key = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![fn_slot, elems[ei]],
                )?
                .unwrap_or(Slot::Reference(None));
            keys_buf[ei] = key;
            let mut found = None;
            for g in 0..group_count {
                if slots_equal(&group_keys[g], &keys_buf[ei], heap) {
                    found = Some(g);
                    break;
                }
            }
            if let Some(g) = found {
                group_members[g].push(ei);
            } else {
                group_keys[group_count] = keys_buf[ei];
                group_members.push(vec![ei]);
                group_count += 1;
            }
        }
        // Assemble the map (key -> ArrayList) after the last callback.
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(i32::try_from(group_count).unwrap_or(i32::MAX));
        #[allow(clippy::needless_range_loop)]
        for g in 0..group_count {
            let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
            let members = &group_members[g];
            heap.get_mut(list_ref)?.fields[0] = Slot::Int(i32::try_from(members.len()).unwrap_or(i32::MAX));
            for &ei in members {
                heap.get_mut(list_ref)?.fields.push(elems[ei]);
            }
            heap.get_mut(map_ref)?.fields.push(group_keys[g]);
            heap.get_mut(map_ref)?
                .fields
                .push(Slot::Reference(Some(list_ref)));
        }
        drop(scope);
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
        let mut key_fn = extract_first_field_arg(heap, collector_ref)?;
        let mut val_fn = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(key_ref)) = key_fn else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(val_ref)) = val_fn else {
            return Err(Error::NullPointerException);
        };
        let key_class = heap.get(key_ref)?.class_name.clone();
        let val_class = heap.get(val_ref)?.class_name.clone();
        let map_ref = collect_into_map(
            heap, out, ops, &mut key_fn, &mut val_fn, &mut elems, &key_class, &val_class,
            "java/util/HashMap",
        )?;
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/ToUnmodifiableMapCollector" {
        // Same as ToMapCollector but produces an UnmodifiableMap.
        let collector_ref = extract_ref_arg(args, 1)?;
        let mut key_fn = extract_first_field_arg(heap, collector_ref)?;
        let mut val_fn = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(key_ref)) = key_fn else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(val_ref)) = val_fn else {
            return Err(Error::NullPointerException);
        };
        let key_class = heap.get(key_ref)?.class_name.clone();
        let val_class = heap.get(val_ref)?.class_name.clone();
        let map_ref = collect_into_map(
            heap, out, ops, &mut key_fn, &mut val_fn, &mut elems, &key_class, &val_class,
            "java/util/UnmodifiableMap",
        )?;
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/ToMapMergeCollector" {
        // Collect into HashMap with merge function for duplicate keys.
        let collector_ref = extract_ref_arg(args, 1)?;
        let mut key_fn = extract_first_field_arg(heap, collector_ref)?;
        let mut val_fn = extract_field_arg(heap, collector_ref, 1)?;
        let mut merge_fn = extract_field_arg(heap, collector_ref, 2)?;
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
        // Accumulate distinct (key, value) entries into pre-sized, PINNED Rust
        // buffers rather than into the heap map during the loop: refs stored into
        // a heap object mid-loop are only forwarded one-hop and alias under
        // multiple collections, whereas the pinned handle stack is forwarded
        // correctly on every collection. `entry_count` tracks the distinct keys
        // (dedup can only shrink, so `n` slots suffice); `k_slot`/`v_slot` are
        // pinned scratch for the results held across the value/merge callbacks.
        let n = elems.len();
        let mut keys_buf: Vec<Slot> = vec![Slot::Reference(None); n];
        let mut vals_buf: Vec<Slot> = vec![Slot::Reference(None); n];
        let mut entry_count = 0usize;
        let mut k_slot = Slot::Reference(None);
        let mut v_slot = Slot::Reference(None);
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut key_fn);
        scope.pin_slot(&mut val_fn);
        scope.pin_slot(&mut merge_fn);
        scope.pin_slots(&mut elems);
        scope.pin_slots(&mut keys_buf);
        scope.pin_slots(&mut vals_buf);
        scope.pin_slot(&mut k_slot);
        scope.pin_slot(&mut v_slot);
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            k_slot = ops
                .invoke(
                    heap,
                    out,
                    &key_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![key_fn, elems[i]],
                )?
                .unwrap_or(Slot::Reference(None));
            v_slot = ops
                .invoke(
                    heap,
                    out,
                    &val_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![val_fn, elems[i]],
                )?
                .unwrap_or(Slot::Reference(None));
            let mut found = None;
            for p in 0..entry_count {
                if slots_equal(&keys_buf[p], &k_slot, heap) {
                    found = Some(p);
                    break;
                }
            }
            if let Some(p) = found {
                // Duplicate key — apply merge function: merge(existing, new).
                let merged = ops
                    .invoke(
                        heap,
                        out,
                        &merge_class,
                        "apply",
                        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                        vec![merge_fn, vals_buf[p], v_slot],
                    )?
                    .unwrap_or(Slot::Reference(None));
                vals_buf[p] = merged;
            } else {
                keys_buf[entry_count] = k_slot;
                vals_buf[entry_count] = v_slot;
                entry_count += 1;
            }
        }
        // Assemble the map after the last callback (no collection runs here).
        let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map_ref)?.fields[0] = Slot::Int(i32::try_from(entry_count).unwrap_or(i32::MAX));
        #[allow(clippy::needless_range_loop)]
        for p in 0..entry_count {
            heap.get_mut(map_ref)?.fields.push(keys_buf[p]);
            heap.get_mut(map_ref)?.fields.push(vals_buf[p]);
        }
        drop(scope);
        Ok(Some(Slot::Reference(Some(map_ref))))
    } else if collector_class == "duke/util/PartitioningByCollector" {
        // Collect into a Map<Boolean, List> partitioned by predicate.
        let collector_ref = extract_ref_arg(args, 1)?;
        let pred_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(mut pred_ref)) = pred_slot else {
            return Err(Error::NullPointerException);
        };
        let pred_class = heap.get(pred_ref)?.class_name.clone();
        // Pin the predicate receiver and element snapshot across the callbacks,
        // and record partition membership as plain element indices (usize — no
        // heap refs, GC-immune). The two ArrayLists are materialised from the
        // pinned snapshot after the loop; storing elements into a heap list
        // during the loop would expose them to multi-collection aliasing.
        let mut true_idx: Vec<usize> = Vec::new();
        let mut false_idx: Vec<usize> = Vec::new();
        let mut scope = NativeRootScope::new();
        scope.pin_ref(&mut pred_ref);
        scope.pin_slots(&mut elems);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![Slot::Reference(Some(pred_ref)), elems[i]],
            )?;
            if matches!(result, Some(Slot::Int(n)) if n != 0) {
                true_idx.push(i);
            } else {
                false_idx.push(i);
            }
        }
        // Materialise both lists and the result map after the last callback.
        let true_list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(true_list)?.fields[0] = Slot::Int(i32::try_from(true_idx.len()).unwrap_or(i32::MAX));
        for &i in &true_idx {
            heap.get_mut(true_list)?.fields.push(elems[i]);
        }
        let false_list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(false_list)?.fields[0] = Slot::Int(i32::try_from(false_idx.len()).unwrap_or(i32::MAX));
        for &i in &false_idx {
            heap.get_mut(false_list)?.fields.push(elems[i]);
        }
        drop(scope);
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
        let Slot::Reference(Some(mut pred_ref)) = pred_slot else {
            return Err(Error::NullPointerException);
        };
        let pred_class = heap.get(pred_ref)?.class_name.clone();
        // Pin the predicate receiver and the element snapshot across the per-
        // element `ops.invoke`. Record partition membership as indices into the
        // pinned snapshot (a growing `Vec<Slot>` of refs would itself go stale
        // and cannot be pinned across reallocation), then materialise each
        // partition from the pinned elements after the loop.
        let mut true_idx: Vec<usize> = Vec::new();
        let mut false_idx: Vec<usize> = Vec::new();
        let mut scope = NativeRootScope::new();
        scope.pin_ref(&mut pred_ref);
        scope.pin_slots(&mut elems);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let elem = elems[i];
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![Slot::Reference(Some(pred_ref)), elem],
            )?;
            if matches!(result, Some(Slot::Int(n)) if n != 0) {
                true_idx.push(i);
            } else {
                false_idx.push(i);
            }
        }
        let true_elems: Vec<Slot> = true_idx.iter().map(|&i| elems[i]).collect();
        let false_elems: Vec<Slot> = false_idx.iter().map(|&i| elems[i]).collect();
        drop(scope);
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
        let mut fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // Pin the mapper receiver (re-passed each iteration) and the element
        // snapshot across the per-element `ops.invoke`; the numeric result is
        // accumulated in a primitive local (no heap ref at risk there).
        let mut sum = 0_i32;
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut fn_slot);
        scope.pin_slots(&mut elems);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let elem = elems[i];
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
        drop(scope);
        // Return boxed Integer
        let boxed = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Int(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingIntCollector" {
        // Average via applyAsInt(elem) for each element.
        let collector_ref = extract_ref_arg(args, 1)?;
        let mut fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // Pin the mapper receiver and element snapshot across the callbacks.
        let mut sum = 0_i64;
        let mut count = 0_usize;
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut fn_slot);
        scope.pin_slots(&mut elems);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let elem = elems[i];
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
        drop(scope);
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
        let mut fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // Pin the mapper receiver and element snapshot across the callbacks.
        let mut sum = 0_i64;
        let mut min = i32::MAX;
        let mut max = i32::MIN;
        let mut count = 0_i64;
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut fn_slot);
        scope.pin_slots(&mut elems);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let elem = elems[i];
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
        drop(scope);
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
        let mut fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // Pin the mapper receiver and element snapshot across the callbacks.
        let mut sum = 0_i64;
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut fn_slot);
        scope.pin_slots(&mut elems);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let elem = elems[i];
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
        drop(scope);
        // Return boxed Long
        let boxed = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Long(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingDoubleCollector" {
        // Average via applyAsDouble(elem) for each element.
        let collector_ref = extract_ref_arg(args, 1)?;
        let mut fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // Pin the mapper receiver and element snapshot across the callbacks.
        let mut sum = 0.0_f64;
        let mut count = 0_usize;
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut fn_slot);
        scope.pin_slots(&mut elems);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let elem = elems[i];
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
        drop(scope);
        #[allow(clippy::cast_precision_loss)]
        let avg = if count == 0 { 0.0 } else { sum / count as f64 };
        // Return boxed Double
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(avg);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/MappingCollector" {
        // MappingCollector: fields[0]=mapper fn, fields[1]=downstream collector
        let collector_ref = extract_ref_arg(args, 1)?;
        let mut mapper_slot = extract_first_field_arg(heap, collector_ref)?;
        let mut downstream_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(mapper_ref)) = mapper_slot else {
            return Err(Error::NullPointerException);
        };
        let mapper_class = heap.get(mapper_ref)?.class_name.clone();
        // Map each element through the mapper function. Pin the mapper receiver,
        // the downstream collector ref (used after the loop), the element
        // snapshot, and the produced-result buffer. `mapped_elems` is pre-sized
        // and index-assigned (never pushed) so its pinned buffer never
        // reallocates while refs live in it across later callbacks.
        let mut mapped_elems: Vec<Slot> = vec![Slot::Reference(None); elems.len()];
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut mapper_slot);
        scope.pin_slot(&mut downstream_slot);
        scope.pin_slots(&mut elems);
        scope.pin_slots(&mut mapped_elems);
        for i in 0..elems.len() {
            let elem = elems[i];
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
            mapped_elems[i] = mapped;
        }
        // Build a temporary stream from mapped elements and collect with downstream
        let mapped_size = i32::try_from(mapped_elems.len()).unwrap_or(0);
        let tmp_stream = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(tmp_stream)?.fields[0] = Slot::Int(mapped_size);
        #[allow(clippy::needless_range_loop)]
        for i in 0..mapped_elems.len() {
            let m = mapped_elems[i];
            heap.get_mut(tmp_stream)?.fields.push(m);
        }
        let tmp_args = vec![Slot::Reference(Some(tmp_stream)), downstream_slot];
        drop(scope);
        native_stream_collect(&tmp_args, heap, out, control, ops)
    } else if collector_class == "duke/util/GroupingBy2Collector" {
        // groupingBy(keyFn, downstream): fields[0]=keyFn, fields[1]=downstream collector
        let collector_ref = extract_ref_arg(args, 1)?;
        let mut fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let mut downstream_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // First pass: group element INDICES by boxed key, accumulating into
        // pinned buffers (`keys_buf`/`group_keys`) and plain usize index lists —
        // never touching a heap map during the callback loop (heap-stored refs
        // alias under multiple collections; the pinned handle stack does not).
        let n = elems.len();
        let mut keys_buf: Vec<Slot> = vec![Slot::Reference(None); n];
        let mut group_keys: Vec<Slot> = vec![Slot::Reference(None); n];
        let mut group_members: Vec<Vec<usize>> = Vec::new();
        let mut group_count = 0usize;
        let mut scope1 = NativeRootScope::new();
        scope1.pin_slot(&mut fn_slot);
        scope1.pin_slot(&mut downstream_slot);
        scope1.pin_slots(&mut elems);
        scope1.pin_slots(&mut keys_buf);
        scope1.pin_slots(&mut group_keys);
        #[allow(clippy::needless_range_loop)]
        for ei in 0..n {
            let key_raw = ops
                .invoke(
                    heap,
                    out,
                    &fn_class,
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    vec![fn_slot, elems[ei]],
                )?
                .unwrap_or(Slot::Reference(None));
            // Box primitive keys so they match via slots_equal, and stash in the
            // pinned buffer so the boxed key survives every later callback.
            keys_buf[ei] = box_primitive_slot(key_raw, heap);
            let mut found = None;
            for g in 0..group_count {
                if slots_equal(&group_keys[g], &keys_buf[ei], heap) {
                    found = Some(g);
                    break;
                }
            }
            if let Some(g) = found {
                group_members[g].push(ei);
            } else {
                group_keys[group_count] = keys_buf[ei];
                group_members.push(vec![ei]);
                group_count += 1;
            }
        }
        drop(scope1);
        // Second pass: apply the downstream collector to each group's elements,
        // accumulating the (key, collected) pairs into pinned buffers. The
        // recursive `native_stream_collect` is a GC point, so `group_keys` and the
        // collected results are kept on the pinned handle stack across it; the
        // result map is assembled only after the last recursive collect.
        let mut collected_vals: Vec<Slot> = vec![Slot::Reference(None); group_count];
        let mut scope2 = NativeRootScope::new();
        scope2.pin_slot(&mut downstream_slot);
        scope2.pin_slots(&mut elems);
        scope2.pin_slots(&mut group_keys);
        scope2.pin_slots(&mut collected_vals);
        #[allow(clippy::needless_range_loop)]
        for g in 0..group_count {
            let members = group_members[g].clone();
            let tmp_stream = heap.allocate("duke/util/Stream".to_string(), 1);
            heap.get_mut(tmp_stream)?.fields[0] =
                Slot::Int(i32::try_from(members.len()).unwrap_or(i32::MAX));
            for &ei in &members {
                heap.get_mut(tmp_stream)?.fields.push(elems[ei]);
            }
            let tmp_args = vec![Slot::Reference(Some(tmp_stream)), downstream_slot];
            let collected = native_stream_collect(&tmp_args, heap, out, control, ops)?
                .unwrap_or(Slot::Reference(None));
            collected_vals[g] = collected;
        }
        // Assemble the result map after the last recursive collect.
        let result_map = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(result_map)?.fields[0] = Slot::Int(i32::try_from(group_count).unwrap_or(i32::MAX));
        #[allow(clippy::needless_range_loop)]
        for g in 0..group_count {
            heap.get_mut(result_map)?.fields.push(group_keys[g]);
            heap.get_mut(result_map)?.fields.push(collected_vals[g]);
        }
        drop(scope2);
        Ok(Some(Slot::Reference(Some(result_map))))
    } else if collector_class == "duke/util/MinByCollector" {
        // minBy(comparator): fields[0] = comparator
        let collector_ref = extract_ref_arg(args, 1)?;
        let mut cmp_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(cmp_ref)) = cmp_slot else {
            let r = make_optional(heap, None);
            return Ok(Some(Slot::Reference(Some(r))));
        };
        let cmp_class = heap.get(cmp_ref)?.class_name.clone();
        // Pin the comparator receiver, the element snapshot, and the running
        // minimum (a heap ref carried across, and compared by, every callback).
        // `min_slot` holds the current best; `has_min` gates it.
        let mut min_slot = Slot::Reference(None);
        let mut has_min = false;
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut cmp_slot);
        scope.pin_slots(&mut elems);
        scope.pin_slot(&mut min_slot);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let is_less = if has_min {
                let result = ops
                    .invoke(
                        heap,
                        out,
                        &cmp_class,
                        "compare",
                        "(Ljava/lang/Object;Ljava/lang/Object;)I",
                        vec![cmp_slot, elems[i], min_slot],
                    )?
                    .unwrap_or(Slot::Int(0));
                matches!(result, Slot::Int(n) if n < 0)
            } else {
                true
            };
            if is_less {
                // Read from the pinned snapshot after the callback so the new
                // running minimum tracks any relocation the compare triggered.
                min_slot = elems[i];
                has_min = true;
            }
        }
        let min = if has_min { Some(min_slot) } else { None };
        drop(scope);
        let r = make_optional(heap, min);
        Ok(Some(Slot::Reference(Some(r))))
    } else if collector_class == "duke/util/MaxByCollector" {
        // maxBy(comparator): fields[0] = comparator
        let collector_ref = extract_ref_arg(args, 1)?;
        let mut cmp_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(cmp_ref)) = cmp_slot else {
            let r = make_optional(heap, None);
            return Ok(Some(Slot::Reference(Some(r))));
        };
        let cmp_class = heap.get(cmp_ref)?.class_name.clone();
        // Pin the comparator receiver, the element snapshot, and the running
        // maximum (a heap ref carried across, and compared by, every callback).
        let mut max_slot = Slot::Reference(None);
        let mut has_max = false;
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut cmp_slot);
        scope.pin_slots(&mut elems);
        scope.pin_slot(&mut max_slot);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let is_greater = if has_max {
                let result = ops
                    .invoke(
                        heap,
                        out,
                        &cmp_class,
                        "compare",
                        "(Ljava/lang/Object;Ljava/lang/Object;)I",
                        vec![cmp_slot, elems[i], max_slot],
                    )?
                    .unwrap_or(Slot::Int(0));
                matches!(result, Slot::Int(n) if n > 0)
            } else {
                true
            };
            if is_greater {
                // Read from the pinned snapshot after the callback so the new
                // running maximum tracks any relocation the compare triggered.
                max_slot = elems[i];
                has_max = true;
            }
        }
        let max = if has_max { Some(max_slot) } else { None };
        drop(scope);
        let r = make_optional(heap, max);
        Ok(Some(Slot::Reference(Some(r))))
    } else if collector_class == "duke/util/SummingDoubleCollector" {
        // summingDouble: fields[0] = ToDoubleFunction
        let collector_ref = extract_ref_arg(args, 1)?;
        let mut fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // Pin the mapper receiver and element snapshot across the callbacks.
        let mut sum = 0.0_f64;
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut fn_slot);
        scope.pin_slots(&mut elems);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let elem = elems[i];
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
        drop(scope);
        let boxed = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed)?.fields[0] = Slot::Double(sum);
        Ok(Some(Slot::Reference(Some(boxed))))
    } else if collector_class == "duke/util/AveragingLongCollector" {
        // averagingLong: fields[0] = ToLongFunction
        let collector_ref = extract_ref_arg(args, 1)?;
        let mut fn_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Err(Error::NullPointerException);
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // Pin the mapper receiver and element snapshot across the callbacks.
        let mut sum = 0_i64;
        let mut count = 0_usize;
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut fn_slot);
        scope.pin_slots(&mut elems);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let elem = elems[i];
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
        drop(scope);
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
        let mut op_slot = extract_first_field_arg(heap, collector_ref)?;
        let Slot::Reference(Some(bop_ref)) = op_slot else {
            return Err(Error::NullPointerException);
        };
        let op_class = heap.get(bop_ref)?.class_name.clone();
        // Pin the operator receiver, the element snapshot, and the running
        // accumulator (a heap ref carried across every fold callback).
        let result = if elems.is_empty() {
            None
        } else {
            let mut acc = elems[0];
            let mut scope = NativeRootScope::new();
            scope.pin_slot(&mut op_slot);
            scope.pin_slots(&mut elems);
            scope.pin_slot(&mut acc);
            #[allow(clippy::needless_range_loop)]
            for i in 1..elems.len() {
                let elem = elems[i];
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
            drop(scope);
            Some(acc)
        };
        let opt_ref = make_optional(heap, result);
        Ok(Some(Slot::Reference(Some(opt_ref))))
    } else if collector_class == "duke/util/ReducingCollector" {
        // reducing(identity, BinaryOperator) → T
        let collector_ref = extract_ref_arg(args, 1)?;
        let identity_slot = extract_first_field_arg(heap, collector_ref)?;
        let mut op_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(bop_ref)) = op_slot else {
            return Err(Error::NullPointerException);
        };
        let op_class = heap.get(bop_ref)?.class_name.clone();
        // Pin the operator receiver, the element snapshot, and the running
        // accumulator (a heap ref carried across every fold callback).
        let mut acc = identity_slot;
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut op_slot);
        scope.pin_slots(&mut elems);
        scope.pin_slot(&mut acc);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let elem = elems[i];
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
        drop(scope);
        Ok(Some(acc))
    } else if collector_class == "duke/util/ReducingMappingCollector" {
        // reducing(identity, mapper, BinaryOperator) → U
        let collector_ref = extract_ref_arg(args, 1)?;
        let identity_slot = extract_first_field_arg(heap, collector_ref)?;
        let mut mapper_slot = extract_field_arg(heap, collector_ref, 1)?;
        let mut op_slot = extract_field_arg(heap, collector_ref, 2)?;
        let Slot::Reference(Some(mapper_ref)) = mapper_slot else {
            return Err(Error::NullPointerException);
        };
        let Slot::Reference(Some(bop_ref)) = op_slot else {
            return Err(Error::NullPointerException);
        };
        let mapper_class = heap.get(mapper_ref)?.class_name.clone();
        let op_class = heap.get(bop_ref)?.class_name.clone();
        // Pin the mapper and operator receivers, the element snapshot, the
        // running accumulator, and the per-iteration mapped value (held across
        // the fold `ops.invoke`). All are held in Rust locals across callbacks.
        let mut acc = identity_slot;
        let mut mapped = Slot::Reference(None);
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut mapper_slot);
        scope.pin_slot(&mut op_slot);
        scope.pin_slots(&mut elems);
        scope.pin_slot(&mut acc);
        scope.pin_slot(&mut mapped);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let elem = elems[i];
            mapped = ops
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
        drop(scope);
        Ok(Some(acc))
    } else if collector_class == "duke/util/CollectingAndThenCollector" {
        // collectingAndThen(downstream, finisher): fields[0]=downstream, fields[1]=finisher
        let collector_ref = extract_ref_arg(args, 1)?;
        let downstream_slot = extract_first_field_arg(heap, collector_ref)?;
        let mut finisher_slot = extract_field_arg(heap, collector_ref, 1)?;
        let Slot::Reference(Some(finisher_ref)) = finisher_slot else {
            return Err(Error::NullPointerException);
        };
        let finisher_class = heap.get(finisher_ref)?.class_name.clone();
        // The recursive downstream `native_stream_collect` is itself a GC point
        // (it runs the downstream collector's callbacks, which allocate and can
        // relocate). Pin the finisher receiver across it: it is held in a Rust
        // local from before the recursive collect until the finisher `ops.invoke`
        // afterwards, so a relocating GC inside the downstream collect would
        // otherwise leave it stale.
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut finisher_slot);
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
        drop(scope);
        Ok(result.or(Some(Slot::Reference(None))))
    } else if collector_class == "duke/util/ToUnmodifiableListCollector" {
        // toUnmodifiableList(): collect into UnmodifiableList (mutations throw).
        let list_ref = heap.allocate("java/util/UnmodifiableList".to_string(), 1);
        heap.get_mut(list_ref)?.fields[0] = Slot::Int(size);
        heap.get_mut(list_ref)?.fields.extend(elems);
        Ok(Some(Slot::Reference(Some(list_ref))))
    } else if !collector_class.starts_with("duke/util/") {
        // A real `java.util.stream.Collector` implementation (not one of Duke's
        // synthetic `duke/util/*` markers) — e.g. a third-party collector such as
        // commons-lang3 `LangCollectors.joining`. Drive the standard Collector
        // protocol so the finisher actually runs, instead of returning the raw
        // element container:
        //   container = supplier().get();
        //   for elem in elems { accumulator().accept(container, elem); }
        //   result = finisher().apply(container);
        let collector_ref = extract_ref_arg(args, 1)?;
        let mut collector_slot = Slot::Reference(Some(collector_ref));

        // This branch drives a chain of callbacks (supplier → get → accumulator →
        // accept per element → finisher → apply) and holds several heap refs in
        // Rust locals across those `ops.invoke` GC points: the collector itself
        // (re-passed to supplier()/accumulator()/finisher()), the result
        // container (produced by get(), fed to every accept and to the final
        // apply), the accumulator BiConsumer (re-passed to every accept), and the
        // element snapshot. Pin them so a relocating GC inside any callback keeps
        // them alive and forwarded.
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut collector_slot);

        // container = collector.supplier().get()
        let supplier = ops
            .invoke(
                heap,
                out,
                &collector_class,
                "supplier",
                "()Ljava/util/function/Supplier;",
                vec![collector_slot],
            )?
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(supplier_ref)) = supplier else {
            return Err(Error::NullPointerException);
        };
        let supplier_class = heap.get(supplier_ref)?.class_name.clone();
        let mut container = ops
            .invoke(
                heap,
                out,
                &supplier_class,
                "get",
                "()Ljava/lang/Object;",
                vec![supplier],
            )?
            .unwrap_or(Slot::Reference(None));
        scope.pin_slot(&mut container);

        // accumulator = collector.accumulator()
        let mut accumulator = ops
            .invoke(
                heap,
                out,
                &collector_class,
                "accumulator",
                "()Ljava/util/function/BiConsumer;",
                vec![collector_slot],
            )?
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(acc_ref)) = accumulator else {
            return Err(Error::NullPointerException);
        };
        let acc_class = heap.get(acc_ref)?.class_name.clone();
        scope.pin_slot(&mut accumulator);
        scope.pin_slots(&mut elems);
        #[allow(clippy::needless_range_loop)]
        for i in 0..elems.len() {
            let elem = elems[i];
            ops.invoke(
                heap,
                out,
                &acc_class,
                "accept",
                "(Ljava/lang/Object;Ljava/lang/Object;)V",
                vec![accumulator, container, elem],
            )?;
        }

        // result = collector.finisher().apply(container)
        let finisher = ops
            .invoke(
                heap,
                out,
                &collector_class,
                "finisher",
                "()Ljava/util/function/Function;",
                vec![collector_slot],
            )?
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fin_ref)) = finisher else {
            // No finisher (should not happen for a well-formed Collector) — the
            // container itself is the result (IDENTITY_FINISH semantics).
            drop(scope);
            return Ok(Some(container));
        };
        let fin_class = heap.get(fin_ref)?.class_name.clone();
        let result = ops.invoke(
            heap,
            out,
            &fin_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![finisher, container],
        )?;
        drop(scope);
        Ok(result.or(Some(Slot::Reference(None))))
    } else {
        // ToListCollector (default): collect into ArrayList.
        let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(list_ref)?.fields[0] = Slot::Int(size);
        heap.get_mut(list_ref)?.fields.extend(elems);
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
    heap.get_mut(new_stream)?.fields.extend(seen);
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
    // Pin the element snapshot: its refs are held (and swapped in place) across
    // every `compareTo` callback and materialised into the result stream after,
    // so a relocating GC inside a callback must not leave them stale. Swapping
    // reorders values within the pinned buffer (its address is unchanged).
    let mut scope = NativeRootScope::new();
    scope.pin_slots(&mut elems);
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
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    drop(scope);
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
    let Slot::Reference(Some(mut pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver and the not-yet-visited element snapshot so a
    // relocating GC inside the predicate cannot leave the re-passed receiver or
    // a later element stale. `scope` is dropped by RAII on any return.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut pred_ref);
    scope.pin_slots(&mut elems);
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
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
    drop(scope);
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
    let Slot::Reference(Some(mut pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver and the not-yet-visited element snapshot so a
    // relocating GC inside the predicate cannot leave the re-passed receiver or
    // a later element stale. `scope` is dropped by RAII on any return.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut pred_ref);
    scope.pin_slots(&mut elems);
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
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
    drop(scope);
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
    let Slot::Reference(Some(mut pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver and the not-yet-visited element snapshot so a
    // relocating GC inside the predicate cannot leave the re-passed receiver or
    // a later element stale. `scope` is dropped by RAII on any return.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut pred_ref);
    scope.pin_slots(&mut elems);
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
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
    drop(scope);
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
    let Slot::Reference(Some(mut op_ref)) = op_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let op_class = heap.get(op_ref)?.class_name.clone();
    let mut reduce_result_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if elems.is_empty() {
        heap.get_mut(reduce_result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(reduce_result_ref))));
    }
    let mut acc = elems[0];
    // Pin every ref held across the fold: the callback receiver, the
    // not-yet-visited element snapshot, the running accumulator (reassigned each
    // iteration — its stack storage is pinned), and the Optional container that
    // is allocated before the loop and written after it. Index-walk the pinned
    // snapshot; the accumulator stays pinned across the final `box_primitive_slot`
    // (which may allocate).
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut op_ref);
    scope.pin_ref(&mut reduce_result_ref);
    scope.pin_slots(&mut elems);
    scope.pin_slot(&mut acc);
    #[allow(clippy::needless_range_loop)]
    for i in 1..elems.len() {
        let elem = elems[i];
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
    drop(scope);
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
    let mut fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(identity));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut acc = identity;
    // Pin the callback receiver slot, the not-yet-visited element snapshot, and
    // the running accumulator (reassigned each iteration — its stack storage is
    // pinned) so a relocating GC inside the accumulator cannot leave them stale.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    scope.pin_slots(&mut elems);
    scope.pin_slot(&mut acc);
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
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
    drop(scope);
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
    let mut consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(Some(Slot::Reference(Some(stream_ref))));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    // Pin the callback receiver slot and the element snapshot: the snapshot is
    // both re-visited across the per-element callback and materialised into the
    // returned stream afterward, so a relocating GC inside the consumer must not
    // leave its refs stale. Keep the pin across the final allocation.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut consumer_slot);
    scope.pin_slots(&mut elems);
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, elem],
        )?;
    }
    // Return a new stream with same elements (consumer may have GC'd things)
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(elems.len()).unwrap_or(0));
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    drop(scope);
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
    heap.get_mut(arr_ref)?.fields.extend(elems);
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

    // Lazy generators: materialise N elements on limit().
    if class_name == "duke/util/GeneratorStream" {
        let supplier_slot = extract_first_field_arg(heap, stream_ref)?;
        let Slot::Reference(Some(mut sup_ref)) = supplier_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let sup_class = heap.get(sup_ref)?.class_name.clone();
        let mut out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(max_size).unwrap_or(0));
        // Pin the supplier receiver and the result stream accumulator: both are
        // held in Rust locals across every `get()` callback. Pinning `out_ref`
        // roots the already-collected elements it holds, so they survive any
        // relocating GC a later callback triggers.
        let mut scope = NativeRootScope::new();
        scope.pin_ref(&mut sup_ref);
        scope.pin_ref(&mut out_ref);
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
        drop(scope);
        return Ok(Some(Slot::Reference(Some(out_ref))));
    }

    if class_name == "duke/util/IteratorStream" {
        // fields[0] = current seed, fields[1] = UnaryOperator fn
        let seed = extract_first_field_arg(heap, stream_ref)?;
        let mut fn_slot = extract_field_arg(heap, stream_ref, 1)?;
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(Slot::Reference(None)));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let mut out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
        heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(max_size).unwrap_or(0));
        let mut current = seed;
        // Pin the callback receiver slot and the result stream accumulator:
        // both are held across every `apply()` callback. Pinning `out_ref` roots
        // the already-pushed elements it holds. `current` is pushed into `out_ref`
        // (thus rooted) before each callback and then reassigned from the
        // callback's result, so it is never read while stale and needs no pin.
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut fn_slot);
        scope.pin_ref(&mut out_ref);
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
        drop(scope);
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
    heap.get_mut(out_ref)?.fields.extend(skipped);
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    // Accumulate the flattened elements directly into the (heap-allocated,
    // pinned) result stream rather than a growing Rust `Vec` — pinning cannot
    // track a `Vec` that reallocates, but a pinned `out_ref` roots every element
    // already pushed into it, so they survive any relocating GC a later callback
    // triggers. Also pin the callback receiver slot and the element snapshot.
    let mut out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(0);
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    scope.pin_slots(&mut elems);
    scope.pin_ref(&mut out_ref);
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
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
            heap.get_mut(out_ref)?.fields.extend(inner_elems);
        }
    }
    let new_size = i32::try_from(heap.get(out_ref)?.fields.len().saturating_sub(1)).unwrap_or(0);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(new_size);
    drop(scope);
    Ok(Some(Slot::Reference(Some(out_ref))))
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(
            heap,
            vec![seed],
        )))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut values = Vec::with_capacity(MAX);
    let mut cur = seed;
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
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
    drop(scope);
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut pred_slot);
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
    drop(scope);
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let elems = int_stream_elems(heap, r);
    if let Slot::Reference(Some(fn_ref)) = fn_slot {
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        // Pin the callback receiver: it is re-passed to `ops.invoke` on every
        // iteration, but native args live in a Copy `Vec<Slot>` the collector
        // never scans, so a relocating GC inside the callback would otherwise
        // leave this ref stale before the next iteration re-passes it.
        let mut scope = NativeRootScope::new();
        scope.pin_slot(&mut fn_slot);
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
        drop(scope);
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
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
    drop(scope);
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
    let mut consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(None);
    };
    let elems = int_stream_elems(heap, r);
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut consumer_slot);
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
    drop(scope);
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    // Elements are primitive ints (no element-ref hazard), but the mapper
    // produces object results that accumulate across later callbacks. Pin the
    // callback receiver slot and the pre-sized, index-assigned result buffer
    // (never pushed, so it never reallocates) so a relocating GC cannot leave
    // an already-produced result stale.
    let mut mapped: Vec<Slot> = vec![Slot::Reference(None); elems.len()];
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    scope.pin_slots(&mut mapped);
    for (i, v) in elems.into_iter().enumerate() {
        let result = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(I)Ljava/lang/Object;",
            vec![fn_slot, Slot::Int(v)],
        )?;
        mapped[i] = result.unwrap_or(Slot::Reference(None));
    }
    let new_size = i32::try_from(mapped.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(new_size);
    #[allow(clippy::needless_range_loop)]
    for i in 0..mapped.len() {
        let elem = mapped[i];
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    drop(scope);
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
/// Native: `NaturalOrderComparator.compare(O,O)I` — delegates to `o1.compareTo(o2)`.
pub(crate) fn native_natural_order_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    // args: [this, o1, o2]
    let o1 = extract_slot_arg(args, 1);
    let o2 = extract_slot_arg(args, 2);
    let o1_class = match &o1 {
        Slot::Reference(Some(r)) => heap.get(*r)?.class_name.clone(),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let result = ops.invoke(
        heap,
        out,
        &o1_class,
        "compareTo",
        "(Ljava/lang/Object;)I",
        vec![o1, o2],
    )?;
    let _ = control;
    Ok(Some(result.unwrap_or(Slot::Int(0))))
}
/// Native: `ReverseOrderComparator.compare(O,O)I` — negates natural order.
pub(crate) fn native_reverse_order_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let result = native_natural_order_compare(args, heap, out, control, ops)?;
    Ok(Some(match result {
        Some(Slot::Int(v)) => Slot::Int(-v),
        other => other.unwrap_or(Slot::Int(0)),
    }))
}
/// Native: `ComparingIntComparator.compare(O,O)I` — calls `fn.applyAsInt(o)` for each element.
pub(crate) fn native_comparing_int_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(mut fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = extract_slot_arg(args, 1);
    let mut b = extract_slot_arg(args, 2);
    // Pin the comparator receiver (re-passed to both `applyAsInt` invokes) and
    // the second key `b`, which is held across the first invoke. Without this a
    // relocating GC inside the first callback leaves `fn_ref`/`b` stale before
    // the second invoke reads them.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut fn_ref);
    scope.pin_slot(&mut b);
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
    drop(scope);
    let result = match (ka, kb) {
        (Slot::Int(ia), Slot::Int(ib)) => ia.cmp(&ib) as i32,
        _ => 0,
    };
    let _ = control;
    Ok(Some(Slot::Int(result)))
}
pub(crate) fn native_condition_await(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    condition_await_common(args, heap, control, None)
}
pub(crate) fn native_condition_await_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let nanos = extract_long_arg(args, 1)?;
    condition_await_common(args, heap, control, Some(nanos))
}
pub(crate) fn native_condition_signal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let condition = condition_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let mut condition_guard = condition
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let lock_state = std::sync::Arc::clone(&condition_guard.lock);
    let lock_guard = lock_state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !reentrant_lock_is_held_by(&lock_guard, thread_id) {
        return Err(illegal_monitor_state_error());
    }
    drop(lock_guard);
    if let Some(waiter) = condition_guard
        .waiters
        .iter_mut()
        .find(|waiter| !waiter.signaled)
    {
        waiter.signaled = true;
        waiter.timed_out = false;
    }
    drop(condition_guard);
    Ok(None)
}
pub(crate) fn native_condition_signal_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let condition = condition_state(heap, this_ref)?;
    let thread_id = current_host_thread_id();
    let mut condition_guard = condition
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let lock_state = std::sync::Arc::clone(&condition_guard.lock);
    let lock_guard = lock_state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !reentrant_lock_is_held_by(&lock_guard, thread_id) {
        return Err(illegal_monitor_state_error());
    }
    drop(lock_guard);
    for waiter in &mut condition_guard.waiters {
        waiter.signaled = true;
        waiter.timed_out = false;
    }
    drop(condition_guard);
    Ok(None)
}
/// Native: `Comparator.comparing compare(Object,Object)I` — compare via key extractor.
pub(crate) fn native_comparing_comparator_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let mut b = extract_slot_arg(args, 2);
    let mut fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    // Pin the key-extractor receiver (`fn_slot`, re-passed to both `apply`
    // invokes) and the second element `b` (held across the first invoke). After
    // the first invoke returns, also pin its object-typed key result `ka`, which
    // is held across the second invoke and read by `compare_treemap_keys`.
    // Without this a relocating GC inside either callback leaves these stale.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    scope.pin_slot(&mut b);
    let mut ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Reference(None));
    scope.pin_slot(&mut ka);
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
    drop(scope);
    // Compare extracted keys via natural ordering (String, Integer, Long, or raw int).
    let cmp = compare_treemap_keys(ka, kb, heap) as i32;
    Ok(Some(Slot::Int(cmp)))
}
/// Native: `Collectors.toSet()Collector` — returns a `ToSetCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_set(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/ToSetCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.toMap(keyFn, valFn)Collector` — stores both functions in `ToMapCollector`.
pub(crate) fn native_collectors_to_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let key_fn = extract_slot_arg(args, 0);
    let val_fn = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ToMapCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = key_fn;
    heap.get_mut(r)?.fields[1] = val_fn;
    Ok(Some(Slot::Reference(Some(r))))
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    // Object elements + object receiver held across each callback; the produced
    // ints are primitives (no accumulator-ref hazard). Pin the receiver slot and
    // the not-yet-visited element snapshot.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    scope.pin_slots(&mut elems);
    let mut values = Vec::with_capacity(elems.len());
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
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
                // Unbox Integer/Short/Byte if the function returned a boxed type.
                let n = match heap.get(r)?.fields.first() {
                    Some(Slot::Int(v)) => *v,
                    _ => 0,
                };
                values.push(n);
            }
            _ => values.push(0),
        }
    }
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
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
    let mut fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(identity)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let values = int_stream_elems(heap, stream_ref);
    let mut acc = identity;
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
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
    drop(scope);
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        // Return empty OptionalInt
        let r = make_optional_int(heap, None);
        return Ok(Some(Slot::Reference(Some(r))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let values = int_stream_elems(heap, stream_ref);
    if values.is_empty() {
        return Ok(Some(Slot::Reference(Some(make_optional_int(heap, None)))));
    }
    let mut acc = values[0];
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
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
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_optional_int(
        heap,
        Some(acc),
    )))))
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
/// Native: `ArrayListIterator.<init>` — no-op; fields set directly by `native_arraylist_iterator`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_arraylist_iter_init(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}
/// Native: `ArrayListIterator.hasNext()Z`
pub(crate) fn native_arraylist_iter_hasnext(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_obj = heap.get(this_ref)?;
    let list_ref = match iter_obj.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Int(0))),
    };
    let cursor = match iter_obj.fields.get(1) {
        Some(Slot::Int(i)) => *i,
        _ => 0,
    };
    let list_size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(cursor < list_size))))
}
/// Native: `ArrayListIterator.next()Object` — returns element at cursor, advances cursor.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (list_ref, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let lr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(Error::NullPointerException),
        };
        let c = match iter_obj.fields.get(1) {
            Some(Slot::Int(i)) => *i,
            _ => 0,
        };
        (lr, c)
    };
    let element = {
        let list_obj = heap.get(list_ref)?;
        match list_obj.fields.get(cursor as usize + 1) {
            Some(slot) => *slot,
            None => {
                return Err(Error::JavaException {
                    class_name: "java/util/NoSuchElementException".to_string(),
                });
            }
        }
    };
    let iter_obj = heap.get_mut(this_ref)?;
    iter_obj.fields[1] = Slot::Int(cursor + 1);
    iter_obj.fields[2] = Slot::Int(cursor); // record last-returned index
    Ok(Some(element))
}
/// Native: `ArrayListIterator.remove()V` — removes the last element returned by `next()`.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_iter_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (list_ref, last, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let lr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(Error::NullPointerException),
        };
        let last = match iter_obj.fields.get(2) {
            Some(Slot::Int(i)) => *i,
            _ => -1,
        };
        let cursor = match iter_obj.fields.get(1) {
            Some(Slot::Int(i)) => *i,
            _ => 0,
        };
        (lr, last, cursor)
    };
    if last < 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalStateException".to_string(),
        });
    }
    // Remove from backing list: shift elements left, decrement size.
    let list_size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    let remove_idx = last as usize + 1; // +1 because fields[0] is size
    let list_obj = heap.get_mut(list_ref)?;
    let new_size = (list_size - 1) as usize;
    list_obj.fields[0] = Slot::Int(list_size - 1);
    list_obj.fields.remove(remove_idx);
    list_obj.fields.push(Slot::Int(0)); // pad to keep capacity stable
    // Adjust cursor: removed element was before cursor, so decrement.
    if last < cursor {
        let iter_obj = heap.get_mut(this_ref)?;
        iter_obj.fields[1] = Slot::Int(cursor - 1);
    }
    // Reset last-returned sentinel.
    heap.get_mut(this_ref)?.fields[2] = Slot::Int(-1);
    let _ = new_size; // used implicitly
    Ok(None)
}
/// Native: `ReversedComparator.compare(a, b)I` — inverts delegate comparison.
pub(crate) fn native_reversed_comparator_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let delegate = extract_first_field_arg(heap, this_ref)?;
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
pub(crate) fn native_properties_enum_has_more_elements(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let index = match fields.get(PROPERTIES_ENUM_INDEX_FIELD) {
        Some(Slot::Int(index)) => *index,
        _ => 0,
    };
    let count = match fields.get(PROPERTIES_ENUM_COUNT_FIELD) {
        Some(Slot::Int(count)) => *count,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(index < count))))
}
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_properties_enum_next_element(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (index, count) = {
        let fields = &heap.get(this_ref)?.fields;
        let index = match fields.get(PROPERTIES_ENUM_INDEX_FIELD) {
            Some(Slot::Int(index)) => *index,
            _ => 0,
        };
        let count = match fields.get(PROPERTIES_ENUM_COUNT_FIELD) {
            Some(Slot::Int(count)) => *count,
            _ => 0,
        };
        (index, count)
    };
    if index < 0 || index >= count {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let slot_idx = PROPERTIES_ENUM_NAMES_START + index as usize;
    let value = heap
        .get(this_ref)?
        .fields
        .get(slot_idx)
        .copied()
        .unwrap_or(Slot::Reference(None));
    heap.get_mut(this_ref)?.fields[PROPERTIES_ENUM_INDEX_FIELD] = Slot::Int(index + 1);
    Ok(Some(value))
}
/// Native: `HashSetIterator.<init>` — no-op; fields are set by `native_hashset_iterator`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_hashset_iter_init(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}
/// Native: `HashSetIterator.hasNext()Z`
pub(crate) fn native_hashset_iter_hasnext(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_obj = heap.get(this_ref)?;
    let set_ref = match iter_obj.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Int(0))),
    };
    let cursor = match iter_obj.fields.get(1) {
        Some(Slot::Int(i)) => *i,
        _ => 0,
    };
    let set_size = match heap.get(set_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(cursor < set_size))))
}
/// Native: `HashSetIterator.next()Object` — returns element at cursor, advances cursor.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_hashset_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (set_ref, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let sr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(Error::NullPointerException),
        };
        let c = match iter_obj.fields.get(1) {
            Some(Slot::Int(i)) => *i,
            _ => 0,
        };
        (sr, c)
    };
    let element = {
        let set_obj = heap.get(set_ref)?;
        match set_obj.fields.get(cursor as usize + 1) {
            Some(slot) => *slot,
            None => {
                return Err(Error::JavaException {
                    class_name: "java/util/NoSuchElementException".to_string(),
                });
            }
        }
    };
    heap.get_mut(this_ref)?.fields[1] = Slot::Int(cursor + 1);
    Ok(Some(element))
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
    let Slot::Reference(Some(mut pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the receiver and the element snapshot; record kept elements as indices
    // into the pinned snapshot and materialise them while still pinned.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut pred_ref);
    scope.pin_slots(&mut elems);
    let mut kept_idx: Vec<usize> = Vec::new();
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(pred_ref)), elem],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept_idx.push(i);
        } else {
            break;
        }
    }
    let new_size = i32::try_from(kept_idx.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for &i in &kept_idx {
        let elem = elems[i];
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    drop(scope);
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
    let Slot::Reference(Some(mut pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the receiver and the element snapshot; record kept elements as indices
    // into the pinned snapshot and materialise them while still pinned.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut pred_ref);
    scope.pin_slots(&mut elems);
    let mut dropping = true;
    let mut kept_idx: Vec<usize> = Vec::new();
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
        if dropping {
            let result = ops.invoke(
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
        kept_idx.push(i);
    }
    let new_size = i32::try_from(kept_idx.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for &i in &kept_idx {
        let elem = elems[i];
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    drop(scope);
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
    // If no comparator provided, fall back to natural-order sort.
    let Slot::Reference(Some(mut comp_ref)) = extract_slot_arg(args, 1)
    else {
        return native_stream_sorted(args, heap, out, control, ops);
    };
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    // Pin the comparator receiver and the element snapshot: the snapshot's refs
    // are held (and swapped in place) across every `compare` callback and
    // materialised into the result stream after. Keep the pin across the final
    // allocation.
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut comp_ref);
    scope.pin_slots(&mut elems);
    // Insertion sort using the provided comparator.
    for i in 1..elems.len() {
        let mut j = i;
        while j > 0 {
            let comp_class = heap.get(comp_ref)?.class_name.clone();
            let cmp = ops.invoke(
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
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    drop(scope);
    Ok(Some(Slot::Reference(Some(new_stream))))
}
/// Native: `Collectors.partitioningBy(Predicate)Collector` — returns a sentinel collector.
pub(crate) fn native_collectors_partitioning_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let pred = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/PartitioningByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = pred;
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_collectors_partitioning_by_downstream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let pred = extract_slot_arg(args, 0);
    let downstream = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/PartitioningByDownstreamCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = pred;
    heap.get_mut(r)?.fields[1] = downstream;
    Ok(Some(Slot::Reference(Some(r))))
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
    let mut vals: Vec<i32> = heap.get(stream_ref)?.fields[1..=size]
        .iter()
        .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
        .collect();
    vals.sort_unstable();
    let new_stream = heap.allocate("duke/util/IntStream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(i32::try_from(vals.len()).unwrap_or(0));
    for v in vals {
        heap.get_mut(new_stream)?.fields.push(Slot::Int(v));
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}
/// Native: `Comparator.thenComparing(Comparator)Comparator` — chains two comparators.
/// Stores primary in `fields[0]`, secondary in `fields[1]`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_then_comparing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let primary = extract_slot_arg(args, 0);
    let secondary = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ThenComparingComparator".to_string(), 2);
    heap.get_mut(r)?.fields[0] = primary;
    heap.get_mut(r)?.fields[1] = secondary;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `ThenComparingComparator.compare(O,O)I` — runs primary then secondary.
pub(crate) fn native_then_comparing_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let mut a = extract_slot_arg(args, 1);
    let mut b = extract_slot_arg(args, 2);
    let primary = extract_first_field_arg(heap, this_ref)?;
    let mut secondary = extract_field_arg(heap, this_ref, 1)?;
    // `secondary`, `a`, and `b` are held across the primary comparator's
    // `compare` invoke (a GC point) and reused for the tie-break invoke.
    // Without pinning, a relocating GC inside the primary callback would leave
    // them stale before the secondary comparator runs.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut secondary);
    scope.pin_slot(&mut a);
    scope.pin_slot(&mut b);
    // Invoke primary.compare(a, b)
    let result = invoke_comparator(primary, a, b, heap, out, ops)?;
    if result != 0 {
        drop(scope);
        return Ok(Some(Slot::Int(result)));
    }
    // Tie-break with secondary
    let result2 = invoke_comparator(secondary, a, b, heap, out, ops)?;
    drop(scope);
    Ok(Some(Slot::Int(result2)))
}
/// Native: `AndPredicate.test(O)Z` — both predicates must return true.
pub(crate) fn native_and_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let mut elem = extract_slot_arg(args, 1);
    let left = extract_first_field_arg(heap, this_ref)?;
    let mut right = extract_field_arg(heap, this_ref, 1)?;
    // `right` and `elem` are held across the left predicate's `test` invoke
    // (a GC point) and reused for the right predicate. Pin them so a relocating
    // GC inside the left callback cannot leave them stale.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut right);
    scope.pin_slot(&mut elem);
    let la = invoke_predicate_test(left, elem, heap, out, ops)?;
    if !la {
        drop(scope);
        return Ok(Some(Slot::Int(0)));
    }
    let rb = invoke_predicate_test(right, elem, heap, out, ops)?;
    drop(scope);
    Ok(Some(Slot::Int(i32::from(rb))))
}
/// Native: `OrPredicate.test(O)Z` — either predicate returning true is sufficient.
pub(crate) fn native_or_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let mut elem = extract_slot_arg(args, 1);
    let left = extract_first_field_arg(heap, this_ref)?;
    let mut right = extract_field_arg(heap, this_ref, 1)?;
    // `right` and `elem` are held across the left predicate's `test` invoke
    // (a GC point) and reused for the right predicate. Pin them so a relocating
    // GC inside the left callback cannot leave them stale.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut right);
    scope.pin_slot(&mut elem);
    let la = invoke_predicate_test(left, elem, heap, out, ops)?;
    if la {
        drop(scope);
        return Ok(Some(Slot::Int(1)));
    }
    let rb = invoke_predicate_test(right, elem, heap, out, ops)?;
    drop(scope);
    Ok(Some(Slot::Int(i32::from(rb))))
}
/// Native: `NegatedPredicate.test(O)Z` — inverts the wrapped predicate.
pub(crate) fn native_negated_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let original = extract_first_field_arg(heap, this_ref)?;
    let result = invoke_predicate_test(original, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(!result))))
}
/// Native: `AndThenFunction.apply(O)O` — applies first then second.
pub(crate) fn native_and_then_function_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let input = extract_slot_arg(args, 1);
    let first = extract_first_field_arg(heap, this_ref)?;
    let mut second = extract_field_arg(heap, this_ref, 1)?;
    // `second` is held across the first function's `apply` invoke (a GC point)
    // and only consumed by the second-stage apply. Pin it so a relocating GC
    // inside the first callback cannot leave it stale.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut second);
    let mid = invoke_function_apply(first, input, heap, out, ops)?;
    let result = invoke_function_apply(second, mid, heap, out, ops)?;
    drop(scope);
    Ok(Some(result))
}
/// Native: `AndThenConsumer.accept(O)V` — runs first then second consumer.
pub(crate) fn native_and_then_consumer_accept(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let mut arg = extract_slot_arg(args, 1);
    let first = extract_first_field_arg(heap, this_ref)?;
    let mut second = extract_field_arg(heap, this_ref, 1)?;
    // `second` and `arg` are held across the first consumer's `accept` invoke
    // (a GC point) and reused for the second consumer. Pin them so a relocating
    // GC inside the first callback cannot leave them stale.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut second);
    scope.pin_slot(&mut arg);
    invoke_consumer_accept(first, arg, heap, out, ops)?;
    invoke_consumer_accept(second, arg, heap, out, ops)?;
    drop(scope);
    Ok(None)
}
/// Native: `ComposeFunction.apply(O)O` — applies inner then outer.
pub(crate) fn native_compose_function_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let input = extract_slot_arg(args, 1);
    let mut outer = extract_first_field_arg(heap, this_ref)?;
    let inner = extract_field_arg(heap, this_ref, 1)?;
    // `outer` is held across the inner function's `apply` invoke (a GC point)
    // and only consumed by the outer-stage apply. Pin it so a relocating GC
    // inside the inner callback cannot leave it stale.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut outer);
    let mid = invoke_function_apply(inner, input, heap, out, ops)?;
    let result = invoke_function_apply(outer, mid, heap, out, ops)?;
    drop(scope);
    Ok(Some(result))
}
/// Native: `BiFunction.andThen(Function)BiFunction` — returns `BiFunctionAndThen` proxy.
pub(crate) fn native_bifunction_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bifunction = extract_slot_arg(args, 0);
    let after = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/BiFunctionAndThen".to_string(), 2);
    heap.get_mut(r)?.fields[0] = bifunction;
    heap.get_mut(r)?.fields[1] = after;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `BiFunctionAndThen.apply(Object,Object)Object` — calls wrapped bifunction then after.
pub(crate) fn native_bifunction_and_then_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let bifunction = extract_first_field_arg(heap, this_ref)?;
    let mut after = extract_field_arg(heap, this_ref, 1)?;
    let Slot::Reference(Some(bf_ref)) = bifunction else {
        return Ok(Some(Slot::Reference(None)));
    };
    let bf_class = heap.get(bf_ref)?.class_name.clone();
    // Pin the `after` function, which is held across the wrapped bifunction's
    // `apply` invoke and only consumed at the following `invoke_function_apply`.
    // Without this a relocating GC inside the first callback leaves `after`
    // stale before the second stage runs it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut after);
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
    drop(scope);
    Ok(Some(invoke_function_apply(after, mid, heap, out, ops)?))
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    // Object elements + object receiver held across each callback; produced
    // longs are primitives. Pin the receiver slot and the element snapshot.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    scope.pin_slots(&mut elems);
    let mut values = Vec::with_capacity(elems.len());
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
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
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
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
    let sum: i64 = heap.get(stream_ref)?.fields[1..=size]
        .iter()
        .map(|s| match s {
            Slot::Long(n) => *n,
            Slot::Int(n) => i64::from(*n),
            _ => 0,
        })
        .sum();
    Ok(Some(Slot::Long(sum)))
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut elems = stream_elements(heap, stream_ref)?;
    // Object elements + object receiver held across each callback; produced
    // doubles are primitives. Pin the receiver slot and the element snapshot.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    scope.pin_slots(&mut elems);
    let mut values = Vec::with_capacity(elems.len());
    #[allow(clippy::needless_range_loop)]
    for i in 0..elems.len() {
        let elem = elems[i];
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
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, values,
    )))))
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
        .filter_map(|s| {
            if let Slot::Long(n) = s {
                Some(*n)
            } else {
                None
            }
        })
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
    let mut fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Long(identity)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = long_stream_elems(heap, r);
    let mut acc = identity;
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
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
    drop(scope);
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
        heap.get_mut(stream_ref)?
            .fields
            .push(Slot::Reference(Some(boxed_ref)));
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut pred_slot);
    for v in elems {
        let result = ops.invoke(
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
    drop(scope);
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(J)J",
            vec![fn_slot, Slot::Long(v)],
        )?;
        result.push(match r {
            Some(Slot::Long(n)) => n,
            Some(Slot::Int(n)) => i64::from(n),
            _ => 0,
        });
    }
    drop(scope);
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
    let mut consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(None);
    };
    let elems = long_stream_elems(heap, r);
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut consumer_slot);
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
    drop(scope);
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(J)I",
            vec![fn_slot, Slot::Long(v)],
        )?;
        result.push(match r {
            Some(Slot::Int(n)) => n,
            _ => 0,
        });
    }
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
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
        .filter_map(|s| {
            if let Slot::Double(d) = s {
                Some(*d)
            } else {
                None
            }
        })
        .collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, values,
    )))))
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
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap,
        vec![v],
    )))))
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut pred_slot);
    for v in elems {
        let result = ops.invoke(
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
    drop(scope);
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(D)D",
            vec![fn_slot, Slot::Double(v)],
        )?;
        result.push(match r {
            Some(Slot::Double(d)) => d,
            Some(Slot::Float(f)) => f64::from(f),
            Some(Slot::Int(n)) => f64::from(n),
            _ => 0.0,
        });
    }
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, result,
    )))))
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
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, values,
    )))))
}
/// Native: `OptionalLong.getAsLong()J`
pub(crate) fn native_optional_long_get_as_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    if !present {
        return Err(Error::MethodNotFound {
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
/// Native: `OptionalLong.isPresent()Z`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_long_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}
/// Native: `Collectors.summingInt(ToIntFunction)Collector` — returns a `SummingIntCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_summing_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/SummingIntCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.averagingInt(ToIntFunction)Collector` — returns an `AveragingIntCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_averaging_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/AveragingIntCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut pred_slot);
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
            return Ok(Some(Slot::Int(1)));
        }
    }
    drop(scope);
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut pred_slot);
    for v in elems {
        let result = ops.invoke(
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
    drop(scope);
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut pred_slot);
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
            return Ok(Some(Slot::Int(0)));
        }
    }
    drop(scope);
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(I)J",
            vec![fn_slot, Slot::Int(v)],
        )?;
        result.push(match r {
            Some(Slot::Long(n)) => n,
            Some(Slot::Int(n)) => i64::from(n),
            _ => 0,
        });
    }
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut pred_slot);
    for v in elems {
        let result = ops.invoke(
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
    drop(scope);
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut pred_slot);
    for v in elems {
        let result = ops.invoke(
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
    drop(scope);
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut pred_slot);
    for v in elems {
        let result = ops.invoke(
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
    drop(scope);
    Ok(Some(Slot::Int(1)))
}
/// Native: `ComparingLongComparator.compare(O,O)I` — calls `fn.applyAsLong(o)` for each element.
pub(crate) fn native_comparing_long_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let mut fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = extract_slot_arg(args, 1);
    let mut b = extract_slot_arg(args, 2);
    // Pin the comparator receiver (`fn_slot`, re-passed to both `applyAsLong`
    // invokes) and the second key `b`, held across the first invoke. Without
    // this a relocating GC inside the first callback leaves them stale before
    // the second invoke reads them.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    scope.pin_slot(&mut b);
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
    drop(scope);
    let result = match (ka, kb) {
        (Slot::Long(la), Slot::Long(lb)) => la.cmp(&lb) as i32,
        (Slot::Int(ia), Slot::Int(ib)) => ia.cmp(&ib) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(result)))
}
/// Native: `Collectors.toUnmodifiableList()Collector` (Java 10) —
/// returns the same `ToListCollector` sentinel; our interpreter treats all lists as modifiable.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_unmodifiable_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/ToUnmodifiableListCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.toUnmodifiableSet()Collector` (Java 10) —
/// returns the same `ToSetCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_unmodifiable_set(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/ToSetCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    for v in elems {
        let sub = ops.invoke(
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
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    for v in elems {
        let sub = ops.invoke(
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
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
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
/// Native: `Collectors.groupingBy(Function, Collector)Collector` — 2-arg version with downstream.
/// Creates a `duke/util/GroupingBy2Collector` with `fields[0]`=keyFn, `fields[1]`=downstream.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_grouping_by_2(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let downstream_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/GroupingBy2Collector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = fn_slot;
    heap.get_mut(r)?.fields[1] = downstream_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.mapping(Function, Collector)Collector` — transforms elements before
/// feeding to a downstream collector.  Creates a `duke/util/MappingCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_mapping(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let mapper_slot = extract_slot_arg(args, 0);
    let downstream_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/MappingCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = mapper_slot;
    heap.get_mut(r)?.fields[1] = downstream_slot;
    Ok(Some(Slot::Reference(Some(r))))
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
    let mut consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(None);
    };
    let elems = double_stream_elems(heap, r);
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut consumer_slot);
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
    drop(scope);
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut pred_slot);
    for v in elems {
        let result = ops.invoke(
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
    drop(scope);
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut pred_slot);
    for v in elems {
        let result = ops.invoke(
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
    drop(scope);
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut pred_slot);
    for v in elems {
        let result = ops.invoke(
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
    drop(scope);
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
    let mut fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Double(identity)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = double_stream_elems(heap, r);
    let mut acc = identity;
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
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
    drop(scope);
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        let opt_ref = make_optional_double_val(heap, None);
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = double_stream_elems(heap, r);
    if elems.is_empty() {
        return Ok(Some(Slot::Reference(Some(make_optional_double_val(
            heap, None,
        )))));
    }
    let mut acc = elems[0];
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
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
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_optional_double_val(
        heap,
        Some(acc),
    )))))
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    for v in elems {
        let sub = ops.invoke(
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
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, result,
    )))))
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(D)I",
            vec![fn_slot, Slot::Double(v)],
        )?;
        result.push(match r {
            Some(Slot::Int(n)) => n,
            _ => 0,
        });
    }
    drop(scope);
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(D)J",
            vec![fn_slot, Slot::Double(v)],
        )?;
        result.push(match r {
            Some(Slot::Long(n)) => n,
            Some(Slot::Int(n)) => i64::from(n),
            _ => 0,
        });
    }
    drop(scope);
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
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, deduped,
    )))))
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
        heap.get_mut(stream_ref)?
            .fields
            .push(Slot::Reference(Some(boxed_ref)));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
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
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
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
    let mut fn_slot = extract_slot_arg(args, 1);
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
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
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
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_optional_long(
        heap,
        Some(acc),
    )))))
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
    let mut fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    // Pin the callback receiver: it is re-passed to `ops.invoke` on every
    // iteration, but native args live in a Copy `Vec<Slot>` the collector
    // never scans, so a relocating GC inside the callback would otherwise
    // leave this ref stale before the next iteration re-passes it.
    let mut scope = NativeRootScope::new();
    scope.pin_slot(&mut fn_slot);
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(J)D",
            vec![fn_slot, Slot::Long(v)],
        )?;
        result.push(match r {
            Some(Slot::Double(d)) => d,
            Some(Slot::Float(f)) => f64::from(f),
            Some(Slot::Int(n)) => f64::from(n),
            _ => 0.0,
        });
    }
    drop(scope);
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, result,
    )))))
}
/// Native: `Collectors.summingLong(ToLongFunction)Collector` — returns a `SummingLongCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_summing_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/SummingLongCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.averagingDouble(ToDoubleFunction)Collector` — returns an `AveragingDoubleCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_averaging_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/AveragingDoubleCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.reducing(U, Function, BinaryOperator)` — returns a
/// `ReducingMappingCollector` with `fields[0]`=identity, `fields[1]`=mapper, `fields[2]`=op.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_reducing_mapping(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let identity_slot = extract_slot_arg(args, 0);
    let mapper_slot = extract_slot_arg(args, 1);
    let op_slot = extract_slot_arg(args, 2);
    let r = heap.allocate("duke/util/ReducingMappingCollector".to_string(), 3);
    heap.get_mut(r)?.fields[0] = identity_slot;
    heap.get_mut(r)?.fields[1] = mapper_slot;
    heap.get_mut(r)?.fields[2] = op_slot;
    Ok(Some(Slot::Reference(Some(r))))
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
    let mut pred_slot = extract_slot_arg(args, 1);
    let mut next_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Err(Error::NullPointerException);
    };
    let Slot::Reference(Some(next_ref)) = next_slot else {
        return Err(Error::NullPointerException);
    };
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let next_class = heap.get(next_ref)?.class_name.clone();

    // Accumulate into a pinned, heap-allocated result stream rather than a
    // growing Rust `Vec` (which pinning cannot follow across reallocation).
    // `current` is passed to the predicate, then read again (pushed) AFTER that
    // callback's GC, so it must be pinned; both callback receiver slots are
    // re-passed every iteration.
    let mut out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(0);
    let mut current = seed;
    let mut scope = NativeRootScope::new();
    scope.pin_ref(&mut out_ref);
    scope.pin_slot(&mut pred_slot);
    scope.pin_slot(&mut next_slot);
    scope.pin_slot(&mut current);
    for _ in 0..10_000usize {
        let test = ops.invoke(
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
        heap.get_mut(out_ref)?.fields.push(current);
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
    let size = i32::try_from(heap.get(out_ref)?.fields.len().saturating_sub(1)).unwrap_or(i32::MAX);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(size);
    drop(scope);
    Ok(Some(Slot::Reference(Some(out_ref))))
}