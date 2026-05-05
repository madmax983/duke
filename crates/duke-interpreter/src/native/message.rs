fn message_digest_algorithm(heap: &duke_gc::Heap, digest_ref: u64) -> Result<String> {
    let (string_algorithm, field_algorithm) = {
        let digest = heap.get(digest_ref)?;
        (
            digest.string_value.clone(),
            digest.fields.get(MESSAGE_DIGEST_ALGORITHM_FIELD).copied(),
        )
    };
    if let Some(algorithm) = string_algorithm {
        return Ok(algorithm);
    }
    match field_algorithm {
        Some(Slot::Reference(Some(algorithm_ref))) => {
            string_value_from_ref(heap, algorithm_ref)
        }
        _ => Ok(String::new()),
    }
}
fn message_digest_buffer(heap: &duke_gc::Heap, digest_ref: u64) -> Result<Vec<u8>> {
    match heap.get(digest_ref)?.fields.get(MESSAGE_DIGEST_BUFFER_FIELD).copied() {
        Some(Slot::Reference(Some(buffer_ref))) => byte_array_from_ref(heap, buffer_ref),
        _ => Ok(Vec::new()),
    }
}
/// Native: `MessageDigest.getInstance(String)MessageDigest` — creates a digest for supported algorithms.
pub(crate) fn native_message_digest_get_instance(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let algorithm_ref = extract_ref_arg(args, 0)?;
    let algorithm = string_value_from_ref(heap, algorithm_ref)?;
    let canonical = require_digest_algorithm(&algorithm)?;
    let canonical_ref = heap.allocate_string(canonical.to_string());
    let digest_ref = heap.allocate("java/security/MessageDigest".to_string(), 2);
    let digest = heap.get_mut(digest_ref)?;
    digest.string_value = Some(canonical.to_string());
    digest.fields[MESSAGE_DIGEST_ALGORITHM_FIELD] = Slot::Reference(Some(canonical_ref));
    digest.fields[MESSAGE_DIGEST_BUFFER_FIELD] = Slot::Reference(None);
    Ok(Some(Slot::Reference(Some(digest_ref))))
}
/// Native: `MessageDigest.getInstance(String,String)MessageDigest`.
pub(crate) fn native_message_digest_get_instance_provider_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = extract_ref_arg(args, 1)?;
    let provider_name = string_value_from_ref(heap, provider_ref)?;
    if !is_duke_provider_name(&provider_name) {
        return Err(Error::JavaException {
            class_name: "java/security/NoSuchProviderException".to_string(),
        });
    }
    native_message_digest_get_instance(&[extract_slot_arg(args, 0)], heap, out, control)
}
/// Native: `MessageDigest.getInstance(String,Provider)MessageDigest`.
pub(crate) fn native_message_digest_get_instance_provider(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = extract_ref_arg(args, 1)?;
    let provider_name = heap.get(provider_ref)?.string_value.clone().unwrap_or_default();
    if !is_duke_provider_name(&provider_name) {
        return Err(Error::JavaException {
            class_name: "java/security/NoSuchAlgorithmException".to_string(),
        });
    }
    native_message_digest_get_instance(&[extract_slot_arg(args, 0)], heap, out, control)
}
/// Native: `MessageDigest.getAlgorithm()String`.
pub(crate) fn native_message_digest_get_algorithm(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let digest_ref = extract_ref_arg(args, 0)?;
    let algorithm = message_digest_algorithm(heap, digest_ref)?;
    let algorithm_ref = heap.allocate_string(algorithm);
    Ok(Some(Slot::Reference(Some(algorithm_ref))))
}
/// Native: `MessageDigest.getProvider()Provider`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_message_digest_get_provider(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = allocate_duke_provider(heap);
    Ok(Some(Slot::Reference(Some(provider_ref))))
}
/// Native: `MessageDigest.update(byte)V`.
pub(crate) fn native_message_digest_update_byte(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let digest_ref = extract_ref_arg(args, 0)?;
    let byte = extract_int_arg(args, 1)?.to_le_bytes()[0];
    append_message_digest_buffer(heap, digest_ref, &[byte])?;
    Ok(None)
}
/// Native: `MessageDigest.update(byte[])V`.
pub(crate) fn native_message_digest_update_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let digest_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    append_message_digest_buffer(heap, digest_ref, &input)?;
    Ok(None)
}
/// Native: `MessageDigest.update(byte[],int,int)V`.
pub(crate) fn native_message_digest_update_bytes_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let digest_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let offset = extract_int_arg(args, 2)?;
    let len = extract_int_arg(args, 3)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    if offset < 0 || len < 0 {
        return Err(Error::ArrayIndexOutOfBounds {
            index: offset.min(len),
            length: input.len(),
        });
    }
    let start = usize::try_from(offset)
        .map_err(|_| Error::ArrayIndexOutOfBounds {
            index: offset,
            length: input.len(),
        })?;
    let count = usize::try_from(len)
        .map_err(|_| Error::ArrayIndexOutOfBounds {
            index: len,
            length: input.len(),
        })?;
    let Some(end) = start.checked_add(count) else {
        return Err(Error::ArrayIndexOutOfBounds {
            index: offset.saturating_add(len),
            length: input.len(),
        });
    };
    if end > input.len() {
        return Err(Error::ArrayIndexOutOfBounds {
            index: offset.saturating_add(len),
            length: input.len(),
        });
    }
    append_message_digest_buffer(heap, digest_ref, &input[start..end])?;
    Ok(None)
}
/// Native: `MessageDigest.digest([B)[B` — one-shot digest of provided bytes.
pub(crate) fn native_message_digest_digest_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let digest_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let algorithm = message_digest_algorithm(heap, digest_ref)?;
    let mut input = message_digest_buffer(heap, digest_ref)?;
    input.extend(byte_array_from_ref(heap, input_ref)?);
    let output = compute_message_digest(&algorithm, &input)?;
    write_message_digest_buffer(heap, digest_ref, &[])?;
    let out_ref = alloc_byte_array(heap, &output);
    Ok(Some(Slot::Reference(Some(out_ref))))
}
/// Native: `MessageDigest.digest()[B` — digest buffered bytes and reset state.
pub(crate) fn native_message_digest_digest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let digest_ref = extract_ref_arg(args, 0)?;
    let algorithm = message_digest_algorithm(heap, digest_ref)?;
    let input = message_digest_buffer(heap, digest_ref)?;
    let output = compute_message_digest(&algorithm, &input)?;
    write_message_digest_buffer(heap, digest_ref, &[])?;
    let out_ref = alloc_byte_array(heap, &output);
    Ok(Some(Slot::Reference(Some(out_ref))))
}
