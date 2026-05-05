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
    let result = native_stringjoiner_tostring(args, heap, out, control)?;
    let len = match result {
        Some(Slot::Reference(Some(r))) => {
            heap.get(r)?.string_value.as_deref().unwrap_or("").len()
        }
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}
