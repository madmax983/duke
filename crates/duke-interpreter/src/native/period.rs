/// Native: `Period.of(int, int, int) -> Period`
pub(crate) fn native_period_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let years = extract_int_arg(args, 0)?;
    let months = extract_int_arg(args, 1)?;
    let days = extract_int_arg(args, 2)?;
    let r = heap.allocate("java/time/Period".to_string(), 3);
    heap.get_mut(r)?.fields[0] = Slot::Int(years);
    heap.get_mut(r)?.fields[1] = Slot::Int(months);
    heap.get_mut(r)?.fields[2] = Slot::Int(days);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Period.ofDays(int) -> Period`
pub(crate) fn native_period_of_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let days = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/time/Period".to_string(), 3);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    heap.get_mut(r)?.fields[1] = Slot::Int(0);
    heap.get_mut(r)?.fields[2] = Slot::Int(days);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Period.ofMonths(int) -> Period`
pub(crate) fn native_period_of_months(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let months = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/time/Period".to_string(), 3);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    heap.get_mut(r)?.fields[1] = Slot::Int(months);
    heap.get_mut(r)?.fields[2] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Period.ofYears(int) -> Period`
pub(crate) fn native_period_of_years(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let years = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/time/Period".to_string(), 3);
    heap.get_mut(r)?.fields[0] = Slot::Int(years);
    heap.get_mut(r)?.fields[1] = Slot::Int(0);
    heap.get_mut(r)?.fields[2] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Period.getYears() -> int`
pub(crate) fn native_period_get_years(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}
/// Native: `Period.getMonths() -> int`
pub(crate) fn native_period_get_months(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}
/// Native: `Period.getDays() -> int`
pub(crate) fn native_period_get_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}
/// Native: `Period.isNegative() -> boolean`
pub(crate) fn native_period_is_negative(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let neg = heap
        .get(this_ref)?
        .fields
        .iter()
        .any(|s| matches!(s, Slot::Int(v) if * v < 0));
    Ok(Some(Slot::Int(i32::from(neg))))
}
/// Native: `Period.isZero() -> boolean`
pub(crate) fn native_period_is_zero(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let zero = heap.get(this_ref)?.fields.iter().all(|s| matches!(s, Slot::Int(0)));
    Ok(Some(Slot::Int(i32::from(zero))))
}
