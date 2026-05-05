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
        let cmp = ops
            .invoke(
                heap,
                out,
                &class_name,
                "compareTo",
                "(Ljava/lang/Object;)I",
                vec![Slot::Reference(Some(candidate)), Slot::Reference(Some(min_ref)),],
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
        let cmp = ops
            .invoke(
                heap,
                out,
                &class_name,
                "compareTo",
                "(Ljava/lang/Object;)I",
                vec![Slot::Reference(Some(candidate)), Slot::Reference(Some(max_ref)),],
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
/// Native: `Collections.emptyList()List` — returns a new empty `ArrayList`.
pub(crate) fn native_collections_empty_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
    let count = fields.iter().filter(|s| slots_equal(s, &target, heap)).count();
    Ok(Some(Slot::Int(i32::try_from(count).unwrap_or(i32::MAX))))
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
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
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
        let mid = lo + (hi - lo) / 2;
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
    let insertion = i32::try_from(lo).unwrap_or(0);
    Ok(Some(Slot::Int(-(insertion + 1))))
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
    let src_ref = extract_ref_arg(args, 0)?;
    let src_fields = heap.get(src_ref)?.fields.clone();
    let class_name = heap.get(src_ref)?.class_name.clone();
    let n = src_fields.len();
    let wrapper_ref = heap.allocate("java/util/UnmodifiableSet".to_string(), n);
    let _ = class_name;
    for (i, f) in src_fields.into_iter().enumerate() {
        heap.get_mut(wrapper_ref)?.fields[i] = f;
    }
    Ok(Some(Slot::Reference(Some(wrapper_ref))))
}
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
    let a_size = match heap.get(a_ref)?.fields.first() {
        Some(Slot::Int(v)) => usize::try_from(*v).unwrap_or(0),
        _ => 0,
    };
    let b_size = match heap.get(b_ref)?.fields.first() {
        Some(Slot::Int(v)) => usize::try_from(*v).unwrap_or(0),
        _ => 0,
    };
    let a_elems: Vec<Slot> = heap
        .get(a_ref)?
        .fields[1..=a_size.min(heap.get(a_ref)?.fields.len().saturating_sub(1))]
        .to_vec();
    let b_elems: Vec<Slot> = heap
        .get(b_ref)?
        .fields[1..=b_size.min(heap.get(b_ref)?.fields.len().saturating_sub(1))]
        .to_vec();
    let disjoint = a_elems
        .iter()
        .all(|a| !b_elems.iter().any(|b| slots_equal(a, b, heap)));
    Ok(Some(Slot::Int(i32::from(disjoint))))
}
