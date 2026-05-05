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
        let service_class_slot = service_loader.fields[SERVICE_LOADER_SERVICE_CLASS_FIELD];
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

    let iter_ref = heap.allocate(
        "duke/util/ServiceLoaderIterator".to_string(),
        SERVICE_ITER_PROVIDERS_START + provider_slots.len(),
    );
    {
        let iter = heap.get_mut(iter_ref)?;
        iter.fields[SERVICE_ITER_SERVICE_CLASS_FIELD] = service_class_slot;
        iter.fields[SERVICE_ITER_LOADER_FIELD] = loader_slot;
        iter.fields[SERVICE_ITER_INDEX_FIELD] = Slot::Int(0);
        iter.fields[SERVICE_ITER_COUNT_FIELD] =
            Slot::Int(i32::try_from(provider_slots.len()).unwrap_or(i32::MAX));
    }
    for (idx, slot) in provider_slots.into_iter().enumerate() {
        heap.write_field(iter_ref, SERVICE_ITER_PROVIDERS_START + idx, slot)?;
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
}

fn service_iterator_index_and_count(heap: &duke_gc::Heap, iter_ref: u64) -> Result<(usize, usize)> {
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

fn instantiate_service_provider(
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    iter_ref: u64,
) -> Result<u64> {
    let (index, count) = service_iterator_index_and_count(heap, iter_ref)?;
    if index >= count {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }

    let (loader_ref, provider_name_ref) = {
        let iter = heap.get(iter_ref)?;
        let loader_ref = match iter.fields.get(SERVICE_ITER_LOADER_FIELD) {
            Some(Slot::Reference(reference)) => *reference,
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "Reference",
                    got: "other",
                });
            }
        };
        let provider_name_ref = match iter.fields.get(SERVICE_ITER_PROVIDERS_START + index) {
            Some(Slot::Reference(Some(reference))) => *reference,
            _ => return Err(Error::NullPointerException),
        };
        (loader_ref, provider_name_ref)
    };
    let provider_binary_name = string_value_from_ref(heap, provider_name_ref)?;
    let provider_internal_name = binary_name_to_internal_name(&provider_binary_name);
    heap.get_mut(iter_ref)?.fields[SERVICE_ITER_INDEX_FIELD] =
        Slot::Int(i32::try_from(index + 1).unwrap_or(i32::MAX));

    let load_result = if let Some(loader_ref) = loader_ref {
        ops.ensure_loaded_with_runtime_loader(heap, loader_ref, &provider_internal_name)
    } else {
        ops.ensure_loaded(&provider_internal_name)
    };
    if let Err(err) = load_result {
        return Err(service_configuration_error(
            &provider_binary_name,
            service_loader_cause_type(&err),
        ));
    }
    let class_key = if let Some(loader_ref) = loader_ref {
        match ops.class_key_for_runtime_loader(heap, loader_ref, &provider_internal_name) {
            Ok(class_key) => class_key,
            Err(err) => {
                return Err(service_configuration_error(
                    &provider_binary_name,
                    service_loader_cause_type(&err),
                ));
            }
        }
    } else {
        match ops.class_key_for_loaded_class(&provider_internal_name) {
            Ok(class_key) => class_key,
            Err(err) => {
                return Err(service_configuration_error(
                    &provider_binary_name,
                    service_loader_cause_type(&err),
                ));
            }
        }
    };

    let reflected = match ops.inspect_class(&class_key) {
        Ok(reflected) => reflected,
        Err(err) => {
            return Err(service_configuration_error(
                &provider_binary_name,
                service_loader_cause_type(&err),
            ));
        }
    };
    let has_public_no_arg_ctor = reflected.methods.iter().any(|method| {
        method.name == "<init>" && method.descriptor == "()V" && method.is_public
    });
    if !has_public_no_arg_ctor {
        return Err(service_configuration_error(
            &provider_binary_name,
            "java.lang.NoSuchMethodException",
        ));
    }

    let instance_ref = match ops.allocate_instance(heap, output, &class_key) {
        Ok(instance_ref) => instance_ref,
        Err(err) => {
            return Err(service_configuration_error(
                &provider_binary_name,
                service_loader_cause_type(&err),
            ));
        }
    };
    match ops.invoke(
        heap,
        output,
        &class_key,
        "<init>",
        "()V",
        vec![Slot::Reference(Some(instance_ref))],
    ) {
        Ok(_) => Ok(instance_ref),
        Err(err) => Err(service_configuration_error(
            &provider_binary_name,
            service_loader_cause_type(&err),
        )),
    }
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
    let Some(Slot::Reference(Some(iter_ref))) =
        native_service_loader_iterator(args, heap, output, control)?
    else {
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
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(i32::try_from(providers.len()).unwrap_or(0));
    for provider_ref in providers {
        heap.get_mut(stream_ref)?
            .fields
            .push(Slot::Reference(Some(provider_ref)));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

// ---------------------------------------------------------------------------
// ArrayListIterator natives
// ---------------------------------------------------------------------------
