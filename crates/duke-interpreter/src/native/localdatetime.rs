fn localdatetime_components_from_ref(
    heap: &duke_gc::Heap,
    localdatetime_ref: u64,
) -> Result<(i32, i32, i32, i32, i32)> {
    let obj = heap.get(localdatetime_ref)?;
    let epoch_day = match obj.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let hour = match obj.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let minute = match obj.fields.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let second = match obj.fields.get(3) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let nanos = match obj.fields.get(4) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok((epoch_day, hour, minute, second, nanos))
}
fn localdatetime_total_nanos_from_ref(
    heap: &duke_gc::Heap,
    localdatetime_ref: u64,
) -> Result<i128> {
    let (epoch_day, hour, minute, second, nanos) = localdatetime_components_from_ref(
        heap,
        localdatetime_ref,
    )?;
    let epoch_seconds = i64::from(epoch_day) * SECONDS_PER_DAY_I64
        + i64::from(hour * 3600 + minute * 60 + second);
    Ok(i128::from(epoch_seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos))
}
/// Native: `LocalDateTime.of(int,int,int,int,int) -> LocalDateTime`
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_localdatetime_of_ymd_hm(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let year = extract_int_arg(args, 0)?;
    let month = extract_int_arg(args, 1)? as u32;
    let day = extract_int_arg(args, 2)? as u32;
    let hour = extract_int_arg(args, 3)?;
    let minute = extract_int_arg(args, 4)?;
    let epoch = ymd_to_epoch_days(year, month, day);
    Ok(
        Some(
            Slot::Reference(
                Some(allocate_localdatetime(heap, epoch, hour, minute, 0, 0)?),
            ),
        ),
    )
}
/// Native: `LocalDateTime.of(int,int,int,int,int,int) -> LocalDateTime`
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_localdatetime_of_ymd_hms(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let year = extract_int_arg(args, 0)?;
    let month = extract_int_arg(args, 1)? as u32;
    let day = extract_int_arg(args, 2)? as u32;
    let hour = extract_int_arg(args, 3)?;
    let minute = extract_int_arg(args, 4)?;
    let second = extract_int_arg(args, 5)?;
    let epoch = ymd_to_epoch_days(year, month, day);
    Ok(
        Some(
            Slot::Reference(
                Some(allocate_localdatetime(heap, epoch, hour, minute, second, 0)?),
            ),
        ),
    )
}
/// Native: `LocalDateTime.of(LocalDate, int, int, int) -> LocalDateTime`
/// Synthetic overload: accepts `LocalDate` ref + hour/minute/second as ints.
pub(crate) fn native_localdatetime_of_date_hms(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let date_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(date_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let hour = extract_int_arg(args, 1)?;
    let minute = extract_int_arg(args, 2)?;
    let second = extract_int_arg(args, 3)?;
    Ok(
        Some(
            Slot::Reference(
                Some(allocate_localdatetime(heap, epoch, hour, minute, second, 0)?),
            ),
        ),
    )
}
/// Native: `LocalDateTime.now() -> LocalDateTime` — returns 1970-01-01T00:00:00 in interpreter.
pub(crate) fn native_localdatetime_now(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    for i in 0..5 {
        heap.get_mut(r)?.fields[i] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `LocalDateTime.getYear() -> int`
pub(crate) fn native_localdatetime_get_year(
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
/// Native: `LocalDateTime.getMonthValue() -> int`
pub(crate) fn native_localdatetime_get_month_value(
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
/// Native: `LocalDateTime.getDayOfMonth() -> int`
pub(crate) fn native_localdatetime_get_day_of_month(
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
/// Native: `LocalDateTime.getHour() -> int`
pub(crate) fn native_localdatetime_get_hour(
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
/// Native: `LocalDateTime.getMinute() -> int`
pub(crate) fn native_localdatetime_get_minute(
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
/// Native: `LocalDateTime.getSecond() -> int`
pub(crate) fn native_localdatetime_get_second(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(3) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}
/// Native: `LocalDateTime.toLocalDate() -> LocalDate`
pub(crate) fn native_localdatetime_to_local_date(
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
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, epoch)?))))
}
/// Native: `LocalDateTime.isBefore(LocalDateTime) -> boolean`
pub(crate) fn native_localdatetime_is_before(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a_epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b_epoch = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    if a_epoch != b_epoch {
        return Ok(Some(Slot::Int(i32::from(a_epoch < b_epoch))));
    }
    for idx in 1..=4 {
        let a = match heap.get(this_ref)?.fields.get(idx) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        let b = match heap.get(other_ref)?.fields.get(idx) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        if a != b {
            return Ok(Some(Slot::Int(i32::from(a < b))));
        }
    }
    Ok(Some(Slot::Int(0)))
}
/// Native: `LocalDateTime.isAfter(LocalDateTime) -> boolean`
pub(crate) fn native_localdatetime_is_after(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a_epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b_epoch = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    if a_epoch != b_epoch {
        return Ok(Some(Slot::Int(i32::from(a_epoch > b_epoch))));
    }
    for idx in 1..=4 {
        let a = match heap.get(this_ref)?.fields.get(idx) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        let b = match heap.get(other_ref)?.fields.get(idx) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        if a != b {
            return Ok(Some(Slot::Int(i32::from(a > b))));
        }
    }
    Ok(Some(Slot::Int(0)))
}
/// Native: `LocalDateTime.toString() -> String` — ISO-8601 format
pub(crate) fn native_localdatetime_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let epoch = match fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let hour = match fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let min = match fields.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let sec = match fields.get(3) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let nanos = match fields.get(4) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (y, m, d) = epoch_days_to_ymd(epoch);
    let mut s = format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}");
    if nanos != 0 {
        let mut fraction = format!("{nanos:09}");
        while fraction.ends_with('0') {
            fraction.pop();
        }
        s.push('.');
        s.push_str(&fraction);
    }
    let sr = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(sr))))
}
/// Native: `LocalDateTime.plusDays(long) -> LocalDateTime`
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_localdatetime_plus_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let days = extract_long_arg(args, 1)?;
    let fields = heap.get(this_ref)?.fields.clone();
    let epoch = match fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let new_epoch = epoch.saturating_add(days as i32);
    Ok(
        Some(
            Slot::Reference(
                Some(
                    allocate_localdatetime(
                        heap,
                        new_epoch,
                        match fields.get(1) {
                            Some(Slot::Int(v)) => *v,
                            _ => 0,
                        },
                        match fields.get(2) {
                            Some(Slot::Int(v)) => *v,
                            _ => 0,
                        },
                        match fields.get(3) {
                            Some(Slot::Int(v)) => *v,
                            _ => 0,
                        },
                        match fields.get(4) {
                            Some(Slot::Int(v)) => *v,
                            _ => 0,
                        },
                    )?,
                ),
            ),
        ),
    )
}
/// Native: `LocalDateTime.withHour(int) -> LocalDateTime`
pub(crate) fn native_localdatetime_with_hour(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let hour = extract_int_arg(args, 1)?;
    let f0 = heap.get(this_ref)?.fields.first().copied().unwrap_or(Slot::Int(0));
    let f1 = heap.get(this_ref)?.fields.get(1).copied().unwrap_or(Slot::Int(0));
    let f2 = heap.get(this_ref)?.fields.get(2).copied().unwrap_or(Slot::Int(0));
    let f3 = heap.get(this_ref)?.fields.get(3).copied().unwrap_or(Slot::Int(0));
    let f4 = heap.get(this_ref)?.fields.get(4).copied().unwrap_or(Slot::Int(0));
    let epoch = match f0 {
        Slot::Int(v) => v,
        _ => 0,
    };
    let minute = match f2 {
        Slot::Int(v) => v,
        _ => 0,
    };
    let second = match f3 {
        Slot::Int(v) => v,
        _ => 0,
    };
    let nanos = match f4 {
        Slot::Int(v) => v,
        _ => 0,
    };
    let _ = f1;
    Ok(
        Some(
            Slot::Reference(
                Some(allocate_localdatetime(heap, epoch, hour, minute, second, nanos)?),
            ),
        ),
    )
}
/// Native: `LocalDateTime.plusHours(long) -> LocalDateTime`
pub(crate) fn native_localdatetime_plus_hours(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let hours = extract_long_arg(args, 1)?;
    let (epoch_day, hour, minute, second, nanos) = localdatetime_components_from_ref(
        heap,
        this_ref,
    )?;
    let total_seconds = i64::from(hour * 3600 + minute * 60 + second) + hours * 3600;
    let day_delta = total_seconds.div_euclid(SECONDS_PER_DAY_I64);
    let second_of_day = total_seconds.rem_euclid(SECONDS_PER_DAY_I64);
    let new_hour = i32::try_from(second_of_day / 3600).unwrap_or(0);
    let new_minute = i32::try_from((second_of_day % 3600) / 60).unwrap_or(0);
    let new_second = i32::try_from(second_of_day % 60).unwrap_or(0);
    #[allow(clippy::cast_possible_truncation)]
    let new_epoch_day = epoch_day.saturating_add(day_delta as i32);
    Ok(
        Some(
            Slot::Reference(
                Some(
                    allocate_localdatetime(
                        heap,
                        new_epoch_day,
                        new_hour,
                        new_minute,
                        new_second,
                        nanos,
                    )?,
                ),
            ),
        ),
    )
}
/// Native: `LocalDateTime.parse(CharSequence) -> LocalDateTime`
pub(crate) fn native_localdatetime_parse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let text = extract_string_arg_value(args, 0, heap)?;
    let (year, month, day, hour, minute, second, nanos) = parse_iso_localdatetime_components(
            &text,
        )
        .ok_or_else(|| date_time_parse_error(&text))?;
    let epoch_day = ymd_to_epoch_days(year, month, day);
    Ok(
        Some(
            Slot::Reference(
                Some(
                    allocate_localdatetime(heap, epoch_day, hour, minute, second, nanos)?,
                ),
            ),
        ),
    )
}
/// Native: `LocalDateTime.parse(CharSequence, DateTimeFormatter) -> LocalDateTime`
pub(crate) fn native_localdatetime_parse_with_formatter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 1)?;
    native_localdatetime_parse(args, heap, out, control)
}
fn localdatetime_compare_impl(
    heap: &duke_gc::Heap,
    this_ref: u64,
    other_ref: u64,
) -> Result<i32> {
    let other = heap.get(other_ref)?;
    if other.class_name != "java/time/LocalDateTime" {
        return Err(class_cast_error());
    }
    let this_parts = localdatetime_components_from_ref(heap, this_ref)?;
    let other_parts = localdatetime_components_from_ref(heap, other_ref)?;
    Ok(ordering_to_int(this_parts.cmp(&other_parts)))
}
/// Native: `LocalDateTime.compareTo(ChronoLocalDateTime) -> int`
pub(crate) fn native_localdatetime_compare_to(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    Ok(Some(Slot::Int(localdatetime_compare_impl(heap, this_ref, other_ref)?)))
}
/// Native: bridge `LocalDateTime.compareTo(Object) -> int`
pub(crate) fn native_localdatetime_compare_to_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_localdatetime_compare_to(args, heap, out, control)
}
/// Native: `LocalDateTime.equals(Object) -> boolean`
pub(crate) fn native_localdatetime_equals(
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
    let equal = other.class_name == "java/time/LocalDateTime"
        && localdatetime_components_from_ref(heap, this_ref)?
            == localdatetime_components_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Int(i32::from(equal))))
}
/// Native: `LocalDateTime.hashCode() -> int`
pub(crate) fn native_localdatetime_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (epoch_day, hour, minute, second, nanos) = localdatetime_components_from_ref(
        heap,
        this_ref,
    )?;
    let mut hash = epoch_day;
    hash = hash.wrapping_mul(31).wrapping_add(hour);
    hash = hash.wrapping_mul(31).wrapping_add(minute);
    hash = hash.wrapping_mul(31).wrapping_add(second);
    hash = hash.wrapping_mul(31).wrapping_add(nanos);
    Ok(Some(Slot::Int(hash)))
}
