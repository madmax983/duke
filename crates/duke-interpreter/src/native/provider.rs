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
    let provider_name = heap.get(provider_ref)?.string_value.clone().unwrap_or_default();
    let service_type = string_value_from_ref(heap, type_ref)?;
    let algorithm = string_value_from_ref(heap, algorithm_ref)?;
    let Some(canonical) = canonical_digest_algorithm(&algorithm) else {
        return Ok(Some(Slot::Reference(None)));
    };
    if !is_duke_provider_name(&provider_name)
        || !is_message_digest_service_type(&service_type)
    {
        return Ok(Some(Slot::Reference(None)));
    }
    let service_ref = allocate_provider_service(
        heap,
        provider_ref,
        MESSAGE_DIGEST_SERVICE_TYPE,
        canonical,
    );
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
    Ok(Some(extract_field_arg(heap, service_ref, PROVIDER_SERVICE_ALGORITHM_FIELD)?))
}
/// Native: `Provider.Service.getType()String`.
pub(crate) fn native_provider_service_get_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let service_ref = extract_ref_arg(args, 0)?;
    Ok(Some(extract_field_arg(heap, service_ref, PROVIDER_SERVICE_TYPE_FIELD)?))
}
/// Native: `Provider.Service.getProvider()Provider`.
pub(crate) fn native_provider_service_get_provider(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let service_ref = extract_ref_arg(args, 0)?;
    Ok(Some(extract_field_arg(heap, service_ref, PROVIDER_SERVICE_PROVIDER_FIELD)?))
}
