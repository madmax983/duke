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
/// Native: `Instant.ofEpochSecond(long) -> Instant`
pub(crate) fn native_instant_of_epoch_second(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let secs = extract_long_arg(args, 0)?;
    Ok(
        Some(
            Slot::Reference(
                Some(
                    allocate_instant_from_total_nanos(
                        heap,
                        i128::from(secs) * NANOS_PER_SECOND_I128,
                    )?,
                ),
            ),
        ),
    )
}
/// Native: `Instant.ofEpochMilli(long) -> Instant`
pub(crate) fn native_instant_of_epoch_milli(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let millis = extract_long_arg(args, 0)?;
    Ok(
        Some(
            Slot::Reference(
                Some(
                    allocate_instant_from_total_nanos(
                        heap,
                        i128::from(millis) * NANOS_PER_MILLI_I128,
                    )?,
                ),
            ),
        ),
    )
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
    let total = i128::from(seconds) * NANOS_PER_SECOND_I128
        + i128::from(nanos_adjustment);
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(heap, total)?))))
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
    Ok(
        Some(
            Slot::Reference(
                Some(
                    allocate_instant_from_total_nanos(
                        heap,
                        i128::from(millis) * NANOS_PER_MILLI_I128,
                    )?,
                ),
            ),
        ),
    )
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
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(heap, total)?))))
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
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(heap, total)?))))
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
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(heap, total)?))))
}
/// Native: `Instant.parse(CharSequence) -> Instant`
pub(crate) fn native_instant_parse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let text = extract_string_arg_value(args, 0, heap)?;
    let (seconds, nanos) = parse_iso_instant_components(&text)
        .ok_or_else(|| date_time_parse_error(&text))?;
    let total = i128::from(seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos);
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(heap, total)?))))
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
    let (year, month, day, hour, minute, second) = epoch_seconds_to_datetime_parts(
        seconds,
    );
    let mut text = format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}"
    );
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
fn instant_compare_impl(
    heap: &duke_gc::Heap,
    this_ref: u64,
    other_ref: u64,
) -> Result<i32> {
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
        && instant_parts_from_ref(heap, this_ref)?
            == instant_parts_from_ref(heap, other_ref)?;
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
