/// Native: `LocalDate.of(int, int, int) -> LocalDate`
#[allow(clippy::cast_sign_loss)] // month/day from Java int are always positive
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
    heap.get_mut(r)?.fields[0] = Slot::Int(0); // epoch 0 = 1970-01-01
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
    #[allow(clippy::cast_possible_wrap)] // month is [1,12], fits i32
    Ok(Some(Slot::Int(month as i32)))
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
    #[allow(clippy::cast_possible_wrap)] // day is [1,31], fits i32
    Ok(Some(Slot::Int(day as i32)))
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
    #[allow(clippy::cast_possible_truncation)] // saturating_add handles out-of-range
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
    #[allow(clippy::cast_possible_truncation)] // saturating_sub handles out-of-range
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
    #[allow(clippy::cast_possible_truncation)] // year range is reasonable for Java dates
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
    let (year, month, day) =
        parse_iso_local_date_components(&text).ok_or_else(|| date_time_parse_error(&text))?;
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

fn localdate_compare_impl(heap: &duke_gc::Heap, this_ref: u64, other_ref: u64) -> Result<i32> {
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

/// Helper: days in a given month of a given year (handles leap years).
const fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30, // months 4,6,9,11 + any invalid input
    }
}

const NANOS_PER_SECOND_I128: i128 = 1_000_000_000;
const NANOS_PER_MILLI_I128: i128 = 1_000_000;
const SECONDS_PER_DAY_I64: i64 = 86_400;

fn clamp_i128_to_i64(value: i128) -> i64 {
    match i64::try_from(value) {
        Ok(value) => value,
        Err(_) if value.is_negative() => i64::MIN,
        Err(_) => i64::MAX,
    }
}

fn clamp_i64_to_i32(value: i64) -> i32 {
    match i32::try_from(value) {
        Ok(value) => value,
        Err(_) if value.is_negative() => i32::MIN,
        Err(_) => i32::MAX,
    }
}

fn normalize_seconds_nanos(total_nanos: i128) -> (i64, i32) {
    let seconds = clamp_i128_to_i64(total_nanos.div_euclid(NANOS_PER_SECOND_I128));
    let nanos = i32::try_from(total_nanos.rem_euclid(NANOS_PER_SECOND_I128)).unwrap_or(0);
    (seconds, nanos)
}

fn shift_year_month_day(year: i32, month: u32, day: u32, delta_months: i64) -> (i32, u32, u32) {
    let total_months = i64::from(year) * 12 + (i64::from(month) - 1) + delta_months;
    let new_year = clamp_i64_to_i32(total_months.div_euclid(12));
    let new_month = u32::try_from(total_months.rem_euclid(12) + 1).unwrap_or(1);
    let new_day = day.min(days_in_month(new_year, new_month));
    (new_year, new_month, new_day)
}

fn parse_fraction_to_nanos(fraction: &str) -> Option<i32> {
    if fraction.is_empty() || fraction.len() > 9 || !fraction.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let digits: i32 = fraction.parse().ok()?;
    let scale = 10_i32.pow(u32::try_from(9 - fraction.len()).ok()?);
    Some(digits.saturating_mul(scale))
}

fn parse_iso_local_date_components(text: &str) -> Option<(i32, u32, u32)> {
    let mut parts = text.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some()
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
    {
        return None;
    }
    Some((year, month, day))
}

fn parse_iso_time_components(text: &str) -> Option<(i32, i32, i32, i32)> {
    let (clock_part, nanos) = match text.split_once('.') {
        Some((clock, fraction)) => (clock, parse_fraction_to_nanos(fraction)?),
        None => (text, 0),
    };
    let mut parts = clock_part.split(':');
    let hour: i32 = parts.next()?.parse().ok()?;
    let minute: i32 = parts.next()?.parse().ok()?;
    let second: i32 = parts.next()?.parse().ok()?;
    if parts.next().is_some()
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }
    Some((hour, minute, second, nanos))
}

fn parse_iso_localdatetime_components(text: &str) -> Option<(i32, u32, u32, i32, i32, i32, i32)> {
    let (date_part, time_part) = text.split_once('T')?;
    let (year, month, day) = parse_iso_local_date_components(date_part)?;
    let (hour, minute, second, nanos) = parse_iso_time_components(time_part)?;
    Some((year, month, day, hour, minute, second, nanos))
}

fn parse_iso_instant_components(text: &str) -> Option<(i64, i32)> {
    let body = text.strip_suffix('Z')?;
    let (year, month, day, hour, minute, second, nanos) = parse_iso_localdatetime_components(body)?;
    let epoch_day = i64::from(ymd_to_epoch_days(year, month, day));
    let epoch_seconds =
        epoch_day * SECONDS_PER_DAY_I64 + i64::from(hour * 3600 + minute * 60 + second);
    Some((epoch_seconds, nanos))
}

fn epoch_seconds_to_datetime_parts(epoch_seconds: i64) -> (i32, u32, u32, i32, i32, i32) {
    let epoch_days = clamp_i64_to_i32(epoch_seconds.div_euclid(SECONDS_PER_DAY_I64));
    let second_of_day = epoch_seconds.rem_euclid(SECONDS_PER_DAY_I64);
    let hour = i32::try_from(second_of_day / 3600).unwrap_or(0);
    let minute = i32::try_from((second_of_day % 3600) / 60).unwrap_or(0);
    let second = i32::try_from(second_of_day % 60).unwrap_or(0);
    let (year, month, day) = epoch_days_to_ymd(epoch_days);
    (year, month, day, hour, minute, second)
}

fn date_time_parse_error(input: &str) -> Error {
    push_pending_java_exception_message(
        "java/time/format/DateTimeParseException",
        format!("Text '{input}' could not be parsed"),
    );
    Error::JavaException {
        class_name: "java/time/format/DateTimeParseException".to_string(),
    }
}

fn class_cast_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/ClassCastException".to_string(),
    }
}

fn duration_parts_from_ref(heap: &duke_gc::Heap, duration_ref: u64) -> Result<(i64, i32)> {
    let obj = heap.get(duration_ref)?;
    let seconds = match obj.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let nanos = match obj.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok((seconds, nanos))
}

fn duration_total_nanos_from_ref(heap: &duke_gc::Heap, duration_ref: u64) -> Result<i128> {
    let (seconds, nanos) = duration_parts_from_ref(heap, duration_ref)?;
    Ok(i128::from(seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos))
}

fn allocate_duration_from_total_nanos(heap: &mut duke_gc::Heap, total_nanos: i128) -> Result<u64> {
    let (seconds, nanos) = normalize_seconds_nanos(total_nanos);
    let duration_ref = heap.allocate("java/time/Duration".to_string(), 2);
    heap.get_mut(duration_ref)?.fields[0] = Slot::Long(seconds);
    heap.get_mut(duration_ref)?.fields[1] = Slot::Int(nanos);
    Ok(duration_ref)
}

fn instant_parts_from_ref(heap: &duke_gc::Heap, instant_ref: u64) -> Result<(i64, i32)> {
    let obj = heap.get(instant_ref)?;
    let seconds = match obj.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let nanos = match obj.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok((seconds, nanos))
}

fn instant_total_nanos_from_ref(heap: &duke_gc::Heap, instant_ref: u64) -> Result<i128> {
    let (seconds, nanos) = instant_parts_from_ref(heap, instant_ref)?;
    Ok(i128::from(seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos))
}

fn allocate_instant_from_total_nanos(heap: &mut duke_gc::Heap, total_nanos: i128) -> Result<u64> {
    let (seconds, nanos) = normalize_seconds_nanos(total_nanos);
    let instant_ref = heap.allocate("java/time/Instant".to_string(), 2);
    heap.get_mut(instant_ref)?.fields[0] = Slot::Long(seconds);
    heap.get_mut(instant_ref)?.fields[1] = Slot::Int(nanos);
    Ok(instant_ref)
}

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
    let (epoch_day, hour, minute, second, nanos) =
        localdatetime_components_from_ref(heap, localdatetime_ref)?;
    let epoch_seconds =
        i64::from(epoch_day) * SECONDS_PER_DAY_I64 + i64::from(hour * 3600 + minute * 60 + second);
    Ok(i128::from(epoch_seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos))
}

fn allocate_localdate(heap: &mut duke_gc::Heap, epoch_day: i32) -> Result<u64> {
    let localdate_ref = heap.allocate("java/time/LocalDate".to_string(), 1);
    heap.get_mut(localdate_ref)?.fields[0] = Slot::Int(epoch_day);
    Ok(localdate_ref)
}

fn allocate_localdatetime(
    heap: &mut duke_gc::Heap,
    epoch_day: i32,
    hour: i32,
    minute: i32,
    second: i32,
    nanos: i32,
) -> Result<u64> {
    let localdatetime_ref = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    heap.get_mut(localdatetime_ref)?.fields[0] = Slot::Int(epoch_day);
    heap.get_mut(localdatetime_ref)?.fields[1] = Slot::Int(hour);
    heap.get_mut(localdatetime_ref)?.fields[2] = Slot::Int(minute);
    heap.get_mut(localdatetime_ref)?.fields[3] = Slot::Int(second);
    heap.get_mut(localdatetime_ref)?.fields[4] = Slot::Int(nanos);
    Ok(localdatetime_ref)
}

// ---- Duration layout: fields[0]=Slot::Long(seconds), fields[1]=Slot::Int(nanos_adj) ----

/// Native: `Duration.ofSeconds(long) -> Duration`
pub(crate) fn native_duration_of_seconds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let secs = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        i128::from(secs) * NANOS_PER_SECOND_I128,
    )?))))
}

/// Native: `Duration.ofMinutes(long) -> Duration`
pub(crate) fn native_duration_of_minutes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let mins = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        i128::from(mins) * 60 * NANOS_PER_SECOND_I128,
    )?))))
}

/// Native: `Duration.ofHours(long) -> Duration`
pub(crate) fn native_duration_of_hours(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let hrs = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        i128::from(hrs) * 3600 * NANOS_PER_SECOND_I128,
    )?))))
}

/// Native: `Duration.ofDays(long) -> Duration`
pub(crate) fn native_duration_of_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let days = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        i128::from(days) * i128::from(SECONDS_PER_DAY_I64) * NANOS_PER_SECOND_I128,
    )?))))
}

/// Native: `Duration.getSeconds() -> long`
pub(crate) fn native_duration_get_seconds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs)))
}

/// Native: `Duration.toSeconds() -> long`
pub(crate) fn native_duration_to_seconds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_duration_get_seconds(args, heap, out, control)
}

/// Native: `Duration.toMinutes() -> long`
pub(crate) fn native_duration_to_minutes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs / 60)))
}

/// Native: `Duration.toHours() -> long`
pub(crate) fn native_duration_to_hours(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs / 3600)))
}

/// Native: `Duration.toDays() -> long`
pub(crate) fn native_duration_to_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs / 86_400)))
}

/// Native: `Duration.plus(Duration) -> Duration`
pub(crate) fn native_duration_plus(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let total = duration_total_nanos_from_ref(heap, this_ref)?
        + duration_total_nanos_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Duration.minus(Duration) -> Duration`
pub(crate) fn native_duration_minus(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let total = duration_total_nanos_from_ref(heap, this_ref)?
        - duration_total_nanos_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Duration.isNegative() -> boolean`
pub(crate) fn native_duration_is_negative(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(secs < 0))))
}

/// Native: `Duration.isZero() -> boolean`
pub(crate) fn native_duration_is_zero(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

/// Native: `Duration.ofMillis(long) -> Duration`
pub(crate) fn native_duration_of_millis(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let millis = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        i128::from(millis) * NANOS_PER_MILLI_I128,
    )?))))
}

/// Native: `Duration.ofNanos(long) -> Duration`
pub(crate) fn native_duration_of_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let nanos = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        i128::from(nanos),
    )?))))
}

/// Native: `Duration.between(start, end) -> Duration`
pub(crate) fn native_duration_between(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let start_ref = extract_ref_arg(args, 0)?;
    let end_ref = extract_ref_arg(args, 1)?;
    let start = heap.get(start_ref)?;
    let total_nanos = match start.class_name.as_str() {
        "java/time/Instant" => {
            instant_total_nanos_from_ref(heap, end_ref)? - instant_total_nanos_from_ref(heap, start_ref)?
        }
        "java/time/LocalDateTime" => {
            localdatetime_total_nanos_from_ref(heap, end_ref)?
                - localdatetime_total_nanos_from_ref(heap, start_ref)?
        }
        "java/time/LocalDate" => {
            let start_epoch = match start.fields.first() {
                Some(Slot::Int(v)) => *v,
                _ => 0,
            };
            let end_epoch = match heap.get(end_ref)?.fields.first() {
                Some(Slot::Int(v)) => *v,
                _ => 0,
            };
            i128::from(end_epoch - start_epoch)
                * i128::from(SECONDS_PER_DAY_I64)
                * NANOS_PER_SECOND_I128
        }
        _ => return Err(class_cast_error()),
    };
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap, total_nanos,
    )?))))
}

/// Native: `Duration.toMillis() -> long`
pub(crate) fn native_duration_to_millis(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let millis = clamp_i128_to_i64(duration_total_nanos_from_ref(heap, this_ref)? / NANOS_PER_MILLI_I128);
    Ok(Some(Slot::Long(millis)))
}

/// Native: `Duration.toNanos() -> long`
pub(crate) fn native_duration_to_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Long(clamp_i128_to_i64(duration_total_nanos_from_ref(
        heap, this_ref,
    )?))))
}

/// Native: `Duration.negated() -> Duration`
pub(crate) fn native_duration_negated(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        -duration_total_nanos_from_ref(heap, this_ref)?,
    )?))))
}

fn duration_compare_impl(heap: &duke_gc::Heap, this_ref: u64, other_ref: u64) -> Result<i32> {
    let other = heap.get(other_ref)?;
    if other.class_name != "java/time/Duration" {
        return Err(class_cast_error());
    }
    let this_total = duration_total_nanos_from_ref(heap, this_ref)?;
    let other_total = duration_total_nanos_from_ref(heap, other_ref)?;
    Ok(ordering_to_int(this_total.cmp(&other_total)))
}

/// Native: `Duration.compareTo(Duration) -> int`
pub(crate) fn native_duration_compare_to(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    Ok(Some(Slot::Int(duration_compare_impl(heap, this_ref, other_ref)?)))
}

/// Native: bridge `Duration.compareTo(Object) -> int`
pub(crate) fn native_duration_compare_to_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_duration_compare_to(args, heap, out, control)
}

/// Native: `Duration.equals(Object) -> boolean`
pub(crate) fn native_duration_equals(
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
    let equal = other.class_name == "java/time/Duration"
        && duration_total_nanos_from_ref(heap, this_ref)?
            == duration_total_nanos_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `Duration.hashCode() -> int`
pub(crate) fn native_duration_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (seconds, nanos) = duration_parts_from_ref(heap, this_ref)?;
    let folded = seconds ^ (seconds >> 32);
    #[allow(clippy::cast_possible_truncation)]
    Ok(Some(Slot::Int((folded as i32) ^ nanos)))
}

// ---- Period layout: fields[0]=years(Int), fields[1]=months(Int), fields[2]=days(Int) ----

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
    let neg = heap.get(this_ref)?.fields.iter().any(|s| matches!(s, Slot::Int(v) if *v < 0));
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

// ---- Instant layout: fields[0]=Slot::Long(epoch_seconds), fields[1]=Slot::Int(nanos_adj) ----

/// Native: `Instant.ofEpochSecond(long) -> Instant`
pub(crate) fn native_instant_of_epoch_second(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let secs = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap,
        i128::from(secs) * NANOS_PER_SECOND_I128,
    )?))))
}

/// Native: `Instant.ofEpochMilli(long) -> Instant`
pub(crate) fn native_instant_of_epoch_milli(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let millis = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap,
        i128::from(millis) * NANOS_PER_MILLI_I128,
    )?))))
}

/// Native: `Instant.getEpochSecond() -> long`
pub(crate) fn native_instant_get_epoch_second(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs)))
}

/// Native: `Instant.toEpochMilli() -> long`
pub(crate) fn native_instant_to_epoch_milli(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
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

/// Native: `Instant.isBefore(Instant) -> boolean`
pub(crate) fn native_instant_is_before(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let this_parts = instant_parts_from_ref(heap, this_ref)?;
    let other_parts = instant_parts_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Int(i32::from(this_parts < other_parts))))
}

/// Native: `Instant.isAfter(Instant) -> boolean`
pub(crate) fn native_instant_is_after(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let this_parts = instant_parts_from_ref(heap, this_ref)?;
    let other_parts = instant_parts_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Int(i32::from(this_parts > other_parts))))
}

/// Native: `Instant.ofEpochSecond(long, long) -> Instant`
pub(crate) fn native_instant_of_epoch_second_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let seconds = extract_long_arg(args, 0)?;
    let nanos_adjustment = extract_long_arg(args, 1)?;
    let total = i128::from(seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos_adjustment);
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Instant.getNano() -> int`
pub(crate) fn native_instant_get_nano(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let nanos = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(nanos)))
}

/// Native: `Instant.now() -> Instant`
pub(crate) fn native_instant_now(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let millis = system_time_to_epoch_millis(std::time::SystemTime::now());
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap,
        i128::from(millis) * NANOS_PER_MILLI_I128,
    )?))))
}

/// Native: `Instant.plusSeconds(long) -> Instant`
pub(crate) fn native_instant_plus_seconds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let seconds = extract_long_arg(args, 1)?;
    let total = instant_total_nanos_from_ref(heap, this_ref)?
        + i128::from(seconds) * NANOS_PER_SECOND_I128;
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Instant.plusNanos(long) -> Instant`
pub(crate) fn native_instant_plus_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let nanos = extract_long_arg(args, 1)?;
    let total = instant_total_nanos_from_ref(heap, this_ref)? + i128::from(nanos);
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Instant.minusMillis(long) -> Instant`
pub(crate) fn native_instant_minus_millis(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let millis = extract_long_arg(args, 1)?;
    let total = instant_total_nanos_from_ref(heap, this_ref)?
        - i128::from(millis) * NANOS_PER_MILLI_I128;
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Instant.parse(CharSequence) -> Instant`
pub(crate) fn native_instant_parse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let text = extract_string_arg_value(args, 0, heap)?;
    let (seconds, nanos) =
        parse_iso_instant_components(&text).ok_or_else(|| date_time_parse_error(&text))?;
    let total = i128::from(seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos);
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Instant.toString() -> String`
pub(crate) fn native_instant_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (seconds, nanos) = instant_parts_from_ref(heap, this_ref)?;
    let (year, month, day, hour, minute, second) = epoch_seconds_to_datetime_parts(seconds);
    let mut text = format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}");
    if nanos != 0 {
        let mut fraction = format!("{nanos:09}");
        while fraction.ends_with('0') {
            fraction.pop();
        }
        text.push('.');
        text.push_str(&fraction);
    }
    text.push('Z');
    let text_ref = heap.allocate_string(text);
    Ok(Some(Slot::Reference(Some(text_ref))))
}

fn instant_compare_impl(heap: &duke_gc::Heap, this_ref: u64, other_ref: u64) -> Result<i32> {
    let other = heap.get(other_ref)?;
    if other.class_name != "java/time/Instant" {
        return Err(class_cast_error());
    }
    let this_parts = instant_parts_from_ref(heap, this_ref)?;
    let other_parts = instant_parts_from_ref(heap, other_ref)?;
    Ok(ordering_to_int(this_parts.cmp(&other_parts)))
}

/// Native: `Instant.compareTo(Instant) -> int`
pub(crate) fn native_instant_compare_to(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    Ok(Some(Slot::Int(instant_compare_impl(heap, this_ref, other_ref)?)))
}

/// Native: bridge `Instant.compareTo(Object) -> int`
pub(crate) fn native_instant_compare_to_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_instant_compare_to(args, heap, out, control)
}

/// Native: `Instant.equals(Object) -> boolean`
pub(crate) fn native_instant_equals(
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
    let equal = other.class_name == "java/time/Instant"
        && instant_parts_from_ref(heap, this_ref)? == instant_parts_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `Instant.hashCode() -> int`
pub(crate) fn native_instant_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (seconds, nanos) = instant_parts_from_ref(heap, this_ref)?;
    let folded = seconds ^ (seconds >> 32);
    #[allow(clippy::cast_possible_truncation)]
    Ok(Some(Slot::Int((folded as i32) ^ nanos)))
}

// ---------------------------------------------------------------------------
// Phase 63: java.time.LocalDateTime
// Layout: fields[0]=epoch_days(Int), fields[1]=hour(Int),
//         fields[2]=minute(Int), fields[3]=second(Int), fields[4]=nano(Int)
// ---------------------------------------------------------------------------

/// Native: `LocalDateTime.of(int,int,int,int,int) -> LocalDateTime`
#[allow(clippy::cast_sign_loss)] // month/day from Java int are always positive
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
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap, epoch, hour, minute, 0, 0,
    )?))))
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
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap, epoch, hour, minute, second, 0,
    )?))))
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
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap, epoch, hour, minute, second, 0,
    )?))))
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
    #[allow(clippy::cast_possible_wrap)] // month is [1,12]
    Ok(Some(Slot::Int(month as i32)))
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
    #[allow(clippy::cast_possible_wrap)] // day is [1,31]
    Ok(Some(Slot::Int(day as i32)))
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
    // same day — compare time fields
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
    Ok(Some(Slot::Int(0))) // equal
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
    Ok(Some(Slot::Int(0))) // equal
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
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
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
    )?))))
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
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap, epoch, hour, minute, second, nanos,
    )?))))
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
    let (epoch_day, hour, minute, second, nanos) =
        localdatetime_components_from_ref(heap, this_ref)?;
    let total_seconds = i64::from(hour * 3600 + minute * 60 + second) + hours * 3600;
    let day_delta = total_seconds.div_euclid(SECONDS_PER_DAY_I64);
    let second_of_day = total_seconds.rem_euclid(SECONDS_PER_DAY_I64);
    let new_hour = i32::try_from(second_of_day / 3600).unwrap_or(0);
    let new_minute = i32::try_from((second_of_day % 3600) / 60).unwrap_or(0);
    let new_second = i32::try_from(second_of_day % 60).unwrap_or(0);
    #[allow(clippy::cast_possible_truncation)]
    let new_epoch_day = epoch_day.saturating_add(day_delta as i32);
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap,
        new_epoch_day,
        new_hour,
        new_minute,
        new_second,
        nanos,
    )?))))
}

/// Native: `LocalDateTime.parse(CharSequence) -> LocalDateTime`
pub(crate) fn native_localdatetime_parse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let text = extract_string_arg_value(args, 0, heap)?;
    let (year, month, day, hour, minute, second, nanos) =
        parse_iso_localdatetime_components(&text).ok_or_else(|| date_time_parse_error(&text))?;
    let epoch_day = ymd_to_epoch_days(year, month, day);
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap,
        epoch_day,
        hour,
        minute,
        second,
        nanos,
    )?))))
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

fn localdatetime_compare_impl(heap: &duke_gc::Heap, this_ref: u64, other_ref: u64) -> Result<i32> {
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
    Ok(Some(Slot::Int(localdatetime_compare_impl(
        heap, this_ref, other_ref,
    )?)))
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
    let (epoch_day, hour, minute, second, nanos) =
        localdatetime_components_from_ref(heap, this_ref)?;
    let mut hash = epoch_day;
    hash = hash.wrapping_mul(31).wrapping_add(hour);
    hash = hash.wrapping_mul(31).wrapping_add(minute);
    hash = hash.wrapping_mul(31).wrapping_add(second);
    hash = hash.wrapping_mul(31).wrapping_add(nanos);
    Ok(Some(Slot::Int(hash)))
}

// ---------------------------------------------------------------------------
// Phase 64: String.indent, StringBuilder.setCharAt, Collections.disjoint,
//           HashMap.computeIfPresent
// ---------------------------------------------------------------------------
