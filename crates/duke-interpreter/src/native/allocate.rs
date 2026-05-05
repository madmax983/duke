pub(crate) fn allocate_standard_charset(
    heap: &mut duke_gc::Heap,
    canonical_name: &str,
) -> u64 {
    if let Some(existing) = heap
        .find_string_backed_object("java/nio/charset/Charset", canonical_name)
    {
        return existing;
    }
    let charset_ref = heap.allocate("java/nio/charset/Charset".to_string(), 0);
    if let Ok(obj) = heap.get_mut(charset_ref) {
        obj.string_value = Some(canonical_name.to_string());
    }
    charset_ref
}
fn allocate_byte_array(heap: &mut duke_gc::Heap, bytes: &[u8]) -> Result<u64> {
    let array_ref = heap.allocate("[B".to_string(), bytes.len());
    for (idx, byte) in bytes.iter().copied().enumerate() {
        heap.write_field(array_ref, idx, java_byte_slot(byte))?;
    }
    Ok(array_ref)
}
fn allocate_manifest_from_bytes(heap: &mut duke_gc::Heap, bytes: &[u8]) -> Result<u64> {
    let raw_ref = heap.allocate_string(String::from_utf8_lossy(bytes).into_owned());
    let manifest_ref = heap.allocate("java/util/jar/Manifest".to_string(), 1);
    heap.get_mut(manifest_ref)?.fields[0] = Slot::Reference(Some(raw_ref));
    Ok(manifest_ref)
}
fn allocate_slot_array(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    elements: &[Slot],
) -> Result<u64> {
    let array_ref = heap.allocate(class_name.to_string(), elements.len());
    heap.get_mut(array_ref)?.fields.clone_from_slice(elements);
    Ok(array_ref)
}
fn allocate_empty_reference_array(
    heap: &mut duke_gc::Heap,
    class_name: &str,
) -> Result<u64> {
    allocate_slot_array(heap, class_name, &[])
}
fn allocate_stack_trace_element(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    method_name: &str,
    file_name: Option<&str>,
    line_number: i32,
) -> Result<u64> {
    let element_ref = heap.allocate(STACK_TRACE_ELEMENT_CLASS.to_string(), 4);
    let declaring_class = class_name.replace('/', ".");
    let class_slot = string_slot(heap, &declaring_class);
    let method_slot = string_slot(heap, method_name);
    let file_slot = optional_string_slot(heap, file_name);
    let obj = heap.get_mut(element_ref)?;
    obj.fields[0] = class_slot;
    obj.fields[1] = method_slot;
    obj.fields[2] = file_slot;
    obj.fields[3] = Slot::Int(line_number);
    Ok(element_ref)
}
fn allocate_resource_url(heap: &mut duke_gc::Heap, url: String) -> Result<u64> {
    allocate_string_backed_object(heap, "java/net/URL", url)
}
fn allocate_resource_enumeration(
    heap: &mut duke_gc::Heap,
    resources: Vec<duke_loader::LocatedResource>,
) -> Result<u64> {
    let enum_ref = heap
        .allocate(
            "duke/util/ResourceEnumeration".to_string(),
            RESOURCE_ENUM_VALUES_START + resources.len(),
        );
    heap.write_field(enum_ref, RESOURCE_ENUM_INDEX_FIELD, Slot::Int(0))?;
    heap.write_field(
        enum_ref,
        RESOURCE_ENUM_COUNT_FIELD,
        Slot::Int(i32::try_from(resources.len()).unwrap_or(i32::MAX)),
    )?;
    for (idx, resource) in resources.into_iter().enumerate() {
        let url_ref = allocate_resource_url(heap, resource.url)?;
        heap.write_field(
            enum_ref,
            RESOURCE_ENUM_VALUES_START + idx,
            Slot::Reference(Some(url_ref)),
        )?;
    }
    Ok(enum_ref)
}
fn allocate_string_backed_object(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    value: String,
) -> Result<u64> {
    let obj_ref = heap.allocate(class_name.to_string(), 1);
    let value_ref = heap.allocate_string(value);
    heap.get_mut(obj_ref)?.fields[0] = Slot::Reference(Some(value_ref));
    Ok(obj_ref)
}
fn allocate_resource_input_stream(
    heap: &mut duke_gc::Heap,
    bytes: Vec<u8>,
) -> Result<u64> {
    let byte_array_ref = heap.allocate("[B".to_string(), bytes.len());
    {
        let array = heap.get_mut(byte_array_ref)?;
        for (idx, byte) in bytes.into_iter().enumerate() {
            array.fields[idx] = Slot::Int(i32::from(byte));
        }
    }
    let stream_ref = heap.allocate("duke/io/ResourceInputStream".to_string(), 3);
    let stream = heap.get_mut(stream_ref)?;
    stream.fields[RESOURCE_STREAM_BYTES_FIELD] = Slot::Reference(Some(byte_array_ref));
    stream.fields[RESOURCE_STREAM_CURSOR_FIELD] = Slot::Int(0);
    stream.fields[RESOURCE_STREAM_CLOSED_FIELD] = Slot::Int(0);
    Ok(stream_ref)
}
fn allocate_boot_archive_entry(
    heap: &mut duke_gc::Heap,
    entry_name: &str,
    is_directory: bool,
) -> Result<u64> {
    let entry_ref = heap.allocate("duke/boot/ArchiveEntry".to_string(), 2);
    let name_ref = heap.allocate_string(entry_name.to_string());
    let entry = heap.get_mut(entry_ref)?;
    entry.fields[BOOT_ARCHIVE_ENTRY_NAME_SLOT] = Slot::Reference(Some(name_ref));
    entry.fields[BOOT_ARCHIVE_ENTRY_DIRECTORY_SLOT] = Slot::Int(i32::from(is_directory));
    Ok(entry_ref)
}
fn allocate_annotation_proxy(
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    annotation: &ReflectedAnnotation,
) -> Result<u64> {
    let element_values = annotation_element_values(annotation, ops)?;
    let type_ref = allocate_class_object(heap, &annotation.type_name)?;
    let mut fields = Vec::with_capacity(1 + element_values.len() * 3);
    fields.push(Slot::Reference(Some(type_ref)));
    for (name, descriptor, value) in element_values {
        let name_ref = heap.allocate_string(name);
        let descriptor_ref = heap.allocate_string(descriptor.clone());
        let value_slot = materialize_annotation_value(
            heap,
            output,
            ops,
            &descriptor,
            &value,
        )?;
        fields.push(Slot::Reference(Some(name_ref)));
        fields.push(Slot::Reference(Some(descriptor_ref)));
        fields.push(value_slot);
    }
    let proxy_ref = heap
        .allocate(
            format!("{ANNOTATION_PROXY_PREFIX}{}", annotation.type_name),
            fields.len(),
        );
    let proxy = heap.get_mut(proxy_ref)?;
    proxy.string_value = Some(annotation.type_name.clone());
    proxy.fields = fields;
    Ok(proxy_ref)
}
fn allocate_annotation_array(
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    annotations: &[ReflectedAnnotation],
) -> Result<Option<Slot>> {
    let annotation_refs = annotations
        .iter()
        .map(|annotation| allocate_annotation_proxy(heap, output, ops, annotation))
        .collect::<Result<Vec<_>>>()?;
    let array_ref = allocate_reference_array(
        heap,
        "[Ljava/lang/annotation/Annotation;",
        &annotation_refs,
    )?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}
fn allocate_executor(heap: &mut duke_gc::Heap, max_workers: usize) -> Result<Slot> {
    let executor_ref = heap
        .allocate("duke/util/concurrent/DukeExecutorService".to_string(), 2);
    {
        let executor = heap.get_mut(executor_ref)?;
        executor.fields[EXECUTOR_SHUTDOWN_FIELD] = Slot::Int(0);
        executor.fields[EXECUTOR_AWAIT_DEADLINE_FIELD] = Slot::Long(0);
        executor.atomic_payload = Some(duke_gc::AtomicPayload::executor(max_workers));
    }
    Ok(Slot::Reference(Some(executor_ref)))
}
fn allocate_read_write_view(
    heap: &mut duke_gc::Heap,
    state: std::sync::Arc<std::sync::Mutex<duke_gc::ReadWriteLockState>>,
    class_name: &str,
    kind: duke_gc::ReadWriteLockViewKind,
) -> Result<Slot> {
    let view_ref = heap.allocate(class_name.to_string(), 0);
    heap.get_mut(view_ref)?.atomic_payload = Some(
        duke_gc::AtomicPayload::read_write_lock_view(state, kind),
    );
    Ok(Slot::Reference(Some(view_ref)))
}
fn allocate_class_object(heap: &mut duke_gc::Heap, class_key: &str) -> Result<u64> {
    if let Some(class_ref) = heap.find_string_backed_object("java/lang/Class", class_key)
    {
        return Ok(class_ref);
    }
    let class_ref = heap.allocate("java/lang/Class".to_string(), 0);
    heap.get_mut(class_ref)?.string_value = Some(class_key.to_string());
    Ok(class_ref)
}
fn allocate_reference_array(
    heap: &mut duke_gc::Heap,
    array_class_name: &str,
    elements: &[u64],
) -> Result<u64> {
    let array_ref = heap.allocate(array_class_name.to_string(), elements.len());
    {
        let array_obj = heap.get_mut(array_ref)?;
        for slot in &mut array_obj.fields {
            *slot = Slot::Reference(None);
        }
    }
    for (idx, element_ref) in elements.iter().enumerate() {
        heap.write_field(array_ref, idx, Slot::Reference(Some(*element_ref)))?;
    }
    Ok(array_ref)
}
fn allocate_reflection_member_object(
    heap: &mut duke_gc::Heap,
    member_class_name: &str,
    declaring_internal_name: &str,
    name: &str,
    descriptor: &str,
    is_public: bool,
    is_static: bool,
) -> Result<u64> {
    let member_ref = heap.allocate(member_class_name.to_string(), 6);
    let declaring_class_ref = allocate_class_object(heap, declaring_internal_name)?;
    let name_ref = heap.allocate_string(name.to_string());
    let descriptor_ref = heap.allocate_string(descriptor.to_string());
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_DECLARING_CLASS_FIELD,
        Slot::Reference(Some(declaring_class_ref)),
    )?;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_NAME_FIELD,
        Slot::Reference(Some(name_ref)),
    )?;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_DESCRIPTOR_FIELD,
        Slot::Reference(Some(descriptor_ref)),
    )?;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_PUBLIC_FIELD,
        Slot::Int(i32::from(is_public)),
    )?;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_STATIC_FIELD,
        Slot::Int(i32::from(is_static)),
    )?;
    heap.write_field(member_ref, REFLECTION_MEMBER_ACCESSIBLE_FIELD, Slot::Int(0))?;
    Ok(member_ref)
}
fn allocate_reflection_instance(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    class: &str,
) -> Result<u64> {
    if !registry.contains(class) {
        registry.ensure_loaded(class, loader)?;
    }
    let class_key = registry.resolve_loaded_class_key(class)?;
    ensure_initialized(registry, loader, heap, output, &class_key, &class_key)?;
    let object_ref = heap
        .allocate(class_key.clone(), total_instance_field_count(registry, &class_key));
    init_object_fields(registry, heap, object_ref, &class_key);
    Ok(object_ref)
}
fn allocate_reference_array_from_slots(
    heap: &mut duke_gc::Heap,
    array_class_name: &str,
    elements: &[Slot],
) -> Result<u64> {
    let array_ref = heap.allocate(array_class_name.to_string(), elements.len());
    let array = heap.get_mut(array_ref)?;
    for slot in &mut array.fields {
        *slot = Slot::Reference(None);
    }
    for (idx, element) in elements.iter().enumerate() {
        array.fields[idx] = *element;
    }
    Ok(array_ref)
}
fn allocate_service_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    ops: &mut dyn CallbackOps,
    loader_arg_index: Option<usize>,
) -> Result<Option<Slot>> {
    let service_class_ref = extract_ref_arg(args, 0)?;
    let service_internal_name = class_internal_name_from_ref(heap, service_class_ref)?;
    let service_binary_name = internal_name_to_binary_name(&service_internal_name);
    let loader_slot = loader_arg_index
        .and_then(|idx| args.get(idx).copied())
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(loader_ref) = loader_slot else {
        return Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        });
    };
    let files = service_configuration_files_via_resources(
        heap,
        ops,
        loader_ref,
        &service_binary_name,
    )?;
    let provider_names = parse_service_provider_names(files)?;
    let loader_ref = heap
        .allocate(
            "java/util/ServiceLoader".to_string(),
            SERVICE_LOADER_PROVIDERS_START + provider_names.len(),
        );
    {
        let service_loader = heap.get_mut(loader_ref)?;
        service_loader.fields[SERVICE_LOADER_SERVICE_CLASS_FIELD] = Slot::Reference(
            Some(service_class_ref),
        );
        service_loader.fields[SERVICE_LOADER_LOADER_FIELD] = loader_slot;
        service_loader.fields[SERVICE_LOADER_COUNT_FIELD] = Slot::Int(
            i32::try_from(provider_names.len()).unwrap_or(i32::MAX),
        );
    }
    for (idx, provider_name) in provider_names.into_iter().enumerate() {
        let name_ref = heap.allocate_string(provider_name);
        heap.write_field(
            loader_ref,
            SERVICE_LOADER_PROVIDERS_START + idx,
            Slot::Reference(Some(name_ref)),
        )?;
    }
    Ok(Some(Slot::Reference(Some(loader_ref))))
}
fn allocate_base64_coder(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    variant: Base64Variant,
) -> Result<u64> {
    let coder_ref = heap.allocate(class_name.to_string(), 1);
    heap.get_mut(coder_ref)?.fields[0] = Slot::Int(variant.field_value());
    Ok(coder_ref)
}
fn allocate_duke_provider(heap: &mut duke_gc::Heap) -> u64 {
    let provider_ref = heap.allocate("java/security/Provider".to_string(), 0);
    if let Ok(provider) = heap.get_mut(provider_ref) {
        provider.string_value = Some(DUKE_SECURITY_PROVIDER.to_string());
    }
    provider_ref
}
fn allocate_provider_service(
    heap: &mut duke_gc::Heap,
    provider_ref: u64,
    service_type: &str,
    algorithm: &str,
) -> u64 {
    let type_ref = heap.allocate_string(service_type.to_string());
    let algorithm_ref = heap.allocate_string(algorithm.to_string());
    let service_ref = heap.allocate("java/security/Provider$Service".to_string(), 3);
    if let Ok(service) = heap.get_mut(service_ref) {
        service.fields[PROVIDER_SERVICE_PROVIDER_FIELD] = Slot::Reference(
            Some(provider_ref),
        );
        service.fields[PROVIDER_SERVICE_TYPE_FIELD] = Slot::Reference(Some(type_ref));
        service.fields[PROVIDER_SERVICE_ALGORITHM_FIELD] = Slot::Reference(
            Some(algorithm_ref),
        );
        service.string_value = Some(format!("{service_type}:{algorithm}"));
    }
    service_ref
}
fn allocate_algorithm_set(
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    algorithms: &[&str],
) -> Result<u64> {
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    for algorithm in algorithms {
        let algorithm_ref = heap.allocate_string((*algorithm).to_string());
        native_hashset_add(
            &[Slot::Reference(Some(set_ref)), Slot::Reference(Some(algorithm_ref))],
            heap,
            out,
            control,
        )?;
    }
    Ok(set_ref)
}
fn allocate_uuid(heap: &mut duke_gc::Heap, msb: i64, lsb: i64) -> Result<u64> {
    let uuid_ref = heap.allocate("java/util/UUID".to_string(), 2);
    write_uuid_bits(heap, uuid_ref, msb, lsb)?;
    Ok(uuid_ref)
}
fn allocate_pattern(
    heap: &mut duke_gc::Heap,
    pattern_str: String,
    flags: i32,
) -> Result<u64> {
    let pat_ref = heap.allocate("java/util/regex/Pattern".to_string(), 1);
    let pat = heap.get_mut(pat_ref)?;
    pat.fields[PATTERN_FLAGS_FIELD] = Slot::Int(flags);
    pat.string_value = Some(pattern_str);
    Ok(pat_ref)
}
fn allocate_process_impl(
    heap: &mut duke_gc::Heap,
    ids: duke_gc::SpawnedProcessIds,
) -> Result<Option<Slot>> {
    let process_ref = heap.allocate("java/lang/ProcessImpl".to_string(), 4);
    let process_obj = heap.get_mut(process_ref)?;
    process_obj.fields[PROCESS_ID_FIELD] = Slot::Int(ids.process_id);
    process_obj.fields[PROCESS_STDIN_FIELD] = Slot::Int(ids.stdin_id);
    process_obj.fields[PROCESS_STDOUT_FIELD] = Slot::Int(ids.stdout_id);
    process_obj.fields[PROCESS_STDERR_FIELD] = Slot::Int(ids.stderr_id);
    Ok(Some(Slot::Reference(Some(process_ref))))
}
fn allocate_process_stream(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    handle_id: i32,
) -> Result<Option<Slot>> {
    let stream_ref = heap.allocate(class_name.to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(handle_id);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
fn allocate_duration_from_total_nanos(
    heap: &mut duke_gc::Heap,
    total_nanos: i128,
) -> Result<u64> {
    let (seconds, nanos) = normalize_seconds_nanos(total_nanos);
    let duration_ref = heap.allocate("java/time/Duration".to_string(), 2);
    heap.get_mut(duration_ref)?.fields[0] = Slot::Long(seconds);
    heap.get_mut(duration_ref)?.fields[1] = Slot::Int(nanos);
    Ok(duration_ref)
}
fn allocate_instant_from_total_nanos(
    heap: &mut duke_gc::Heap,
    total_nanos: i128,
) -> Result<u64> {
    let (seconds, nanos) = normalize_seconds_nanos(total_nanos);
    let instant_ref = heap.allocate("java/time/Instant".to_string(), 2);
    heap.get_mut(instant_ref)?.fields[0] = Slot::Long(seconds);
    heap.get_mut(instant_ref)?.fields[1] = Slot::Int(nanos);
    Ok(instant_ref)
}
fn allocate_localdate(heap: &mut duke_gc::Heap, epoch_day: i32) -> Result<u64> {
    let localdate_ref = heap.allocate("java/time/LocalDate".to_string(), 1);
    heap.get_mut(localdate_ref)?.fields[0] = Slot::Int(epoch_day);
    Ok(localdate_ref)
}
fn allocate_localdatetime(
    heap: &mut duke_gc::Heap,
    epoch_day: i32,
    hour: i32,
    minute: i32,
    second: i32,
    nanos: i32,
) -> Result<u64> {
    let localdatetime_ref = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    heap.get_mut(localdatetime_ref)?.fields[0] = Slot::Int(epoch_day);
    heap.get_mut(localdatetime_ref)?.fields[1] = Slot::Int(hour);
    heap.get_mut(localdatetime_ref)?.fields[2] = Slot::Int(minute);
    heap.get_mut(localdatetime_ref)?.fields[3] = Slot::Int(second);
    heap.get_mut(localdatetime_ref)?.fields[4] = Slot::Int(nanos);
    Ok(localdatetime_ref)
}
