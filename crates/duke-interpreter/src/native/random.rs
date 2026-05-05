/// Advance the LCG and return `bits` high bits of the new state.
#[allow(clippy::cast_possible_truncation)]
const fn random_next(seed: u64, bits: u32) -> (u64, i32) {
    let new_seed = seed.wrapping_mul(RANDOM_MULTIPLIER).wrapping_add(RANDOM_ADDEND)
        & RANDOM_MASK;
    let value = (new_seed >> (48 - bits)) as i32;
    (new_seed, value)
}
/// Native: `Random.<init>()V` — seed from current time.
#[allow(clippy::unnecessary_wraps, clippy::cast_possible_wrap)]
pub(crate) fn native_random_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let nanos = u64::from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos()),
    );
    let initial = (nanos ^ RANDOM_MULTIPLIER) & RANDOM_MASK;
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(initial as i64);
    Ok(None)
}
/// Native: `Random.<init>(J)V` — seed with explicit long value.
#[allow(clippy::unnecessary_wraps, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
pub(crate) fn native_random_init_seed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let seed = match args.get(1).copied() {
        Some(Slot::Long(v)) => v as u64,
        Some(Slot::Int(v)) => v as u64,
        _ => 0,
    };
    let initial = (seed ^ RANDOM_MULTIPLIER) & RANDOM_MASK;
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(initial as i64);
    Ok(None)
}
/// Retrieve and advance seed from `fields[0]`, returning new seed and `bits` high bits.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
fn random_step(
    heap: &mut duke_gc::Heap,
    this_ref: u64,
    bits: u32,
) -> Result<(u64, i32)> {
    let old_seed = match heap.get(this_ref)?.fields.first().copied() {
        Some(Slot::Long(v)) => v as u64,
        _ => 0,
    };
    let (new_seed, value) = random_next(old_seed, bits);
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(new_seed as i64);
    Ok((new_seed, value))
}
/// Native: `Random.nextInt()I` — full-range random int.
pub(crate) fn native_random_next_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, v) = random_step(heap, this_ref, 32)?;
    Ok(Some(Slot::Int(v)))
}
/// Native: `Random.nextInt(I)I` — bounded random int [0, bound).
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]
pub(crate) fn native_random_next_int_bound(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let bound = extract_int_arg(args, 1)?;
    if bound <= 0 {
        return Err(duke_runtime::Error::JavaException {
            class_name: "java/lang/IllegalArgumentException".to_string(),
        });
    }
    let bound_u = bound as u32;
    loop {
        let (_, bits) = random_step(heap, this_ref, 31)?;
        let bits_u = bits as u32;
        let val = bits_u % bound_u;
        if bits_u.wrapping_sub(val).wrapping_add(bound_u - 1) < u32::MAX {
            return Ok(Some(Slot::Int(val as i32)));
        }
    }
}
/// Native: `Random.nextLong()J` — 64-bit random long (two 32-bit calls).
pub(crate) fn native_random_next_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, hi) = random_step(heap, this_ref, 32)?;
    let (_, lo) = random_step(heap, this_ref, 32)?;
    let v = (i64::from(hi) << 32) + i64::from(lo);
    Ok(Some(Slot::Long(v)))
}
/// Native: `Random.nextDouble()D` — uniform [0.0, 1.0).
#[allow(clippy::cast_precision_loss)]
pub(crate) fn native_random_next_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, hi) = random_step(heap, this_ref, 26)?;
    let (_, lo) = random_step(heap, this_ref, 27)?;
    let combined = (i64::from(hi) << 27) + i64::from(lo);
    let v = combined as f64 / (1u64 << 53) as f64;
    Ok(Some(Slot::Double(v)))
}
/// Native: `Random.nextFloat()F` — uniform [0.0, 1.0) as float.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn native_random_next_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, bits) = random_step(heap, this_ref, 24)?;
    let v = bits as f32 / (1u32 << 24) as f32;
    Ok(Some(Slot::Float(v)))
}
/// Native: `Random.nextBoolean()Z` — random boolean.
pub(crate) fn native_random_next_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, v) = random_step(heap, this_ref, 1)?;
    Ok(Some(Slot::Int(v)))
}
