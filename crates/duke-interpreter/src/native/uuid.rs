fn uuid_bits_from_object(obj: &duke_gc::HeapObject) -> Option<(i64, i64)> {
    match (obj.fields.get(UUID_MSB_FIELD), obj.fields.get(UUID_LSB_FIELD)) {
        (Some(Slot::Long(msb)), Some(Slot::Long(lsb))) => Some((*msb, *lsb)),
        _ => None,
    }
}
fn uuid_bits_from_ref(heap: &duke_gc::Heap, uuid_ref: u64) -> Result<(i64, i64)> {
    uuid_bits_from_object(heap.get(uuid_ref)?)
        .ok_or(Error::TypeMismatch {
            expected: "UUID fields",
            got: "other",
        })
}
fn uuid_bits_from_bytes(bytes: &[u8; 16]) -> (i64, i64) {
    let mut msb_bytes = [0_u8; 8];
    let mut lsb_bytes = [0_u8; 8];
    msb_bytes.copy_from_slice(&bytes[..8]);
    lsb_bytes.copy_from_slice(&bytes[8..]);
    (i64::from_be_bytes(msb_bytes), i64::from_be_bytes(lsb_bytes))
}
fn uuid_invalid_format() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    }
}
fn uuid_hex_nibble(byte: u8) -> Option<u64> {
    match byte {
        b'0'..=b'9' => Some(u64::from(byte - b'0')),
        b'a'..=b'f' => Some(u64::from(byte - b'a' + 10)),
        b'A'..=b'F' => Some(u64::from(byte - b'A' + 10)),
        _ => None,
    }
}
fn uuid_to_string(msb: i64, lsb: i64) -> String {
    let msb = msb.cast_unsigned();
    let lsb = lsb.cast_unsigned();
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}", (msb >> 32) & 0xffff_ffff, (msb >> 16) &
        0xffff, msb & 0xffff, (lsb >> 48) & 0xffff, lsb & 0xffff_ffff_ffff
    )
}
/// Native: `UUID.<init>(long,long)V` — stores the two canonical 64-bit halves.
pub(crate) fn native_uuid_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let msb = extract_long_arg(args, 1)?;
    let lsb = extract_long_arg(args, 2)?;
    write_uuid_bits(heap, this_ref, msb, lsb)?;
    Ok(None)
}
/// Native: `UUID.randomUUID()UUID` — RFC 4122 version-4 UUID from host CSPRNG.
pub(crate) fn native_uuid_random_uuid(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| Error::JavaException {
            class_name: "java/lang/InternalError".to_string(),
        })?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let (msb, lsb) = uuid_bits_from_bytes(&bytes);
    let uuid_ref = allocate_uuid(heap, msb, lsb)?;
    Ok(Some(Slot::Reference(Some(uuid_ref))))
}
/// Native: `UUID.nameUUIDFromBytes(byte[])UUID` — RFC 4122 version-3 MD5 UUID.
pub(crate) fn native_uuid_name_uuid_from_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let input_ref = extract_ref_arg(args, 0)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    let digest = compute_message_digest("MD5", &input)?;
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let (msb, lsb) = uuid_bits_from_bytes(&bytes);
    let uuid_ref = allocate_uuid(heap, msb, lsb)?;
    Ok(Some(Slot::Reference(Some(uuid_ref))))
}
/// Native: `UUID.fromString(String)UUID` — parses canonical 8-4-4-4-12 UUID text.
pub(crate) fn native_uuid_from_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let text_ref = extract_ref_arg(args, 0)?;
    let text = string_value_from_ref(heap, text_ref)?;
    let (msb, lsb) = parse_uuid_string(&text)?;
    let uuid_ref = allocate_uuid(heap, msb, lsb)?;
    Ok(Some(Slot::Reference(Some(uuid_ref))))
}
/// Native: `UUID.getMostSignificantBits()long`.
pub(crate) fn native_uuid_get_most_significant_bits(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (msb, _) = uuid_bits_from_ref(heap, this_ref)?;
    Ok(Some(Slot::Long(msb)))
}
/// Native: `UUID.getLeastSignificantBits()long`.
pub(crate) fn native_uuid_get_least_significant_bits(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, lsb) = uuid_bits_from_ref(heap, this_ref)?;
    Ok(Some(Slot::Long(lsb)))
}
/// Native: `UUID.version()int`.
pub(crate) fn native_uuid_version(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (msb, _) = uuid_bits_from_ref(heap, this_ref)?;
    let version = i32::try_from((msb.cast_unsigned() >> 12) & 0x0f)
        .expect("UUID version nibble fits in i32");
    Ok(Some(Slot::Int(version)))
}
/// Native: `UUID.variant()int`.
pub(crate) fn native_uuid_variant(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, lsb) = uuid_bits_from_ref(heap, this_ref)?;
    let lsb = lsb.cast_unsigned();
    let variant = if (lsb >> 63) == 0 {
        0
    } else if (lsb >> 62) == 0b10 {
        2
    } else if (lsb >> 61) == 0b110 {
        6
    } else {
        7
    };
    Ok(Some(Slot::Int(variant)))
}
/// Native: `UUID.toString()String`.
pub(crate) fn native_uuid_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (msb, lsb) = uuid_bits_from_ref(heap, this_ref)?;
    let string_ref = heap.allocate_string(uuid_to_string(msb, lsb));
    Ok(Some(Slot::Reference(Some(string_ref))))
}
/// Native: `UUID.equals(Object)boolean`.
pub(crate) fn native_uuid_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    let this_bits = uuid_bits_from_ref(heap, this_ref)?;
    let other = heap.get(other_ref)?;
    let equal = other.class_name == "java/util/UUID"
        && uuid_bits_from_object(other)
            .is_some_and(|other_bits| other_bits == this_bits);
    Ok(Some(Slot::Int(i32::from(equal))))
}
/// Native: `UUID.hashCode()int`.
pub(crate) fn native_uuid_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (msb, lsb) = uuid_bits_from_ref(heap, this_ref)?;
    let hilo = msb.cast_unsigned() ^ lsb.cast_unsigned();
    let high = u32::try_from(hilo >> 32).expect("upper 32 bits fit in u32");
    let low = u32::try_from(hilo & 0xffff_ffff).expect("lower 32 bits fit in u32");
    Ok(Some(Slot::Int((high ^ low).cast_signed())))
}
/// Native: `UUID.compareTo(UUID)int`.
pub(crate) fn native_uuid_compare_to(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let this_bits = uuid_bits_from_ref(heap, this_ref)?;
    let other_bits = uuid_bits_from_ref(heap, other_ref)?;
    let ordering = this_bits
        .0
        .cmp(&other_bits.0)
        .then_with(|| this_bits.1.cmp(&other_bits.1));
    Ok(
        Some(
            Slot::Int(
                match ordering {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                },
            ),
        ),
    )
}
/// Native: version-1 UUID accessors are intentionally deferred for v3/v4 UUIDs.
pub(crate) fn native_uuid_unsupported_version1_accessor(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Err(Error::JavaException {
        class_name: "java/lang/UnsupportedOperationException".to_string(),
    })
}
