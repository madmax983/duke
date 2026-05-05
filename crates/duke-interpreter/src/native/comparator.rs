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
/// Native: `Comparator.thenComparing(Comparator)Comparator` — chains two comparators.
/// Stores primary in `fields[0]`, secondary in `fields[1]`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_then_comparing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let primary = extract_slot_arg(args, 0);
    let secondary = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ThenComparingComparator".to_string(), 2);
    heap.get_mut(r)?.fields[0] = primary;
    heap.get_mut(r)?.fields[1] = secondary;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Comparator.comparingLong(ToLongFunction)Comparator` — wraps key extractor.
/// Creates a `duke/util/ComparingLongComparator` with `fields[0] = fn_ref`.
pub(crate) fn native_comparator_comparing_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_ref = extract_ref_arg(args, 0)?;
    let r = heap.allocate("duke/util/ComparingLongComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(Some(fn_ref));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Creates a `duke/util/ComparingDoubleComparator` with `fields[0] = fn_ref`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_comparing_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_ref = extract_ref_arg(args, 0)?;
    let r = heap.allocate("duke/util/ComparingDoubleComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(Some(fn_ref));
    Ok(Some(Slot::Reference(Some(r))))
}
