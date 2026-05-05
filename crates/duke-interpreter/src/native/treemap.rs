/// Check if two `TreeMap` keys are equal (same value semantics as `compare_treemap_keys` == Equal).
fn treemap_keys_equal(a: Slot, b: Slot, heap: &duke_gc::Heap) -> bool {
    compare_treemap_keys(a, b, heap) == std::cmp::Ordering::Equal
}
/// Native: `TreeMap.<init>()V`
pub(crate) fn native_treemap_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}
/// Native: `TreeMap.put(K,V)V` — inserts in sorted key order.
pub(crate) fn native_treemap_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let val = extract_slot_arg(args, 2);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    for i in 0..size {
        let existing_key = heap.get(this_ref)?.fields[1 + i * 2];
        if treemap_keys_equal(existing_key, key, heap) {
            heap.get_mut(this_ref)?.fields[2 + i * 2] = val;
            return Ok(Some(Slot::Reference(None)));
        }
    }
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
/// Native: `TreeMap.get(K)V`
pub(crate) fn native_treemap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
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
/// Native: `TreeMap.containsKey(K)Z`
pub(crate) fn native_treemap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let found = !matches!(
        native_treemap_get(args, heap, out, control) ?, Some(Slot::Reference(None)) |
        None
    );
    Ok(Some(Slot::Int(i32::from(found))))
}
/// Native: `TreeMap.size()I`
pub(crate) fn native_treemap_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(
        Some(
            match heap.get(this_ref)?.fields.first() {
                Some(Slot::Int(n)) => Slot::Int(*n),
                _ => Slot::Int(0),
            },
        ),
    )
}
/// Native: `TreeMap.firstKey()K` — returns the smallest key (index 0 in sorted list).
pub(crate) fn native_treemap_first_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    if size == 0 {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}
/// Native: `TreeMap.lastKey()K` — returns the largest key.
pub(crate) fn native_treemap_last_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    Ok(Some(heap.get(this_ref)?.fields[1 + (size - 1) * 2]))
}
/// Native: `TreeMap.remove(K)V`
pub(crate) fn native_treemap_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
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
/// Native: `TreeMap.isEmpty()Z`
pub(crate) fn native_treemap_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(
        Some(
            Slot::Int(
                match heap.get(this_ref)?.fields.first() {
                    Some(Slot::Int(0)) | None => 1,
                    _ => 0,
                },
            ),
        ),
    )
}
/// Native: `TreeMap.headMap(toKey)SortedMap` — returns a new `TreeMap` with keys strictly less than toKey.
pub(crate) fn native_treemap_head_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let to_key = extract_slot_arg(args, 1);
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
/// Native: `TreeMap.tailMap(fromKey)SortedMap` — returns a new `TreeMap` with keys >= fromKey.
pub(crate) fn native_treemap_tail_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let from_key = extract_slot_arg(args, 1);
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
/// Native: `TreeMap.forEach(BiConsumer)V`
pub(crate) fn native_treemap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_hashmap_for_each(args, heap, out, control, ops)
}
/// Native: `TreeMap.keySet()Set` — delegates to `HashMap` keySet (same field layout).
pub(crate) fn native_treemap_key_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_hashmap_key_set(args, heap, out, control)
}
/// Native: `TreeMap.values()Collection` — delegates to `HashMap` values (same field layout).
pub(crate) fn native_treemap_values(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_hashmap_values(args, heap, out, control)
}
/// Native: `TreeMap.getOrDefault(Object,Object)Object` — looks up key; returns default if absent.
pub(crate) fn native_treemap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let default_val = extract_slot_arg(args, 2);
    let found_idx = {
        let fields = &heap.get(this_ref)?.fields;
        find_hashmap_entry_index(fields, &key, heap)
    };
    Ok(
        Some(
            match found_idx {
                Some(i) => heap.get(this_ref)?.fields[i + 1],
                None => default_val,
            },
        ),
    )
}
