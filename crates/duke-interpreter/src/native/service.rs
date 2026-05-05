fn service_configuration_files_via_resources(
    heap: &mut duke_gc::Heap,
    ops: &mut dyn CallbackOps,
    loader_ref: Option<u64>,
    service_binary_name: &str,
) -> Result<Vec<Vec<u8>>> {
    let resource_name = format!("META-INF/services/{service_binary_name}");
    let enum_ref = if let Some(loader_ref) = loader_ref {
        let resource_name_ref = heap.allocate_string(resource_name);
        let enum_slot = native_class_loader_get_resources(
                &[
                    Slot::Reference(Some(loader_ref)),
                    Slot::Reference(Some(resource_name_ref)),
                ],
                heap,
                &mut Vec::new(),
                &mut NativeControl::default(),
                ops,
            )?
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(enum_ref)) = enum_slot else {
            return Ok(Vec::new());
        };
        enum_ref
    } else {
        let resources = ops.find_resource_entries(heap, None, &resource_name)?;
        allocate_resource_enumeration(heap, resources)?
    };
    read_resource_enumeration_bytes(enum_ref, heap)
}
fn service_loader_cause_type(err: &Error) -> &'static str {
    match err {
        Error::ClassNotFound { .. } => "java.lang.ClassNotFoundException",
        Error::JavaException { class_name } => {
            match class_name.as_str() {
                "java/lang/ClassNotFoundException" => "java.lang.ClassNotFoundException",
                "java/lang/IllegalAccessException" => "java.lang.IllegalAccessException",
                "java/lang/InstantiationException" => "java.lang.InstantiationException",
                "java/lang/NoSuchMethodException" => "java.lang.NoSuchMethodException",
                _ => "java.lang.RuntimeException",
            }
        }
        Error::InstantiationError { .. } => "java.lang.InstantiationException",
        Error::NullPointerException => "java.lang.NullPointerException",
        _ => "duke_runtime.Error",
    }
}
fn service_configuration_error(provider_name: &str, cause_type: &str) -> Error {
    push_pending_java_exception_message(
        "java/util/ServiceConfigurationError",
        format!("{provider_name}: provider construction failed due to {cause_type}"),
    );
    Error::JavaException {
        class_name: "java/util/ServiceConfigurationError".to_string(),
    }
}
pub(crate) fn native_service_loader_load(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    allocate_service_loader(args, heap, ops, None)
}
pub(crate) fn native_service_loader_load_with_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    allocate_service_loader(args, heap, ops, Some(1))
}
pub(crate) fn native_service_loader_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (service_class_slot, loader_slot, provider_slots) = {
        let service_loader = heap.get(this_ref)?;
        let count = match service_loader.fields.get(SERVICE_LOADER_COUNT_FIELD) {
            Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
            _ => 0,
        };
        let service_class_slot = service_loader
            .fields[SERVICE_LOADER_SERVICE_CLASS_FIELD];
        let loader_slot = service_loader.fields[SERVICE_LOADER_LOADER_FIELD];
        let provider_slots: Vec<Slot> = service_loader
            .fields
            .iter()
            .skip(SERVICE_LOADER_PROVIDERS_START)
            .take(count)
            .copied()
            .collect();
        (service_class_slot, loader_slot, provider_slots)
    };
    let iter_ref = heap
        .allocate(
            "duke/util/ServiceLoaderIterator".to_string(),
            SERVICE_ITER_PROVIDERS_START + provider_slots.len(),
        );
    {
        let iter = heap.get_mut(iter_ref)?;
        iter.fields[SERVICE_ITER_SERVICE_CLASS_FIELD] = service_class_slot;
        iter.fields[SERVICE_ITER_LOADER_FIELD] = loader_slot;
        iter.fields[SERVICE_ITER_INDEX_FIELD] = Slot::Int(0);
        iter.fields[SERVICE_ITER_COUNT_FIELD] = Slot::Int(
            i32::try_from(provider_slots.len()).unwrap_or(i32::MAX),
        );
    }
    for (idx, slot) in provider_slots.into_iter().enumerate() {
        heap.write_field(iter_ref, SERVICE_ITER_PROVIDERS_START + idx, slot)?;
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
}
fn service_iterator_index_and_count(
    heap: &duke_gc::Heap,
    iter_ref: u64,
) -> Result<(usize, usize)> {
    let iter = heap.get(iter_ref)?;
    let index = match iter.fields.get(SERVICE_ITER_INDEX_FIELD) {
        Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
        _ => 0,
    };
    let count = match iter.fields.get(SERVICE_ITER_COUNT_FIELD) {
        Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
        _ => 0,
    };
    Ok((index, count))
}
pub(crate) fn native_service_loader_iter_init(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(None)
}
pub(crate) fn native_service_loader_iter_has_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let iter_ref = extract_ref_arg(args, 0)?;
    let (index, count) = service_iterator_index_and_count(heap, iter_ref)?;
    Ok(Some(Slot::Int(i32::from(index < count))))
}
pub(crate) fn native_service_loader_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let iter_ref = extract_ref_arg(args, 0)?;
    let provider_ref = instantiate_service_provider(heap, output, ops, iter_ref)?;
    Ok(Some(Slot::Reference(Some(provider_ref))))
}
pub(crate) fn native_service_loader_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let Some(Slot::Reference(Some(iter_ref))) = native_service_loader_iterator(
        args,
        heap,
        output,
        control,
    )? else {
        return Ok(Some(Slot::Reference(None)));
    };
    let mut providers = Vec::new();
    loop {
        let (index, count) = service_iterator_index_and_count(heap, iter_ref)?;
        if index >= count {
            break;
        }
        let provider_ref = instantiate_service_provider(heap, output, ops, iter_ref)?;
        providers.push(provider_ref);
    }
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(
        i32::try_from(providers.len()).unwrap_or(0),
    );
    for provider_ref in providers {
        heap.get_mut(stream_ref)?.fields.push(Slot::Reference(Some(provider_ref)));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
