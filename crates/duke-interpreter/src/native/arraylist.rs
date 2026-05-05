fn arraylist_reference_elements(
    heap: &duke_gc::Heap,
    list_ref: u64,
) -> Result<Vec<u64>> {
    let list_obj = heap.get(list_ref)?;
    let size = match list_obj.fields.first().copied() {
        Some(Slot::Int(value)) if value > 0 => usize::try_from(value).unwrap_or(0),
        _ => 0,
    };
    let mut refs = Vec::with_capacity(size);
    for slot in list_obj.fields.iter().skip(1).take(size) {
        if let Slot::Reference(Some(reference)) = slot {
            refs.push(*reference);
        }
    }
    Ok(refs)
}
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
        native_arraylist_add(
            &[Slot::Reference(Some(this_ref)), elem],
            heap,
            out,
            control,
        )?;
    }
    Ok(Some(Slot::Int(i32::from(modified))))
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
    let elems: Vec<Slot> = heap
        .get(list_ref)?
        .fields[1..=usize::try_from(size).unwrap_or(0)]
        .to_vec();
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(size);
    for elem in elems {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
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
        _ => return Err(Error::NullPointerException),
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1)))
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
    obj.fields
        .get(idx + 1)
        .map_or_else(
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
    let iter_ref = heap.allocate("duke/util/ArrayListIterator".to_string(), 3);
    {
        let iter_obj = heap.get_mut(iter_ref)?;
        iter_obj.fields[0] = Slot::Reference(Some(this_ref));
        iter_obj.fields[1] = Slot::Int(0);
        iter_obj.fields[2] = Slot::Int(-1);
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
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
    iter_obj.fields[2] = Slot::Int(cursor);
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
    let list_size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    let remove_idx = last as usize + 1;
    let list_obj = heap.get_mut(list_ref)?;
    let new_size = (list_size - 1) as usize;
    list_obj.fields[0] = Slot::Int(list_size - 1);
    list_obj.fields.remove(remove_idx);
    list_obj.fields.push(Slot::Int(0));
    if last < cursor {
        let iter_obj = heap.get_mut(this_ref)?;
        iter_obj.fields[1] = Slot::Int(cursor - 1);
    }
    heap.get_mut(this_ref)?.fields[2] = Slot::Int(-1);
    let _ = new_size;
    Ok(None)
}
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
    let src_elems: Vec<Slot> = {
        let fields = &heap.get(this_ref)?.fields;
        fields.iter().skip(1 + from).take(new_len).copied().collect()
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
            }
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
/// Native: `ArrayList.forEach(Consumer)V` — invokes consumer.accept(elem) for each element.
pub(crate) fn native_arraylist_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(consumer_ref)) = extract_slot_arg(args, 1) else {
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
