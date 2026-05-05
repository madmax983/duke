fn url_class_loader_paths(
    registry: &ClassRegistry,
    heap: &duke_gc::Heap,
    loader_ref: u64,
) -> Result<Vec<String>> {
    let loader_class = heap.get(loader_ref)?.class_name.clone();
    if !class_extends(registry, &loader_class, "java/net/URLClassLoader") {
        return Ok(Vec::new());
    }
    let Ok(ucp_slot) = field_slot_idx(registry, &loader_class, "ucp") else {
        return Ok(Vec::new());
    };
    let Some(ucp_ref) = archive_ref_from_slot(heap, loader_ref, ucp_slot)? else {
        return Ok(Vec::new());
    };
    let ucp_class = heap.get(ucp_ref)?.class_name.clone();
    let Ok(path_slot) = field_slot_idx(registry, &ucp_class, "path") else {
        return Ok(Vec::new());
    };
    let Some(path_list_ref) = archive_ref_from_slot(heap, ucp_ref, path_slot)? else {
        return Ok(Vec::new());
    };
    let mut paths = Vec::new();
    for url_ref in arraylist_reference_elements(heap, path_list_ref)? {
        if let Ok(spec) = string_backed_object_value(heap, url_ref)
            && let Some(path) = class_path_from_url_spec(&spec)
        {
            paths.push(path);
        }
    }
    Ok(paths)
}
fn url_path_string(spec: &str) -> String {
    if let Some(path) = spec.strip_prefix("jar:") {
        return path.to_string();
    }
    if let Some(path) = spec.strip_prefix("file://") {
        return percent_decode(path);
    }
    spec.to_string()
}
pub(crate) fn native_url_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let spec_slot = extract_slot_arg(args, 1);
    match spec_slot {
        Slot::Reference(Some(_)) => {
            heap.get_mut(this_ref)?.fields[0] = spec_slot;
            Ok(None)
        }
        Slot::Reference(None) => Err(Error::NullPointerException),
        _ => {
            Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            })
        }
    }
}
pub(crate) fn native_url_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first().copied() {
        Some(slot @ Slot::Reference(Some(_))) => Ok(Some(slot)),
        Some(Slot::Reference(None)) => Err(Error::NullPointerException),
        _ => {
            Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            })
        }
    }
}
pub(crate) fn native_url_to_external_form(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_url_to_string(args, heap, out, control)
}
pub(crate) fn native_url_to_uri(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let spec = string_backed_object_value(heap, this_ref)?;
    let uri_ref = allocate_string_backed_object(heap, "java/net/URI", spec)?;
    Ok(Some(Slot::Reference(Some(uri_ref))))
}
pub(crate) fn native_url_get_path(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let spec = string_backed_object_value(heap, this_ref)?;
    let path_ref = heap.allocate_string(url_path_string(&spec));
    Ok(Some(Slot::Reference(Some(path_ref))))
}
pub(crate) fn native_url_open_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let spec = string_backed_object_value(heap, this_ref)?;
    let bytes = read_resource_bytes_from_url_spec(&spec)?;
    let stream_ref = allocate_resource_input_stream(heap, bytes)?;
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
pub(crate) fn native_url_class_loader_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let urls_ref = extract_ref_arg(args, 1)?;
    let url_slots = heap.get(urls_ref)?.fields.clone();
    let path_list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(path_list_ref))], heap, out, control)?;
    for url_slot in url_slots {
        match url_slot {
            Slot::Reference(Some(entry_ref)) => {
                native_arraylist_add(
                    &[
                        Slot::Reference(Some(path_list_ref)),
                        Slot::Reference(Some(entry_ref)),
                    ],
                    heap,
                    out,
                    control,
                )?;
            }
            Slot::Reference(None) => return Err(Error::NullPointerException),
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "Reference",
                    got: "other",
                });
            }
        }
    }
    let ucp_ref = heap.allocate("jdk/internal/loader/URLClassPath".to_string(), 1);
    heap.get_mut(ucp_ref)?.fields[0] = Slot::Reference(Some(path_list_ref));
    heap.get_mut(this_ref)?.fields[0] = Slot::Reference(Some(ucp_ref));
    Ok(None)
}
pub(crate) fn native_url_class_loader_load_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let binary_name = string_value_from_ref(heap, name_ref)?;
    let internal_name = binary_name_to_internal_name(&binary_name);
    if let Some(class_key) = ops.ensure_parent_loaded(&internal_name)? {
        let class_ref = allocate_class_object(heap, &class_key)?;
        return Ok(Some(Slot::Reference(Some(class_ref))));
    }
    match ops.ensure_loaded_with_runtime_loader(heap, this_ref, &internal_name) {
        Ok(()) => {
            let class_key = ops
                .class_key_for_runtime_loader(heap, this_ref, &internal_name)?;
            let class_ref = allocate_class_object(heap, &class_key)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        Err(Error::ClassNotFound { .. }) => {
            Err(Error::JavaException {
                class_name: "java/lang/ClassNotFoundException".to_string(),
            })
        }
        Err(err) => Err(err),
    }
}
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_url_set_url_stream_handler_factory(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}
