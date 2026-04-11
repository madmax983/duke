/// Native: `LocalDateTime.withHour(int) -> LocalDateTime`
pub(crate) fn native_localdatetime_with_hour(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let hour = extract_int_arg(args, 1)?;
    let fields = heap.get(this_ref)?.fields.clone();
    let r = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    for i in 0..5 {
        heap.get_mut(r)?.fields[i] = fields.get(i).copied().unwrap_or(Slot::Int(0));
    }
    heap.get_mut(r)?.fields[1] = Slot::Int(hour);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDateTime.plusDays(long) -> LocalDateTime`
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_localdatetime_plus_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let days = extract_long_arg(args, 1)?;
    let fields = heap.get(this_ref)?.fields.clone();
    let epoch = match fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let new_epoch = epoch.saturating_add(days as i32);
    let r = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    heap.get_mut(r)?.fields[0] = Slot::Int(new_epoch);
    for i in 1..5 {
        heap.get_mut(r)?.fields[i] = fields.get(i).copied().unwrap_or(Slot::Int(0));
    }
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDateTime.toString() -> String` — ISO-8601 format
pub(crate) fn native_localdatetime_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(this_ref)?.fields.clone();
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
    let (y, m, d) = epoch_days_to_ymd(epoch);
    let s = format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}");
    let sr = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(sr))))
}

/// Native: `LocalDateTime.isAfter(LocalDateTime) -> boolean`
pub(crate) fn native_localdatetime_is_after(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
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
    let fields_a: Vec<Slot> = heap.get(this_ref)?.fields.clone();
    let fields_b: Vec<Slot> = heap.get(other_ref)?.fields.clone();
    for idx in 1..=3 {
        let a = match fields_a.get(idx) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        let b = match fields_b.get(idx) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        if a != b {
            return Ok(Some(Slot::Int(i32::from(a > b))));
        }
    }
    Ok(Some(Slot::Int(0))) // equal
}

/// Native: `LocalDateTime.isBefore(LocalDateTime) -> boolean`
pub(crate) fn native_localdatetime_is_before(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
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
    // same day — compare time fields
    let fields_a: Vec<Slot> = heap.get(this_ref)?.fields.clone();
    let fields_b: Vec<Slot> = heap.get(other_ref)?.fields.clone();
    for idx in 1..=3 {
        let a = match fields_a.get(idx) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        let b = match fields_b.get(idx) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        if a != b {
            return Ok(Some(Slot::Int(i32::from(a < b))));
        }
    }
    Ok(Some(Slot::Int(0))) // equal
}

/// Native: `LocalDateTime.toLocalDate() -> LocalDate`
pub(crate) fn native_localdatetime_to_local_date(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let r = heap.allocate("java/time/LocalDate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(epoch);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDateTime.getSecond() -> int`
pub(crate) fn native_localdatetime_get_second(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(3) {
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
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}

/// Native: `LocalDateTime.getHour() -> int`
pub(crate) fn native_localdatetime_get_hour(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}

/// Native: `LocalDateTime.getDayOfMonth() -> int`
pub(crate) fn native_localdatetime_get_day_of_month(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (_, _, day) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_wrap)] // day is [1,31]
    Ok(Some(Slot::Int(day as i32)))
}

/// Native: `LocalDateTime.getMonthValue() -> int`
pub(crate) fn native_localdatetime_get_month_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (_, month, _) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_wrap)] // month is [1,12]
    Ok(Some(Slot::Int(month as i32)))
}

/// Native: `LocalDateTime.getYear() -> int`
pub(crate) fn native_localdatetime_get_year(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (year, _, _) = epoch_days_to_ymd(epoch);
    Ok(Some(Slot::Int(year)))
}

/// Native: `LocalDateTime.now() -> LocalDateTime` — returns 1970-01-01T00:00:00 in interpreter.
pub(crate) fn native_localdatetime_now(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    for i in 0..5 {
        heap.get_mut(r)?.fields[i] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDateTime.of(LocalDate, int, int, int) -> LocalDateTime`
/// Synthetic overload: accepts `LocalDate` ref + hour/minute/second as ints.
pub(crate) fn native_localdatetime_of_date_hms(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let date_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(date_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let hour = extract_int_arg(args, 1)?;
    let minute = extract_int_arg(args, 2)?;
    let second = extract_int_arg(args, 3)?;
    let r = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    heap.get_mut(r)?.fields[0] = Slot::Int(epoch);
    heap.get_mut(r)?.fields[1] = Slot::Int(hour);
    heap.get_mut(r)?.fields[2] = Slot::Int(minute);
    heap.get_mut(r)?.fields[3] = Slot::Int(second);
    heap.get_mut(r)?.fields[4] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDateTime.of(int,int,int,int,int,int) -> LocalDateTime`
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_localdatetime_of_ymd_hms(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let year = extract_int_arg(args, 0)?;
    let month = extract_int_arg(args, 1)? as u32;
    let day = extract_int_arg(args, 2)? as u32;
    let hour = extract_int_arg(args, 3)?;
    let minute = extract_int_arg(args, 4)?;
    let second = extract_int_arg(args, 5)?;
    let epoch = ymd_to_epoch_days(year, month, day);
    let r = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    heap.get_mut(r)?.fields[0] = Slot::Int(epoch);
    heap.get_mut(r)?.fields[1] = Slot::Int(hour);
    heap.get_mut(r)?.fields[2] = Slot::Int(minute);
    heap.get_mut(r)?.fields[3] = Slot::Int(second);
    heap.get_mut(r)?.fields[4] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDateTime.of(int,int,int,int,int) -> LocalDateTime`
#[allow(clippy::cast_sign_loss)] // month/day from Java int are always positive
pub(crate) fn native_localdatetime_of_ymd_hm(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let year = extract_int_arg(args, 0)?;
    let month = extract_int_arg(args, 1)? as u32;
    let day = extract_int_arg(args, 2)? as u32;
    let hour = extract_int_arg(args, 3)?;
    let minute = extract_int_arg(args, 4)?;
    let epoch = ymd_to_epoch_days(year, month, day);
    let r = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    heap.get_mut(r)?.fields[0] = Slot::Int(epoch);
    heap.get_mut(r)?.fields[1] = Slot::Int(hour);
    heap.get_mut(r)?.fields[2] = Slot::Int(minute);
    heap.get_mut(r)?.fields[3] = Slot::Int(0);
    heap.get_mut(r)?.fields[4] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Instant.isAfter(Instant) -> boolean`
pub(crate) fn native_instant_is_after(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let b = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(a > b))))
}

/// Native: `Instant.isBefore(Instant) -> boolean`
pub(crate) fn native_instant_is_before(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let b = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(a < b))))
}

/// Native: `Instant.toEpochMilli() -> long`
pub(crate) fn native_instant_to_epoch_milli(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let nanos = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs * 1000 + i64::from(nanos) / 1_000_000)))
}

/// Native: `Instant.getEpochSecond() -> long`
pub(crate) fn native_instant_get_epoch_second(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs)))
}

/// Native: `Instant.ofEpochMilli(long) -> Instant`
pub(crate) fn native_instant_of_epoch_milli(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let millis = extract_long_arg(args, 0)?;
    let secs = millis / 1000;
    #[allow(clippy::cast_possible_truncation)] // nanos = [0, 999_000_000], fits i32
    let nanos = ((millis % 1000) * 1_000_000) as i32;
    let r = heap.allocate("java/time/Instant".to_string(), 2);
    heap.get_mut(r)?.fields[0] = Slot::Long(secs);
    heap.get_mut(r)?.fields[1] = Slot::Int(nanos);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Instant.ofEpochSecond(long) -> Instant`
pub(crate) fn native_instant_of_epoch_second(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let secs = extract_long_arg(args, 0)?;
    let r = heap.allocate("java/time/Instant".to_string(), 2);
    heap.get_mut(r)?.fields[0] = Slot::Long(secs);
    heap.get_mut(r)?.fields[1] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Period.isZero() -> boolean`
pub(crate) fn native_period_is_zero(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let f = heap.get(this_ref)?.fields.clone();
    let zero = f.iter().all(|s| matches!(s, Slot::Int(0)));
    Ok(Some(Slot::Int(i32::from(zero))))
}

/// Native: `Period.isNegative() -> boolean`
pub(crate) fn native_period_is_negative(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let f = heap.get(this_ref)?.fields.clone();
    let neg = f.iter().any(|s| matches!(s, Slot::Int(v) if *v < 0));
    Ok(Some(Slot::Int(i32::from(neg))))
}

/// Native: `Period.getDays() -> int`
pub(crate) fn native_period_get_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(2) {
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
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}

/// Native: `Period.getYears() -> int`
pub(crate) fn native_period_get_years(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}

/// Native: `Period.ofYears(int) -> Period`
pub(crate) fn native_period_of_years(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let years = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/time/Period".to_string(), 3);
    heap.get_mut(r)?.fields[0] = Slot::Int(years);
    heap.get_mut(r)?.fields[1] = Slot::Int(0);
    heap.get_mut(r)?.fields[2] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Period.ofMonths(int) -> Period`
pub(crate) fn native_period_of_months(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let months = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/time/Period".to_string(), 3);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    heap.get_mut(r)?.fields[1] = Slot::Int(months);
    heap.get_mut(r)?.fields[2] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Period.ofDays(int) -> Period`
pub(crate) fn native_period_of_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let days = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/time/Period".to_string(), 3);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    heap.get_mut(r)?.fields[1] = Slot::Int(0);
    heap.get_mut(r)?.fields[2] = Slot::Int(days);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Period.of(int, int, int) -> Period`
pub(crate) fn native_period_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let years = extract_int_arg(args, 0)?;
    let months = extract_int_arg(args, 1)?;
    let days = extract_int_arg(args, 2)?;
    let r = heap.allocate("java/time/Period".to_string(), 3);
    heap.get_mut(r)?.fields[0] = Slot::Int(years);
    heap.get_mut(r)?.fields[1] = Slot::Int(months);
    heap.get_mut(r)?.fields[2] = Slot::Int(days);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Duration.isZero() -> boolean`
pub(crate) fn native_duration_is_zero(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let nano = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(secs == 0 && nano == 0))))
}

/// Native: `Duration.isNegative() -> boolean`
pub(crate) fn native_duration_is_negative(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(secs < 0))))
}

/// Native: `Duration.minus(Duration) -> Duration`
pub(crate) fn native_duration_minus(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a_secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let a_nano = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b_secs = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let b_nano = match heap.get(other_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let total_nano = i64::from(a_nano) - i64::from(b_nano);
    let carry = if total_nano < 0 {
        (total_nano - 999_999_999) / 1_000_000_000
    } else {
        total_nano / 1_000_000_000
    };
    #[allow(clippy::cast_possible_truncation)] // rem fits in i32: [-(999_999_999), 999_999_999]
    let rem_nano = (total_nano - carry * 1_000_000_000) as i32;
    let r = heap.allocate("java/time/Duration".to_string(), 2);
    heap.get_mut(r)?.fields[0] = Slot::Long(a_secs - b_secs + carry);
    heap.get_mut(r)?.fields[1] = Slot::Int(rem_nano);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Duration.plus(Duration) -> Duration`
pub(crate) fn native_duration_plus(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a_secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let a_nano = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b_secs = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let b_nano = match heap.get(other_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let total_nano = i64::from(a_nano) + i64::from(b_nano);
    let carry = total_nano / 1_000_000_000;
    #[allow(clippy::cast_possible_truncation)] // rem fits in i32: [0, 999_999_999]
    let rem_nano = (total_nano % 1_000_000_000) as i32;
    let r = heap.allocate("java/time/Duration".to_string(), 2);
    heap.get_mut(r)?.fields[0] = Slot::Long(a_secs + b_secs + carry);
    heap.get_mut(r)?.fields[1] = Slot::Int(rem_nano);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Duration.toDays() -> long`
pub(crate) fn native_duration_to_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs / 86_400)))
}

/// Native: `Duration.toHours() -> long`
pub(crate) fn native_duration_to_hours(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs / 3600)))
}

/// Native: `Duration.toMinutes() -> long`
pub(crate) fn native_duration_to_minutes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs / 60)))
}

/// Native: `Duration.toSeconds() -> long`
pub(crate) fn native_duration_to_seconds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_duration_get_seconds(args, heap, out, control)
}

/// Native: `Duration.getSeconds() -> long`
pub(crate) fn native_duration_get_seconds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs)))
}

/// Native: `Duration.ofDays(long) -> Duration`
pub(crate) fn native_duration_of_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let days = extract_long_arg(args, 0)?;
    let r = heap.allocate("java/time/Duration".to_string(), 2);
    heap.get_mut(r)?.fields[0] = Slot::Long(days * 86_400);
    heap.get_mut(r)?.fields[1] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Duration.ofHours(long) -> Duration`
pub(crate) fn native_duration_of_hours(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let hrs = extract_long_arg(args, 0)?;
    let r = heap.allocate("java/time/Duration".to_string(), 2);
    heap.get_mut(r)?.fields[0] = Slot::Long(hrs * 3600);
    heap.get_mut(r)?.fields[1] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Duration.ofMinutes(long) -> Duration`
pub(crate) fn native_duration_of_minutes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let mins = extract_long_arg(args, 0)?;
    let r = heap.allocate("java/time/Duration".to_string(), 2);
    heap.get_mut(r)?.fields[0] = Slot::Long(mins * 60);
    heap.get_mut(r)?.fields[1] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Duration.ofSeconds(long) -> Duration`
pub(crate) fn native_duration_of_seconds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let secs = extract_long_arg(args, 0)?;
    let r = heap.allocate("java/time/Duration".to_string(), 2);
    heap.get_mut(r)?.fields[0] = Slot::Long(secs);
    heap.get_mut(r)?.fields[1] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDate.toString() -> String`
pub(crate) fn native_localdate_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
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

/// Native: `LocalDate.toEpochDay() -> long`
pub(crate) fn native_localdate_to_epoch_day(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(i64::from(epoch))))
}

/// Native: `LocalDate.isEqual(LocalDate) -> boolean`
pub(crate) fn native_localdate_is_equal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
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

/// Native: `LocalDate.isAfter(LocalDate) -> boolean`
pub(crate) fn native_localdate_is_after(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
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

/// Native: `LocalDate.isBefore(LocalDate) -> boolean`
pub(crate) fn native_localdate_is_before(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
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

/// Native: `LocalDate.plusYears(long) -> LocalDate`
pub(crate) fn native_localdate_plus_years(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let years = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (y, m, d) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_truncation)] // year range is reasonable for Java dates
    let ny = y + years as i32;
    let max_day = days_in_month(ny, m);
    let nd = d.min(max_day);
    let new_epoch = ymd_to_epoch_days(ny, m, nd);
    let r = heap.allocate("java/time/LocalDate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(new_epoch);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDate.plusMonths(long) -> LocalDate`
pub(crate) fn native_localdate_plus_months(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let months = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (y, m, d) = epoch_days_to_ymd(epoch);
    let total_months = i64::from(y) * 12 + (i64::from(m) - 1) + months;
    #[allow(clippy::cast_possible_truncation)] // year range is reasonable for Java dates
    let ny = (total_months / 12) as i32;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // rem is [0,11] so +1 is [1,12], always positive and fits u32
    let nm = ((total_months % 12) + 1) as u32;
    // clamp day to valid range for that month
    let max_day = days_in_month(ny, nm);
    let nd = d.min(max_day);
    let new_epoch = ymd_to_epoch_days(ny, nm, nd);
    let r = heap.allocate("java/time/LocalDate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(new_epoch);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDate.minusDays(long) -> LocalDate`
pub(crate) fn native_localdate_minus_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let days = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    #[allow(clippy::cast_possible_truncation)] // saturating_sub handles out-of-range
    let new_epoch = epoch.saturating_sub(days as i32);
    let r = heap.allocate("java/time/LocalDate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(new_epoch);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDate.plusDays(long) -> LocalDate`
pub(crate) fn native_localdate_plus_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let days = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    #[allow(clippy::cast_possible_truncation)] // saturating_add handles out-of-range
    let new_epoch = epoch.saturating_add(days as i32);
    let r = heap.allocate("java/time/LocalDate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(new_epoch);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDate.getDayOfMonth() -> int`
pub(crate) fn native_localdate_get_day_of_month(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (_, _, day) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_wrap)] // day is [1,31], fits i32
    Ok(Some(Slot::Int(day as i32)))
}

/// Native: `LocalDate.getMonthValue() -> int`
pub(crate) fn native_localdate_get_month_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (_, month, _) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_wrap)] // month is [1,12], fits i32
    Ok(Some(Slot::Int(month as i32)))
}

/// Native: `LocalDate.getYear() -> int`
pub(crate) fn native_localdate_get_year(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (year, _, _) = epoch_days_to_ymd(epoch);
    Ok(Some(Slot::Int(year)))
}

/// Native: `LocalDate.now() -> LocalDate` — returns 1970-01-01 (epoch 0) in this interpreter.
pub(crate) fn native_localdate_now(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate("java/time/LocalDate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(0); // epoch 0 = 1970-01-01
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDate.of(int, int, int) -> LocalDate`
#[allow(clippy::cast_sign_loss)] // month/day from Java int are always positive
pub(crate) fn native_localdate_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let year = extract_int_arg(args, 0)?;
    let month = extract_int_arg(args, 1)? as u32;
    let day = extract_int_arg(args, 2)? as u32;
    let epoch = ymd_to_epoch_days(year, month, day);
    let r = heap.allocate("java/time/LocalDate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(epoch);
    Ok(Some(Slot::Reference(Some(r))))
}
