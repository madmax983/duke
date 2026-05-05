/// Native: `HashMap.putAll(Map)V` — copies all entries from the source `HashMap`.
pub(crate) fn native_hashmap_put_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_ref = extract_ref_arg(args, 1)?;
    let fields = heap.get(src_ref)?.fields.clone();
    let mut i = 1;
    while i + 1 < fields.len() {
        let k = fields[i];
        let v = fields[i + 1];
        native_hashmap_put(
            &[Slot::Reference(Some(this_ref)), k, v],
            heap,
            out,
            control,
        )?;
        i += 2;
    }
    Ok(None)
}
/// Native: `HashMap.computeIfAbsent(K, Function)V` — returns existing value or computes and stores it.
pub(crate) fn native_hashmap_compute_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let existing = native_hashmap_get(
        &[Slot::Reference(Some(this_ref)), key],
        heap,
        out,
        control,
    )?;
    if let Some(v) = existing && !matches!(v, Slot::Reference(None)) {
        return Ok(Some(v));
    }
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let computed = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![Slot::Reference(Some(fn_ref)), key],
        )?;
    if let Some(value) = computed && !matches!(value, Slot::Reference(None)) {
        native_hashmap_put(
            &[Slot::Reference(Some(this_ref)), key, value],
            heap,
            out,
            control,
        )?;
        return Ok(Some(value));
    }
    Ok(Some(Slot::Reference(None)))
}
/// Native: `HashMap.forEach(BiConsumer)V` — iterates key-value pairs, invoking `accept(k, v)`.
pub(crate) fn native_hashmap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let consumer_ref = extract_ref_arg(args, 1)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let pairs: Vec<(Slot, Slot)> = (0..size)
        .map(|i| {
            let key = heap
                .get(this_ref)
                .map_or(Slot::Reference(None), |o| o.fields[1 + i * 2]);
            let val = heap
                .get(this_ref)
                .map_or(Slot::Reference(None), |o| o.fields[2 + i * 2]);
            (key, val)
        })
        .collect();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for (key, val) in pairs {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;Ljava/lang/Object;)V",
            vec![Slot::Reference(Some(consumer_ref)), key, val],
        )?;
    }
    let _ = control;
    Ok(None)
}
/// Native: `HashMap.replaceAll(BiFunction<K,V,V>) -> void`
/// Replaces each value with the result of applying the function to (key, value).
pub(crate) fn native_hashmap_replace_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_ref = extract_ref_arg(args, 1)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let keys: Vec<Slot> = (0..size)
        .map(|i| {
            heap.get(this_ref).map_or(Slot::Reference(None), |o| o.fields[1 + i * 2])
        })
        .collect();
    for (i, key) in keys.iter().enumerate() {
        let old_val = heap
            .get(this_ref)
            .map_or(Slot::Reference(None), |o| o.fields[2 + i * 2]);
        let new_val = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                vec![Slot::Reference(Some(fn_ref)), * key, old_val],
            )?;
        if let Some(v) = new_val {
            heap.get_mut(this_ref)?.fields[2 + i * 2] = v;
        }
    }
    let _ = control;
    Ok(None)
}
/// Native: `HashMap.keySet()` — returns a new `HashSet` containing all keys.
pub(crate) fn native_hashmap_key_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields_len = heap.get(this_ref)?.fields.len();
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    let mut i = 1usize;
    while i < fields_len {
        let key = heap.get(this_ref)?.fields[i];
        native_hashset_add(&[Slot::Reference(Some(set_ref)), key], heap, out, control)?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}
/// Native: `HashMap.values()` — returns a new `ArrayList` containing all values.
/// `HashMap` fields: `[size, key0, val0, key1, val1, ...]`; values are at even indices 2, 4, 6, ...
pub(crate) fn native_hashmap_values(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields_len = heap.get(this_ref)?.fields.len();
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    let mut i = 2usize;
    while i < fields_len {
        let val = heap.get(this_ref)?.fields[i];
        native_arraylist_add(
            &[Slot::Reference(Some(list_ref)), val],
            heap,
            out,
            control,
        )?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}
/// Native: `HashMap.entrySet()` — returns a new `HashSet` of `java/util/Map$Entry` objects.
pub(crate) fn native_hashmap_entry_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields_len = heap.get(this_ref)?.fields.len();
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    let mut i = 1usize;
    while i + 1 < fields_len {
        let key = heap.get(this_ref)?.fields[i];
        let val = heap.get(this_ref)?.fields[i + 1];
        let entry_ref = heap.allocate("java/util/Map$Entry".to_string(), 2);
        {
            let entry_obj = heap.get_mut(entry_ref)?;
            entry_obj.fields[0] = key;
            entry_obj.fields[1] = val;
        }
        native_hashset_add(
            &[Slot::Reference(Some(set_ref)), Slot::Reference(Some(entry_ref))],
            heap,
            out,
            control,
        )?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}
/// Native: `HashMap.putIfAbsent(K,V)Object` — inserts only if key is absent; returns existing or null.
pub(crate) fn native_hashmap_put_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let val = extract_slot_arg(args, 2);
    let i_opt = find_hashmap_entry_index(&heap.get(this_ref)?.fields, &key, heap);
    if let Some(i) = i_opt {
        return Ok(Some(heap.get(this_ref)?.fields[i + 1]));
    }
    let obj = heap.get_mut(this_ref)?;
    if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
        *sz += 1;
    }
    obj.fields.push(key);
    obj.fields.push(val);
    Ok(Some(Slot::Reference(None)))
}
/// Native: `HashMap.clear()V` — removes all entries.
pub(crate) fn native_hashmap_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields.truncate(1);
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}
/// Native: `HashMap.containsValue(Object)Z` — returns 1 if any entry has this value.
pub(crate) fn native_hashmap_contains_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let mut i = 2usize;
    loop {
        let len = heap.get(this_ref)?.fields.len();
        if i >= len {
            break;
        }
        let slot = heap.get(this_ref)?.fields[i];
        if slots_equal(&slot, &target, heap) {
            return Ok(Some(Slot::Int(1)));
        }
        i += 2;
    }
    Ok(Some(Slot::Int(0)))
}
/// Helper: find key index in `HashMap` fields (`fields[0]`=size, `fields[1,3,5..]`=keys, `fields[2,4,6..]`=vals).
fn hashmap_find_key(fields: &[Slot], key: Slot, heap: &duke_gc::Heap) -> Option<usize> {
    let size = match fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return None,
    };
    for i in 0..size {
        let key_slot = fields.get(1 + i * 2)?;
        if slots_equal(key_slot, &key, heap) {
            return Some(1 + i * 2);
        }
    }
    None
}
/// Native: `HashMap.compute(K, BiFunction)V` — compute new value from old (possibly null).
pub(crate) fn native_hashmap_compute(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let fields = &heap.get(this_ref)?.fields;
    let old_value = hashmap_find_key(fields, key, heap)
        .and_then(|ki| fields.get(ki + 1).copied())
        .unwrap_or(Slot::Reference(None));
    let new_value = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, key, old_value],
        )?;
    let is_null = matches!(new_value, None | Some(Slot::Reference(None)));
    let new_val = new_value.unwrap_or(Slot::Reference(None));
    let fields2 = &heap.get(this_ref)?.fields;
    if let Some(ki) = hashmap_find_key(fields2, key, heap) {
        if is_null {
            let fields3 = &mut heap.get_mut(this_ref)?.fields;
            fields3.remove(ki + 1);
            fields3.remove(ki);
            let size = match fields3.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            if let Some(first) = fields3.first_mut() {
                *first = Slot::Int(size - 1);
            }
        } else {
            heap.get_mut(this_ref)?.fields[ki + 1] = new_val;
        }
    } else if !is_null {
        let size = match heap.get(this_ref)?.fields.first().copied() {
            Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
            _ => 0,
        };
        heap.get_mut(this_ref)?.fields.push(key);
        heap.get_mut(this_ref)?.fields.push(new_val);
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(
            i32::try_from(size + 1).unwrap_or(i32::MAX),
        );
    }
    Ok(Some(new_val))
}
/// Native: `HashMap.merge(K, V, BiFunction)V` — put V if absent, else merge with `BiFunction`.
pub(crate) fn native_hashmap_merge(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let new_val_slot = extract_slot_arg(args, 2);
    let fn_slot = extract_slot_arg(args, 3);
    let fields = &heap.get(this_ref)?.fields;
    let old_ki = hashmap_find_key(fields, key, heap);
    if let Some(ki) = old_ki {
        let old_value = fields.get(ki + 1).copied().unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(old_value));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let merged = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                vec![fn_slot, old_value, new_val_slot],
            )?;
        let merged_raw = merged.unwrap_or(Slot::Reference(None));
        let merged_val = box_primitive_slot(merged_raw, heap);
        heap.get_mut(this_ref)?.fields[ki + 1] = merged_val;
        Ok(Some(merged_val))
    } else {
        let size = match heap.get(this_ref)?.fields.first().copied() {
            Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
            _ => 0,
        };
        heap.get_mut(this_ref)?.fields.push(key);
        heap.get_mut(this_ref)?.fields.push(new_val_slot);
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(
            i32::try_from(size + 1).unwrap_or(i32::MAX),
        );
        Ok(Some(new_val_slot))
    }
}
pub(crate) fn native_hashmap_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.is_empty() {
        obj.fields.push(Slot::Int(0));
    } else {
        obj.fields[0] = Slot::Int(0);
    }
    Ok(None)
}
/// Native: `HashMap.put(Object, Object)Object` — inserts or updates a key-value pair.
/// Returns the old value if the key was already present, or null if it is new.
pub(crate) fn native_hashmap_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let val = extract_slot_arg(args, 2);
    let i_opt = find_hashmap_entry_index(&heap.get(this_ref)?.fields, &key, heap);
    if let Some(i) = i_opt {
        let old = heap.get(this_ref)?.fields[i + 1];
        heap.get_mut(this_ref)?.fields[i + 1] = val;
        return Ok(Some(old));
    }
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(Error::NullPointerException),
    }
    obj.fields.push(key);
    obj.fields.push(val);
    Ok(Some(Slot::Reference(None)))
}
/// Native: `HashMap.get(Object)Object` — returns value for key, or null if absent.
pub(crate) fn native_hashmap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fields = &heap.get(this_ref)?.fields;
    Ok(
        Some(
            find_hashmap_entry_index(fields, &key, heap)
                .map_or(Slot::Reference(None), |i| fields[i + 1]),
        ),
    )
}
/// Native: `HashMap.containsKey(Object)Z` — returns 1 if key present, 0 otherwise.
pub(crate) fn native_hashmap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fields = &heap.get(this_ref)?.fields;
    if find_hashmap_entry_index(fields, &key, heap).is_some() {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}
/// Native: `HashMap.size()I` — returns entry count from fields\[0\].
pub(crate) fn native_hashmap_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Ok(Some(Slot::Int(*n))),
        _ => Ok(Some(Slot::Int(0))),
    }
}
/// Native: `HashMap.remove(Object)Object` — removes a key-value pair, returns old value or null.
/// Uses swap-remove (swaps target pair with last pair) for O(1) deletion.
pub(crate) fn native_hashmap_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let i_opt = find_hashmap_entry_index(&heap.get(this_ref)?.fields, &key, heap);
    if let Some(i) = i_opt {
        let old_val = heap.get(this_ref)?.fields[i + 1];
        let obj = heap.get_mut(this_ref)?;
        let last_val_idx = obj.fields.len() - 1;
        let last_key_idx = obj.fields.len() - 2;
        obj.fields.swap(i + 1, last_val_idx);
        obj.fields.swap(i, last_key_idx);
        obj.fields.truncate(obj.fields.len() - 2);
        match obj.fields.first_mut() {
            Some(Slot::Int(sz)) => *sz -= 1,
            _ => return Err(Error::NullPointerException),
        }
        Ok(Some(old_val))
    } else {
        Ok(Some(Slot::Reference(None)))
    }
}
/// Native: `HashMap.isEmpty()Z` — returns 1 if size == 0, else 0.
pub(crate) fn native_hashmap_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) => Ok(Some(Slot::Int(1))),
        Some(Slot::Int(_)) => Ok(Some(Slot::Int(0))),
        _ => Ok(Some(Slot::Int(1))),
    }
}
/// Native: `HashMap.getOrDefault(Object, Object)Object` — returns value for key, or default if absent.
pub(crate) fn native_hashmap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let default = extract_slot_arg(args, 2);
    let fields = &heap.get(this_ref)?.fields;
    Ok(
        Some(
            find_hashmap_entry_index(fields, &key, heap)
                .map_or(default, |i| fields[i + 1]),
        ),
    )
}
/// Native: `HashMap.replace(Object, Object)Object` — updates value for existing key,
/// returns the old value or null if key was absent.
pub(crate) fn native_hashmap_replace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let new_val = extract_slot_arg(args, 2);
    let i_opt = find_hashmap_entry_index(&heap.get(this_ref)?.fields, &key, heap);
    if let Some(i) = i_opt {
        let old = heap.get(this_ref)?.fields[i + 1];
        heap.get_mut(this_ref)?.fields[i + 1] = new_val;
        Ok(Some(old))
    } else {
        Ok(Some(Slot::Reference(None)))
    }
}
/// Native: `HashMap.computeIfPresent(K, BiFunction<K,V,V>) -> V`
/// If key is present, applies the function to (key, `old_value`); replaces with result.
/// If function returns null, removes the key.
pub(crate) fn native_hashmap_compute_if_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let existing = native_hashmap_get(
        &[Slot::Reference(Some(this_ref)), key],
        heap,
        out,
        control,
    )?;
    let old_value = match existing {
        Some(v) if !matches!(v, Slot::Reference(None)) => v,
        _ => return Ok(Some(Slot::Reference(None))),
    };
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let new_value = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![Slot::Reference(Some(fn_ref)), key, old_value],
        )?;
    match new_value {
        Some(v) if !matches!(v, Slot::Reference(None)) => {
            native_hashmap_put(
                &[Slot::Reference(Some(this_ref)), key, v],
                heap,
                out,
                control,
            )?;
            Ok(Some(v))
        }
        _ => {
            native_hashmap_remove(
                &[Slot::Reference(Some(this_ref)), key],
                heap,
                out,
                control,
            )?;
            Ok(Some(Slot::Reference(None)))
        }
    }
}
/// Native: `HashMap.remove(Object, Object) -> boolean`
/// Conditional remove: only removes if key is present AND value equals the provided value.
pub(crate) fn native_hashmap_remove_key_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let expected_val = extract_slot_arg(args, 2);
    let fields = heap.get(this_ref)?.fields.clone();
    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let actual_val = fields[i + 1];
        if slots_equal(&actual_val, &expected_val, heap) {
            native_hashmap_remove(
                &[Slot::Reference(Some(this_ref)), key],
                heap,
                out,
                control,
            )?;
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}
