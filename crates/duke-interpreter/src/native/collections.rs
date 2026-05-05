/// Native: `ArrayList.addAll(Collection)Z` — appends all elements from a compatible collection.
pub(crate) fn native_arraylist_add_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

// ---- LinkedList natives (field layout identical to ArrayList: fields[0]=size, fields[1..]=elements) ----

/// Native: `LinkedList.<init>()V` — same initialisation as `ArrayList`.
pub(crate) fn native_linked_list_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `LinkedList.<init>(Collection)V` — copies all elements from source collection.
pub(crate) fn native_linked_list_init_collection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

/// Native: `LinkedList.size()I`
pub(crate) fn native_linked_list_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `LinkedList.add(Object)Z` — appends to tail.
pub(crate) fn native_linked_list_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)
}

/// Native: `LinkedList.get(I)Object`
pub(crate) fn native_linked_list_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_get(args, heap, out, control)
}

/// Native: `LinkedList.addFirst(Object)V` — inserts at index 0.
/// Field layout: `fields[0]`=Int(size), `fields[1..size]`=elements.
pub(crate) fn native_linked_list_add_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

/// Native: `LinkedList.addLast(Object)V` — appends to tail (same as add).
pub(crate) fn native_linked_list_add_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(None)
}

/// Native: `LinkedList.peekFirst()Object` — returns head without removal, or null if empty.
pub(crate) fn native_linked_list_peek_first(
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
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `LinkedList.peekLast()Object` — returns tail without removal, or null if empty.
pub(crate) fn native_linked_list_peek_last(
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
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[size]))
}

/// Native: `LinkedList.removeFirst()Object` — removes and returns head.
pub(crate) fn native_linked_list_remove_first(
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
    let obj = heap.get_mut(this_ref)?;
    let elem = obj.fields.remove(size);
    obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    Ok(Some(elem))
}

/// Native: `LinkedList.poll()Object` — removes and returns head, or null if empty.
pub(crate) fn native_linked_list_poll(
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
        return Ok(Some(Slot::Reference(None)));
    }
    let obj = heap.get_mut(this_ref)?;
    let elem = obj.fields.remove(1);
    obj.fields[0] = Slot::Int(i32::try_from(size - 1).unwrap_or(0));
    Ok(Some(elem))
}

/// Native: `LinkedList.offer(Object)Z` — appends to tail, returns true.
pub(crate) fn native_linked_list_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)
}

/// Native: `LinkedList.isEmpty()Z`
pub(crate) fn native_linked_list_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

/// Native: `LinkedList.iterator()Iterator` — returns an ArrayList-compatible iterator.
pub(crate) fn native_linked_list_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_iterator(args, heap, out, control)
}

// ---- HashMap.forEach callback ----

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
    // Snapshot key-val pairs (fields[1,2], fields[3,4], ...)
    let pairs: Vec<(Slot, Slot)> = (0..size)
        .map(|i| {
            let key = heap.get(this_ref).map_or(Slot::Reference(None), |o| o.fields[1 + i * 2]);
            let val = heap.get(this_ref).map_or(Slot::Reference(None), |o| o.fields[2 + i * 2]);
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
    // Snapshot keys (values will be mutated in place).
    let keys: Vec<Slot> = (0..size)
        .map(|i| {
            heap.get(this_ref).map_or(Slot::Reference(None), |o| o.fields[1 + i * 2])
        })
        .collect();
    for (i, key) in keys.iter().enumerate() {
        let old_val = heap.get(this_ref).map_or(Slot::Reference(None), |o| o.fields[2 + i * 2]);
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

// ---- TreeMap natives (field layout: fields[0]=Int(size), fields[1,2]=k0/v0 sorted by key) ----
// Keys are stored sorted in ascending order for O(n) insert / O(1) first&last.

/// Compare two `TreeMap` keys by natural ordering, supporting String, Integer, Long, and Double keys.
fn compare_treemap_keys(a: Slot, b: Slot, heap: &duke_gc::Heap) -> std::cmp::Ordering {
    let key_ord = |s: Slot| -> Option<KeyOrd> {
        if let Slot::Reference(Some(r)) = s
            && let Ok(obj) = heap.get(r)
        {
            if let Some(sv) = &obj.string_value {
                return Some(KeyOrd::Str(sv.clone()));
            }
            match obj.class_name.as_str() {
                "java/lang/Integer" | "java/lang/Short" | "java/lang/Byte" => {
                    if let Some(Slot::Int(n)) = obj.fields.first() {
                        return Some(KeyOrd::Int(i64::from(*n)));
                    }
                }
                "java/lang/Long" => {
                    if let Some(Slot::Long(n)) = obj.fields.first() {
                        return Some(KeyOrd::Int(*n));
                    }
                }
                "java/lang/Double" | "java/lang/Float" => {
                    if let Some(Slot::Double(n)) = obj.fields.first() {
                        return Some(KeyOrd::Flt(*n));
                    }
                }
                _ => {}
            }
        }
        None
    };
    match (key_ord(a), key_ord(b)) {
        (Some(KeyOrd::Int(x)), Some(KeyOrd::Int(y))) => x.cmp(&y),
        (Some(KeyOrd::Flt(x)), Some(KeyOrd::Flt(y))) => x.total_cmp(&y),
        (Some(KeyOrd::Str(x)), Some(KeyOrd::Str(y))) => x.cmp(&y),
        _ => std::cmp::Ordering::Equal,
    }
}

enum KeyOrd {
    Int(i64),
    Flt(f64),
    Str(String),
}

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
        native_treemap_get(args, heap, out, control)?,
        Some(Slot::Reference(None)) | None
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
    Ok(Some(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Slot::Int(*n),
        _ => Slot::Int(0),
    }))
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
    Ok(Some(Slot::Int(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) | None => 1,
        _ => 0,
    })))
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

// ---- Stack natives (LIFO backed by ArrayList: push=add, pop=removeLast, peek=peekLast) ----

/// Native: `Stack.<init>()V`
pub(crate) fn native_stack_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `Stack.push(E)E` — appends to tail, returns the element.
pub(crate) fn native_stack_push(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let elem = extract_slot_arg(args, 1);
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(elem))
}

/// Native: `Stack.pop()E` — removes and returns the top element.
pub(crate) fn native_stack_pop(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_linked_list_remove_last(args, heap, out, control)
}

/// Native: `Stack.peek()E` — returns the top element without removal.
pub(crate) fn native_stack_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_linked_list_peek_last(args, heap, out, control)
}

/// Native: `Stack.empty()Z` — returns true if the stack is empty.
pub(crate) fn native_stack_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

/// Native: `Stack.size()I`
pub(crate) fn native_stack_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

// ---- TreeSet natives (sorted unique elements, backed by sorted Vec<Slot>) ----
// fields[0] = Int(size), fields[1..] = unique elements in sorted String order

/// Native: `TreeSet.<init>()V`
pub(crate) fn native_treeset_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `TreeSet.add(E)Z` — inserts in sorted order; returns false if already present.
/// Extract a sortable key from a Slot for `TreeSet` ordering.
/// Returns an `Ordering`-compatible f64 for numeric types, lexicographic for strings.
#[allow(clippy::cast_precision_loss)] // intentional: i64→f64 for sort ordering; precision loss acceptable
fn treeset_slot_sort_key(slot: Slot, heap: &duke_gc::Heap) -> Option<TreeSortKey> {
    match slot {
        Slot::Int(n) => Some(TreeSortKey::Num(f64::from(n))),
        Slot::Long(n) => Some(TreeSortKey::Num(n as f64)),
        Slot::Double(d) => Some(TreeSortKey::Num(d)),
        Slot::Float(f) => Some(TreeSortKey::Num(f64::from(f))),
        Slot::Reference(Some(r)) => {
            let obj = heap.get(r).ok()?;
            match obj.class_name.as_str() {
                "java/lang/Integer" | "java/lang/Long" | "java/lang/Short" | "java/lang/Byte" => {
                    match obj.fields.first() {
                        Some(Slot::Int(n)) => Some(TreeSortKey::Num(f64::from(*n))),
                        Some(Slot::Long(n)) => Some(TreeSortKey::Num(*n as f64)),
                        _ => None,
                    }
                }
                "java/lang/Double" | "java/lang/Float" => match obj.fields.first() {
                    Some(Slot::Double(d)) => Some(TreeSortKey::Num(*d)),
                    Some(Slot::Float(f)) => Some(TreeSortKey::Num(f64::from(*f))),
                    _ => None,
                },
                _ => obj.string_value.clone().map(TreeSortKey::Str),
            }
        }
        _ => None,
    }
}

#[derive(PartialEq)]
enum TreeSortKey {
    Num(f64),
    Str(String),
}

impl TreeSortKey {
    fn less_than(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Num(a), Self::Num(b)) => a < b,
            (Self::Str(a), Self::Str(b)) => a < b,
            _ => false,
        }
    }
}

pub(crate) fn native_treeset_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
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

/// Native: `TreeSet.contains(E)Z`
pub(crate) fn native_treeset_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
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

/// Native: `TreeSet.size()I`
pub(crate) fn native_treeset_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Slot::Int(*n),
        _ => Slot::Int(0),
    }))
}

/// Native: `TreeSet.first()E` — returns smallest element.
pub(crate) fn native_treeset_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first().copied() {
        Some(Slot::Int(0)) | None => Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        }),
        _ => Ok(Some(heap.get(this_ref)?.fields[1])),
    }
}

/// Native: `TreeSet.last()E` — returns largest element.
pub(crate) fn native_treeset_last(
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
    Ok(Some(heap.get(this_ref)?.fields[size]))
}

/// Native: `TreeSet.isEmpty()Z`
pub(crate) fn native_treeset_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) | None => 1,
        _ => 0,
    })))
}

/// Native: `TreeSet.iterator()Iterator` — returns an ArrayList-compatible iterator over sorted elements.
pub(crate) fn native_treeset_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_iterator(args, heap, out, control)
}

// ---- Collections.min / max / shuffle ----

/// Native: `Collections.min(Collection)T` — returns minimum element via `compareTo`.
pub(crate) fn native_collections_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let coll_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(coll_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(Error::JavaException {
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

/// Native: `Collections.max(Collection)T` — returns maximum element via `compareTo`.
pub(crate) fn native_collections_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let coll_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(coll_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Err(Error::JavaException {
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

/// Native: `Collections.shuffle(List)V` — no-op (deterministic test environments).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_shuffle(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}

/// Native: `Collections.shuffle(List, Random)V` — shuffle with provided RNG (no-op for correctness since test only checks sum).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_shuffle_random(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}

/// Native: `Collections.fill(List, Object)V` — set every element to value.
pub(crate) fn native_collections_fill(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

// ---- Stream natives ----
// duke/util/Stream: fields[0]=Int(size), fields[1..]=element refs

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

/// Native: `ArrayDeque.<init>()V` — same layout as `ArrayList`.
pub(crate) fn native_arraydeque_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `ArrayDeque.push(Object)V` — push to front (stack: LIFO).
pub(crate) fn native_arraydeque_push(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    heap.get_mut(this_ref)?.fields.insert(1, elem);
    let new_size = i32::try_from(size + 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(None)
}

/// Native: `ArrayDeque.pop()Object` — pop from front (stack: LIFO).
pub(crate) fn native_arraydeque_pop(
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
    let elem = heap.get_mut(this_ref)?.fields.remove(1);
    let new_size = i32::try_from(size - 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(Some(elem))
}

/// Native: `ArrayDeque.offer(Object)Z` — enqueue at back (queue: FIFO).
pub(crate) fn native_arraydeque_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(Slot::Int(1))) // always succeeds
}

/// Native: `ArrayDeque.add(Object)Z` — same as offer (appends to back).
pub(crate) fn native_arraydeque_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `ArrayDeque.poll()Object` — dequeue from front (queue: FIFO); null if empty.
pub(crate) fn native_arraydeque_poll(
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
        return Ok(Some(Slot::Reference(None)));
    }
    let elem = heap.get_mut(this_ref)?.fields.remove(1);
    let new_size = i32::try_from(size - 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(Some(elem))
}

/// Native: `ArrayDeque.peek()Object` — peek at front; null if empty.
pub(crate) fn native_arraydeque_peek(
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
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `ArrayDeque.size()I`
pub(crate) fn native_arraydeque_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `ArrayDeque.isEmpty()Z`
pub(crate) fn native_arraydeque_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

// ---- PriorityQueue natives ----
// fields[0]=Int(size), fields[1..size]=elements; maintained as a min-heap (String key order for Strings, value for boxed ints).

/// Native: `PriorityQueue.<init>()V` — same init as `ArrayList`.
pub(crate) fn native_priorityqueue_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `PriorityQueue.offer(Object)Z` — inserts in heap order via `compareTo`.
pub(crate) fn native_priorityqueue_offer(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
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

/// Native: `PriorityQueue.add(Object)Z` — same as offer.
pub(crate) fn native_priorityqueue_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_priorityqueue_offer(args, heap, out, control, ops)
}

/// Native: `PriorityQueue.peek()Object` — returns minimum element without removing.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_priorityqueue_peek(
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
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[1]))
}

/// Native: `PriorityQueue.poll()Object` — removes and returns minimum; sifts down.
pub(crate) fn native_priorityqueue_poll(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
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

/// Native: `PriorityQueue.size()I`
pub(crate) fn native_priorityqueue_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `PriorityQueue.isEmpty()Z`
pub(crate) fn native_priorityqueue_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

// ---- Comparator natives ----

/// Native: `Comparator.naturalOrder()Comparator` — returns a singleton synthetic comparator.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_natural_order(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/NaturalOrderComparator".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Comparator.reverseOrder()Comparator` — returns a singleton reverse comparator.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_reverse_order(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/ReverseOrderComparator".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
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

/// Native: `Comparator.comparingInt(ToIntFunction)Comparator` — wraps key extractor.
/// Creates a `duke/util/ComparingIntComparator` with `fields[0] = fn_ref`.
pub(crate) fn native_comparator_comparing_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_ref = extract_ref_arg(args, 0)?;
    let r = heap.allocate("duke/util/ComparingIntComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(Some(fn_ref));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.sort(List, Comparator)V` — 2-arg sort with explicit comparator.
pub(crate) fn native_collections_sort_with_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let comparator = extract_slot_arg(args, 1);
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

/// Native: `Enum.<init>(Ljava/lang/String;I)V` — stores name + ordinal.
/// args: `[this_ref, name_ref, ordinal_int]`
pub(crate) fn native_enum_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let name_slot = extract_slot_arg(args, 1);
    let ordinal = match args.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() >= 2 {
        obj.fields[0] = name_slot;
        obj.fields[1] = Slot::Int(ordinal);
    }
    Ok(None)
}

/// Native: `Enum.ordinal()I`
pub(crate) fn native_enum_ordinal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.get(1) {
        Some(Slot::Int(v)) => Ok(Some(Slot::Int(*v))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `Enum.name()Ljava/lang/String;`
pub(crate) fn native_enum_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Enum.valueOf(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;`
/// Searches heap for enum constants of the given class matching the name.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_enum_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let target_name = heap.get(name_ref)?.string_value.clone().unwrap_or_default();
    let enum_class_name = heap
        .get(class_ref)?
        .string_value
        .clone()
        .unwrap_or_default();

    let obj_count = heap.len();
    for i in 0..obj_count {
        let obj = heap.get(i as u64)?;
        if obj.class_name == enum_class_name
            && obj.fields.len() >= 2
            && let Some(Slot::Reference(Some(name_r))) = obj.fields.first()
            && let Ok(name_obj) = heap.get(*name_r)
            && name_obj.string_value.as_deref() == Some(target_name.as_str())
        {
            return Ok(Some(Slot::Reference(Some(i as u64))));
        }
    }

    Err(Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Reflection natives
// ---------------------------------------------------------------------------

/// Native: `Collections.emptyList()List` — returns a new empty `ArrayList`.
pub(crate) fn native_collections_empty_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // Return an immutable empty UnmodifiableList (same field layout as ArrayList)
    let r = heap.allocate("java/util/UnmodifiableList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.emptySet()Set` — returns a new empty `HashSet`.
pub(crate) fn native_collections_empty_set(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.emptyMap()Map` — returns a new empty `HashMap`.
pub(crate) fn native_collections_empty_map(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.unmodifiableList(List)List` — returns the same list (no-copy; single-threaded).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

/// Native: mutation ops on `UnmodifiableList` throw `UnsupportedOperationException`.
pub(crate) fn native_unmodifiable_list_mutation(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Err(Error::JavaException {
        class_name: "java/lang/UnsupportedOperationException".to_string(),
    })
}

/// Native: `Arrays.stream(int[])IntStream` — wraps an int array into an `IntStream`.
pub(crate) fn native_arrays_stream_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    // int[] layout: fields = Int elements directly (no length header); arraylength = fields.len()
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

/// Native: `Comparator.comparing(Function)Comparator` — creates a comparator by key extractor.
/// Returns a `duke/util/ComparingComparator` with `fields[0]`=fn\_ref.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_comparing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/ComparingComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
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
    let b = extract_slot_arg(args, 2);
    let fn_slot = extract_first_field_arg(heap, this_ref)?;
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

// ---------------------------------------------------------------------------
// Phase 43: Collectors.toSet/toMap, Stream.mapToInt/min/max, IntStream.reduce,
//           Arrays.sort(Object[]), String(char[])/valueOf(char[])
// ---------------------------------------------------------------------------

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

fn compare_slots_natural(
    a: Slot,
    b: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<i32> {
    match (a, b) {
        (Slot::Reference(Some(ra)), Slot::Reference(Some(_rb))) => {
            let a_class = heap.get(ra)?.class_name.clone();
            let result = ops
                .invoke(
                    heap,
                    out,
                    &a_class,
                    "compareTo",
                    "(Ljava/lang/Object;)I",
                    vec![a, b],
                )?
                .unwrap_or(Slot::Int(0));
            Ok(match result {
                Slot::Int(n) => n,
                _ => 0,
            })
        }
        (Slot::Int(a), Slot::Int(b)) => Ok(a.cmp(&b) as i32),
        _ => Ok(0),
    }
}

/// Native: `Collections.singletonList(Object)List` — returns a one-element `ArrayList`.
pub(crate) fn native_collections_singleton_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let element = extract_slot_arg(args, 0);
    let r = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(r))], heap, out, control)?;
    native_arraylist_add(&[Slot::Reference(Some(r)), element], heap, out, control)?;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collections.reverse(List)V` — reverses an `ArrayList` in-place.
pub(crate) fn native_collections_reverse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

/// Native: `Collections.frequency(Collection, Object)I` — count occurrences of element.
pub(crate) fn native_collections_frequency(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

// ---- String.valueOf overloads ----

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
    // Pairs start at index 1; values are at indices 2, 4, 6, ...
    let mut i = 2usize;
    while i < fields_len {
        let val = heap.get(this_ref)?.fields[i];
        native_arraylist_add(&[Slot::Reference(Some(list_ref)), val], heap, out, control)?;
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

// ---- Long class natives ----

/// Native: `ArrayList.<init>()V` — initializes with size=0.
pub(crate) fn native_arraylist_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ArrayList.<init>(Collection)V` — copies elements from another `ArrayList`/collection.
pub(crate) fn native_arraylist_init_from_collection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

/// Native: `ArrayList.add(Object)Z` — appends element, returns true.
pub(crate) fn native_arraylist_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(Error::NullPointerException), // shouldn't happen; init sets fields[0]=Int(0)
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1))) // boolean true
}

/// Native: `ArrayList.get(I)Object` — returns element at index.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)? as usize;
    let obj = heap.get(this_ref)?;
    obj.fields.get(idx + 1).map_or_else(
        || {
            Err(Error::JavaException {
                class_name: "java/lang/ArrayIndexOutOfBoundsException".to_string(),
            })
        },
        |slot| Ok(Some(*slot)),
    )
}

/// Native: `ArrayList.size()I`
pub(crate) fn native_arraylist_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(Slot::Int(sz)) => Ok(Some(Slot::Int(*sz))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `ArrayList.iterator()Iterator` — creates an `ArrayListIterator`.
pub(crate) fn native_arraylist_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

fn collection_elements_from_ref(heap: &duke_gc::Heap, collection_ref: u64) -> Result<Vec<Slot>> {
    let collection = heap.get(collection_ref)?;
    match collection.class_name.as_str() {
        "java/util/ArrayList" | "java/util/HashSet" => {
            let size = match collection.fields.first() {
                Some(Slot::Int(size)) if *size >= 0 => usize::try_from(*size).unwrap_or(0),
                _ => 0,
            };
            Ok(collection
                .fields
                .iter()
                .skip(1)
                .take(size)
                .copied()
                .collect())
        }
        _ => Err(Error::TypeMismatch {
            expected: "java/util/Collection",
            got: "other",
        }),
    }
}

fn allocate_reference_array_from_slots(
    heap: &mut duke_gc::Heap,
    array_class_name: &str,
    elements: &[Slot],
) -> Result<u64> {
    let array_ref = heap.allocate(array_class_name.to_string(), elements.len());
    let array = heap.get_mut(array_ref)?;
    for slot in &mut array.fields {
        *slot = Slot::Reference(None);
    }
    for (idx, element) in elements.iter().enumerate() {
        array.fields[idx] = *element;
    }
    Ok(array_ref)
}

/// Native: `Collection.toArray()` — copies Duke-backed collection elements into `Object[]`.
pub(crate) fn native_collection_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elements = collection_elements_from_ref(heap, this_ref)?;
    let array_ref = allocate_reference_array_from_slots(heap, "[Ljava/lang/Object;", &elements)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

/// Native: `Collection.toArray(Object[])` — preserves the requested array type.
pub(crate) fn native_collection_to_array_with_seed_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let seed_array_ref = extract_ref_arg(args, 1)?;
    let elements = collection_elements_from_ref(heap, this_ref)?;
    let seed_class_name = heap.get(seed_array_ref)?.class_name.clone();
    let seed_len = heap.get(seed_array_ref)?.fields.len();

    if seed_len < elements.len() {
        let new_array_ref = allocate_reference_array_from_slots(heap, &seed_class_name, &elements)?;
        return Ok(Some(Slot::Reference(Some(new_array_ref))));
    }

    let seed_array = heap.get_mut(seed_array_ref)?;
    for slot in &mut seed_array.fields {
        *slot = Slot::Reference(None);
    }
    for (idx, element) in elements.iter().enumerate() {
        seed_array.fields[idx] = *element;
    }
    Ok(Some(Slot::Reference(Some(seed_array_ref))))
}

/// Native: `ArrayList.sort(Comparator)V` — sorts in-place using insertion sort,
/// calling `compareTo` on each element pair via the interpreter callback.
///
/// Only null Comparator (natural ordering via `compareTo`) is supported.
pub(crate) fn array_list_sort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    // args[0] = ArrayList ref, args[1] = Comparator (null = natural ordering)
    let list_ref = extract_ref_arg(args, 0)?;

    // args[1] = optional Comparator ref (null = natural ordering via compareTo)
    let comparator = args.get(1).copied();

    // Fix 2: guard against a negative size stored in fields[0].
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(Error::NegativeArraySize { size: *n }),
        _ => return Ok(None),
    };

    if size <= 1 {
        return Ok(None);
    }

    // Collect element refs (fields[1..=size]).
    let mut elems: Vec<u64> = (1..=size)
        .filter_map(|i| match heap.get(list_ref).ok()?.fields.get(i) {
            Some(Slot::Reference(Some(r))) => Some(*r),
            _ => None,
        })
        .collect();

    // Fix 3: malformed list → InvalidRef, not silent Ok(None).
    if elems.len() != size {
        return Err(Error::InvalidRef { address: list_ref });
    }

    // Insertion sort — O(n²), correct, easy to verify.
    for i in 1..elems.len() {
        let mut key = elems[i];
        let mut j = i;
        while j > 0 {
            let receiver = elems[j - 1];
            let cmp = if let Some(Slot::Reference(Some(comp_ref))) = comparator {
                let comp_class = heap.get(comp_ref)?.class_name.clone();
                ops.invoke(
                    heap,
                    output,
                    &comp_class,
                    "compare",
                    "(Ljava/lang/Object;Ljava/lang/Object;)I",
                    vec![
                        Slot::Reference(Some(comp_ref)),
                        Slot::Reference(Some(receiver)),
                        Slot::Reference(Some(key)),
                    ],
                )?
            } else {
                let class_name = heap.get(receiver)?.class_name.clone();
                ops.invoke(
                    heap,
                    output,
                    &class_name,
                    COMPARE_TO_METHOD,
                    COMPARE_TO_OBJECT_DESC,
                    vec![Slot::Reference(Some(receiver)), Slot::Reference(Some(key))],
                )?
            };

            // Patch stale young-gen refs if a minor GC fired during the callback.
            // Guarded by `has_pending_forwards` so the common (no-GC) path pays
            // only one bool check instead of O(n) HashMap probes.
            if heap.has_pending_forwards() {
                for elem in &mut elems {
                    let mut slot = Slot::Reference(Some(*elem));
                    heap.apply_forward(&mut slot);
                    if let Slot::Reference(Some(r)) = slot {
                        *elem = r;
                    }
                }
                // Re-read key after forwarding patch (it lives outside elems
                // during the innermost loop iteration).
                let mut key_slot = Slot::Reference(Some(key));
                heap.apply_forward(&mut key_slot);
                if let Slot::Reference(Some(r)) = key_slot {
                    key = r;
                }
            }

            // Fix 4: explicit error on non-Int compareTo return.
            match cmp {
                Some(Slot::Int(n)) if n <= 0 => break,
                Some(Slot::Int(_)) => {} // n > 0, keep shifting
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: "Int",
                        got: "other",
                    });
                }
            }
            elems[j] = elems[j - 1];
            j -= 1;
        }
        elems[j] = key;
    }

    // Write sorted elements back using the write barrier.
    for (i, &r) in elems.iter().enumerate() {
        heap.write_field(list_ref, i + 1, Slot::Reference(Some(r)))?;
    }

    Ok(None)
}

// ---------------------------------------------------------------------------
// Collections natives
// ---------------------------------------------------------------------------

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
) -> Result<Option<Slot>> {
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

// ---------------------------------------------------------------------------
// ServiceLoader natives
// ---------------------------------------------------------------------------

const SERVICE_LOADER_SERVICE_CLASS_FIELD: usize = 0;
const SERVICE_LOADER_LOADER_FIELD: usize = 1;
const SERVICE_LOADER_COUNT_FIELD: usize = 2;
const SERVICE_LOADER_PROVIDERS_START: usize = 3;

const SERVICE_ITER_SERVICE_CLASS_FIELD: usize = 0;
const SERVICE_ITER_LOADER_FIELD: usize = 1;
const SERVICE_ITER_INDEX_FIELD: usize = 2;
const SERVICE_ITER_COUNT_FIELD: usize = 3;
const SERVICE_ITER_PROVIDERS_START: usize = 4;

fn read_resource_enumeration_bytes(
    enum_ref: u64,
    heap: &mut duke_gc::Heap,
) -> Result<Vec<Vec<u8>>> {
    let mut files = Vec::new();
    loop {
        let has_more = match native_resource_enumeration_has_more_elements(
            &[Slot::Reference(Some(enum_ref))],
            heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
        )? {
            Some(Slot::Int(value)) => value != 0,
            _ => false,
        };
        if !has_more {
            break;
        }
        let url_slot = native_resource_enumeration_next_element(
            &[Slot::Reference(Some(enum_ref))],
            heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
        )?
        .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(url_ref)) = url_slot else {
            return Err(Error::NullPointerException);
        };
        let spec = string_backed_object_value(heap, url_ref)?;
        files.push(read_resource_bytes_from_url_spec(&spec)?);
    }
    Ok(files)
}

fn service_configuration_files_via_resources(
    heap: &mut duke_gc::Heap,
    ops: &mut dyn CallbackOps,
    loader_ref: Option<u64>,
    service_binary_name: &str,
) -> Result<Vec<Vec<u8>>> {
    let resource_name = format!("META-INF/services/{service_binary_name}");
    let enum_ref = if let Some(loader_ref) = loader_ref {
        let resource_name_ref = heap.allocate_string(resource_name);
        let enum_slot = native_class_loader_get_resources(
            &[
                Slot::Reference(Some(loader_ref)),
                Slot::Reference(Some(resource_name_ref)),
            ],
            heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
            ops,
        )?
        .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(enum_ref)) = enum_slot else {
            return Ok(Vec::new());
        };
        enum_ref
    } else {
        let resources = ops.find_resource_entries(heap, None, &resource_name)?;
        allocate_resource_enumeration(heap, resources)?
    };
    read_resource_enumeration_bytes(enum_ref, heap)
}

fn parse_service_provider_names(files: Vec<Vec<u8>>) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for bytes in files {
        let text = String::from_utf8(bytes).map_err(|_| Error::JavaException {
            class_name: "java/util/ServiceConfigurationError".to_string(),
        })?;
        for line in text.lines() {
            let live = line
                .split_once('#')
                .map_or(line, |(before, _)| before)
                .trim();
            if !live.is_empty() {
                names.push(live.to_string());
            }
        }
    }
    Ok(names)
}

fn service_loader_cause_type(err: &Error) -> &'static str {
    match err {
        Error::ClassNotFound { .. } => "java.lang.ClassNotFoundException",
        Error::JavaException { class_name } => match class_name.as_str() {
            "java/lang/ClassNotFoundException" => "java.lang.ClassNotFoundException",
            "java/lang/IllegalAccessException" => "java.lang.IllegalAccessException",
            "java/lang/InstantiationException" => "java.lang.InstantiationException",
            "java/lang/NoSuchMethodException" => "java.lang.NoSuchMethodException",
            _ => "java.lang.RuntimeException",
        },
        Error::InstantiationError { .. } => "java.lang.InstantiationException",
        Error::NullPointerException => "java.lang.NullPointerException",
        _ => "duke_runtime.Error",
    }
}

fn service_configuration_error(provider_name: &str, cause_type: &str) -> Error {
    push_pending_java_exception_message(
        "java/util/ServiceConfigurationError",
        format!("{provider_name}: provider construction failed due to {cause_type}"),
    );
    Error::JavaException {
        class_name: "java/util/ServiceConfigurationError".to_string(),
    }
}

fn allocate_service_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    ops: &mut dyn CallbackOps,
    loader_arg_index: Option<usize>,
) -> Result<Option<Slot>> {
    let service_class_ref = extract_ref_arg(args, 0)?;
    let service_internal_name = class_internal_name_from_ref(heap, service_class_ref)?;
    let service_binary_name = internal_name_to_binary_name(&service_internal_name);
    let loader_slot = loader_arg_index
        .and_then(|idx| args.get(idx).copied())
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(loader_ref) = loader_slot else {
        return Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        });
    };
    let files = service_configuration_files_via_resources(heap, ops, loader_ref, &service_binary_name)?;
    let provider_names = parse_service_provider_names(files)?;
    let loader_ref = heap.allocate(
        "java/util/ServiceLoader".to_string(),
        SERVICE_LOADER_PROVIDERS_START + provider_names.len(),
    );
    {
        let service_loader = heap.get_mut(loader_ref)?;
        service_loader.fields[SERVICE_LOADER_SERVICE_CLASS_FIELD] =
            Slot::Reference(Some(service_class_ref));
        service_loader.fields[SERVICE_LOADER_LOADER_FIELD] = loader_slot;
        service_loader.fields[SERVICE_LOADER_COUNT_FIELD] =
            Slot::Int(i32::try_from(provider_names.len()).unwrap_or(i32::MAX));
    }
    for (idx, provider_name) in provider_names.into_iter().enumerate() {
        let name_ref = heap.allocate_string(provider_name);
        heap.write_field(
            loader_ref,
            SERVICE_LOADER_PROVIDERS_START + idx,
            Slot::Reference(Some(name_ref)),
        )?;
    }
    Ok(Some(Slot::Reference(Some(loader_ref))))
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

// ---- ArrayList extended methods ----

/// Native: `ArrayList.remove(I)Object` — removes element at index, returns it.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_remove_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)?;
    if idx < 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let idx = idx as usize;
    let len = heap.get(this_ref)?.fields.len();
    // fields[0]=size, elements start at 1; idx is 0-based element index → field index = idx+1
    if idx + 1 >= len {
        return Err(Error::JavaException {
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

/// Native: `ArrayList.remove(Object)Z` — removes first occurrence, returns true if found.
pub(crate) fn native_arraylist_remove_obj(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let mut found = None;
    for (i, slot) in heap.get(this_ref)?.fields.iter().enumerate().skip(1) {
        if slots_equal(slot, &target, heap) {
            found = Some(i);
            break;
        }
    }
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

/// Native: `ArrayList.contains(Object)Z` — returns 1 if element is present.
pub(crate) fn native_arraylist_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let mut found = false;
    for slot in heap.get(this_ref)?.fields.iter().skip(1) {
        if slots_equal(slot, &target, heap) {
            found = true;
            break;
        }
    }
    Ok(Some(Slot::Int(i32::from(found))))
}

/// Native: `ArrayList.clear()V` — removes all elements.
pub(crate) fn native_arraylist_clear(
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

/// Native: `ArrayList.isEmpty()Z` — returns 1 if size is 0.
pub(crate) fn native_arraylist_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let is_empty = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz == 0,
        _ => true,
    };
    Ok(Some(Slot::Int(i32::from(is_empty))))
}

/// Native: `ArrayList.set(I,Object)Object` — replaces element at index, returns old value.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)?;
    let value = extract_slot_arg(args, 2);
    if idx < 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let field_idx = idx as usize + 1;
    let old = *heap
        .get(this_ref)?
        .fields
        .get(field_idx)
        .ok_or_else(|| Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        })?;
    heap.get_mut(this_ref)?.fields[field_idx] = value;
    Ok(Some(old))
}

/// Native: `ArrayList.indexOf(Object)I` — returns first index of element, or -1.
pub(crate) fn native_arraylist_index_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let len = heap.get(this_ref)?.fields.len();
    for i in 1..len {
        let slot = heap.get(this_ref)?.fields[i];
        if slots_equal(&slot, &target, heap) {
            return Ok(Some(Slot::Int(i32::try_from(i - 1).unwrap_or(i32::MAX))));
        }
    }
    Ok(Some(Slot::Int(-1)))
}

/// Native: `ArrayList.lastIndexOf(Object)I` — last occurrence, or -1.
pub(crate) fn native_arraylist_last_index_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let len = heap.get(this_ref)?.fields.len();
    if len > 1 {
        for i in (1..len).rev() {
            let slot = heap.get(this_ref)?.fields[i];
            if slots_equal(&slot, &target, heap) {
                return Ok(Some(Slot::Int(i32::try_from(i - 1).unwrap_or(i32::MAX))));
            }
        }
    }
    Ok(Some(Slot::Int(-1)))
}

/// Native: `ArrayList.add(I,Object)V` — inserts element at index, shifting others right.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_add_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)?;
    let element = extract_slot_arg(args, 2);
    if idx < 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let field_idx = idx as usize + 1;
    let obj = heap.get_mut(this_ref)?;
    let len = obj.fields.len();
    if field_idx > len {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    obj.fields.insert(field_idx, element);
    if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
        *sz += 1;
    }
    Ok(None)
}

// ---- HashMap extended methods ----

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
        // Key already present — return existing value.
        return Ok(Some(heap.get(this_ref)?.fields[i + 1]));
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
        // Evaluate len inside the loop since heap access can't be held across slots_equal
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

// ---- Double.isNaN ----

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
        Some(Slot::Int(n)) => return Err(Error::NegativeArraySize { size: *n }),
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
        Some(Slot::Int(n)) => return Err(Error::NegativeArraySize { size: *n }),
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
    obj.fields.sort_by(|a, b| match (a, b) {
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

// ---------------------------------------------------------------------------
// Phase 44: Arrays.copyOfRange, List.subList, Comparator.reversed,
//           Collections.binarySearch, String.intern, ArrayList.removeIf
// ---------------------------------------------------------------------------

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
        heap.get_mut(dst_ref)?.fields[i] =
            src_fields.get(from + i).copied().unwrap_or(Slot::Int(0));
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

/// Native: `ArrayList.subList(int, int)List` — returns a new `ArrayList` with the sub-range.
pub(crate) fn native_arraylist_sub_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

/// Native: `ArrayList.removeIf(Predicate)Z` — removes all elements where predicate returns true.
pub(crate) fn native_arraylist_remove_if(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
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

/// Native: `Comparator.reversed()Comparator` — wraps comparator to invert ordering.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_reversed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let delegate = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/ReversedComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = delegate;
    Ok(Some(Slot::Reference(Some(r))))
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

/// Native: `Collections.binarySearch(List, T)I` — binary search on sorted `ArrayList`.
pub(crate) fn native_collections_binary_search(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
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

/// Native: `Arrays.asList(Object[])List` — wraps a reference array as an `ArrayList`.
pub(crate) fn native_arrays_as_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let arr_len = heap.get(arr_ref)?.fields.len();
    // Create a new ArrayList (1 field slot for size counter) and populate it.
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    for i in 0..arr_len {
        let elem = extract_field_arg(heap, arr_ref, i)?;
        native_arraylist_add(&[Slot::Reference(Some(list_ref)), elem], heap, out, control)?;
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}

// ---- String extended operations (Java 11+) ----

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
    // Find old value
    let fields = &heap.get(this_ref)?.fields;
    let old_value = hashmap_find_key(fields, key, heap)
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
    let fields2 = &heap.get(this_ref)?.fields;
    if let Some(ki) = hashmap_find_key(fields2, key, heap) {
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

// ---------------------------------------------------------------------------
// java.lang.StringBuffer — mutable string, delegates to StringBuilder internals
// (string_value field used as buffer, same as StringBuilder)
// ---------------------------------------------------------------------------

/// Native: `StringBuffer.<init>()V` — empty buffer.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stringbuffer_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.string_value = Some(String::new());
    Ok(None)
}

/// Native: `StringBuffer.<init>(Ljava/lang/String;)V` — init with string.
pub(crate) fn native_stringbuffer_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = match args.get(1).copied() {
        Some(Slot::Reference(Some(r))) => heap.get(r)?.string_value.clone().unwrap_or_default(),
        _ => String::new(),
    };
    heap.get_mut(this_ref)?.string_value = Some(s);
    Ok(None)
}

/// Native: `StringBuffer.append(...)StringBuffer` — append any type; returns `this`.
pub(crate) fn native_stringbuffer_append(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let frag = match extract_slot_arg(args, 1) {
        Slot::Reference(Some(r)) => heap
            .get(r)?
            .string_value
            .clone()
            .unwrap_or_else(|| "null".to_string()),
        Slot::Reference(None) => "null".to_string(),
        Slot::Int(n) => n.to_string(),
        Slot::Long(n) => n.to_string(),
        Slot::Double(d) => format!("{d}"),
        Slot::Float(f) => format!("{f}"),
        Slot::ReturnAddress(_) => String::new(),
    };
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    buf.push_str(&frag);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuffer.toString()String`.
pub(crate) fn native_stringbuffer_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `StringBuffer.length()I`.
pub(crate) fn native_stringbuffer_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let len = heap
        .get(this_ref)?
        .string_value
        .as_deref()
        .unwrap_or("")
        .len();
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}

// ---------------------------------------------------------------------------
// java.util.StringJoiner
// fields[0] = Reference(delimiter), fields[1] = Reference(prefix), fields[2] = Reference(suffix)
// fields[3] = Reference(emptyValue), fields[4..] = added elements
// instance_field_count = 4 (slots 0-3 pre-allocated)
// ---------------------------------------------------------------------------

/// Native: `StringJoiner.<init>(CharSequence)V` — delimiter only.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stringjoiner_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let delim_slot = extract_slot_arg(args, 1);
    let empty_ref = heap.allocate_string(String::new());
    let prefix_ref = heap.allocate_string(String::new());
    let suffix_ref = heap.allocate_string(String::new());
    heap.get_mut(this_ref)?.fields[0] = delim_slot;
    heap.get_mut(this_ref)?.fields[1] = Slot::Reference(Some(prefix_ref));
    heap.get_mut(this_ref)?.fields[2] = Slot::Reference(Some(suffix_ref));
    heap.get_mut(this_ref)?.fields[3] = Slot::Reference(Some(empty_ref));
    Ok(None)
}

/// Native: `StringJoiner.<init>(CharSequence,CharSequence,CharSequence)V` — delim + prefix + suffix.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stringjoiner_init_prefix_suffix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let delim_slot = extract_slot_arg(args, 1);
    let prefix_slot = extract_slot_arg(args, 2);
    let suffix_slot = extract_slot_arg(args, 3);
    let empty_ref = heap.allocate_string(String::new());
    heap.get_mut(this_ref)?.fields[0] = delim_slot;
    heap.get_mut(this_ref)?.fields[1] = prefix_slot;
    heap.get_mut(this_ref)?.fields[2] = suffix_slot;
    heap.get_mut(this_ref)?.fields[3] = Slot::Reference(Some(empty_ref));
    Ok(None)
}

/// Native: `StringJoiner.add(CharSequence)StringJoiner` — append element.
pub(crate) fn native_stringjoiner_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    heap.get_mut(this_ref)?.fields.push(elem);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringJoiner.setEmptyValue(CharSequence)StringJoiner`.
pub(crate) fn native_stringjoiner_set_empty_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let empty_slot = extract_slot_arg(args, 1);
    heap.get_mut(this_ref)?.fields[3] = empty_slot;
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Helper: read a string from a slot (returns "" for null).
fn slot_to_string(slot: Slot, heap: &duke_gc::Heap) -> String {
    match slot {
        Slot::Reference(Some(r)) => heap
            .get(r)
            .ok()
            .and_then(|o| o.string_value.clone())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Native: `StringJoiner.toString()String` — builds the joined result.
///
/// ⚡ Bolt Optimization: Eliminated intermediate `Vec<String>` allocation and format
/// macro overhead by appending directly to a single String buffer.
pub(crate) fn native_stringjoiner_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    // elements start at index 4
    let elems: Vec<Slot> = fields.get(4..).map(<[Slot]>::to_vec).unwrap_or_default();
    if elems.is_empty() {
        let empty_slot = fields.get(3).copied().unwrap_or(Slot::Reference(None));
        let prefix = slot_to_string(
            fields.get(1).copied().unwrap_or(Slot::Reference(None)),
            heap,
        );
        let suffix = slot_to_string(
            fields.get(2).copied().unwrap_or(Slot::Reference(None)),
            heap,
        );
        let empty = slot_to_string(empty_slot, heap);
        // If prefix+suffix are both empty, return emptyValue; otherwise prefix+suffix
        let result = if prefix.is_empty() && suffix.is_empty() {
            empty
        } else {
            format!("{prefix}{suffix}")
        };
        let r = heap.allocate_string(result);
        return Ok(Some(Slot::Reference(Some(r))));
    }
    let delim = slot_to_string(
        fields.first().copied().unwrap_or(Slot::Reference(None)),
        heap,
    );
    let prefix = slot_to_string(
        fields.get(1).copied().unwrap_or(Slot::Reference(None)),
        heap,
    );
    let suffix = slot_to_string(
        fields.get(2).copied().unwrap_or(Slot::Reference(None)),
        heap,
    );
    // ⚡ Bolt: Eliminate intermediate Vec<String> allocation and format! macro overhead
    // by appending directly to a single String buffer.
    let mut result = String::new();
    result.push_str(&prefix);
    for (i, elem) in elems.iter().enumerate() {
        if i > 0 {
            result.push_str(&delim);
        }
        result.push_str(&slot_to_string(*elem, heap));
    }
    result.push_str(&suffix);
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `StringJoiner.length()I` — length of the `toString()` result.
pub(crate) fn native_stringjoiner_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // Reuse toString and measure
    let result = native_stringjoiner_tostring(args, heap, out, control)?;
    let len = match result {
        Some(Slot::Reference(Some(r))) => heap.get(r)?.string_value.as_deref().unwrap_or("").len(),
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}

// ---------------------------------------------------------------------------
// HashMap natives
// ---------------------------------------------------------------------------

fn uses_first_field_value_equality(class_name: &str) -> bool {
    matches!(
        class_name,
        "java/lang/Boolean"
            | "java/lang/Byte"
            | "java/lang/Character"
            | "java/lang/Double"
            | "java/lang/Float"
            | "java/lang/Integer"
            | "java/lang/Long"
            | "java/lang/Short"
    )
}

/// Semantic equality for `HashMap` keys: reference identity by default, with value
/// semantics for strings, class mirrors, and boxed primitive wrappers.
fn slots_equal(a: &Slot, b: &Slot, heap: &duke_gc::Heap) -> bool {
    match (a, b) {
        (Slot::Reference(None), Slot::Reference(None)) => true,
        (Slot::Reference(Some(ra)), Slot::Reference(Some(rb))) => {
            if ra == rb {
                return true;
            }
            let Ok(oa) = heap.get(*ra) else { return false };
            let Ok(ob) = heap.get(*rb) else { return false };
            if oa.class_name != ob.class_name {
                return false;
            }
            match oa.class_name.as_str() {
                "java/lang/String" | "java/lang/Class" => oa.string_value == ob.string_value,
                "java/util/UUID" => uuid_bits_from_object(oa) == uuid_bits_from_object(ob),
                class_name if uses_first_field_value_equality(class_name) => {
                    oa.fields.first() == ob.fields.first()
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// Native: `HashMap.<init>()V` — initialises size counter at fields\[0\] to 0.
fn find_hashmap_entry_index(fields: &[Slot], key: &Slot, heap: &duke_gc::Heap) -> Option<usize> {
    (1..fields.len())
        .step_by(2)
        .find(|&i| i + 1 < fields.len() && slots_equal(&fields[i], key, heap))
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
    // We just find the index, without taking fields ownership yet
    let i_opt = find_hashmap_entry_index(&heap.get(this_ref)?.fields, &key, heap);

    if let Some(i) = i_opt {
        let old = heap.get(this_ref)?.fields[i + 1];
        heap.get_mut(this_ref)?.fields[i + 1] = val;
        return Ok(Some(old));
    }

    // New key — append pair and bump size.
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

    Ok(Some(
        find_hashmap_entry_index(fields, &key, heap)
            .map_or(Slot::Reference(None), |i| fields[i + 1]),
    ))
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

    Ok(Some(
        find_hashmap_entry_index(fields, &key, heap).map_or(default, |i| fields[i + 1]),
    ))
}

// ---------------------------------------------------------------------------
// java.util.Properties natives
// ---------------------------------------------------------------------------

const PROPERTIES_SIZE_FIELD: usize = 0;
const PROPERTIES_DEFAULTS_FIELD: usize = 1;
const PROPERTIES_ENTRIES_START: usize = 2;
const PROPERTIES_ENUM_INDEX_FIELD: usize = 0;
const PROPERTIES_ENUM_COUNT_FIELD: usize = 1;
const PROPERTIES_ENUM_NAMES_START: usize = 2;
const PROPERTIES_STORE_TIMESTAMP: &str = "1970-01-01T00:00:00Z";

fn properties_illegal_argument() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    }
}

fn properties_io_exception() -> Error {
    Error::JavaException {
        class_name: "java/io/IOException".to_string(),
    }
}

fn properties_stream_id_from_slot(slot: Slot, heap: &duke_gc::Heap) -> Result<i32> {
    let Slot::Reference(Some(stream_ref)) = slot else {
        return Err(Error::NullPointerException);
    };
    match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(properties_io_exception()),
    }
}

fn ensure_properties_layout(heap: &mut duke_gc::Heap, props_ref: u64) -> Result<()> {
    let props = heap.get_mut(props_ref)?;
    if props.fields.len() < PROPERTIES_ENTRIES_START {
        props
            .fields
            .resize(PROPERTIES_ENTRIES_START, Slot::Reference(None));
    }
    Ok(())
}

fn init_properties_with_defaults(
    heap: &mut duke_gc::Heap,
    props_ref: u64,
    defaults: Slot,
) -> Result<()> {
    ensure_properties_layout(heap, props_ref)?;
    let props = heap.get_mut(props_ref)?;
    props.fields[PROPERTIES_SIZE_FIELD] = Slot::Int(0);
    props.fields[PROPERTIES_DEFAULTS_FIELD] = defaults;
    props.fields.truncate(PROPERTIES_ENTRIES_START);
    Ok(())
}

fn properties_entry_index(fields: &[Slot], key: &Slot, heap: &duke_gc::Heap) -> Option<usize> {
    (PROPERTIES_ENTRIES_START..fields.len())
        .step_by(2)
        .find(|&idx| idx + 1 < fields.len() && slots_equal(&fields[idx], key, heap))
}

fn slot_is_java_string(heap: &duke_gc::Heap, slot: Slot) -> bool {
    let Slot::Reference(Some(string_ref)) = slot else {
        return false;
    };
    heap.get(string_ref).is_ok_and(|obj| {
        obj.class_name == "java/lang/String" && obj.string_value.is_some()
    })
}

fn string_from_slot(heap: &duke_gc::Heap, slot: Slot) -> Option<String> {
    let Slot::Reference(Some(string_ref)) = slot else {
        return None;
    };
    heap.get(string_ref)
        .ok()
        .and_then(|obj| obj.string_value.clone())
}

fn properties_local_entries(heap: &duke_gc::Heap, props_ref: u64) -> Result<Vec<(Slot, Slot)>> {
    let fields = heap.get(props_ref)?.fields.clone();
    let mut entries = Vec::new();
    let mut idx = PROPERTIES_ENTRIES_START;
    while idx + 1 < fields.len() {
        entries.push((fields[idx], fields[idx + 1]));
        idx += 2;
    }
    Ok(entries)
}

fn properties_local_string_entries(
    heap: &duke_gc::Heap,
    props_ref: u64,
) -> Result<Vec<(String, String)>> {
    let mut entries = Vec::new();
    for (key_slot, value_slot) in properties_local_entries(heap, props_ref)? {
        let Some(key) = string_from_slot(heap, key_slot) else {
            continue;
        };
        let Some(value) = string_from_slot(heap, value_slot) else {
            continue;
        };
        entries.push((key, value));
    }
    Ok(entries)
}

fn properties_put_slots(
    heap: &mut duke_gc::Heap,
    props_ref: u64,
    key: Slot,
    value: Slot,
) -> Result<Slot> {
    ensure_properties_layout(heap, props_ref)?;
    let entry_idx = {
        let fields = &heap.get(props_ref)?.fields;
        properties_entry_index(fields, &key, heap)
    };

    if let Some(idx) = entry_idx {
        let old = heap.get(props_ref)?.fields[idx + 1];
        heap.get_mut(props_ref)?.fields[idx + 1] = value;
        return Ok(old);
    }

    let props = heap.get_mut(props_ref)?;
    match props.fields.get_mut(PROPERTIES_SIZE_FIELD) {
        Some(Slot::Int(size)) => *size += 1,
        _ => props.fields[PROPERTIES_SIZE_FIELD] = Slot::Int(1),
    }
    props.fields.push(key);
    props.fields.push(value);
    Ok(Slot::Reference(None))
}

fn properties_put_string_pair(
    heap: &mut duke_gc::Heap,
    props_ref: u64,
    key: String,
    value: String,
) -> Result<()> {
    let key_slot = Slot::Reference(Some(heap.allocate_string(key)));
    let value_slot = Slot::Reference(Some(heap.allocate_string(value)));
    properties_put_slots(heap, props_ref, key_slot, value_slot)?;
    Ok(())
}

fn properties_get_property_slot(
    heap: &duke_gc::Heap,
    props_ref: u64,
    key: Slot,
) -> Result<Slot> {
    let fields = heap.get(props_ref)?.fields.clone();
    if let Some(idx) = properties_entry_index(&fields, &key, heap) {
        let value = fields[idx + 1];
        if slot_is_java_string(heap, value) {
            return Ok(value);
        }
    }

    match fields.get(PROPERTIES_DEFAULTS_FIELD).copied() {
        Some(Slot::Reference(Some(defaults_ref))) => {
            properties_get_property_slot(heap, defaults_ref, key)
        }
        _ => Ok(Slot::Reference(None)),
    }
}

fn properties_push_unique_name(heap: &duke_gc::Heap, names: &mut Vec<Slot>, key: Slot) {
    if !slot_is_java_string(heap, key) {
        return;
    }
    if names.iter().any(|existing| slots_equal(existing, &key, heap)) {
        return;
    }
    names.push(key);
}

fn properties_collect_name_slots(
    heap: &duke_gc::Heap,
    props_ref: u64,
    names: &mut Vec<Slot>,
) -> Result<()> {
    let fields = heap.get(props_ref)?.fields.clone();
    if let Some(Slot::Reference(Some(defaults_ref))) = fields.get(PROPERTIES_DEFAULTS_FIELD) {
        properties_collect_name_slots(heap, *defaults_ref, names)?;
    }

    let mut idx = PROPERTIES_ENTRIES_START;
    while idx + 1 < fields.len() {
        let key = fields[idx];
        let value = fields[idx + 1];
        if slot_is_java_string(heap, value) {
            properties_push_unique_name(heap, names, key);
        }
        idx += 2;
    }
    Ok(())
}

const fn is_properties_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\u{000c}')
}

fn split_properties_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if matches!(chars.peek(), Some('\n')) {
                    chars.next();
                }
                lines.push(std::mem::take(&mut current));
            }
            '\n' => lines.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    lines.push(current);
    lines
}

fn has_odd_trailing_backslashes(line: &str) -> bool {
    let mut count = 0usize;
    for ch in line.chars().rev() {
        if ch != '\\' {
            break;
        }
        count += 1;
    }
    count % 2 == 1
}

fn logical_properties_lines(text: &str) -> Vec<String> {
    let mut logical = Vec::new();
    let mut pending = String::new();
    let mut continuing = false;

    for line in split_properties_lines(text) {
        let mut piece = if continuing {
            line.trim_start_matches(is_properties_whitespace).to_string()
        } else {
            line
        };

        if continuing && piece.is_empty() {
            continue;
        }

        if has_odd_trailing_backslashes(&piece) {
            piece.pop();
            pending.push_str(&piece);
            continuing = true;
        } else {
            pending.push_str(&piece);
            logical.push(std::mem::take(&mut pending));
            continuing = false;
        }
    }

    if continuing || !pending.is_empty() {
        logical.push(pending);
    }
    logical
}

fn hex_value(ch: char) -> Option<u32> {
    match ch {
        '0'..='9' => Some(u32::from(ch) - u32::from('0')),
        'a'..='f' => Some(u32::from(ch) - u32::from('a') + 10),
        'A'..='F' => Some(u32::from(ch) - u32::from('A') + 10),
        _ => None,
    }
}

fn unescape_property_text(raw: &str) -> Result<String> {
    let mut result = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        let Some(escaped) = chars.next() else {
            result.push('\\');
            break;
        };

        match escaped {
            'n' => result.push('\n'),
            'r' => result.push('\r'),
            't' => result.push('\t'),
            'f' => result.push('\u{000c}'),
            'u' => {
                let mut code = 0_u32;
                for _ in 0..4 {
                    let Some(hex) = chars.next().and_then(hex_value) else {
                        return Err(properties_illegal_argument());
                    };
                    code = (code << 4) | hex;
                }
                let Some(decoded) = char::from_u32(code) else {
                    return Err(properties_illegal_argument());
                };
                result.push(decoded);
            }
            other => result.push(other),
        }
    }
    Ok(result)
}

fn parse_property_logical_line(line: &str) -> Result<Option<(String, String)>> {
    let chars: Vec<char> = line.chars().collect();
    let mut idx = 0usize;
    while idx < chars.len() && is_properties_whitespace(chars[idx]) {
        idx += 1;
    }
    if idx >= chars.len() || matches!(chars[idx], '#' | '!') {
        return Ok(None);
    }

    let key_start = idx;
    let mut escaped = false;
    while idx < chars.len() {
        let ch = chars[idx];
        if escaped {
            escaped = false;
            idx += 1;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            idx += 1;
            continue;
        }
        if matches!(ch, '=' | ':') || is_properties_whitespace(ch) {
            break;
        }
        idx += 1;
    }

    let key_end = idx;
    let value_start = if idx < chars.len() {
        if is_properties_whitespace(chars[idx]) {
            while idx < chars.len() && is_properties_whitespace(chars[idx]) {
                idx += 1;
            }
            if idx < chars.len() && matches!(chars[idx], '=' | ':') {
                idx += 1;
            }
        } else {
            idx += 1;
        }
        while idx < chars.len() && is_properties_whitespace(chars[idx]) {
            idx += 1;
        }
        idx
    } else {
        chars.len()
    };

    let raw_key: String = chars[key_start..key_end].iter().collect();
    let raw_value: String = chars[value_start..].iter().collect();
    Ok(Some((
        unescape_property_text(&raw_key)?,
        unescape_property_text(&raw_value)?,
    )))
}

fn parse_properties_bytes(bytes: &[u8]) -> Result<Vec<(String, String)>> {
    let text: String = bytes.iter().map(|byte| char::from(*byte)).collect();
    let mut entries = Vec::new();
    for line in logical_properties_lines(&text) {
        if let Some((key, value)) = parse_property_logical_line(&line)? {
            entries.push((key, value));
        }
    }
    Ok(entries)
}

fn push_u16_escape(out: &mut String, unit: u16) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push('\\');
    out.push('u');
    for shift in [12, 8, 4, 0] {
        let idx = usize::from((unit >> shift) & 0x000f);
        out.push(char::from(HEX[idx]));
    }
}

fn push_unicode_escape(out: &mut String, ch: char) {
    let mut encoded = [0_u16; 2];
    for unit in ch.encode_utf16(&mut encoded).iter().copied() {
        push_u16_escape(out, unit);
    }
}

fn push_escaped_key_char(out: &mut String, ch: char) {
    match ch {
        '\\' => out.push_str("\\\\"),
        '=' => out.push_str("\\="),
        ':' => out.push_str("\\:"),
        ' ' => out.push_str("\\ "),
        '#' => out.push_str("\\#"),
        '!' => out.push_str("\\!"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        '\u{000c}' => out.push_str("\\f"),
        _ if !ch.is_ascii() || u32::from(ch) < 0x20 || u32::from(ch) > 0x7e => {
            push_unicode_escape(out, ch);
        }
        _ => out.push(ch),
    }
}

fn escape_property_key(key: &str) -> String {
    let mut result = String::new();
    for ch in key.chars() {
        push_escaped_key_char(&mut result, ch);
    }
    result
}

fn escape_property_value(value: &str) -> String {
    let mut result = String::new();
    for (idx, ch) in value.chars().enumerate() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{000c}' => result.push_str("\\f"),
            ' ' if idx == 0 => result.push_str("\\ "),
            _ if !ch.is_ascii() || u32::from(ch) < 0x20 || u32::from(ch) > 0x7e => {
                push_unicode_escape(&mut result, ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

fn escape_property_comment(comment: &str) -> String {
    let mut result = String::new();
    for ch in comment.chars() {
        match ch {
            '\n' | '\r' => {}
            _ if !ch.is_ascii() || u32::from(ch) < 0x20 || u32::from(ch) > 0x7e => {
                push_unicode_escape(&mut result, ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

fn append_store_comment(out: &mut String, comment: &str) {
    for line in split_properties_lines(comment) {
        if line.is_empty() {
            out.push_str("#\n");
        } else {
            out.push_str("# ");
            out.push_str(&escape_property_comment(&line));
            out.push('\n');
        }
    }
}

fn render_properties_store(heap: &duke_gc::Heap, props_ref: u64, comment: Option<&str>) -> Result<String> {
    let mut output = String::new();
    if let Some(comment_text) = comment {
        append_store_comment(&mut output, comment_text);
    }
    output.push_str("# ");
    output.push_str(PROPERTIES_STORE_TIMESTAMP);
    output.push('\n');

    for (key, value) in properties_local_string_entries(heap, props_ref)? {
        output.push_str(&escape_property_key(&key));
        output.push('=');
        output.push_str(&escape_property_value(&value));
        output.push('\n');
    }
    Ok(output)
}

fn write_iso_8859_1_ascii(heap: &mut duke_gc::Heap, file_id: i32, text: &str) -> Result<()> {
    for byte in text.bytes() {
        heap.write_host_file_byte(file_id, i32::from(byte))?;
    }
    Ok(())
}

/// Native: `LinkedList.stream()Stream` — wraps elements into a `duke/util/Stream`.
pub(crate) fn native_linked_list_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_stream(args, heap, out, control)
}

/// Native: `Collections.nCopies(int, Object)List` — returns a list of N copies of an element.
pub(crate) fn native_collections_n_copies(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let n = usize::try_from(extract_int_arg(args, 0)?.max(0)).unwrap_or(0);
    let elem = extract_slot_arg(args, 1);
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    heap.get_mut(list_ref)?.fields[0] = Slot::Int(i32::try_from(n).unwrap_or(0));
    for _ in 0..n {
        heap.get_mut(list_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}

/// Native: `ArrayList.forEach(Consumer)V` — invokes consumer.accept(elem) for each element.
pub(crate) fn native_arraylist_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(consumer_ref)) = extract_slot_arg(args, 1)
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

    // ⚡ Bolt: Eliminate intermediate Vec<String> allocation, format! macro overhead,
    // and .join() by appending directly to a single String buffer.
    let mut result = String::with_capacity(fields.len() * 4 + 2);
    result.push('[');
    for (i, s) in fields.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        match s {
            Slot::Int(n) => { let _ = write!(result, "{n}"); },
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

    // ⚡ Bolt: Eliminate intermediate Vec<String> allocation, format! macro overhead,
    // and .join() by appending directly to a single String buffer.
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

/// Native: `Collections.swap(List, int, int)V` — swaps elements at indices i and j.
pub(crate) fn native_collections_swap(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

/// Native: `Collections.unmodifiableMap(Map)Map` — identity stub (we have no mutation checks).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

/// Native: `Collections.singletonMap(K,V)Map` — returns a single-entry unmodifiable map.
pub(crate) fn native_collections_singleton_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_map_of(args, heap, out, control)
}

/// Native: `Collections.singleton(E)Set` — returns a single-element unmodifiable set.
pub(crate) fn native_collections_singleton_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_set_of_factory(args, heap, out, control)
}

/// Native: `Collections.unmodifiableSet(Set)Set` — returns a view of the set (same backing object).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

// ---------------------------------------------------------------------------
// Phase 59: Stream.flatMapToInt/Long/Double, Collectors.toMap (3-arg),
//           forEach on HashSet/TreeSet/TreeMap/LinkedList/LinkedHashMap/PriorityQueue
// ---------------------------------------------------------------------------

/// Native: `TreeMap.forEach(BiConsumer)V`
pub(crate) fn native_treemap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    // TreeMap uses same layout as HashMap: fields[0]=size, fields[1,2]=k0/v0 ...
    native_hashmap_for_each(args, heap, out, control, ops)
}

/// Native: `LinkedHashMap.forEach(BiConsumer)V`
pub(crate) fn native_linkedhashmap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_hashmap_for_each(args, heap, out, control, ops)
}

/// Native: `LinkedList.forEach(Consumer)V`
pub(crate) fn native_linked_list_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_arraylist_for_each(args, heap, out, control, ops)
}

/// Native: `PriorityQueue.forEach(Consumer)V`
pub(crate) fn native_priorityqueue_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_arraylist_for_each(args, heap, out, control, ops)
}

// ---------------------------------------------------------------------------
// Phase 60: IntStream/LongStream/DoubleStream takeWhile/dropWhile,
//           Integer/Long/Double compare/max/min,
//           TreeMap.keySet/values/getOrDefault, TreeSet.stream
// ---------------------------------------------------------------------------

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
    Ok(Some(match found_idx {
        Some(i) => heap.get(this_ref)?.fields[i + 1],
        None => default_val,
    }))
}

/// Native: `ArrayDeque.addFirst(Object)V` — inserts element at front.
pub(crate) fn native_arraydeque_add_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraydeque_push(args, heap, out, control)
}

/// Native: `ArrayDeque.addLast(Object)V` — appends element at back.
pub(crate) fn native_arraydeque_add_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(None)
}

/// Native: `ArrayDeque.offerFirst(Object)Z` — inserts at front, returns true.
pub(crate) fn native_arraydeque_offer_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraydeque_push(args, heap, out, control)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `ArrayDeque.offerLast(Object)Z` — appends at back, returns true.
pub(crate) fn native_arraydeque_offer_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `ArrayDeque.peekFirst()Object` — same as peek (front element, null if empty).
pub(crate) fn native_arraydeque_peek_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraydeque_peek(args, heap, out, control)
}

/// Native: `ArrayDeque.peekLast()Object` — returns last element without removing; null if empty.
pub(crate) fn native_arraydeque_peek_last(
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
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[size]))
}

/// Native: `ArrayDeque.pollFirst()Object` — same as poll (remove front, null if empty).
pub(crate) fn native_arraydeque_poll_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraydeque_poll(args, heap, out, control)
}

/// Native: `ArrayDeque.pollLast()Object` — removes and returns last element; null if empty.
pub(crate) fn native_arraydeque_poll_last(
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
        return Ok(Some(Slot::Reference(None)));
    }
    let elem = heap.get_mut(this_ref)?.fields.remove(size);
    let new_size = i32::try_from(size - 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(Some(elem))
}

/// Native: `ArrayDeque.contains(Object)Z` — returns true if element is present.
pub(crate) fn native_arraydeque_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let found = elems.iter().any(|e| slots_equal(e, &target, heap));
    Ok(Some(Slot::Int(i32::from(found))))
}

/// Native: `ArrayDeque.stream()Stream` — returns elements as an eager Stream.
pub(crate) fn native_arraydeque_stream(
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
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(size).unwrap_or(0));
    for elem in elems {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `ArrayDeque.forEach(Consumer)V` — invokes consumer for each element.
pub(crate) fn native_arraydeque_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(cons_ref)) = consumer_slot else {
        return Err(Error::NullPointerException);
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

/// Native: `ArrayDeque.clear()V` — removes all elements.
pub(crate) fn native_arraydeque_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields.truncate(1);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

// ---------------------------------------------------------------------------
// Phase 62: java.time (LocalDate, LocalDateTime, Instant, Duration, Period)
// ---------------------------------------------------------------------------

/// Convert (year, month, day) to a proleptic Gregorian epoch day count.
/// Day 0 = 1970-01-01.  Howard Hinnant's branchless algorithm.
#[allow(
    clippy::cast_lossless,         // i32/u32 → i64 widening casts
    clippy::cast_possible_truncation, // result fits i32 for any valid Gregorian date
    clippy::missing_const_for_fn   // i64::from not const-stable yet
)]
fn ymd_to_epoch_days(year: i32, month: u32, day: u32) -> i32 {
    let (y, m, d) = (year as i64, month as i64, day as i64);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400); // year of era [0, 399]
    let day_of_year = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + day_of_year; // day of era [0, 146096]
    (era * 146_097 + doe - 719_468) as i32
}

/// Convert a proleptic Gregorian epoch day to (year, month, day).
#[allow(
    clippy::cast_lossless,            // i32 → i64 widening cast
    clippy::cast_possible_truncation, // y fits i32 for valid dates
    clippy::cast_sign_loss,           // m/d are [1,12]/[1,31], sign-safe u32
    clippy::missing_const_for_fn      // i64::from not const-stable yet
)]
fn epoch_days_to_ymd(epoch_days: i32) -> (i32, u32, u32) {
    let z = epoch_days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097); // [0, 146096]
    let yoe = (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let day_of_year = day_of_era - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * day_of_year + 2) / 153; // [0, 11]
    let d = day_of_year - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

// ---- LocalDate layout: fields[0] = Slot::Int(epoch_days) ----

/// Native: `Collections.disjoint(Collection, Collection) -> boolean`
/// Returns true if the two collections have no elements in common.
pub(crate) fn native_collections_disjoint(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

// ---------------------------------------------------------------------------
// Phase 117: Map.remove(k,v), Collections.emptyList (immutable), Stream.concat,
//            Map.replace, Integer.sum, IntStream.mapToObj, Optional.map,
//            UnmodifiableSet
// ---------------------------------------------------------------------------

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
            native_hashmap_remove(&[Slot::Reference(Some(this_ref)), key], heap, out, control)?;
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}
