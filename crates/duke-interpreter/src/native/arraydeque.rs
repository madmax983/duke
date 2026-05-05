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
    Ok(Some(Slot::Int(1)))
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
