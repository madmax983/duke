fn base64_variant_arg(args: &[Slot], heap: &duke_gc::Heap) -> Result<Base64Variant> {
    let coder_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Int(value)) = heap.get(coder_ref)?.fields.first() else {
        return Err(Error::TypeMismatch {
            expected: "Base64 variant",
            got: "other",
        });
    };
    Base64Variant::from_field(*value).ok_or_else(invalid_base64_error)
}
fn base64_decode_value(byte: u8, variant: Base64Variant) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' if variant != Base64Variant::Url => Some(62),
        b'/' if variant != Base64Variant::Url => Some(63),
        b'-' if variant == Base64Variant::Url => Some(62),
        b'_' if variant == Base64Variant::Url => Some(63),
        _ => None,
    }
}
pub(crate) fn native_base64_get_encoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let encoder_ref = allocate_base64_coder(
        heap,
        "java/util/Base64$Encoder",
        Base64Variant::Standard,
    )?;
    Ok(Some(Slot::Reference(Some(encoder_ref))))
}
pub(crate) fn native_base64_get_mime_encoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let encoder_ref = allocate_base64_coder(
        heap,
        "java/util/Base64$Encoder",
        Base64Variant::Mime,
    )?;
    Ok(Some(Slot::Reference(Some(encoder_ref))))
}
pub(crate) fn native_base64_get_url_encoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let encoder_ref = allocate_base64_coder(
        heap,
        "java/util/Base64$Encoder",
        Base64Variant::Url,
    )?;
    Ok(Some(Slot::Reference(Some(encoder_ref))))
}
pub(crate) fn native_base64_get_decoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let decoder_ref = allocate_base64_coder(
        heap,
        "java/util/Base64$Decoder",
        Base64Variant::Standard,
    )?;
    Ok(Some(Slot::Reference(Some(decoder_ref))))
}
pub(crate) fn native_base64_get_mime_decoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let decoder_ref = allocate_base64_coder(
        heap,
        "java/util/Base64$Decoder",
        Base64Variant::Mime,
    )?;
    Ok(Some(Slot::Reference(Some(decoder_ref))))
}
pub(crate) fn native_base64_get_url_decoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let decoder_ref = allocate_base64_coder(
        heap,
        "java/util/Base64$Decoder",
        Base64Variant::Url,
    )?;
    Ok(Some(Slot::Reference(Some(decoder_ref))))
}
pub(crate) fn native_base64_encoder_encode_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let variant = base64_variant_arg(args, heap)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    let string_ref = heap.allocate_string(encode_base64(&input, variant));
    Ok(Some(Slot::Reference(Some(string_ref))))
}
pub(crate) fn native_base64_encoder_encode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let variant = base64_variant_arg(args, heap)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    let encoded = encode_base64(&input, variant);
    let array_ref = alloc_byte_array(heap, &encoded.into_bytes());
    Ok(Some(Slot::Reference(Some(array_ref))))
}
pub(crate) fn native_base64_decoder_decode_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let variant = base64_variant_arg(args, heap)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = string_value_from_ref(heap, input_ref)?;
    let decoded = decode_base64(input.as_bytes(), variant)?;
    let array_ref = alloc_byte_array(heap, &decoded);
    Ok(Some(Slot::Reference(Some(array_ref))))
}
pub(crate) fn native_base64_decoder_decode_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let variant = base64_variant_arg(args, heap)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    let decoded = decode_base64(&input, variant)?;
    let array_ref = alloc_byte_array(heap, &decoded);
    Ok(Some(Slot::Reference(Some(array_ref))))
}
