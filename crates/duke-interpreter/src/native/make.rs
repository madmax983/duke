/// Helper: create an `ArrayList` from a slice of `Slot`s.
fn make_list_from_slots(
    elems: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<u64> {
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    for &elem in elems {
        native_arraylist_add(
            &[Slot::Reference(Some(list_ref)), elem],
            heap,
            out,
            control,
        )?;
    }
    Ok(list_ref)
}
/// Helper: create a `HashSet` from a slice of `Slot`s.
fn make_set_from_slots(
    elems: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<u64> {
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    for &elem in elems {
        native_hashset_add(&[Slot::Reference(Some(set_ref)), elem], heap, out, control)?;
    }
    Ok(set_ref)
}
/// Helper: allocate a `JoiningCollector` with 3 fields: delimiter, prefix, suffix.
fn make_joining_collector(
    heap: &mut duke_gc::Heap,
    delimiter: &str,
    prefix: &str,
    suffix: &str,
) -> u64 {
    let collector_ref = heap.allocate("duke/util/JoiningCollector".to_string(), 3);
    let delim_ref = heap.allocate_string(delimiter.to_string());
    let prefix_ref = heap.allocate_string(prefix.to_string());
    let suffix_ref = heap.allocate_string(suffix.to_string());
    let obj = heap.get_mut(collector_ref).expect("just allocated");
    obj.fields[0] = Slot::Reference(Some(delim_ref));
    obj.fields[1] = Slot::Reference(Some(prefix_ref));
    obj.fields[2] = Slot::Reference(Some(suffix_ref));
    collector_ref
}
/// Build an `IntStream` heap object from a `Vec<i32>`.
fn make_int_stream(heap: &mut duke_gc::Heap, values: Vec<i32>) -> u64 {
    let n = i32::try_from(values.len()).unwrap_or(0);
    let r = heap.allocate("duke/util/IntStream".to_string(), 1);
    heap.get_mut(r).expect("fresh").fields[0] = Slot::Int(n);
    for v in values {
        heap.get_mut(r).expect("fresh").fields.push(Slot::Int(v));
    }
    r
}
/// Create an `OptionalInt` heap object. `None` = empty, `Some(v)` = present.
fn make_optional_int(heap: &mut duke_gc::Heap, value: Option<i32>) -> u64 {
    let r = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    if let Some(v) = value {
        heap.get_mut(r).expect("fresh").fields[0] = Slot::Int(v);
        heap.get_mut(r).expect("fresh").fields[1] = Slot::Int(1);
    } else {
        heap.get_mut(r).expect("fresh").fields[1] = Slot::Int(0);
    }
    r
}
/// Create a `java/util/Optional` heap object. `None` = empty, `Some(slot)` = present.
fn make_optional(heap: &mut duke_gc::Heap, value: Option<Slot>) -> u64 {
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r).expect("fresh").fields[0] = value.unwrap_or(Slot::Reference(None));
    r
}
/// Allocates a `duke/util/LongStream` with `fields[0]=Int(size), fields[1..n]=Long(value)`.
fn make_long_stream(heap: &mut duke_gc::Heap, values: Vec<i64>) -> u64 {
    let r = heap.allocate("duke/util/LongStream".to_string(), 1);
    if let Ok(obj) = heap.get_mut(r) {
        obj.fields[0] = Slot::Int(i32::try_from(values.len()).unwrap_or(0));
        for v in values {
            obj.fields.push(Slot::Long(v));
        }
    }
    r
}
/// Allocates a `duke/util/DoubleStream` with `fields[0]=Int(size), fields[1..n]=Double(value)`.
fn make_double_stream(heap: &mut duke_gc::Heap, values: Vec<f64>) -> u64 {
    let r = heap.allocate("duke/util/DoubleStream".to_string(), 1);
    if let Ok(obj) = heap.get_mut(r) {
        obj.fields[0] = Slot::Int(i32::try_from(values.len()).unwrap_or(0));
        for v in values {
            obj.fields.push(Slot::Double(v));
        }
    }
    r
}
/// Allocate an `OptionalLong`: `fields[0]=Long(value)`, `fields[1]=Int(present)`.
fn make_optional_long(heap: &mut duke_gc::Heap, value: Option<i64>) -> u64 {
    let r = heap.allocate("duke/util/OptionalLong".to_string(), 2);
    if let Ok(obj) = heap.get_mut(r) {
        if let Some(v) = value {
            obj.fields[0] = Slot::Long(v);
            obj.fields[1] = Slot::Int(1);
        } else {
            obj.fields[1] = Slot::Int(0);
        }
    }
    r
}
/// Allocate an `OptionalDouble` (for LongStream/DoubleStream average/min/max).
fn make_optional_double_val(heap: &mut duke_gc::Heap, value: Option<f64>) -> u64 {
    let r = heap.allocate("duke/util/OptionalDouble".to_string(), 2);
    if let Ok(obj) = heap.get_mut(r) {
        if let Some(v) = value {
            obj.fields[0] = Slot::Double(v);
            obj.fields[1] = Slot::Int(1);
        } else {
            obj.fields[1] = Slot::Int(0);
        }
    }
    r
}
