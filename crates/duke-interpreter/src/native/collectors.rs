/// Native: `Collectors.joining(delim)Collector` — returns a joining collector with delimiter.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_joining(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let delim = match args.first() {
        Some(Slot::Reference(Some(r))) => {
            heap.get(*r).ok().and_then(|o| o.string_value.clone()).unwrap_or_default()
        }
        _ => String::new(),
    };
    let collector_ref = make_joining_collector(heap, &delim, "", "");
    Ok(Some(Slot::Reference(Some(collector_ref))))
}
/// Native: `Collectors.joining()Collector` — no-arg version (empty delimiter).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_joining_no_arg(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let collector_ref = make_joining_collector(heap, "", "", "");
    Ok(Some(Slot::Reference(Some(collector_ref))))
}
/// Native: `Collectors.joining(delim, prefix, suffix)Collector` — full 3-arg joining collector.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_joining_full(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let read_str = |heap: &duke_gc::Heap, idx: usize| -> String {
        match args.get(idx) {
            Some(Slot::Reference(Some(r))) => {
                heap.get(*r)
                    .ok()
                    .and_then(|o| o.string_value.clone())
                    .unwrap_or_default()
            }
            _ => String::new(),
        }
    };
    let delim = read_str(heap, 0);
    let prefix = read_str(heap, 1);
    let suffix = read_str(heap, 2);
    let collector_ref = make_joining_collector(heap, &delim, &prefix, &suffix);
    Ok(Some(Slot::Reference(Some(collector_ref))))
}
/// Native: `Collectors.toList()Collector` — returns a sentinel collector object.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let collector_ref = heap.allocate("duke/util/ToListCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(collector_ref))))
}
/// Native: `Collectors.counting()Collector` — returns a counting collector sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_counting(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/CountingCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.groupingBy(Function)Collector` — returns a grouping-by collector.
/// Stores `fn_slot` in `fields[0]`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_grouping_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/GroupingByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.toSet()Collector` — returns a `ToSetCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_set(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/ToSetCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.toMap(keyFn, valFn)Collector` — stores both functions in `ToMapCollector`.
pub(crate) fn native_collectors_to_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let key_fn = extract_slot_arg(args, 0);
    let val_fn = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ToMapCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = key_fn;
    heap.get_mut(r)?.fields[1] = val_fn;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.partitioningBy(Predicate)Collector` — returns a sentinel collector.
pub(crate) fn native_collectors_partitioning_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let pred = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/PartitioningByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = pred;
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_collectors_partitioning_by_downstream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let pred = extract_slot_arg(args, 0);
    let downstream = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/PartitioningByDownstreamCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = pred;
    heap.get_mut(r)?.fields[1] = downstream;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.summingInt(ToIntFunction)Collector` — returns a `SummingIntCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_summing_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/SummingIntCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.averagingInt(ToIntFunction)Collector` — returns an `AveragingIntCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_averaging_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/AveragingIntCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.toMap(keyFn, valFn, mergeFn)Collector` — stores three functions.
pub(crate) fn native_collectors_to_map_merge(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let key_fn = extract_slot_arg(args, 0);
    let val_fn = extract_slot_arg(args, 1);
    let merge_fn = extract_slot_arg(args, 2);
    let r = heap.allocate("duke/util/ToMapMergeCollector".to_string(), 3);
    heap.get_mut(r)?.fields[0] = key_fn;
    heap.get_mut(r)?.fields[1] = val_fn;
    heap.get_mut(r)?.fields[2] = merge_fn;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.toUnmodifiableList()Collector` (Java 10) —
/// returns the same `ToListCollector` sentinel; our interpreter treats all lists as modifiable.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_unmodifiable_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/ToUnmodifiableListCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.toUnmodifiableSet()Collector` (Java 10) —
/// returns the same `ToSetCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_unmodifiable_set(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/ToSetCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.groupingBy(Function, Collector)Collector` — 2-arg version with downstream.
/// Creates a `duke/util/GroupingBy2Collector` with `fields[0]`=keyFn, `fields[1]`=downstream.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_grouping_by_2(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let downstream_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/GroupingBy2Collector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = fn_slot;
    heap.get_mut(r)?.fields[1] = downstream_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.mapping(Function, Collector)Collector` — transforms elements before
/// feeding to a downstream collector.  Creates a `duke/util/MappingCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_mapping(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let mapper_slot = extract_slot_arg(args, 0);
    let downstream_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/MappingCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = mapper_slot;
    heap.get_mut(r)?.fields[1] = downstream_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.summingLong(ToLongFunction)Collector` — returns a `SummingLongCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_summing_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/SummingLongCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.minBy(Comparator)Collector` — returns a `MinByCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_min_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let cmp_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/MinByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = cmp_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.maxBy(Comparator)Collector` — returns a `MaxByCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_max_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let cmp_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/MaxByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = cmp_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.summingDouble(ToDoubleFunction)Collector` — returns a `SummingDoubleCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_summing_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/SummingDoubleCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.averagingLong(ToLongFunction)Collector` — returns an `AveragingLongCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_averaging_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/AveragingLongCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.toUnmodifiableMap(keyFn, valueFn)Collector` — same sentinel as `ToMapCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_unmodifiable_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let key_fn_slot = extract_slot_arg(args, 0);
    let val_fn_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ToUnmodifiableMapCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = key_fn_slot;
    heap.get_mut(r)?.fields[1] = val_fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.collectingAndThen(downstream, finisher)Collector` — returns a
/// `CollectingAndThenCollector` with `fields[0]`=downstream, `fields[1]`=finisher.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_collecting_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let downstream_slot = extract_slot_arg(args, 0);
    let finisher_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/CollectingAndThenCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = downstream_slot;
    heap.get_mut(r)?.fields[1] = finisher_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.averagingDouble(ToDoubleFunction)Collector` — returns an `AveragingDoubleCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_averaging_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/AveragingDoubleCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.reducing(BinaryOperator)` — returns a
/// `ReducingNoIdentityCollector` with `fields[0]`=op.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_reducing_no_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let op_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/ReducingNoIdentityCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = op_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.reducing(T, BinaryOperator)` — returns a
/// `ReducingCollector` with `fields[0]`=identity, `fields[1]`=op.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_reducing_with_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let identity_slot = extract_slot_arg(args, 0);
    let op_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ReducingCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = identity_slot;
    heap.get_mut(r)?.fields[1] = op_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Collectors.reducing(U, Function, BinaryOperator)` — returns a
/// `ReducingMappingCollector` with `fields[0]`=identity, `fields[1]`=mapper, `fields[2]`=op.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_reducing_mapping(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let identity_slot = extract_slot_arg(args, 0);
    let mapper_slot = extract_slot_arg(args, 1);
    let op_slot = extract_slot_arg(args, 2);
    let r = heap.allocate("duke/util/ReducingMappingCollector".to_string(), 3);
    heap.get_mut(r)?.fields[0] = identity_slot;
    heap.get_mut(r)?.fields[1] = mapper_slot;
    heap.get_mut(r)?.fields[2] = op_slot;
    Ok(Some(Slot::Reference(Some(r))))
}
