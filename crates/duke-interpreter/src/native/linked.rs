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
/// Native: `LinkedList.stream()Stream` — wraps elements into a `duke/util/Stream`.
pub(crate) fn native_linked_list_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_stream(args, heap, out, control)
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
