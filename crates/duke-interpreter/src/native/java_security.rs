/// Native: `Throwable.<init>(Throwable)V` - stores only the cause.
pub(crate) fn native_throwable_init_cause(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    if let Some(&cause_slot) = args.get(1)
        && let Ok(obj) = heap.get_mut(this_ref)
        && !obj.fields.is_empty()
    {
        obj.fields[THROWABLE_CAUSE_FIELD] = cause_slot;
    }
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}
/// Native: `AccessController.doPrivileged(PrivilegedAction)Object`.
pub(crate) fn native_access_controller_do_privileged_action(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    invoke_privileged_action(args, heap, out, ops, false)
}
/// Native: `AccessController.doPrivileged(PrivilegedExceptionAction)Object`.
pub(crate) fn native_access_controller_do_privileged_exception_action(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    invoke_privileged_action(args, heap, out, ops, true)
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
    let provider_name = heap
        .get(provider_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
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
    let start = usize::try_from(offset).map_err(|_| Error::ArrayIndexOutOfBounds {
        index: offset,
        length: input.len(),
    })?;
    let count = usize::try_from(len).map_err(|_| Error::ArrayIndexOutOfBounds {
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
/// Native: `Provider.getName()String`.
pub(crate) fn native_provider_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = extract_ref_arg(args, 0)?;
    let name = heap
        .get(provider_ref)?
        .string_value
        .clone()
        .unwrap_or_else(|| DUKE_SECURITY_PROVIDER.to_string());
    let name_ref = heap.allocate_string(name);
    Ok(Some(Slot::Reference(Some(name_ref))))
}
/// Native: `Provider.getService(String,String)Provider.Service`.
pub(crate) fn native_provider_get_service(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = extract_ref_arg(args, 0)?;
    let type_ref = extract_ref_arg(args, 1)?;
    let algorithm_ref = extract_ref_arg(args, 2)?;
    let provider_name = heap
        .get(provider_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let service_type = string_value_from_ref(heap, type_ref)?;
    let algorithm = string_value_from_ref(heap, algorithm_ref)?;
    let Some(canonical) = canonical_digest_algorithm(&algorithm) else {
        return Ok(Some(Slot::Reference(None)));
    };
    if !is_duke_provider_name(&provider_name) || !is_message_digest_service_type(&service_type) {
        return Ok(Some(Slot::Reference(None)));
    }
    let service_ref =
        allocate_provider_service(heap, provider_ref, MESSAGE_DIGEST_SERVICE_TYPE, canonical);
    Ok(Some(Slot::Reference(Some(service_ref))))
}
/// Native: `Provider.Service.getAlgorithm()String`.
pub(crate) fn native_provider_service_get_algorithm(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let service_ref = extract_ref_arg(args, 0)?;
    Ok(Some(extract_field_arg(
        heap,
        service_ref,
        PROVIDER_SERVICE_ALGORITHM_FIELD,
    )?))
}
/// Native: `Provider.Service.getType()String`.
pub(crate) fn native_provider_service_get_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let service_ref = extract_ref_arg(args, 0)?;
    Ok(Some(extract_field_arg(
        heap,
        service_ref,
        PROVIDER_SERVICE_TYPE_FIELD,
    )?))
}
/// Native: `Provider.Service.getProvider()Provider`.
pub(crate) fn native_provider_service_get_provider(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let service_ref = extract_ref_arg(args, 0)?;
    Ok(Some(extract_field_arg(
        heap,
        service_ref,
        PROVIDER_SERVICE_PROVIDER_FIELD,
    )?))
}
/// Native: `Security.getProvider(String)Provider`.
pub(crate) fn native_security_get_provider(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let name = string_value_from_ref(heap, name_ref)?;
    if !is_duke_provider_name(&name) {
        return Ok(Some(Slot::Reference(None)));
    }
    let provider_ref = allocate_duke_provider(heap);
    Ok(Some(Slot::Reference(Some(provider_ref))))
}
/// Native: `Security.getProviders()Provider[]`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_security_get_providers(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = allocate_duke_provider(heap);
    let provider_array_ref = heap.allocate("[Ljava/security/Provider;".to_string(), 1);
    if let Ok(providers) = heap.get_mut(provider_array_ref) {
        providers.fields[0] = Slot::Reference(Some(provider_ref));
    }
    Ok(Some(Slot::Reference(Some(provider_array_ref))))
}
/// Native: `Security.getAlgorithms(String)Set`.
pub(crate) fn native_security_get_algorithms(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let service_type_ref = extract_ref_arg(args, 0)?;
    let service_type = string_value_from_ref(heap, service_type_ref)?;
    let algorithms = if is_message_digest_service_type(&service_type) {
        &MESSAGE_DIGEST_ALGORITHMS[..]
    } else {
        &[][..]
    };
    let set_ref = allocate_algorithm_set(heap, out, control, algorithms)?;
    Ok(Some(Slot::Reference(Some(set_ref))))
}
/// Native: `SecureRandom.nextBytes([B)V` — fills target array using host CSPRNG.
pub(crate) fn native_secure_random_next_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let array_ref = extract_ref_arg(args, 1)?;
    let mut bytes = vec![0_u8; heap.get(array_ref)?.fields.len()];
    getrandom::fill(&mut bytes).map_err(|_| Error::JavaException {
        class_name: "java/lang/InternalError".to_string(),
    })?;
    let arr = heap.get_mut(array_ref)?;
    for (idx, byte) in bytes.iter().copied().enumerate() {
        arr.fields[idx] = Slot::Int(i32::from(byte));
    }
    Ok(None)
}
/// Native: `SecureRandom.generateSeed(I)[B` — returns a fresh random byte array.
pub(crate) fn native_secure_random_generate_seed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let len = usize::try_from(extract_int_arg(args, 1)?.max(0)).unwrap_or(0);
    let mut bytes = vec![0_u8; len];
    getrandom::fill(&mut bytes).map_err(|_| Error::JavaException {
        class_name: "java/lang/InternalError".to_string(),
    })?;
    let out_ref = alloc_byte_array(heap, &bytes);
    Ok(Some(Slot::Reference(Some(out_ref))))
}