/// Native: `Map.of(K,V,...)Map` — pairs of args become entries in a new `HashMap`.
pub(crate) fn native_map_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
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
/// Native: `Map$Entry.getKey()Object` — returns the key field.
pub(crate) fn native_map_entry_get_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_first_field_arg(heap, this_ref)?;
    Ok(Some(key))
}
/// Native: `Map$Entry.getValue()Object` — returns the value field.
pub(crate) fn native_map_entry_get_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_field_arg(heap, this_ref, 1)?;
    Ok(Some(val))
}
/// Native: `Map.copyOf(Map)Map` — returns an unmodifiable copy backed by `HashMap`.
pub(crate) fn native_map_copy_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let copy_ref = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(copy_ref))], heap, out, control)?;
    let size = match heap.get(src_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let pairs: Vec<Slot> = heap.get(src_ref)?.fields[1..=size * 2].to_vec();
    let mut i = 0;
    while i + 1 < pairs.len() {
        let k = pairs[i];
        let v = pairs[i + 1];
        native_hashmap_put(
            &[Slot::Reference(Some(copy_ref)), k, v],
            heap,
            out,
            control,
        )?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(copy_ref))))
}
/// Native: `Map.entry(K,V)Map.Entry` — creates an immutable Map.Entry.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_map_entry_factory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let key = extract_slot_arg(args, 0);
    let val = extract_slot_arg(args, 1);
    let r = heap.allocate("java/util/Map$Entry".to_string(), 2);
    heap.get_mut(r)?.fields[0] = key;
    heap.get_mut(r)?.fields[1] = val;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Map.ofEntries(Map.Entry[])Map` — builds a `HashMap` from varargs Entry array.
pub(crate) fn native_map_of_entries(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(map_ref))], heap, out, control)?;
    if let Some(Slot::Reference(Some(arr_ref))) = args.first().copied() {
        let entries: Vec<Slot> = heap.get(arr_ref)?.fields.clone();
        for entry_slot in entries {
            let Slot::Reference(Some(entry_ref)) = entry_slot else {
                continue;
            };
            let key = extract_first_field_arg(heap, entry_ref)?;
            let val = extract_field_arg(heap, entry_ref, 1)?;
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
