fn runtime_loader_paths(
    registry: &ClassRegistry,
    heap: &duke_gc::Heap,
    loader_ref: u64,
) -> Result<Vec<String>> {
    if let Some(path) = launched_class_loader_archive_path(registry, heap, loader_ref)? {
        return Ok(vec![path]);
    }
    url_class_loader_paths(registry, heap, loader_ref)
}
fn runtime_visible_annotations_from_attrs(
    cp: &[Option<CpEntry>],
    attrs: &[duke_classfile::types::AttributeInfo],
) -> Vec<ReflectedAnnotation> {
    attrs
        .iter()
        .find_map(|attr| {
            if let duke_classfile::types::AttributeData::RuntimeVisibleAnnotations(
                annotations,
            ) = &attr.data
            {
                Some(
                    annotations
                        .iter()
                        .filter_map(|annotation| resolve_annotation(cp, annotation))
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default()
}
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_runtime_get_runtime(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let runtime_ref = heap.allocate("java/lang/Runtime".to_string(), 0);
    Ok(Some(Slot::Reference(Some(runtime_ref))))
}
pub(crate) fn native_runtime_exec_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(_))) => {}
        _ => return Err(Error::NullPointerException),
    }
    let command = string_array_from_slot(extract_slot_arg(args, 1), heap)?;
    spawn_process_impl(heap, &command, None)
}
pub(crate) fn native_runtime_exec_array_dir(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(_))) => {}
        _ => return Err(Error::NullPointerException),
    }
    let command = string_array_from_slot(extract_slot_arg(args, 1), heap)?;
    let cwd = optional_file_path_from_slot(extract_slot_arg(args, 3), heap)?;
    spawn_process_impl(heap, &command, cwd.as_deref())
}
