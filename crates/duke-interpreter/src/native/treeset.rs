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
#[allow(clippy::cast_precision_loss)]
fn treeset_slot_sort_key(slot: Slot, heap: &duke_gc::Heap) -> Option<TreeSortKey> {
    match slot {
        Slot::Int(n) => Some(TreeSortKey::Num(f64::from(n))),
        Slot::Long(n) => Some(TreeSortKey::Num(n as f64)),
        Slot::Double(d) => Some(TreeSortKey::Num(d)),
        Slot::Float(f) => Some(TreeSortKey::Num(f64::from(f))),
        Slot::Reference(Some(r)) => {
            let obj = heap.get(r).ok()?;
            match obj.class_name.as_str() {
                "java/lang/Integer" | "java/lang/Long" | "java/lang/Short"
                | "java/lang/Byte" => {
                    match obj.fields.first() {
                        Some(Slot::Int(n)) => Some(TreeSortKey::Num(f64::from(*n))),
                        Some(Slot::Long(n)) => Some(TreeSortKey::Num(*n as f64)),
                        _ => None,
                    }
                }
                "java/lang/Double" | "java/lang/Float" => {
                    match obj.fields.first() {
                        Some(Slot::Double(d)) => Some(TreeSortKey::Num(*d)),
                        Some(Slot::Float(f)) => Some(TreeSortKey::Num(f64::from(*f))),
                        _ => None,
                    }
                }
                _ => obj.string_value.clone().map(TreeSortKey::Str),
            }
        }
        _ => None,
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
    for i in 0..size {
        let ex = heap.get(this_ref)?.fields[1 + i];
        let ex_key = treeset_slot_sort_key(ex, heap);
        if ex_key == elem_key {
            return Ok(Some(Slot::Int(0)));
        }
    }
    let insert_pos = {
        let mut pos = size;
        for i in 0..size {
            let ex = heap.get(this_ref)?.fields[1 + i];
            let ex_key = treeset_slot_sort_key(ex, heap);
            if let (Some(ek), Some(exk)) = (&elem_key, &ex_key) && ek.less_than(exk) {
                pos = i;
                break;
            }
        }
        pos
    };
    let obj = heap.get_mut(this_ref)?;
    obj.fields.insert(1 + insert_pos, elem);
    obj.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(1)))
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
            Slot::Reference(Some(r)) => {
                heap.get(*r).ok().and_then(|o| o.string_value.clone())
            }
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
    Ok(
        Some(
            match heap.get(this_ref)?.fields.first() {
                Some(Slot::Int(n)) => Slot::Int(*n),
                _ => Slot::Int(0),
            },
        ),
    )
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
        Some(Slot::Int(0)) | None => {
            Err(Error::JavaException {
                class_name: "java/util/NoSuchElementException".to_string(),
            })
        }
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
/// Native: `TreeSet.iterator()Iterator` — returns an ArrayList-compatible iterator over sorted elements.
pub(crate) fn native_treeset_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_iterator(args, heap, out, control)
}
/// Native: `TreeSet.forEach(Consumer)V`
pub(crate) fn native_treeset_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_hashset_for_each(args, heap, out, control, ops)
}
/// Native: `TreeSet.stream()Stream` — wraps sorted elements into a `duke/util/Stream`.
pub(crate) fn native_treeset_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_hashset_stream(args, heap, out, control)
}
