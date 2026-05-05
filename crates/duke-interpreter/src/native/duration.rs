fn duration_parts_from_ref(
    heap: &duke_gc::Heap,
    duration_ref: u64,
) -> Result<(i64, i32)> {
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
fn duration_total_nanos_from_ref(
    heap: &duke_gc::Heap,
    duration_ref: u64,
) -> Result<i128> {
    let (seconds, nanos) = duration_parts_from_ref(heap, duration_ref)?;
    Ok(i128::from(seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos))
}
/// Native: `Duration.ofSeconds(long) -> Duration`
pub(crate) fn native_duration_of_seconds(
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
                    allocate_duration_from_total_nanos(
                        heap,
                        i128::from(secs) * NANOS_PER_SECOND_I128,
                    )?,
                ),
            ),
        ),
    )
}
/// Native: `Duration.ofMinutes(long) -> Duration`
pub(crate) fn native_duration_of_minutes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let mins = extract_long_arg(args, 0)?;
    Ok(
        Some(
            Slot::Reference(
                Some(
                    allocate_duration_from_total_nanos(
                        heap,
                        i128::from(mins) * 60 * NANOS_PER_SECOND_I128,
                    )?,
                ),
            ),
        ),
    )
}
/// Native: `Duration.ofHours(long) -> Duration`
pub(crate) fn native_duration_of_hours(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let hrs = extract_long_arg(args, 0)?;
    Ok(
        Some(
            Slot::Reference(
                Some(
                    allocate_duration_from_total_nanos(
                        heap,
                        i128::from(hrs) * 3600 * NANOS_PER_SECOND_I128,
                    )?,
                ),
            ),
        ),
    )
}
/// Native: `Duration.ofDays(long) -> Duration`
pub(crate) fn native_duration_of_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let days = extract_long_arg(args, 0)?;
    Ok(
        Some(
            Slot::Reference(
                Some(
                    allocate_duration_from_total_nanos(
                        heap,
                        i128::from(days) * i128::from(SECONDS_PER_DAY_I64)
                            * NANOS_PER_SECOND_I128,
                    )?,
                ),
            ),
        ),
    )
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
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(heap, total)?))))
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
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(heap, total)?))))
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
    Ok(
        Some(
            Slot::Reference(
                Some(
                    allocate_duration_from_total_nanos(
                        heap,
                        i128::from(millis) * NANOS_PER_MILLI_I128,
                    )?,
                ),
            ),
        ),
    )
}
/// Native: `Duration.ofNanos(long) -> Duration`
pub(crate) fn native_duration_of_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let nanos = extract_long_arg(args, 0)?;
    Ok(
        Some(
            Slot::Reference(
                Some(allocate_duration_from_total_nanos(heap, i128::from(nanos))?),
            ),
        ),
    )
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
            instant_total_nanos_from_ref(heap, end_ref)?
                - instant_total_nanos_from_ref(heap, start_ref)?
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
            i128::from(end_epoch - start_epoch) * i128::from(SECONDS_PER_DAY_I64)
                * NANOS_PER_SECOND_I128
        }
        _ => return Err(class_cast_error()),
    };
    Ok(
        Some(
            Slot::Reference(Some(allocate_duration_from_total_nanos(heap, total_nanos)?)),
        ),
    )
}
/// Native: `Duration.toMillis() -> long`
pub(crate) fn native_duration_to_millis(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let millis = clamp_i128_to_i64(
        duration_total_nanos_from_ref(heap, this_ref)? / NANOS_PER_MILLI_I128,
    );
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
    Ok(
        Some(
            Slot::Long(clamp_i128_to_i64(duration_total_nanos_from_ref(heap, this_ref)?)),
        ),
    )
}
/// Native: `Duration.negated() -> Duration`
pub(crate) fn native_duration_negated(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(
        Some(
            Slot::Reference(
                Some(
                    allocate_duration_from_total_nanos(
                        heap,
                        -duration_total_nanos_from_ref(heap, this_ref)?,
                    )?,
                ),
            ),
        ),
    )
}
fn duration_compare_impl(
    heap: &duke_gc::Heap,
    this_ref: u64,
    other_ref: u64,
) -> Result<i32> {
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
