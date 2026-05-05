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
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    heap.get_mut(this_ref)?.fields.push(elem);
    let new_size = size + 1;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(new_size).unwrap_or(0));
    let mut i = new_size;
    while i > 1 {
        let parent_idx = i / 2;
        let child_slot = heap.get(this_ref)?.fields[i];
        let parent_slot = heap.get(this_ref)?.fields[parent_idx];
        let (Slot::Reference(Some(child_ref)), Slot::Reference(Some(parent_ref))) = (
            child_slot,
            parent_slot,
        ) else {
            break;
        };
        let class_child = heap.get(child_ref)?.class_name.clone();
        let cmp = ops
            .invoke(
                heap,
                out,
                &class_child,
                "compareTo",
                "(Ljava/lang/Object;)I",
                vec![
                    Slot::Reference(Some(child_ref)), Slot::Reference(Some(parent_ref)),
                ],
            )?;
        if matches!(cmp, Some(Slot::Int(n)) if n < 0) {
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
            let (Slot::Reference(Some(cur_ref)), Slot::Reference(Some(left_ref))) = (
                cur_slot,
                left_slot,
            ) else {
                break;
            };
            let class_left = heap.get(left_ref)?.class_name.clone();
            let cmp = ops
                .invoke(
                    heap,
                    out,
                    &class_left,
                    "compareTo",
                    "(Ljava/lang/Object;)I",
                    vec![
                        Slot::Reference(Some(left_ref)), Slot::Reference(Some(cur_ref)),
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
            let (Slot::Reference(Some(small_ref)), Slot::Reference(Some(right_ref))) = (
                small_slot,
                right_slot,
            ) else {
                break;
            };
            let class_right = heap.get(right_ref)?.class_name.clone();
            let cmp = ops
                .invoke(
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
