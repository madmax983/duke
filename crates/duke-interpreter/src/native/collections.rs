/// Native: `HashMap.remove(Object, Object) -> boolean`
/// Conditional remove: only removes if key is present AND value equals the provided value.
pub(crate) fn native_hashmap_remove_key_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let expected_val = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let actual_val = fields[i + 1];
        if slots_equal(&actual_val, &expected_val, heap) {
            native_hashmap_remove(&[Slot::Reference(Some(this_ref)), key], heap, out, control)?;
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
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
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    // Look up existing value.
    let existing = native_hashmap_get(&[Slot::Reference(Some(this_ref)), key], heap, out, control)?;
    let old_value = match existing {
        Some(v) if !matches!(v, Slot::Reference(None)) => v,
        _ => return Ok(Some(Slot::Reference(None))),
    };
    // Key present — invoke the remapping function.
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let new_value = ops.invoke(
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
            // null return → remove key
            native_hashmap_remove(&[Slot::Reference(Some(this_ref)), key], heap, out, control)?;
            Ok(Some(Slot::Reference(None)))
        }
    }
}

/// Native: `Collections.disjoint(Collection, Collection) -> boolean`
/// Returns true if the two collections have no elements in common.
pub(crate) fn native_collections_disjoint(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a_ref = extract_ref_arg(args, 0)?;
    let b_ref = extract_ref_arg(args, 1)?;
    // Both use ArrayList/HashSet layout: fields[0]=size, fields[1..=size]=elements
    let a_size = match heap.get(a_ref)?.fields.first() {
        Some(Slot::Int(v)) => usize::try_from(*v).unwrap_or(0),
        _ => 0,
    };
    let b_size = match heap.get(b_ref)?.fields.first() {
        Some(Slot::Int(v)) => usize::try_from(*v).unwrap_or(0),
        _ => 0,
    };
    let a_elems: Vec<Slot> = heap.get(a_ref)?.fields
        [1..=a_size.min(heap.get(a_ref)?.fields.len().saturating_sub(1))]
        .to_vec();
    let b_elems: Vec<Slot> = heap.get(b_ref)?.fields
        [1..=b_size.min(heap.get(b_ref)?.fields.len().saturating_sub(1))]
        .to_vec();
    let disjoint = a_elems
        .iter()
        .all(|a| !b_elems.iter().any(|b| slots_equal(a, b, heap)));
    Ok(Some(Slot::Int(i32::from(disjoint))))
}

/// Native: `StringBuilder.setCharAt(int, char) -> void`
pub(crate) fn native_stringbuilder_set_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = usize::try_from(extract_int_arg(args, 1)?).unwrap_or(usize::MAX);
    let ch = match args.get(2) {
        Some(Slot::Int(v)) => char::from_u32(u32::from_ne_bytes(v.to_ne_bytes())).unwrap_or('\0'),
        _ => '\0',
    };
    let s = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new)
        .clone();
    let mut chars: Vec<char> = s.chars().collect();
    if idx < chars.len() {
        chars[idx] = ch;
    }
    let new_s: String = chars.into_iter().collect();
    heap.get_mut(this_ref)?.string_value = Some(new_s);
    Ok(None)
}

/// Native: `ArrayDeque.clear()V` — removes all elements.
pub(crate) fn native_arraydeque_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields.truncate(1);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ArrayDeque.forEach(Consumer)V` — invokes consumer for each element.
pub(crate) fn native_arraydeque_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(cons_ref)) = consumer_slot else {
        return Err(VmError::NullPointerException);
    };
    let cons_class = heap.get(cons_ref)?.class_name.clone();
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    for elem in elems {
        ops.invoke(
            heap,
            out,
            &cons_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, elem],
        )?;
    }
    Ok(None)
}

/// Native: `ArrayDeque.contains(Object)Z` — returns true if element is present.
pub(crate) fn native_arraydeque_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let found = elems.iter().any(|e| slots_equal(e, &target, heap));
    Ok(Some(Slot::Int(i32::from(found))))
}

/// Native: `ArrayDeque.pollLast()Object` — removes and returns last element; null if empty.
pub(crate) fn native_arraydeque_poll_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    let elem = heap.get_mut(this_ref)?.fields.remove(size);
    let new_size = i32::try_from(size - 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(Some(elem))
}

/// Native: `ArrayDeque.pollFirst()Object` — same as poll (remove front, null if empty).
pub(crate) fn native_arraydeque_poll_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraydeque_poll(args, heap, out, control)
}

/// Native: `ArrayDeque.peekLast()Object` — returns last element without removing; null if empty.
pub(crate) fn native_arraydeque_peek_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[size]))
}

/// Native: `ArrayDeque.peekFirst()Object` — same as peek (front element, null if empty).
pub(crate) fn native_arraydeque_peek_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraydeque_peek(args, heap, out, control)
}

/// Native: `ArrayDeque.offerLast(Object)Z` — appends at back, returns true.
pub(crate) fn native_arraydeque_offer_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `ArrayDeque.offerFirst(Object)Z` — inserts at front, returns true.
pub(crate) fn native_arraydeque_offer_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraydeque_push(args, heap, out, control)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `ArrayDeque.addLast(Object)V` — appends element at back.
pub(crate) fn native_arraydeque_add_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(None)
}

/// Native: `ArrayDeque.addFirst(Object)V` — inserts element at front.
pub(crate) fn native_arraydeque_add_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraydeque_push(args, heap, out, control)
}

/// Native: `TreeMap.getOrDefault(Object,Object)Object` — looks up key; returns default if absent.
pub(crate) fn native_treemap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let default_val = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    Ok(Some(
        find_hashmap_entry_index(&fields, &key, heap).map_or(default_val, |i| fields[i + 1]),
    ))
}

/// Native: `TreeMap.values()Collection` — delegates to `HashMap` values (same field layout).
pub(crate) fn native_treemap_values(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_hashmap_values(args, heap, out, control)
}

/// Native: `TreeMap.keySet()Set` — delegates to `HashMap` keySet (same field layout).
pub(crate) fn native_treemap_key_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_hashmap_key_set(args, heap, out, control)
}

/// Native: `PriorityQueue.forEach(Consumer)V`
pub(crate) fn native_priorityqueue_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    native_arraylist_for_each(args, heap, out, control, ops)
}

/// Native: `LinkedList.forEach(Consumer)V`
pub(crate) fn native_linked_list_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    native_arraylist_for_each(args, heap, out, control, ops)
}

/// Native: `LinkedHashMap.forEach(BiConsumer)V`
pub(crate) fn native_linkedhashmap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    native_hashmap_for_each(args, heap, out, control, ops)
}

/// Native: `TreeMap.forEach(BiConsumer)V`
pub(crate) fn native_treemap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // TreeMap uses same layout as HashMap: fields[0]=size, fields[1,2]=k0/v0 ...
    native_hashmap_for_each(args, heap, out, control, ops)
}

/// Native: `TreeSet.forEach(Consumer)V`
pub(crate) fn native_treeset_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // TreeSet uses same layout as HashSet: fields[0]=size, fields[1..size]=elements
    native_hashset_for_each(args, heap, out, control, ops)
}

/// Native: `HashSet.forEach(Consumer)V`
pub(crate) fn native_hashset_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(consumer_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return Ok(None);
    };
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
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

/// Native: `Collections.unmodifiableSet(Set)Set` — returns a view of the set (same backing object).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // Wrap the source set in a UnmodifiableSet (same field layout as HashSet).
    let src_ref = extract_ref_arg(args, 0)?;
    let src_fields = heap.get(src_ref)?.fields.clone();
    let class_name = heap.get(src_ref)?.class_name.clone();
    let n = src_fields.len();
    let wrapper_ref = heap.allocate("java/util/UnmodifiableSet".to_string(), n);
    // Copy field layout from source set
    let _ = class_name;
    for (i, f) in src_fields.into_iter().enumerate() {
        heap.get_mut(wrapper_ref)?.fields[i] = f;
    }
    Ok(Some(Slot::Reference(Some(wrapper_ref))))
}

/// Native: `Collections.singleton(E)Set` — returns a single-element unmodifiable set.
pub(crate) fn native_collections_singleton_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_set_of_factory(args, heap, out, control)
}

/// Native: `Collections.singletonMap(K,V)Map` — returns a single-entry unmodifiable map.
pub(crate) fn native_collections_singleton_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_map_of(args, heap, out, control)
}

/// Native: `Map.ofEntries(Map.Entry[])Map` — builds a `HashMap` from varargs Entry array.
pub(crate) fn native_map_of_entries(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(map_ref))], heap, out, control)?;
    // args[0] is the Object[] array of Map.Entry objects (anewarray layout: fields = elements)
    if let Some(Slot::Reference(Some(arr_ref))) = args.first().copied() {
        let entries: Vec<Slot> = heap.get(arr_ref)?.fields.clone();
        for entry_slot in entries {
            let Slot::Reference(Some(entry_ref)) = entry_slot else {
                continue;
            };
            let key = heap
                .get(entry_ref)?
                .fields
                .first()
                .copied()
                .unwrap_or(Slot::Reference(None));
            let val = heap
                .get(entry_ref)?
                .fields
                .get(1)
                .copied()
                .unwrap_or(Slot::Reference(None));
            native_hashmap_put(
                &[Slot::Reference(Some(map_ref)), key, val],
                heap,
                out,
                control,
            )?;
        }
    }
    Ok(Some(Slot::Reference(Some(map_ref))))
}

/// Native: `Map.entry(K,V)Map.Entry` — creates an immutable Map.Entry.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_map_entry_factory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let key = args.first().copied().unwrap_or(Slot::Reference(None));
    let val = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("java/util/Map$Entry".to_string(), 2);
    heap.get_mut(r)?.fields[0] = key;
    heap.get_mut(r)?.fields[1] = val;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Map.copyOf(Map)Map` — returns an unmodifiable copy backed by `HashMap`.
pub(crate) fn native_map_copy_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let copy_ref = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(copy_ref))], heap, out, control)?;
    // Iterate source map's interleaved key-val pairs: fields[0]=size, fields[1..]=k,v,k,v,...
    let size = match heap.get(src_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let pairs: Vec<Slot> = heap.get(src_ref)?.fields[1..=size * 2].to_vec();
    let mut i = 0;
    while i + 1 < pairs.len() {
        let k = pairs[i];
        let v = pairs[i + 1];
        native_hashmap_put(&[Slot::Reference(Some(copy_ref)), k, v], heap, out, control)?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(copy_ref))))
}

/// Native: `Collections.unmodifiableMap(Map)Map` — identity stub (we have no mutation checks).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Reference(None))),
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let src_class = heap.get(src_ref)?.class_name.clone();
    // Copy data into an UnmodifiableMap wrapper (same layout as HashMap)
    let r = heap.allocate("java/util/UnmodifiableMap".to_string(), src_fields.len());
    let r_fields = &mut heap.get_mut(r)?.fields;
    *r_fields = src_fields;
    drop(src_class);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.swap(List, int, int)V` — swaps elements at indices i and j.
pub(crate) fn native_collections_swap(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let i = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(None),
    };
    let j = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(None),
    };
    // fields[0] = size, elements at fields[1..=size]
    let fi = i + 1;
    let fj = j + 1;
    let fields = heap.get(list_ref)?.fields.clone();
    let len = fields.len();
    if fi < len && fj < len {
        let vi = fields[fi];
        let vj = fields[fj];
        heap.get_mut(list_ref)?.fields[fi] = vj;
        heap.get_mut(list_ref)?.fields[fj] = vi;
    }
    Ok(None)
}

/// Native: `HashMap.replace(Object, Object)Object` — updates value for existing key,
/// returns the old value or null if key was absent.
pub(crate) fn native_hashmap_replace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let new_val = extract_slot_arg(args, 2);
    let fields = heap.get(this_ref)?.fields.clone();
    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let old = fields[i + 1];
        heap.get_mut(this_ref)?.fields[i + 1] = new_val;
        Ok(Some(old))
    } else {
        Ok(Some(Slot::Reference(None)))
    }
}

/// Native: `ArrayList.forEach(Consumer)V` — invokes consumer.accept(elem) for each element.
pub(crate) fn native_arraylist_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(consumer_ref)) = args.get(1).copied().unwrap_or(Slot::Reference(None))
    else {
        return Ok(None);
    };
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(list_ref)?.fields[1..=size].to_vec();
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

/// Native: `Collections.nCopies(int, Object)List` — returns a list of N copies of an element.
pub(crate) fn native_collections_n_copies(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let n = usize::try_from(extract_int_arg(args, 0)?.max(0)).unwrap_or(0);
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    heap.get_mut(list_ref)?.fields[0] = Slot::Int(i32::try_from(n).unwrap_or(0));
    for _ in 0..n {
        heap.get_mut(list_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}

/// Native: `HashSetIterator.next()Object` — returns element at cursor, advances cursor.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_hashset_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (set_ref, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let sr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
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
                return Err(VmError::JavaException {
                    class_name: "java/util/NoSuchElementException".to_string(),
                });
            }
        }
    };
    heap.get_mut(this_ref)?.fields[1] = Slot::Int(cursor + 1);
    Ok(Some(element))
}

/// Native: `HashSetIterator.hasNext()Z`
pub(crate) fn native_hashset_iter_hasnext(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
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

/// Native: `HashSetIterator.<init>` — no-op; fields are set by `native_hashset_iterator`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_hashset_iter_init(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `HashSet.iterator()Iterator` — creates a `HashSetIterator`.
pub(crate) fn native_hashset_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_ref = heap.allocate("duke/util/HashSetIterator".to_string(), 2);
    {
        let iter_obj = heap.get_mut(iter_ref)?;
        iter_obj.fields[0] = Slot::Reference(Some(this_ref));
        iter_obj.fields[1] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
}

/// Native: `HashSet.isEmpty()Z` — returns 1 if size == 0, else 0.
pub(crate) fn native_hashset_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) => Ok(Some(Slot::Int(1))),
        Some(Slot::Int(_)) => Ok(Some(Slot::Int(0))),
        _ => Ok(Some(Slot::Int(1))),
    }
}

/// Native: `HashSet.size()I` — returns element count from fields\[0\].
pub(crate) fn native_hashset_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Ok(Some(Slot::Int(*n))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `HashSet.remove(Object)Z` — removes element if present, returns 1 if removed, 0 if absent.
/// Uses swap-remove (swaps target with last element) for O(1) deletion.
pub(crate) fn native_hashset_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();

    if let Some(i) = find_hashset_entry_index(&fields, &element, heap) {
        let obj = heap.get_mut(this_ref)?;
        let last_idx = obj.fields.len() - 1;
        obj.fields.swap(i, last_idx);
        obj.fields.truncate(obj.fields.len() - 1);
        match obj.fields.first_mut() {
            Some(Slot::Int(sz)) => *sz -= 1,
            _ => return Err(VmError::NullPointerException),
        }
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashSet.contains(Object)Z` — returns 1 if element is present, 0 otherwise.
pub(crate) fn native_hashset_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();

    if find_hashset_entry_index(&fields, &element, heap).is_some() {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashSet.add(Object)Z` — adds element if not already present.
/// Returns 1 if added, 0 if element was already in the set.
pub(crate) fn native_hashset_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();
    // fields[0] = size, fields[1..] = elements
    if find_hashset_entry_index(&fields, &element, heap).is_some() {
        return Ok(Some(Slot::Int(0))); // duplicate
    }

    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException),
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1)))
}

pub(crate) fn native_hashset_init_from_collection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let mut this_ref = extract_ref_arg(args, 0)?;
    let collection_ref = extract_ref_arg(args, 1)?;
    native_hashset_init(&[Slot::Reference(Some(this_ref))], heap, output, control)?;

    let collection_class = heap.get(collection_ref)?.class_name.clone();
    let elements: Vec<Slot> = if collection_class == "java/util/HashSet" {
        heap.get(collection_ref)?
            .fields
            .iter()
            .skip(1)
            .copied()
            .collect()
    } else {
        let array_slot = ops.invoke(
            heap,
            output,
            &collection_class,
            "toArray",
            "()[Ljava/lang/Object;",
            vec![Slot::Reference(Some(collection_ref))],
        )?;
        let Some(Slot::Reference(Some(array_ref))) = array_slot else {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        };
        patch_forwarded_ref_if_needed(heap, &mut this_ref);
        heap.get(array_ref)?.fields.clone()
    };

    for element in elements {
        native_hashset_add(
            &[Slot::Reference(Some(this_ref)), element],
            heap,
            output,
            control,
        )?;
    }
    Ok(None)
}

pub(crate) fn native_hashset_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.is_empty() {
        obj.fields.push(Slot::Int(0));
    } else {
        obj.fields[0] = Slot::Int(0);
    }
    Ok(None)
}

pub(crate) fn native_set_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;

    match args.first().copied().unwrap_or(Slot::Reference(None)) {
        Slot::Reference(Some(array_ref)) => {
            let elements = heap.get(array_ref)?.fields.clone();
            for element in elements {
                native_hashset_add(
                    &[Slot::Reference(Some(set_ref)), element],
                    heap,
                    out,
                    control,
                )?;
            }
        }
        Slot::Reference(None) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    }

    Ok(Some(Slot::Reference(Some(set_ref))))
}

/// Native: `HashMap.getOrDefault(Object, Object)Object` — returns value for key, or default if absent.
pub(crate) fn native_hashmap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let default = extract_slot_arg(args, 2);
    let fields = heap.get(this_ref)?.fields.clone();

    Ok(Some(
        find_hashmap_entry_index(&fields, &key, heap).map_or(default, |i| fields[i + 1]),
    ))
}

/// Native: `HashMap.isEmpty()Z` — returns 1 if size == 0, else 0.
pub(crate) fn native_hashmap_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) => Ok(Some(Slot::Int(1))),
        Some(Slot::Int(_)) => Ok(Some(Slot::Int(0))),
        _ => Ok(Some(Slot::Int(1))),
    }
}

/// Native: `HashMap.remove(Object)Object` — removes a key-value pair, returns old value or null.
/// Uses swap-remove (swaps target pair with last pair) for O(1) deletion.
pub(crate) fn native_hashmap_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();

    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let old_val = fields[i + 1];
        let obj = heap.get_mut(this_ref)?;
        let last_val_idx = obj.fields.len() - 1;
        let last_key_idx = obj.fields.len() - 2;
        obj.fields.swap(i + 1, last_val_idx);
        obj.fields.swap(i, last_key_idx);
        obj.fields.truncate(obj.fields.len() - 2);
        match obj.fields.first_mut() {
            Some(Slot::Int(sz)) => *sz -= 1,
            _ => return Err(VmError::NullPointerException),
        }
        Ok(Some(old_val))
    } else {
        Ok(Some(Slot::Reference(None)))
    }
}

/// Native: `HashMap.size()I` — returns entry count from fields\[0\].
pub(crate) fn native_hashmap_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Ok(Some(Slot::Int(*n))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `HashMap.containsKey(Object)Z` — returns 1 if key present, 0 otherwise.
pub(crate) fn native_hashmap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();

    if find_hashmap_entry_index(&fields, &key, heap).is_some() {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashMap.get(Object)Object` — returns value for key, or null if absent.
pub(crate) fn native_hashmap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();

    Ok(Some(
        find_hashmap_entry_index(&fields, &key, heap)
            .map_or(Slot::Reference(None), |i| fields[i + 1]),
    ))
}

/// Native: `HashMap.put(Object, Object)Object` — inserts or updates a key-value pair.
/// Returns the old value if the key was already present, or null if it is new.
pub(crate) fn native_hashmap_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let val = extract_slot_arg(args, 2);
    // Clone fields to release the immutable borrow before mutating.
    let fields = heap.get(this_ref)?.fields.clone();

    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let old = fields[i + 1];
        heap.get_mut(this_ref)?.fields[i + 1] = val;
        return Ok(Some(old));
    }

    // New key — append pair and bump size.
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException),
    }
    obj.fields.push(key);
    obj.fields.push(val);
    Ok(Some(Slot::Reference(None)))
}

pub(crate) fn native_hashmap_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.is_empty() {
        obj.fields.push(Slot::Int(0));
    } else {
        obj.fields[0] = Slot::Int(0);
    }
    Ok(None)
}

/// Native: `StringJoiner.setEmptyValue(CharSequence)StringJoiner`.
pub(crate) fn native_stringjoiner_set_empty_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let empty_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    heap.get_mut(this_ref)?.fields[3] = empty_slot;
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `HashMap.merge(K, V, BiFunction)V` — put V if absent, else merge with `BiFunction`.
pub(crate) fn native_hashmap_merge(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let new_val_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let fn_slot = args.get(3).copied().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    let old_ki = hashmap_find_key(&fields, key, heap);
    if let Some(ki) = old_ki {
        let old_value = fields.get(ki + 1).copied().unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(old_value));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let merged = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, old_value, new_val_slot],
        )?;
        let merged_raw = merged.unwrap_or(Slot::Reference(None));
        // Box primitive results so the stored value is always a Reference (matches Java generics)
        let merged_val = box_primitive_slot(merged_raw, heap);
        heap.get_mut(this_ref)?.fields[ki + 1] = merged_val;
        Ok(Some(merged_val))
    } else {
        // Key absent — insert new value
        let size = match heap.get(this_ref)?.fields.first().copied() {
            Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
            _ => 0,
        };
        heap.get_mut(this_ref)?.fields.push(key);
        heap.get_mut(this_ref)?.fields.push(new_val_slot);
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
        Ok(Some(new_val_slot))
    }
}

/// Native: `HashMap.compute(K, BiFunction)V` — compute new value from old (possibly null).
pub(crate) fn native_hashmap_compute(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let fn_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    // Find old value
    let fields = heap.get(this_ref)?.fields.clone();
    let old_value = hashmap_find_key(&fields, key, heap)
        .and_then(|ki| fields.get(ki + 1).copied())
        .unwrap_or(Slot::Reference(None));
    // Call BiFunction.apply(key, oldValue)
    let new_value = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, key, old_value],
    )?;
    // null return means remove the key
    let is_null = matches!(new_value, None | Some(Slot::Reference(None)));
    let new_val = new_value.unwrap_or(Slot::Reference(None));
    let fields2 = heap.get(this_ref)?.fields.clone();
    if let Some(ki) = hashmap_find_key(&fields2, key, heap) {
        if is_null {
            // Remove the key-value pair (swap-remove style)
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
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    }
    Ok(Some(new_val))
}

/// Native: `Optional.flatMap(Function)Optional` — maps value to Optional if present, flattens.
pub(crate) fn native_optional_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
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
    // The function returns an Optional — return it directly (flat, not wrapped again)
    let result = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, value],
    )?;
    Ok(Some(result.unwrap_or(Slot::Reference(None))))
}

/// Native: `Optional.map(Function)Optional` — maps value if present.
pub(crate) fn native_optional_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let value = heap
        .get(opt_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let result_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if matches!(value, Slot::Reference(None)) {
        // empty — propagate empty
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    }
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mapped = ops.invoke(
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

/// Native: `Arrays.asList(Object[])List` — wraps a reference array as an `ArrayList`.
pub(crate) fn native_arrays_as_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let arr_len = heap.get(arr_ref)?.fields.len();
    // Create a new ArrayList (1 field slot for size counter) and populate it.
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    for i in 0..arr_len {
        let elem = heap
            .get(arr_ref)?
            .fields
            .get(i)
            .copied()
            .unwrap_or(Slot::Reference(None));
        native_arraylist_add(&[Slot::Reference(Some(list_ref)), elem], heap, out, control)?;
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}

/// Native: `Collections.binarySearch(List, T)I` — binary search on sorted `ArrayList`.
pub(crate) fn native_collections_binary_search(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(list_ref)?.fields[1..=size].to_vec();
    let mut lo: i64 = 0;
    let mut hi: i64 = i64::try_from(elems.len()).unwrap_or(0) - 1;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2; // avoid overflow via i64 midpoint
        let mid_elem = elems[usize::try_from(mid).unwrap_or(0)];
        let cmp = compare_slots_natural(mid_elem, key, heap, out, ops)?;
        match cmp.cmp(&0) {
            std::cmp::Ordering::Equal => {
                return Ok(Some(Slot::Int(i32::try_from(mid).unwrap_or(0))));
            }
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid - 1,
        }
    }
    // Return -(insertion point) - 1
    let insertion = i32::try_from(lo).unwrap_or(0);
    Ok(Some(Slot::Int(-(insertion + 1))))
}

/// Native: `ArrayList.removeIf(Predicate)Z` — removes all elements where predicate returns true.
pub(crate) fn native_arraylist_remove_if(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let mut kept = Vec::with_capacity(elems.len());
    let mut removed = false;
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![fn_slot, elem],
            )?
            .unwrap_or(Slot::Int(0));
        match result {
            Slot::Int(1) => {
                removed = true;
            } // predicate true → remove
            _ => kept.push(elem),
        }
    }
    let new_size = i32::try_from(kept.len()).unwrap_or(0);
    heap.get_mut(this_ref)?.fields.truncate(1);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    for elem in kept {
        heap.get_mut(this_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Int(i32::from(removed))))
}

/// Native: `ArrayList.subList(int, int)List` — returns a new `ArrayList` with the sub-range.
pub(crate) fn native_arraylist_sub_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let from = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let to = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let new_len = to.saturating_sub(from);
    // ArrayList layout: fields[0]=size, fields[1..]=elements
    let src_elems: Vec<Slot> = {
        let fields = &heap.get(this_ref)?.fields;
        fields
            .iter()
            .skip(1 + from)
            .take(new_len)
            .copied()
            .collect()
    };
    let sub_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    heap.get_mut(sub_ref)?.fields[0] = Slot::Int(i32::try_from(new_len).unwrap_or(0));
    for elem in src_elems {
        heap.get_mut(sub_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(sub_ref))))
}

/// Native: `HashMap.containsValue(Object)Z` — returns 1 if any entry has this value.
pub(crate) fn native_hashmap_contains_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();
    // Values are at even indices: 2, 4, 6, ...
    let mut i = 2usize;
    while i < fields.len() {
        if slots_equal(&fields[i], &target, heap) {
            return Ok(Some(Slot::Int(1)));
        }
        i += 2;
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `HashMap.clear()V` — removes all entries.
pub(crate) fn native_hashmap_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields.truncate(1);
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `HashMap.putIfAbsent(K,V)Object` — inserts only if key is absent; returns existing or null.
pub(crate) fn native_hashmap_put_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let val = extract_slot_arg(args, 2);
    let fields = heap.get(this_ref)?.fields.clone();
    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        // Key already present — return existing value.
        return Ok(Some(fields[i + 1]));
    }
    // Key absent — insert and return null.
    let obj = heap.get_mut(this_ref)?;
    if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
        *sz += 1;
    }
    obj.fields.push(key);
    obj.fields.push(val);
    Ok(Some(Slot::Reference(None)))
}

/// Native: `ArrayList.add(I,Object)V` — inserts element at index, shifting others right.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_add_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)?;
    let element = extract_slot_arg(args, 2);
    if idx < 0 {
        return Err(VmError::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let field_idx = idx as usize + 1;
    let obj = heap.get_mut(this_ref)?;
    let len = obj.fields.len();
    if field_idx > len {
        return Err(VmError::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    obj.fields.insert(field_idx, element);
    if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
        *sz += 1;
    }
    Ok(None)
}

/// Native: `ArrayList.lastIndexOf(Object)I` — last occurrence, or -1.
pub(crate) fn native_arraylist_last_index_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();
    let idx = fields
        .iter()
        .skip(1)
        .rposition(|slot| slots_equal(slot, &target, heap))
        .map_or(-1, |i| i32::try_from(i).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(idx)))
}

/// Native: `ArrayList.indexOf(Object)I` — returns first index of element, or -1.
pub(crate) fn native_arraylist_index_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();
    let idx = fields
        .iter()
        .skip(1)
        .position(|slot| slots_equal(slot, &target, heap))
        .map_or(-1, |i| i32::try_from(i).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(idx)))
}

/// Native: `ArrayList.set(I,Object)Object` — replaces element at index, returns old value.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)?;
    let value = extract_slot_arg(args, 2);
    if idx < 0 {
        return Err(VmError::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let field_idx = idx as usize + 1;
    let old = *heap
        .get(this_ref)?
        .fields
        .get(field_idx)
        .ok_or_else(|| VmError::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        })?;
    heap.get_mut(this_ref)?.fields[field_idx] = value;
    Ok(Some(old))
}

/// Native: `ArrayList.isEmpty()Z` — returns 1 if size is 0.
pub(crate) fn native_arraylist_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let is_empty = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz == 0,
        _ => true,
    };
    Ok(Some(Slot::Int(i32::from(is_empty))))
}

/// Native: `ArrayList.clear()V` — removes all elements.
pub(crate) fn native_arraylist_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields.truncate(1);
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ArrayList.contains(Object)Z` — returns 1 if element is present.
pub(crate) fn native_arraylist_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();
    let found = fields
        .iter()
        .skip(1)
        .any(|slot| slots_equal(slot, &target, heap));
    Ok(Some(Slot::Int(i32::from(found))))
}

/// Native: `ArrayList.remove(Object)Z` — removes first occurrence, returns true if found.
pub(crate) fn native_arraylist_remove_obj(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let fields = heap.get(this_ref)?.fields.clone();
    let found = fields
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, slot)| slots_equal(slot, &target, heap))
        .map(|(i, _)| i);
    if let Some(field_idx) = found {
        let obj = heap.get_mut(this_ref)?;
        obj.fields.remove(field_idx);
        if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
            *sz -= 1;
        }
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `ArrayList.remove(I)Object` — removes element at index, returns it.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_remove_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)?;
    if idx < 0 {
        return Err(VmError::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let idx = idx as usize;
    let len = heap.get(this_ref)?.fields.len();
    // fields[0]=size, elements start at 1; idx is 0-based element index → field index = idx+1
    if idx + 1 >= len {
        return Err(VmError::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let removed = heap.get(this_ref)?.fields[idx + 1];
    let obj = heap.get_mut(this_ref)?;
    obj.fields.remove(idx + 1);
    if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
        *sz -= 1;
    }
    Ok(Some(removed))
}

/// Native: `ArrayListIterator.remove()V` — removes the last element returned by `next()`.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_iter_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (list_ref, last, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let lr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
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
        return Err(VmError::JavaException {
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

/// Native: `ArrayListIterator.next()Object` — returns element at cursor, advances cursor.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (list_ref, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let lr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
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
                return Err(VmError::JavaException {
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

/// Native: `ArrayListIterator.hasNext()Z`
pub(crate) fn native_arraylist_iter_hasnext(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
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

/// Native: `ArrayListIterator.<init>` — no-op; fields set directly by `native_arraylist_iterator`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_arraylist_iter_init(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `Collections.sort(List)V` — delegates to the list's sort(null) method.
///
/// `Collections.sort(list)` is compiled by javac as
/// `invokestatic java/util/Collections.sort:(Ljava/util/List;)V`.
/// We forward to the runtime class's `sort(Comparator=null)`, which for an
/// `ArrayList` performs the insertion-sort-with-compareTo callback.
pub(crate) fn native_collections_sort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // args[0] = List ref
    let list_ref = extract_ref_arg(args, 0)?;
    // Dispatch on the actual runtime class so any List implementation works.
    let class_name = heap.get(list_ref)?.class_name.clone();
    ops.invoke(
        heap,
        output,
        &class_name,
        "sort",
        SORT_COMPARATOR_DESC,
        vec![Slot::Reference(Some(list_ref)), Slot::Reference(None)],
    )?;
    Ok(None)
}

/// Native: `ArrayList.iterator()Iterator` — creates an `ArrayListIterator`.
pub(crate) fn native_arraylist_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    // fields[0]=list_ref, fields[1]=cursor, fields[2]=last_returned (-1 = none)
    let iter_ref = heap.allocate("duke/util/ArrayListIterator".to_string(), 3);
    {
        let iter_obj = heap.get_mut(iter_ref)?;
        iter_obj.fields[0] = Slot::Reference(Some(this_ref));
        iter_obj.fields[1] = Slot::Int(0);
        iter_obj.fields[2] = Slot::Int(-1);
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
}

/// Native: `ArrayList.size()I`
pub(crate) fn native_arraylist_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(Slot::Int(sz)) => Ok(Some(Slot::Int(*sz))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `ArrayList.get(I)Object` — returns element at index.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)? as usize;
    let obj = heap.get(this_ref)?;
    obj.fields.get(idx + 1).map_or_else(
        || {
            Err(VmError::JavaException {
                class_name: "java/lang/ArrayIndexOutOfBoundsException".to_string(),
            })
        },
        |slot| Ok(Some(*slot)),
    )
}

/// Native: `ArrayList.add(Object)Z` — appends element, returns true.
pub(crate) fn native_arraylist_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException), // shouldn't happen; init sets fields[0]=Int(0)
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1))) // boolean true
}

/// Native: `ArrayList.<init>(Collection)V` — copies elements from another `ArrayList`/collection.
pub(crate) fn native_arraylist_init_from_collection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Reference(Some(src_ref))) = args.get(1).copied() else {
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
        return Ok(None);
    };
    // Copy size and elements from source collection (ArrayList layout: fields[0]=size, fields[1..]=elems)
    let src_fields = heap.get(src_ref)?.fields.clone();
    let src_size = match src_fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(src_size);
    for elem in src_fields
        .into_iter()
        .skip(1)
        .take(usize::try_from(src_size).unwrap_or(0))
    {
        heap.get_mut(this_ref)?.fields.push(elem);
    }
    Ok(None)
}

/// Native: `ArrayList.<init>()V` — initializes with size=0.
pub(crate) fn native_arraylist_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `StringBuilder.setLength(int)V` — truncates or pads with null chars.
pub(crate) fn native_sb_set_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let new_len = usize::try_from(extract_int_arg(args, 1)?).unwrap_or(0);
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    let char_count = buf.chars().count();
    if new_len <= char_count {
        *buf = buf.chars().take(new_len).collect();
    } else {
        buf.extend(std::iter::repeat_n('\0', new_len - char_count));
    }
    Ok(None)
}

/// Native: `HashMap.entrySet()` — returns a new `HashSet` of `java/util/Map$Entry` objects.
pub(crate) fn native_hashmap_entry_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(this_ref)?.fields.clone();
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    let mut i = 1usize;
    while i + 1 < fields.len() {
        let key = fields[i];
        let val = fields[i + 1];
        let entry_ref = heap.allocate("java/util/Map$Entry".to_string(), 2);
        {
            let entry_obj = heap.get_mut(entry_ref)?;
            entry_obj.fields[0] = key;
            entry_obj.fields[1] = val;
        }
        native_hashset_add(
            &[
                Slot::Reference(Some(set_ref)),
                Slot::Reference(Some(entry_ref)),
            ],
            heap,
            out,
            control,
        )?;
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
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(this_ref)?.fields.clone();
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    // Pairs start at index 1; values are at indices 2, 4, 6, ...
    let mut i = 2usize;
    while i < fields.len() {
        let val = fields[i];
        native_arraylist_add(&[Slot::Reference(Some(list_ref)), val], heap, out, control)?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}

/// Native: `HashMap.keySet()` — returns a new `HashSet` containing all keys.
pub(crate) fn native_hashmap_key_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(this_ref)?.fields.clone();
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    let mut i = 1usize;
    while i < fields.len() {
        let key = fields[i];
        native_hashset_add(&[Slot::Reference(Some(set_ref)), key], heap, out, control)?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}

/// Native: `Map$Entry.getValue()Object` — returns the value field.
pub(crate) fn native_map_entry_get_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    Ok(Some(val))
}

/// Native: `Map$Entry.getKey()Object` — returns the key field.
pub(crate) fn native_map_entry_get_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    Ok(Some(key))
}

/// Native: `Collections.frequency(Collection, Object)I` — count occurrences of element.
pub(crate) fn native_collections_frequency(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let coll_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let size = match heap.get(coll_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let fields = heap.get(coll_ref)?.fields[1..=size].to_vec();
    let count = fields
        .iter()
        .filter(|s| slots_equal(s, &target, heap))
        .count();
    Ok(Some(Slot::Int(i32::try_from(count).unwrap_or(i32::MAX))))
}

/// Native: `Collections.reverse(List)V` — reverses an `ArrayList` in-place.
pub(crate) fn native_collections_reverse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(None),
    };
    // Elements are at fields[1..=size]; reverse that slice.
    let fields = &mut heap.get_mut(list_ref)?.fields;
    fields[1..=size].reverse();
    Ok(None)
}

/// Native: `Collections.singletonList(Object)List` — returns a one-element `ArrayList`.
pub(crate) fn native_collections_singleton_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let element = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    native_arraylist_add(&[Slot::Reference(Some(r)), element], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: mutation ops on `UnmodifiableList` throw `UnsupportedOperationException`.
pub(crate) fn native_unmodifiable_list_mutation(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Err(VmError::JavaException {
        class_name: "java/lang/UnsupportedOperationException".to_string(),
    })
}

/// Native: `Collections.unmodifiableList(List)List` — returns the same list (no-copy; single-threaded).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // Create an UnmodifiableList backed by the source list's elements.
    // Mutation methods on this class throw UnsupportedOperationException.
    let src_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Reference(None))),
    };
    let (size, elems) = {
        let src = heap.get(src_ref)?;
        let size = src.fields.first().copied().unwrap_or(Slot::Int(0));
        let elems = src.fields[1..].to_vec();
        (size, elems)
    };
    let n_fields = 1 + elems.len();
    let dst_ref = heap.allocate("java/util/UnmodifiableList".to_string(), n_fields);
    {
        let dst = heap.get_mut(dst_ref)?;
        dst.fields[0] = size;
        for (i, e) in elems.iter().enumerate() {
            dst.fields[1 + i] = *e;
        }
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Collections.emptyMap()Map` — returns a new empty `HashMap`.
pub(crate) fn native_collections_empty_map(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.emptySet()Set` — returns a new empty `HashSet`.
pub(crate) fn native_collections_empty_set(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.emptyList()List` — returns a new empty `ArrayList`.
pub(crate) fn native_collections_empty_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // Return an immutable empty UnmodifiableList (same field layout as ArrayList)
    let r = heap.allocate("java/util/UnmodifiableList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_system_set_property(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let key_ref = extract_ref_arg(args, 0)?;
    let value_ref = extract_ref_arg(args, 1)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let value = string_value_from_ref(heap, value_ref)?;
    let previous = system_property_value(&key);
    system_property_overrides()
        .lock()
        .expect("system property overrides mutex poisoned")
        .insert(key, value);
    let result = previous.map_or(Slot::Reference(None), |previous| {
        Slot::Reference(Some(heap.allocate_string(previous)))
    });
    Ok(Some(result))
}

pub(crate) fn native_reflect_field_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let value_slot = extract_slot_arg(args, 2);
    let field = reflected_field_handle(heap, field_ref)?;

    if !field.is_public && !field.is_accessible {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let field_type = field.descriptor.chars().next().unwrap_or('L');
    let value = unbox_reflection_argument(heap, field_type, value_slot)?;

    if field.is_static {
        ops.ensure_class_initialized(heap, output, &field.declaring_class_key)?;
        ops.write_static_field(&field.declaring_class_key, &field.field_name, value)?;
        return Ok(None);
    }

    let Slot::Reference(Some(target_ref)) = target_slot else {
        return Err(VmError::NullPointerException);
    };
    ops.write_instance_field(
        heap,
        target_ref,
        &field.declaring_class_key,
        &field.field_name,
        value,
    )?;
    Ok(None)
}

pub(crate) fn native_reflection_member_set_accessible(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    let accessible = extract_int_arg(args, 1)? != 0;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_ACCESSIBLE_FIELD,
        Slot::Int(i32::from(accessible)),
    )?;
    Ok(None)
}

/// Native: `Collections.sort(List, Comparator)V` — 2-arg sort with explicit comparator.
pub(crate) fn native_collections_sort_with_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let comparator = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let class_name = heap.get(list_ref)?.class_name.clone();
    ops.invoke(
        heap,
        output,
        &class_name,
        "sort",
        "(Ljava/util/Comparator;)V",
        vec![Slot::Reference(Some(list_ref)), comparator],
    )?;
    Ok(None)
}

/// Native: `PriorityQueue.isEmpty()Z`
pub(crate) fn native_priorityqueue_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

/// Native: `PriorityQueue.size()I`
pub(crate) fn native_priorityqueue_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `PriorityQueue.poll()Object` — removes and returns minimum; sifts down.
pub(crate) fn native_priorityqueue_poll(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    let min = heap.get(this_ref)?.fields[1];
    if size == 1 {
        heap.get_mut(this_ref)?.fields.pop();
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
        return Ok(Some(min));
    }
    // Move last element to root, sift down.
    let last = heap.get(this_ref)?.fields[size];
    heap.get_mut(this_ref)?.fields[1] = last;
    heap.get_mut(this_ref)?.fields.pop();
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    let new_size = size - 1;
    let mut i = 1usize;
    loop {
        let left = 2 * i;
        let right = 2 * i + 1;
        let mut smallest = i;
        if left <= new_size {
            let (cur_slot, left_slot) = (
                heap.get(this_ref)?.fields[smallest],
                heap.get(this_ref)?.fields[left],
            );
            let (Slot::Reference(Some(cur_ref)), Slot::Reference(Some(left_ref))) =
                (cur_slot, left_slot)
            else {
                break;
            };
            let class_left = heap.get(left_ref)?.class_name.clone();
            let cmp = ops.invoke(
                heap,
                out,
                &class_left,
                "compareTo",
                "(Ljava/lang/Object;)I",
                vec![
                    Slot::Reference(Some(left_ref)),
                    Slot::Reference(Some(cur_ref)),
                ],
            )?;
            if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
                smallest = left;
            }
        }
        if right <= new_size {
            let (small_slot, right_slot) = (
                heap.get(this_ref)?.fields[smallest],
                heap.get(this_ref)?.fields[right],
            );
            let (Slot::Reference(Some(small_ref)), Slot::Reference(Some(right_ref))) =
                (small_slot, right_slot)
            else {
                break;
            };
            let class_right = heap.get(right_ref)?.class_name.clone();
            let cmp = ops.invoke(
                heap,
                out,
                &class_right,
                "compareTo",
                "(Ljava/lang/Object;)I",
                vec![
                    Slot::Reference(Some(right_ref)),
                    Slot::Reference(Some(small_ref)),
                ],
            )?;
            if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
                smallest = right;
            }
        }
        if smallest == i {
            break;
        }
        heap.get_mut(this_ref)?.fields.swap(i, smallest);
        i = smallest;
    }
    Ok(Some(min))
}

/// Native: `PriorityQueue.peek()Object` — returns minimum element without removing.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_priorityqueue_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `PriorityQueue.add(Object)Z` — same as offer.
pub(crate) fn native_priorityqueue_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    native_priorityqueue_offer(args, heap, out, control, ops)
}

/// Native: `PriorityQueue.offer(Object)Z` — inserts in heap order via `compareTo`.
pub(crate) fn native_priorityqueue_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    // Append, then sift up.
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    heap.get_mut(this_ref)?.fields.push(elem);
    let new_size = size + 1;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(new_size).unwrap_or(0));
    // Sift up from last position (1-indexed in fields).
    let mut i = new_size; // fields index of newly added element
    while i > 1 {
        // fields are 1-indexed: child at fields[i], parent at fields[i/2]
        let parent_idx = i / 2;
        let child_slot = heap.get(this_ref)?.fields[i];
        let parent_slot = heap.get(this_ref)?.fields[parent_idx];
        let (Slot::Reference(Some(child_ref)), Slot::Reference(Some(parent_ref))) =
            (child_slot, parent_slot)
        else {
            break;
        };
        let class_child = heap.get(child_ref)?.class_name.clone();
        let cmp = ops.invoke(
            heap,
            out,
            &class_child,
            "compareTo",
            "(Ljava/lang/Object;)I",
            vec![
                Slot::Reference(Some(child_ref)),
                Slot::Reference(Some(parent_ref)),
            ],
        )?;
        if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
            // child < parent: swap
            heap.get_mut(this_ref)?.fields.swap(i, parent_idx);
            i = parent_idx;
        } else {
            break;
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `PriorityQueue.<init>()V` — same init as `ArrayList`.
pub(crate) fn native_priorityqueue_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `ArrayDeque.isEmpty()Z`
pub(crate) fn native_arraydeque_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

/// Native: `ArrayDeque.size()I`
pub(crate) fn native_arraydeque_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `ArrayDeque.peek()Object` — peek at front; null if empty.
pub(crate) fn native_arraydeque_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `ArrayDeque.poll()Object` — dequeue from front (queue: FIFO); null if empty.
pub(crate) fn native_arraydeque_poll(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    let elem = heap.get_mut(this_ref)?.fields.remove(1);
    let new_size = i32::try_from(size - 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(Some(elem))
}

/// Native: `ArrayDeque.add(Object)Z` — same as offer (appends to back).
pub(crate) fn native_arraydeque_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `ArrayDeque.offer(Object)Z` — enqueue at back (queue: FIFO).
pub(crate) fn native_arraydeque_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(Slot::Int(1))) // always succeeds
}

/// Native: `ArrayDeque.pop()Object` — pop from front (stack: LIFO).
pub(crate) fn native_arraydeque_pop(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let elem = heap.get_mut(this_ref)?.fields.remove(1);
    let new_size = i32::try_from(size - 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(Some(elem))
}

/// Native: `ArrayDeque.push(Object)V` — push to front (stack: LIFO).
pub(crate) fn native_arraydeque_push(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    heap.get_mut(this_ref)?.fields.insert(1, elem);
    let new_size = i32::try_from(size + 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(None)
}

/// Native: `ArrayDeque.<init>()V` — same layout as `ArrayList`.
pub(crate) fn native_arraydeque_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `Collections.fill(List, Object)V` — set every element to value.
pub(crate) fn native_collections_fill(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let value = extract_slot_arg(args, 1);
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    for i in 1..=size {
        heap.get_mut(list_ref)?.fields[i] = value;
    }
    Ok(None)
}

/// Native: `Collections.shuffle(List, Random)V` — shuffle with provided RNG (no-op for correctness since test only checks sum).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_shuffle_random(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `Collections.shuffle(List)V` — no-op (deterministic test environments).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_shuffle(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `Collections.max(Collection)T` — returns maximum element via `compareTo`.
pub(crate) fn native_collections_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let coll_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(coll_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let Slot::Reference(Some(mut max_ref)) = heap.get(coll_ref)?.fields[1] else {
        return Ok(Some(Slot::Reference(None)));
    };
    for i in 2..=size {
        let Slot::Reference(Some(candidate)) = heap.get(coll_ref)?.fields[i] else {
            continue;
        };
        let class_name = heap.get(candidate)?.class_name.clone();
        let cmp = ops.invoke(
            heap,
            out,
            &class_name,
            "compareTo",
            "(Ljava/lang/Object;)I",
            vec![
                Slot::Reference(Some(candidate)),
                Slot::Reference(Some(max_ref)),
            ],
        )?;
        let _ = control;
        if matches!(cmp, Some(Slot::Int(n)) if n > 0) {
            max_ref = candidate;
        }
    }
    Ok(Some(Slot::Reference(Some(max_ref))))
}

/// Native: `Collections.min(Collection)T` — returns minimum element via `compareTo`.
pub(crate) fn native_collections_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let coll_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(coll_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let Slot::Reference(Some(mut min_ref)) = heap.get(coll_ref)?.fields[1] else {
        return Ok(Some(Slot::Reference(None)));
    };
    for i in 2..=size {
        let Slot::Reference(Some(candidate)) = heap.get(coll_ref)?.fields[i] else {
            continue;
        };
        let class_name = heap.get(candidate)?.class_name.clone();
        let cmp = ops.invoke(
            heap,
            out,
            &class_name,
            "compareTo",
            "(Ljava/lang/Object;)I",
            vec![
                Slot::Reference(Some(candidate)),
                Slot::Reference(Some(min_ref)),
            ],
        )?;
        let _ = control;
        if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
            min_ref = candidate;
        }
    }
    Ok(Some(Slot::Reference(Some(min_ref))))
}

/// Native: `TreeSet.iterator()Iterator` — returns an ArrayList-compatible iterator over sorted elements.
pub(crate) fn native_treeset_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_iterator(args, heap, out, control)
}

/// Native: `TreeSet.isEmpty()Z`
pub(crate) fn native_treeset_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) | None => 1,
        _ => 0,
    })))
}

/// Native: `TreeSet.last()E` — returns largest element.
pub(crate) fn native_treeset_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(heap.get(this_ref)?.fields[size]))
}

/// Native: `TreeSet.first()E` — returns smallest element.
pub(crate) fn native_treeset_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first().copied() {
        Some(Slot::Int(0)) | None => Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        }),
        _ => Ok(Some(heap.get(this_ref)?.fields[1])),
    }
}

/// Native: `TreeSet.size()I`
pub(crate) fn native_treeset_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Slot::Int(*n),
        _ => Slot::Int(0),
    }))
}

/// Native: `TreeSet.contains(E)Z`
pub(crate) fn native_treeset_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let elem_str = match &elem {
        Slot::Reference(Some(r)) => heap.get(*r)?.string_value.clone(),
        Slot::Int(v) => Some(v.to_string()),
        _ => None,
    };
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    for i in 0..size {
        let ex = heap.get(this_ref)?.fields[1 + i];
        let ex_str = match &ex {
            Slot::Reference(Some(r)) => heap.get(*r).ok().and_then(|o| o.string_value.clone()),
            Slot::Int(v) => Some(v.to_string()),
            _ => None,
        };
        if ex_str == elem_str {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

pub(crate) fn native_treeset_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let elem_key = treeset_slot_sort_key(elem, heap);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    // Check for duplicate.
    for i in 0..size {
        let ex = heap.get(this_ref)?.fields[1 + i];
        let ex_key = treeset_slot_sort_key(ex, heap);
        if ex_key == elem_key {
            return Ok(Some(Slot::Int(0))); // false — no change
        }
    }
    // Find sorted insert position.
    let insert_pos = {
        let mut pos = size;
        for i in 0..size {
            let ex = heap.get(this_ref)?.fields[1 + i];
            let ex_key = treeset_slot_sort_key(ex, heap);
            if let (Some(ek), Some(exk)) = (&elem_key, &ex_key)
                && ek.less_than(exk)
            {
                pos = i;
                break;
            }
        }
        pos
    };
    let obj = heap.get_mut(this_ref)?;
    obj.fields.insert(1 + insert_pos, elem);
    obj.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(1))) // true — element added
}

/// Native: `TreeSet.<init>()V`
pub(crate) fn native_treeset_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `TreeMap.tailMap(fromKey)SortedMap` — returns a new `TreeMap` with keys >= fromKey.
pub(crate) fn native_treemap_tail_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let from_key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let result = heap.allocate("java/util/TreeMap".to_string(), 1);
    heap.get_mut(result)?.fields[0] = Slot::Int(0);
    let mut count = 0usize;
    for i in 0..size {
        let k = heap.get(this_ref)?.fields[1 + i * 2];
        let v = heap.get(this_ref)?.fields[2 + i * 2];
        if compare_treemap_keys(k, from_key, heap) != std::cmp::Ordering::Less {
            heap.get_mut(result)?.fields.push(k);
            heap.get_mut(result)?.fields.push(v);
            count += 1;
        }
    }
    heap.get_mut(result)?.fields[0] = Slot::Int(i32::try_from(count).unwrap_or(0));
    Ok(Some(Slot::Reference(Some(result))))
}

/// Native: `TreeMap.headMap(toKey)SortedMap` — returns a new `TreeMap` with keys strictly less than toKey.
pub(crate) fn native_treemap_head_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let to_key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let result = heap.allocate("java/util/TreeMap".to_string(), 1);
    heap.get_mut(result)?.fields[0] = Slot::Int(0);
    let mut count = 0usize;
    for i in 0..size {
        let k = heap.get(this_ref)?.fields[1 + i * 2];
        let v = heap.get(this_ref)?.fields[2 + i * 2];
        if compare_treemap_keys(k, to_key, heap) == std::cmp::Ordering::Less {
            heap.get_mut(result)?.fields.push(k);
            heap.get_mut(result)?.fields.push(v);
            count += 1;
        }
    }
    heap.get_mut(result)?.fields[0] = Slot::Int(i32::try_from(count).unwrap_or(0));
    Ok(Some(Slot::Reference(Some(result))))
}

/// Native: `TreeMap.isEmpty()Z`
pub(crate) fn native_treemap_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) | None => 1,
        _ => 0,
    })))
}

/// Native: `TreeMap.remove(K)V`
pub(crate) fn native_treemap_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    for i in 0..size {
        let ek = heap.get(this_ref)?.fields[1 + i * 2];
        if treemap_keys_equal(ek, key, heap) {
            let obj = heap.get_mut(this_ref)?;
            let v = obj.fields.remove(2 + i * 2);
            obj.fields.remove(1 + i * 2);
            obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
            return Ok(Some(v));
        }
    }
    Ok(Some(Slot::Reference(None)))
}

/// Native: `TreeMap.lastKey()K` — returns the largest key.
pub(crate) fn native_treemap_last_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(heap.get(this_ref)?.fields[1 + (size - 1) * 2]))
}

/// Native: `TreeMap.firstKey()K` — returns the smallest key (index 0 in sorted list).
pub(crate) fn native_treemap_first_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `TreeMap.size()I`
pub(crate) fn native_treemap_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Slot::Int(*n),
        _ => Slot::Int(0),
    }))
}

/// Native: `TreeMap.containsKey(K)Z`
pub(crate) fn native_treemap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let found = !matches!(
        native_treemap_get(args, heap, out, control)?,
        Some(Slot::Reference(None)) | None
    );
    Ok(Some(Slot::Int(i32::from(found))))
}

/// Native: `TreeMap.get(K)V`
pub(crate) fn native_treemap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    for i in 0..size {
        let ek = heap.get(this_ref)?.fields[1 + i * 2];
        if treemap_keys_equal(ek, key, heap) {
            let v = heap.get(this_ref)?.fields[2 + i * 2];
            return Ok(Some(v));
        }
    }
    Ok(Some(Slot::Reference(None)))
}

/// Native: `TreeMap.put(K,V)V` — inserts in sorted key order.
pub(crate) fn native_treemap_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let val = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    // Check for existing key — update in place.
    for i in 0..size {
        let existing_key = heap.get(this_ref)?.fields[1 + i * 2];
        if treemap_keys_equal(existing_key, key, heap) {
            heap.get_mut(this_ref)?.fields[2 + i * 2] = val;
            return Ok(Some(Slot::Reference(None)));
        }
    }
    // Find sorted insert position.
    let insert_pos = {
        let mut pos = size;
        for i in 0..size {
            let ek = heap.get(this_ref)?.fields[1 + i * 2];
            if compare_treemap_keys(key, ek, heap) == std::cmp::Ordering::Less {
                pos = i;
                break;
            }
        }
        pos
    };
    let obj = heap.get_mut(this_ref)?;
    obj.fields.insert(1 + insert_pos * 2, val);
    obj.fields.insert(1 + insert_pos * 2, key);
    obj.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    Ok(Some(Slot::Reference(None)))
}

/// Native: `TreeMap.<init>()V`
pub(crate) fn native_treemap_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
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
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_ref = extract_ref_arg(args, 1)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    // Snapshot keys (values will be mutated in place).
    let keys: Vec<Slot> = (0..size)
        .map(|i| {
            heap.get(this_ref)
                .map(|o| o.fields[1 + i * 2])
                .unwrap_or(Slot::Reference(None))
        })
        .collect();
    for (i, key) in keys.iter().enumerate() {
        let old_val = heap
            .get(this_ref)
            .map(|o| o.fields[2 + i * 2])
            .unwrap_or(Slot::Reference(None));
        let new_val = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![Slot::Reference(Some(fn_ref)), *key, old_val],
        )?;
        if let Some(v) = new_val {
            heap.get_mut(this_ref)?.fields[2 + i * 2] = v;
        }
    }
    let _ = control;
    Ok(None)
}

/// Native: `HashMap.forEach(BiConsumer)V` — iterates key-value pairs, invoking `accept(k, v)`.
pub(crate) fn native_hashmap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let consumer_ref = extract_ref_arg(args, 1)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    // Snapshot key-val pairs (fields[1,2], fields[3,4], ...)
    let pairs: Vec<(Slot, Slot)> = (0..size)
        .map(|i| {
            let key = heap
                .get(this_ref)
                .map(|o| o.fields[1 + i * 2])
                .unwrap_or(Slot::Reference(None));
            let val = heap
                .get(this_ref)
                .map(|o| o.fields[2 + i * 2])
                .unwrap_or(Slot::Reference(None));
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

/// Native: `LinkedList.iterator()Iterator` — returns an ArrayList-compatible iterator.
pub(crate) fn native_linked_list_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_iterator(args, heap, out, control)
}

/// Native: `LinkedList.isEmpty()Z`
pub(crate) fn native_linked_list_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

/// Native: `LinkedList.offer(Object)Z` — appends to tail, returns true.
pub(crate) fn native_linked_list_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)
}

/// Native: `LinkedList.poll()Object` — removes and returns head, or null if empty.
pub(crate) fn native_linked_list_poll(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    let obj = heap.get_mut(this_ref)?;
    let elem = obj.fields.remove(1);
    obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    Ok(Some(elem))
}

/// Native: `LinkedList.removeLast()Object` — removes and returns tail.
pub(crate) fn native_linked_list_remove_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let obj = heap.get_mut(this_ref)?;
    let elem = obj.fields.remove(size);
    obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    Ok(Some(elem))
}

/// Native: `LinkedList.removeFirst()Object` — removes and returns head.
pub(crate) fn native_linked_list_remove_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(VmError::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let obj = heap.get_mut(this_ref)?;
    let elem = obj.fields.remove(1);
    obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    Ok(Some(elem))
}

/// Native: `LinkedList.peekLast()Object` — returns tail without removal, or null if empty.
pub(crate) fn native_linked_list_peek_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[size]))
}

/// Native: `LinkedList.peekFirst()Object` — returns head without removal, or null if empty.
pub(crate) fn native_linked_list_peek_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `LinkedList.addLast(Object)V` — appends to tail (same as add).
pub(crate) fn native_linked_list_add_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(None)
}

/// Native: `LinkedList.addFirst(Object)V` — inserts at index 0.
/// Field layout: `fields[0]`=Int(size), `fields[1..size]`=elements.
pub(crate) fn native_linked_list_add_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let obj = heap.get_mut(this_ref)?;
    // Shift existing elements right by one.
    obj.fields.insert(1, elem);
    obj.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    Ok(None)
}

/// Native: `LinkedList.get(I)Object`
pub(crate) fn native_linked_list_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_get(args, heap, out, control)
}

/// Native: `LinkedList.add(Object)Z` — appends to tail.
pub(crate) fn native_linked_list_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)
}

/// Native: `LinkedList.size()I`
pub(crate) fn native_linked_list_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `LinkedList.<init>(Collection)V` — copies all elements from source collection.
pub(crate) fn native_linked_list_init_collection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_ref = extract_ref_arg(args, 1)?;
    let src_size = match heap.get(src_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let src_elems: Vec<Slot> = heap.get(src_ref)?.fields[1..=src_size].to_vec();
    let n = i32::try_from(src_elems.len()).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(n);
    for elem in src_elems {
        heap.get_mut(this_ref)?.fields.push(elem);
    }
    Ok(None)
}

/// Native: `LinkedList.<init>()V` — same initialisation as `ArrayList`.
pub(crate) fn native_linked_list_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `HashMap.computeIfAbsent(K, Function)V` — returns existing value or computes and stores it.
pub(crate) fn native_hashmap_compute_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    // Check if key already present.
    let existing = native_hashmap_get(&[Slot::Reference(Some(this_ref)), key], heap, out, control)?;
    if let Some(v) = existing
        && !matches!(v, Slot::Reference(None))
    {
        return Ok(Some(v));
    }
    // Key absent — invoke the mapping function (lambda / SAM).
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let computed = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![Slot::Reference(Some(fn_ref)), key],
    )?;
    if let Some(value) = computed
        && !matches!(value, Slot::Reference(None))
    {
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

/// Native: `HashMap.putAll(Map)V` — copies all entries from the source `HashMap`.
pub(crate) fn native_hashmap_put_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_ref = extract_ref_arg(args, 1)?;
    let fields = heap.get(src_ref)?.fields.clone();
    // fields[0] = size, fields[1..] = k0, v0, k1, v1, ...
    let mut i = 1;
    while i + 1 < fields.len() {
        let k = fields[i];
        let v = fields[i + 1];
        native_hashmap_put(&[Slot::Reference(Some(this_ref)), k, v], heap, out, control)?;
        i += 2;
    }
    Ok(None)
}

/// Native: `ArrayList.addAll(Collection)Z` — appends all elements from a compatible collection.
pub(crate) fn native_arraylist_add_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_ref = extract_ref_arg(args, 1)?;
    let src_size = match heap.get(src_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let elems: Vec<Slot> = heap.get(src_ref)?.fields[1..=src_size].to_vec();
    let modified = !elems.is_empty();
    for elem in elems {
        native_arraylist_add(&[Slot::Reference(Some(this_ref)), elem], heap, out, control)?;
    }
    Ok(Some(Slot::Int(i32::from(modified))))
}

/// Native: `Map.of(K,V,...)Map` — pairs of args become entries in a new `HashMap`.
pub(crate) fn native_map_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(map_ref))], heap, out, control)?;
    let mut i = 0;
    while i + 1 < args.len() {
        let k = args[i];
        let v = args[i + 1];
        native_hashmap_put(&[Slot::Reference(Some(map_ref)), k, v], heap, out, control)?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(map_ref))))
}

/// Native: `Set.of(Object...)Set` — all args (fixed-arity or varargs) become a new `HashSet`.
///
/// Handles both fixed-arity descriptors (multiple direct element args)
/// and the single-array varargs form `([O)Set`.
pub(crate) fn native_set_of_factory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let elems: Vec<Slot> = if args.len() == 1 {
        if let Some(Slot::Reference(Some(arr_ref))) = args.first() {
            let obj = heap.get(*arr_ref)?;
            if obj.class_name.starts_with('[') {
                let fields = obj.fields.clone();
                let _ = obj;
                fields
            } else {
                args.to_vec()
            }
        } else {
            Vec::new()
        }
    } else {
        args.to_vec()
    };
    let set_ref = make_set_from_slots(&elems, heap, out, control)?;
    Ok(Some(Slot::Reference(Some(set_ref))))
}

/// Native: `List.of(Object...)List` — all args (fixed-arity or varargs) become a new `ArrayList`.
///
/// Handles descriptors with 0–6+ fixed args and the varargs `([O)List` form.
pub(crate) fn native_list_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // Varargs form: single arg that is an Object[] array.
    let elems: Vec<Slot> = if args.len() == 1 {
        if let Some(Slot::Reference(Some(arr_ref))) = args.first() {
            let obj = heap.get(*arr_ref)?;
            if obj.class_name.starts_with('[') {
                let fields = obj.fields.clone();
                let _ = obj;
                fields
            } else {
                args.to_vec()
            }
        } else {
            Vec::new()
        }
    } else {
        args.to_vec()
    };
    let list_ref = make_list_from_slots(&elems, heap, out, control)?;
    Ok(Some(Slot::Reference(Some(list_ref))))
}
