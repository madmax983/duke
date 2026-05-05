/// Native: `LocalDate.of(int, int, int) -> LocalDate`
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_localdate_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let year = extract_int_arg(args, 0)?;
    let month = extract_int_arg(args, 1)? as u32;
    let day = extract_int_arg(args, 2)? as u32;
    let epoch = ymd_to_epoch_days(year, month, day);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, epoch)?))))
}
/// Native: `LocalDate.now() -> LocalDate` — returns 1970-01-01 (epoch 0) in this interpreter.
pub(crate) fn native_localdate_now(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("java/time/LocalDate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `LocalDate.getYear() -> int`
pub(crate) fn native_localdate_get_year(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (year, _, _) = epoch_days_to_ymd(epoch);
    Ok(Some(Slot::Int(year)))
}
/// Native: `LocalDate.getMonthValue() -> int`
pub(crate) fn native_localdate_get_month_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (_, month, _) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_wrap)] Ok(Some(Slot::Int(month as i32)))
}
/// Native: `LocalDate.getDayOfMonth() -> int`
pub(crate) fn native_localdate_get_day_of_month(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (_, _, day) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_wrap)] Ok(Some(Slot::Int(day as i32)))
}
/// Native: `LocalDate.plusDays(long) -> LocalDate`
pub(crate) fn native_localdate_plus_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let days = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    #[allow(clippy::cast_possible_truncation)]
    let new_epoch = epoch.saturating_add(days as i32);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, new_epoch)?))))
}
/// Native: `LocalDate.minusDays(long) -> LocalDate`
pub(crate) fn native_localdate_minus_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let days = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    #[allow(clippy::cast_possible_truncation)]
    let new_epoch = epoch.saturating_sub(days as i32);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, new_epoch)?))))
}
/// Native: `LocalDate.plusMonths(long) -> LocalDate`
pub(crate) fn native_localdate_plus_months(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let months = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (y, m, d) = epoch_days_to_ymd(epoch);
    let (ny, nm, nd) = shift_year_month_day(y, m, d, months);
    let new_epoch = ymd_to_epoch_days(ny, nm, nd);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, new_epoch)?))))
}
/// Native: `LocalDate.plusYears(long) -> LocalDate`
pub(crate) fn native_localdate_plus_years(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let years = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (y, m, d) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_truncation)]
    let ny = y + years as i32;
    let max_day = days_in_month(ny, m);
    let nd = d.min(max_day);
    let new_epoch = ymd_to_epoch_days(ny, m, nd);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, new_epoch)?))))
}
/// Native: `LocalDate.isBefore(LocalDate) -> boolean`
pub(crate) fn native_localdate_is_before(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(a < b))))
}
/// Native: `LocalDate.isAfter(LocalDate) -> boolean`
pub(crate) fn native_localdate_is_after(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(a > b))))
}
/// Native: `LocalDate.isEqual(LocalDate) -> boolean`
pub(crate) fn native_localdate_is_equal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(a == b))))
}
/// Native: `LocalDate.toEpochDay() -> long`
pub(crate) fn native_localdate_to_epoch_day(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(i64::from(epoch))))
}
/// Native: `LocalDate.toString() -> String`
pub(crate) fn native_localdate_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (y, m, d) = epoch_days_to_ymd(epoch);
    let s = format!("{y:04}-{m:02}-{d:02}");
    let sr = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(sr))))
}
/// Native: `LocalDate.minusMonths(long) -> LocalDate`
pub(crate) fn native_localdate_minus_months(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let months = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (year, month, day) = epoch_days_to_ymd(epoch);
    let (new_year, new_month, new_day) = shift_year_month_day(year, month, day, -months);
    let new_epoch = ymd_to_epoch_days(new_year, new_month, new_day);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, new_epoch)?))))
}
/// Native: `LocalDate.withYear(int) -> LocalDate`
pub(crate) fn native_localdate_with_year(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let year = extract_int_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (_, month, day) = epoch_days_to_ymd(epoch);
    let new_day = day.min(days_in_month(year, month));
    let new_epoch = ymd_to_epoch_days(year, month, new_day);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, new_epoch)?))))
}
/// Native: `LocalDate.parse(CharSequence) -> LocalDate`
pub(crate) fn native_localdate_parse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let text = extract_string_arg_value(args, 0, heap)?;
    let (year, month, day) = parse_iso_local_date_components(&text)
        .ok_or_else(|| date_time_parse_error(&text))?;
    let epoch = ymd_to_epoch_days(year, month, day);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, epoch)?))))
}
/// Native: `LocalDate.parse(CharSequence, DateTimeFormatter) -> LocalDate`
pub(crate) fn native_localdate_parse_with_formatter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 1)?;
    native_localdate_parse(args, heap, out, control)
}
fn localdate_compare_impl(
    heap: &duke_gc::Heap,
    this_ref: u64,
    other_ref: u64,
) -> Result<i32> {
    let other = heap.get(other_ref)?;
    if other.class_name != "java/time/LocalDate" {
        return Err(class_cast_error());
    }
    let this_epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let other_epoch = match other.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(ordering_to_int(this_epoch.cmp(&other_epoch)))
}
/// Native: `LocalDate.compareTo(ChronoLocalDate) -> int`
pub(crate) fn native_localdate_compare_to(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    Ok(Some(Slot::Int(localdate_compare_impl(heap, this_ref, other_ref)?)))
}
/// Native: bridge `LocalDate.compareTo(Object) -> int`
pub(crate) fn native_localdate_compare_to_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_localdate_compare_to(args, heap, out, control)
}
/// Native: `LocalDate.equals(Object) -> boolean`
pub(crate) fn native_localdate_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    let other = heap.get(other_ref)?;
    let equal = other.class_name == "java/time/LocalDate"
        && other.fields.first() == heap.get(this_ref)?.fields.first();
    Ok(Some(Slot::Int(i32::from(equal))))
}
/// Native: `LocalDate.hashCode() -> int`
pub(crate) fn native_localdate_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(epoch)))
}
