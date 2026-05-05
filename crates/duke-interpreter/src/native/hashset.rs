pub(crate) fn native_hashset_init(
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
pub(crate) fn native_hashset_init_from_collection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let mut this_ref = extract_ref_arg(args, 0)?;
    let collection_ref = extract_ref_arg(args, 1)?;
    native_hashset_init(&[Slot::Reference(Some(this_ref))], heap, output, control)?;
    let collection_class = heap.get(collection_ref)?.class_name.clone();
    let elements: Vec<Slot> = if collection_class == "java/util/HashSet" {
        heap.get(collection_ref)?.fields.iter().skip(1).copied().collect()
    } else {
        let array_slot = ops
            .invoke(
                heap,
                output,
                &collection_class,
                "toArray",
                "()[Ljava/lang/Object;",
                vec![Slot::Reference(Some(collection_ref))],
            )?;
        let Some(Slot::Reference(Some(array_ref))) = array_slot else {
            return Err(Error::TypeMismatch {
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
/// Native: `HashSet.add(Object)Z` — adds element if not already present.
/// Returns 1 if added, 0 if element was already in the set.
pub(crate) fn native_hashset_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    if find_hashset_entry_index(&heap.get(this_ref)?.fields, &element, heap).is_some() {
        return Ok(Some(Slot::Int(0)));
    }
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(Error::NullPointerException),
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1)))
}
/// Native: `HashSet.contains(Object)Z` — returns 1 if element is present, 0 otherwise.
pub(crate) fn native_hashset_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let fields = &heap.get(this_ref)?.fields;
    if find_hashset_entry_index(fields, &element, heap).is_some() {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}
/// Native: `HashSet.remove(Object)Z` — removes element if present, returns 1 if removed, 0 if absent.
/// Uses swap-remove (swaps target with last element) for O(1) deletion.
pub(crate) fn native_hashset_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let i_opt = find_hashset_entry_index(&heap.get(this_ref)?.fields, &element, heap);
    if let Some(i) = i_opt {
        let obj = heap.get_mut(this_ref)?;
        let last_idx = obj.fields.len() - 1;
        obj.fields.swap(i, last_idx);
        obj.fields.truncate(obj.fields.len() - 1);
        match obj.fields.first_mut() {
            Some(Slot::Int(sz)) => *sz -= 1,
            _ => return Err(Error::NullPointerException),
        }
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}
/// Native: `HashSet.size()I` — returns element count from fields\[0\].
pub(crate) fn native_hashset_size(
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
/// Native: `HashSet.isEmpty()Z` — returns 1 if size == 0, else 0.
pub(crate) fn native_hashset_is_empty(
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
/// Native: `HashSet.iterator()Iterator` — creates a `HashSetIterator`.
pub(crate) fn native_hashset_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_ref = heap.allocate("duke/util/HashSetIterator".to_string(), 2);
    {
        let iter_obj = heap.get_mut(iter_ref)?;
        iter_obj.fields[0] = Slot::Reference(Some(this_ref));
        iter_obj.fields[1] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
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
/// Native: `HashSet.stream()Stream` — wraps elements into a `duke/util/Stream`.
pub(crate) fn native_hashset_stream(
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
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(i32::try_from(size).unwrap_or(0));
    for elem in elems {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
/// Native: `HashSet.forEach(Consumer)V`
pub(crate) fn native_hashset_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(consumer_ref)) = extract_slot_arg(args, 1) else {
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
