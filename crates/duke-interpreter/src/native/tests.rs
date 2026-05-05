#[cfg(test)]
mod charset_codec_tests {
    use super::*;

    #[test]
    fn charset_aliases_map_to_canonical_variants() {
        assert_eq!(charset_for_name("utf8"), Some(StandardCharset::Utf8));
        assert_eq!(charset_for_name("latin1"), Some(StandardCharset::Iso88591));
        assert_eq!(charset_for_name("ASCII"), Some(StandardCharset::UsAscii));
        assert_eq!(charset_for_name("not-a-charset"), None);
    }

    #[test]
    fn utf8_decode_replaces_malformed_sequence() {
        assert_eq!(
            decode_string_with_charset(&[0xc3, 0x28], StandardCharset::Utf8),
            "\u{fffd}("
        );
    }

    #[test]
    fn utf16_encodes_bom_and_decodes_surrogate_pair() {
        let value = decode_string_with_charset(
            &[0xf0, 0x9f, 0x98, 0x80, b' ', b'e', b'm', b'o', b'j', b'i'],
            StandardCharset::Utf8,
        );
        let bytes = encode_string_with_charset(&value, StandardCharset::Utf16);
        assert_eq!(&bytes[0..2], &[0xfe, 0xff]);
        assert_eq!(decode_string_with_charset(&bytes, StandardCharset::Utf16), value);
    }

    #[test]
    fn ascii_and_latin1_encode_unmappable_as_question_mark() {
        assert_eq!(
            encode_string_with_charset("\u{20ac}", StandardCharset::UsAscii),
            vec![b'?']
        );
        assert_eq!(
            encode_string_with_charset("\u{20ac}", StandardCharset::Iso88591),
            vec![b'?']
        );
    }

    #[test]
    fn standard_charset_allocation_is_canonical_per_heap() {
        let mut heap = duke_gc::Heap::new();
        let first = allocate_standard_charset(&mut heap, "UTF-8");
        let second = allocate_standard_charset(&mut heap, "UTF-8");
        assert_eq!(first, second);
        assert_eq!(
            heap.get(first).expect("charset object").string_value.as_deref(),
            Some("UTF-8")
        );
    }
}

fn file_path_from_ref(file_ref: u64, heap: &duke_gc::Heap) -> Result<std::path::PathBuf> {
    let path_ref = match heap.get(file_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(Error::NullPointerException),
    };
    Ok(std::path::PathBuf::from(string_value_from_ref(
        heap, path_ref,
    )?))
}

fn file_path_from_this(args: &[Slot], heap: &duke_gc::Heap) -> Result<std::path::PathBuf> {
    let this_ref = extract_ref_arg(args, 0)?;
    file_path_from_ref(this_ref, heap)
}

fn archive_path_from_slot(
    heap: &duke_gc::Heap,
    archive_ref: u64,
    slot_idx: usize,
) -> Result<Option<String>> {
    let Some(file_ref) = archive_ref_from_slot(heap, archive_ref, slot_idx)? else {
        return Ok(None);
    };
    Ok(Some(
        file_path_from_ref(file_ref, heap)?
            .to_string_lossy()
            .to_string(),
    ))
}

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

fn launched_class_loader_archive_path(
    registry: &ClassRegistry,
    heap: &duke_gc::Heap,
    loader_ref: u64,
) -> Result<Option<String>> {
    if heap.get(loader_ref)?.class_name
        != "org/springframework/boot/loader/launch/LaunchedClassLoader"
    {
        return Ok(None);
    }
    let root_archive_slot = field_slot_idx(
        registry,
        "org/springframework/boot/loader/launch/LaunchedClassLoader",
        "rootArchive",
    )?;
    let Some(archive_ref) = archive_ref_from_slot(heap, loader_ref, root_archive_slot)? else {
        return Ok(None);
    };
    boot_archive_path_from_ref(registry, heap, archive_ref)
}

fn class_extends(registry: &ClassRegistry, class_name: &str, expected_super: &str) -> bool {
    let mut current = Some(class_name.to_string());
    while let Some(name) = current {
        if name == expected_super {
            return true;
        }
        current = registry.get(&name).ok().and_then(|ctx| ctx.super_class.clone());
    }
    false
}

fn arraylist_reference_elements(heap: &duke_gc::Heap, list_ref: u64) -> Result<Vec<u64>> {
    let list_obj = heap.get(list_ref)?;
    let size = match list_obj.fields.first().copied() {
        Some(Slot::Int(value)) if value > 0 => usize::try_from(value).unwrap_or(0),
        _ => 0,
    };
    let mut refs = Vec::with_capacity(size);
    for slot in list_obj.fields.iter().skip(1).take(size) {
        if let Slot::Reference(Some(reference)) = slot {
            refs.push(*reference);
        }
    }
    Ok(refs)
}

fn class_path_from_url_spec(spec: &str) -> Option<String> {
    if let Some(jar_spec) = spec.strip_prefix("jar:")
        && let Some(file_url) = jar_spec.split("!/").next()
    {
        return file_url_to_path(file_url)
            .ok()
            .map(|path| path.to_string_lossy().to_string());
    }
    if spec.starts_with("file://") {
        return file_url_to_path(spec)
            .ok()
            .map(|path| path.to_string_lossy().to_string());
    }
    None
}

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

fn archive_ref_from_slot(
    heap: &duke_gc::Heap,
    obj_ref: u64,
    slot_idx: usize,
) -> Result<Option<u64>> {
    match heap.get(obj_ref)?.fields.get(slot_idx).copied() {
        Some(Slot::Reference(Some(r))) => Ok(Some(r)),
        Some(Slot::Reference(None)) | None => Ok(None),
        Some(_) => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

#[cfg(test)]
fn boot_archive_entry_name(heap: &duke_gc::Heap, entry_ref: u64) -> Result<String> {
    let Some(Slot::Reference(Some(name_ref))) = heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_NAME_SLOT)
        .copied()
    else {
        return Err(Error::NullPointerException);
    };
    string_value_from_ref(heap, name_ref)
}

fn boot_archive_entry_is_directory_flag(heap: &duke_gc::Heap, entry_ref: u64) -> Result<bool> {
    match heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_DIRECTORY_SLOT)
        .copied()
    {
        Some(Slot::Int(value)) => Ok(value != 0),
        _ => Err(Error::TypeMismatch {
            expected: "Int",
            got: "other",
        }),
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
        .copied()
    else {
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
    Ok(Some(Slot::Int(i32::from(
        boot_archive_entry_is_directory_flag(heap, entry_ref)?,
    ))))
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
        &[
            Slot::Reference(Some(set_ref)),
            Slot::Reference(Some(url_ref)),
        ],
        heap,
        out,
        control,
    )?;
    Ok(())
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
    match ops.invoke(
        heap,
        out,
        &predicate_class,
        "test",
        "(Ljava/lang/Object;)Z",
        vec![
            Slot::Reference(Some(predicate_ref)),
            Slot::Reference(Some(entry_ref)),
        ],
    )? {
        Some(Slot::Int(value)) => Ok(value != 0),
        Some(other) => Err(Error::TypeMismatch {
            expected: "Int",
            got: match other {
                Slot::Long(_) => "Long",
                Slot::Float(_) => "Float",
                Slot::Double(_) => "Double",
                Slot::Reference(_) => "Reference",
                Slot::ReturnAddress(_) => "ReturnAddress",
                Slot::Int(_) => unreachable!(),
            },
        }),
        None => Ok(false),
    }
}

fn open_boot_archive_reader(path: &std::path::Path) -> Result<duke_loader::ZipReader> {
    duke_loader::ZipReader::open(path).map_err(|err| match err {
        duke_loader::Error::Io { .. } => Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        },
        _ => Error::JavaException {
            class_name: "java/util/zip/ZipException".to_string(),
        },
    })
}

fn archive_file_ref_at(heap: &duke_gc::Heap, archive_ref: u64, slot: usize) -> Result<u64> {
    match heap.get(archive_ref)?.fields.get(slot).copied() {
        Some(Slot::Reference(Some(file_ref))) => Ok(file_ref),
        Some(Slot::Reference(None)) => Err(Error::NullPointerException),
        _ => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

fn patch_forwarded_slot_if_needed(heap: &duke_gc::Heap, slot: &mut Slot) {
    if heap.has_pending_forwards() {
        heap.apply_forward(slot);
    }
}

fn patch_forwarded_ref_if_needed(heap: &duke_gc::Heap, reference: &mut u64) {
    let mut slot = Slot::Reference(Some(*reference));
    patch_forwarded_slot_if_needed(heap, &mut slot);
    if let Slot::Reference(Some(new_ref)) = slot {
        *reference = new_ref;
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
    let mut entry_names: Vec<String> = reader.entry_names().map(ToOwned::to_owned).collect();
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
            let url_spec = format!("jar:{}!/{}", path_to_file_url(&archive_path), entry_name);
            boot_archive_hashset_add_url(set_ref, url_spec, heap, out, control)?;
        }
    }

    Ok(Some(Slot::Reference(Some(set_ref))))
}

fn list_directory_children_sorted(path: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
    let iter = std::fs::read_dir(path).map_err(|_| Error::JavaException {
        class_name: "java/io/IOException".to_string(),
    })?;
    let mut children = Vec::new();
    for entry in iter {
        let entry = entry.map_err(|_| Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        })?;
        children.push(entry.path());
    }
    children.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .cmp(right.file_name().unwrap_or_default())
    });
    Ok(children)
}

fn exploded_archive_relative_entry_name(
    root_path: &std::path::Path,
    entry_path: &std::path::Path,
    is_directory: bool,
) -> String {
    let mut relative = entry_path
        .strip_prefix(root_path)
        .unwrap_or(entry_path)
        .to_string_lossy()
        .replace('\\', "/");
    if is_directory && !relative.ends_with('/') {
        relative.push('/');
    }
    relative
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

    let mut pending: VecDeque<std::path::PathBuf> = list_directory_children_sorted(&root_path)?
        .into_iter()
        .collect();
    while let Some(entry_path) = pending.pop_front() {
        let is_directory = entry_path.is_dir();
        let entry_name =
            exploded_archive_relative_entry_name(&root_path, &entry_path, is_directory);
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
            let exploded_slot = ops.instance_field_slot(
                "org/springframework/boot/loader/launch/LaunchedClassLoader",
                "exploded",
            )?;
            let root_archive_slot = ops.instance_field_slot(
                "org/springframework/boot/loader/launch/LaunchedClassLoader",
                "rootArchive",
            )?;
            let loader_obj = heap.get_mut(this_ref)?;
            loader_obj.fields[exploded_slot] = Slot::Int(i32::from(exploded != 0));
            loader_obj.fields[root_archive_slot] = Slot::Reference(Some(archive_ref));
            Ok(None)
        }
        _ => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

#[cfg(test)]
mod havoc_proptest_math {
    use super::*;
    use proptest::prelude::*;
    use std::io::sink;

    proptest! {
        #[test]
        fn fuzz_native_math_floor_div_int(a in any::<i32>(), b in any::<i32>()) {
            let mut heap = duke_gc::Heap::new();
            let mut control = NativeControl::default();
            let args = vec![Slot::Int(a), Slot::Int(b)];

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = native_math_floor_div_int(&args, &mut heap, &mut sink(), &mut control);
            }));

            assert!(result.is_ok(), "Panic on a={a}, b={b}");
        }
    }
}

// ---- System.arraycopy native ----

#[cfg(test)]
mod havoc_thread_join_itself {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn test_join_java_thread_itself() {
        let runtime = Arc::new(Mutex::new(CompletionRuntime::default()));
        let runtime_clone = runtime.clone();

        let (tx, rx) = std::sync::mpsc::channel();
        let (tx_panic, rx_panic) = std::sync::mpsc::channel();

        let handle = thread::spawn(move || -> Result<()> {
            rx.recv().unwrap();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = join_java_thread(&runtime_clone, 0);
            }));
            if let Err(e) = result {
                if let Some(s) = e.downcast_ref::<&str>() {
                    tx_panic.send((*s).to_string()).unwrap();
                } else if let Some(s) = e.downcast_ref::<String>() {
                    tx_panic.send(s.clone()).unwrap();
                }
            }
            Ok(())
        });

        {
            let mut rt = runtime.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            rt.handles.insert(0, handle);
            let mut record = crate::threading::ThreadRecord::new(123, 0);
            record.finished = false;
            rt.threads.register(record);
        }

        tx.send(()).unwrap();

        // It should NOT panic, but rather return Ok(())
        let res = rx_panic.recv_timeout(std::time::Duration::from_millis(50));
        assert!(res.is_err(), "Expected no panic, but received one!");
    }
}

#[cfg(test)]
mod havoc_string_indent_overflow {
    use super::*;
    use std::io::sink;
    use duke_gc::Heap;
    use duke_runtime::Slot;

    #[test]

    fn test_string_indent_overflow() {
        let mut heap = Heap::new();
        let r = heap.allocate_string("hello\nworld".to_string());

        let args = vec![Slot::Reference(Some(r)), Slot::Int(i32::MIN)];
        let mut control = NativeControl::default();

        let _ = native_string_indent(&args, &mut heap, &mut sink(), &mut control);
    }
}

#[cfg(test)]
mod tests_zip_coverage {
    use super::*;

    #[test]
    fn zip_registry_error_coverage() {
        // ID 999 doesn't exist
        let err = zip_entry_count(999).unwrap_err();
        assert!(matches!(err, Error::JavaException { .. }));

        let err = zip_get_entry_info(999, "test").unwrap_err();
        assert!(matches!(err, Error::JavaException { .. }));

        let err = zip_read_entry(999, "test").unwrap_err();
        assert!(matches!(err, Error::JavaException { .. }));

        // Removing non-existent shouldn't panic
        zip_close(999);
    }
}

#[cfg(test)]
mod tests_zip_open_coverage {
    use super::*;

    #[test]
    fn zip_open_io_error() {
        let err = zip_open(std::path::Path::new("/does/not/exist/ever/zip.zip")).unwrap_err();
        assert!(matches!(err, Error::JavaException { ref class_name } if class_name == "java/io/FileNotFoundException"));
    }

    #[test]
    fn zip_open_format_error() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("bad_zip_format.zip");
        std::fs::write(&path, b"not a zip file").unwrap();
        let err = zip_open(&path).unwrap_err();
        assert!(matches!(err, Error::JavaException { ref class_name } if class_name == "java/util/zip/ZipException"));
        std::fs::remove_file(&path).unwrap();
    }







#[cfg(test)]
mod havoc_coverage_tests {
    use super::*;

    #[test]
    fn test_system_property_value_fallback() {
        assert!(system_property_value_fallback("file.separator").is_some());
        assert!(system_property_value_fallback("path.separator").is_some());
        assert!(system_property_value_fallback("line.separator").is_some());
        assert!(system_property_value_fallback("os.name").is_some());
        assert!(system_property_value_fallback("unknown.property").is_none());
        assert!(system_property_value_fallback("java.version").is_some());
        assert!(system_property_value_fallback("user.dir").is_some());
    }
}

}

#[cfg(test)]
mod havoc_string_repeat_oom {
    use super::*;
    use std::io::sink;
    use duke_gc::Heap;
    use duke_runtime::Slot;

    #[test]
    fn test_string_repeat_oom_trigger() {
        let mut heap = Heap::new();
        let r = heap.allocate_string("12345678901234567890".to_string());

        let args = vec![Slot::Reference(Some(r)), Slot::Int(i32::MAX)];
        let mut control = NativeControl::default();

        let result = native_string_repeat(&args, &mut heap, &mut sink(), &mut control);
        let err = result.unwrap_err();
        assert!(matches!(err, Error::JavaException { ref class_name } if class_name == "java/lang/OutOfMemoryError"));
    }
}

#[cfg(test)]
mod sentry_tests {
    use super::*;

    #[test]
    fn test_zip_functions_error_cases() {
        let invalid_id = -999;

        let count_err = zip_entry_count(invalid_id).unwrap_err();
        assert!(matches!(count_err, Error::JavaException { ref class_name } if class_name == "java/io/IOException"));

        let info_err = zip_get_entry_info(invalid_id, "test").unwrap_err();
        assert!(matches!(info_err, Error::JavaException { ref class_name } if class_name == "java/io/IOException"));

        let read_err = zip_read_entry(invalid_id, "test").unwrap_err();
        assert!(matches!(read_err, Error::JavaException { ref class_name } if class_name == "java/io/IOException"));

        // This shouldn't panic
        zip_close(invalid_id);
    }
}

#[cfg(test)]
mod havoc_string_indent_overflow_positive {
    use super::*;
    use std::io::sink;
    use duke_gc::Heap;
    use duke_runtime::Slot;

    #[test]
    fn test_string_indent_overflow_trigger() {
        let mut heap = Heap::new();
        let r = heap.allocate_string("hello\nworld".to_string());

        let args = vec![Slot::Reference(Some(r)), Slot::Int(i32::MAX)];
        let mut control = NativeControl::default();

        let result = native_string_indent(&args, &mut heap, &mut sink(), &mut control);
        let err = result.unwrap_err();
        assert!(matches!(err, Error::JavaException { ref class_name } if class_name == "java/lang/OutOfMemoryError"));
    }
}

#[cfg(test)]
mod native_helper_tests {
    use super::*;

    #[test]
    fn should_return_error_when_extract_ref_arg_receives_int() {
        let args = vec![Slot::Int(42)];
        let res = extract_ref_arg(&args, 0);
        assert!(matches!(res, Err(Error::NullPointerException)));
    }

    #[test]
    fn should_return_error_when_extract_ref_arg_out_of_bounds() {
        let args = vec![];
        let res = extract_ref_arg(&args, 0);
        assert!(matches!(res, Err(Error::NullPointerException)));
    }

    #[test]
    fn should_return_error_when_extract_io_fd_receives_no_fields() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/lang/Object".to_string(), 0);
        let res = extract_io_fd(&heap, obj_ref);
        assert!(matches!(res, Err(Error::JavaException { .. })));
    }

    #[test]
    fn should_return_error_when_extract_io_fd_at_receives_no_fields() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/lang/Object".to_string(), 0);
        let res = extract_io_fd_at(&heap, obj_ref, 10);
        assert!(matches!(res, Err(Error::JavaException { .. })));
    }

    #[test]
    fn should_return_error_when_extract_int_arg_receives_ref() {
        let args = vec![Slot::Reference(None)];
        let res = extract_int_arg(&args, 0);
        assert!(matches!(res, Err(Error::TypeMismatch { .. })));
    }

    #[test]
    fn should_return_error_when_extract_int_arg_out_of_bounds() {
        let args = vec![];
        let res = extract_int_arg(&args, 0);
        assert!(matches!(res, Err(Error::TypeMismatch { .. })));
    }

    #[test]
    fn should_extract_null_for_out_of_bounds_field_arg() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/lang/Object".to_string(), 0);
        let res = extract_field_arg(&heap, obj_ref, 5).unwrap();
        assert_eq!(res, Slot::Reference(None));
    }

    #[test]
    fn should_extract_null_for_empty_first_field_arg() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/lang/Object".to_string(), 0);
        let res = extract_first_field_arg(&heap, obj_ref).unwrap();
        assert_eq!(res, Slot::Reference(None));
    }

    #[test]
    fn should_extract_null_for_out_of_bounds_slot_arg() {
        let args = vec![];
        let res = extract_slot_arg(&args, 0);
        assert_eq!(res, Slot::Reference(None));
    }
}
