fn boot_archive_path_from_ref(
    registry: &ClassRegistry,
    heap: &duke_gc::Heap,
    archive_ref: u64,
) -> Result<Option<String>> {
    let archive_class = heap.get(archive_ref)?.class_name.clone();
    match archive_class.as_str() {
        "org/springframework/boot/loader/launch/JarFileArchive" => {
            let file_slot = field_slot_idx(registry, &archive_class, "file")?;
            archive_path_from_slot(heap, archive_ref, file_slot)
        }
        "org/springframework/boot/loader/launch/ExplodedArchive" => {
            let root_slot = field_slot_idx(registry, &archive_class, "rootDirectory")?;
            archive_path_from_slot(heap, archive_ref, root_slot)
        }
        _ => Ok(None),
    }
}
pub(crate) fn native_boot_nested_jar_file_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_jar_file_get_manifest(args, heap, out, control)
}
pub(crate) fn native_boot_jar_file_archive_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let jar_file_ref = match heap
        .get(this_ref)?
        .fields
        .get(BOOT_JAR_FILE_ARCHIVE_JAR_FILE_SLOT)
        .copied()
    {
        Some(Slot::Reference(Some(jar_file_ref))) => jar_file_ref,
        Some(Slot::Reference(None)) | None => return Err(Error::NullPointerException),
        Some(_) => {
            return Err(Error::TypeMismatch {
                expected: "reference",
                got: "other",
            });
        }
    };
    native_jar_file_get_manifest(
        &[Slot::Reference(Some(jar_file_ref))],
        heap,
        out,
        control,
    )
}
pub(crate) fn native_boot_archive_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.class_name.as_str() {
        "org/springframework/boot/loader/launch/JarFileArchive" => {
            native_boot_jar_file_archive_get_manifest(args, heap, out, control)
        }
        "org/springframework/boot/loader/launch/ExplodedArchive" => {
            native_boot_exploded_archive_get_manifest(args, heap, out, control)
        }
        _ => {
            Err(Error::MethodNotFound {
                name: format!("{}.getManifest", heap.get(this_ref) ?.class_name),
                descriptor: "()Ljava/util/jar/Manifest;".to_string(),
            })
        }
    }
}
pub(crate) fn native_boot_exploded_archive_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    if let Some(slot @ Slot::Reference(Some(_))) = heap
        .get(this_ref)?
        .fields
        .get(BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT)
        .copied()
    {
        return Ok(Some(slot));
    }
    let root_directory_ref = match heap
        .get(this_ref)?
        .fields
        .get(BOOT_EXPLODED_ARCHIVE_ROOT_DIRECTORY_SLOT)
        .copied()
    {
        Some(Slot::Reference(Some(root_directory_ref))) => root_directory_ref,
        Some(Slot::Reference(None)) | None => return Err(Error::NullPointerException),
        Some(_) => {
            return Err(Error::TypeMismatch {
                expected: "reference",
                got: "other",
            });
        }
    };
    let manifest_path = file_path_from_ref(root_directory_ref, heap)?
        .join("META-INF/MANIFEST.MF");
    let manifest_bytes = match std::fs::read(&manifest_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(Slot::Reference(None)));
        }
        Err(_) => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
        }
    };
    let manifest_ref = allocate_manifest_from_bytes(heap, &manifest_bytes)?;
    heap.get_mut(this_ref)?.fields[BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT] = Slot::Reference(
        Some(manifest_ref),
    );
    Ok(Some(Slot::Reference(Some(manifest_ref))))
}
#[cfg(test)]
fn boot_archive_entry_name(heap: &duke_gc::Heap, entry_ref: u64) -> Result<String> {
    let Some(Slot::Reference(Some(name_ref))) = heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_NAME_SLOT)
        .copied() else {
        return Err(Error::NullPointerException);
    };
    string_value_from_ref(heap, name_ref)
}
fn boot_archive_entry_is_directory_flag(
    heap: &duke_gc::Heap,
    entry_ref: u64,
) -> Result<bool> {
    match heap.get(entry_ref)?.fields.get(BOOT_ARCHIVE_ENTRY_DIRECTORY_SLOT).copied() {
        Some(Slot::Int(value)) => Ok(value != 0),
        _ => {
            Err(Error::TypeMismatch {
                expected: "Int",
                got: "other",
            })
        }
    }
}
pub(crate) fn native_boot_archive_entry_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let entry_ref = extract_ref_arg(args, 0)?;
    let Some(slot @ Slot::Reference(Some(_))) = heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_NAME_SLOT)
        .copied() else {
        return Err(Error::NullPointerException);
    };
    Ok(Some(slot))
}
pub(crate) fn native_boot_archive_entry_is_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let entry_ref = extract_ref_arg(args, 0)?;
    Ok(
        Some(
            Slot::Int(i32::from(boot_archive_entry_is_directory_flag(heap, entry_ref)?)),
        ),
    )
}
fn boot_archive_hashset_add_url(
    set_ref: u64,
    url_spec: String,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<()> {
    let url_ref = allocate_string_backed_object(heap, "java/net/URL", url_spec)?;
    native_hashset_add(
        &[Slot::Reference(Some(set_ref)), Slot::Reference(Some(url_ref))],
        heap,
        out,
        control,
    )?;
    Ok(())
}
fn boot_archive_predicate_accepts(
    predicate_ref: u64,
    entry_name: &str,
    is_directory: bool,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<bool> {
    let predicate_class = heap.get(predicate_ref)?.class_name.clone();
    let entry_ref = allocate_boot_archive_entry(heap, entry_name, is_directory)?;
    match ops
        .invoke(
            heap,
            out,
            &predicate_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(predicate_ref)), Slot::Reference(Some(entry_ref)),],
        )?
    {
        Some(Slot::Int(value)) => Ok(value != 0),
        Some(other) => {
            Err(Error::TypeMismatch {
                expected: "Int",
                got: match other {
                    Slot::Long(_) => "Long",
                    Slot::Float(_) => "Float",
                    Slot::Double(_) => "Double",
                    Slot::Reference(_) => "Reference",
                    Slot::ReturnAddress(_) => "ReturnAddress",
                    Slot::Int(_) => unreachable!(),
                },
            })
        }
        None => Ok(false),
    }
}
pub(crate) fn native_boot_jar_file_archive_get_class_path_urls(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let archive_ref = extract_ref_arg(args, 0)?;
    let include_predicate_ref = extract_ref_arg(args, 1)?;
    let _ = extract_ref_arg(args, 2)?;
    let archive_file_ref = archive_file_ref_at(heap, archive_ref, 0)?;
    let archive_path = file_path_from_ref(archive_file_ref, heap)?;
    let reader = open_boot_archive_reader(&archive_path)?;
    let mut entry_names: Vec<String> = reader
        .entry_names()
        .map(ToOwned::to_owned)
        .collect();
    entry_names.sort_unstable();
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    for entry_name in entry_names {
        let is_directory = entry_name.ends_with('/');
        if boot_archive_predicate_accepts(
            include_predicate_ref,
            &entry_name,
            is_directory,
            heap,
            out,
            ops,
        )? {
            let url_spec = format!(
                "jar:{}!/{}", path_to_file_url(& archive_path), entry_name
            );
            boot_archive_hashset_add_url(set_ref, url_spec, heap, out, control)?;
        }
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}
pub(crate) fn native_boot_exploded_archive_get_class_path_urls(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let archive_ref = extract_ref_arg(args, 0)?;
    let include_predicate_ref = extract_ref_arg(args, 1)?;
    let search_predicate_ref = extract_ref_arg(args, 2)?;
    let root_directory_ref = archive_file_ref_at(heap, archive_ref, 0)?;
    let root_path = file_path_from_ref(root_directory_ref, heap)?;
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    let mut pending: VecDeque<std::path::PathBuf> = list_directory_children_sorted(
            &root_path,
        )?
        .into_iter()
        .collect();
    while let Some(entry_path) = pending.pop_front() {
        let is_directory = entry_path.is_dir();
        let entry_name = exploded_archive_relative_entry_name(
            &root_path,
            &entry_path,
            is_directory,
        );
        if is_directory
            && boot_archive_predicate_accepts(
                search_predicate_ref,
                &entry_name,
                true,
                heap,
                out,
                ops,
            )?
        {
            let children = list_directory_children_sorted(&entry_path)?;
            for child in children.into_iter().rev() {
                pending.push_front(child);
            }
        }
        if boot_archive_predicate_accepts(
            include_predicate_ref,
            &entry_name,
            is_directory,
            heap,
            out,
            ops,
        )? {
            boot_archive_hashset_add_url(
                set_ref,
                path_to_file_url(&entry_path),
                heap,
                out,
                control,
            )?;
        }
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}
pub(crate) fn native_boot_launched_class_loader_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let exploded = extract_int_arg(args, 1)?;
    let archive_ref = extract_ref_arg(args, 2)?;
    let _ = extract_ref_arg(args, 3)?;
    match args.get(4) {
        Some(Slot::Reference(_)) => {
            let exploded_slot = ops
                .instance_field_slot(
                    "org/springframework/boot/loader/launch/LaunchedClassLoader",
                    "exploded",
                )?;
            let root_archive_slot = ops
                .instance_field_slot(
                    "org/springframework/boot/loader/launch/LaunchedClassLoader",
                    "rootArchive",
                )?;
            let loader_obj = heap.get_mut(this_ref)?;
            loader_obj.fields[exploded_slot] = Slot::Int(i32::from(exploded != 0));
            loader_obj.fields[root_archive_slot] = Slot::Reference(Some(archive_ref));
            Ok(None)
        }
        _ => {
            Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            })
        }
    }
}
