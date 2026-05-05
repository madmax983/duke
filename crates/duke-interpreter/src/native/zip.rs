fn zip_files() -> &'static RwLock<HashMap<i32, duke_loader::ZipReader>> {
    static ZIP_FILES: OnceLock<RwLock<HashMap<i32, duke_loader::ZipReader>>> = OnceLock::new();
    ZIP_FILES.get_or_init(|| RwLock::new(HashMap::new()))
}
fn zip_open(path: &std::path::Path) -> Result<i32> {
    let reader = duke_loader::ZipReader::open(path)
        .map_err(|err| match err {
            duke_loader::Error::Io { .. } => {
                Error::JavaException {
                    class_name: "java/io/FileNotFoundException".to_string(),
                }
            }
            _ => {
                Error::JavaException {
                    class_name: "java/util/zip/ZipException".to_string(),
                }
            }
        })?;
    let id = NEXT_ZIP_ID.fetch_add(1, Ordering::Relaxed);
    zip_files()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(id, reader);
    Ok(id)
}
fn zip_entry_count(id: i32) -> Result<usize> {
    let map = zip_files().read().unwrap_or_else(std::sync::PoisonError::into_inner);
    map.get(&id)
        .map_or_else(
            || Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            }),
            |reader| Ok(reader.entry_count()),
        )
}
fn zip_get_entry_info(id: i32, name: &str) -> Result<Option<duke_loader::ZipEntryInfo>> {
    let map = zip_files().read().unwrap_or_else(std::sync::PoisonError::into_inner);
    map.get(&id)
        .map_or_else(
            || Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            }),
            |reader| Ok(reader.get_entry(name).cloned()),
        )
}
fn zip_read_entry(id: i32, name: &str) -> Result<Vec<u8>> {
    let map = zip_files().read().unwrap_or_else(std::sync::PoisonError::into_inner);
    map.get(&id)
        .map_or_else(
            || Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            }),
            |reader| {
                reader
                    .read_entry(name)
                    .map_err(|_| Error::JavaException {
                        class_name: "java/util/zip/ZipException".into(),
                    })
            },
        )
}
fn zip_close(id: i32) {
    zip_files().write().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&id);
}
/// Native: `ZipFile.<init>(String)` — open and index a ZIP/JAR archive.
pub(crate) fn native_zip_file_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path_ref = extract_ref_arg(args, 1)?;
    let path_str = heap
        .get(path_ref)?
        .string_value
        .as_deref()
        .ok_or(Error::NullPointerException)?
        .to_string();
    let fd = zip_open(std::path::Path::new(&path_str))?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(fd);
    Ok(None)
}
/// Native: `ZipFile.getEntry(String) -> ZipEntry` — look up an entry by name.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_zip_file_get_entry(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let entry_name = heap
        .get(name_ref)?
        .string_value
        .as_deref()
        .ok_or(Error::NullPointerException)?
        .to_string();
    let info = zip_get_entry_info(fd, &entry_name)?;
    let Some(info) = info else {
        return Ok(Some(Slot::Reference(None)));
    };
    let name_heap_ref = heap.allocate_string(info.name);
    let entry_ref = heap.allocate("java/util/zip/ZipEntry".to_string(), 6);
    let entry_obj = heap.get_mut(entry_ref)?;
    entry_obj.fields[0] = Slot::Reference(Some(name_heap_ref));
    entry_obj.fields[1] = Slot::Int(info.compressed_size as i32);
    entry_obj.fields[2] = Slot::Int((info.compressed_size >> 32) as i32);
    entry_obj.fields[3] = Slot::Int(info.uncompressed_size as i32);
    entry_obj.fields[4] = Slot::Int((info.uncompressed_size >> 32) as i32);
    entry_obj.fields[5] = Slot::Int(i32::from(info.compression_method));
    Ok(Some(Slot::Reference(Some(entry_ref))))
}
/// Native: `ZipFile.getInputStream(ZipEntry) -> InputStream`
pub(crate) fn native_zip_file_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let entry_ref = extract_ref_arg(args, 1)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let name_slot_ref = match heap.get(entry_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(Error::NullPointerException),
    };
    let entry_name = heap
        .get(name_slot_ref)?
        .string_value
        .as_deref()
        .ok_or(Error::NullPointerException)?
        .to_string();
    let data = zip_read_entry(fd, &entry_name)?;
    let buf_fd = heap.open_host_byte_buffer(data);
    let is_ref = heap.allocate("duke/zip/ByteBufferInputStream".to_string(), 1);
    let is_obj = heap.get_mut(is_ref)?;
    is_obj.fields[0] = Slot::Int(buf_fd);
    Ok(Some(Slot::Reference(Some(is_ref))))
}
/// Native: `ZipFile.close()`
pub(crate) fn native_zip_file_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) => *id,
        _ => return Ok(None),
    };
    zip_close(fd);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}
/// Native: `ZipFile.size() -> int`
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_zip_file_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let count = zip_entry_count(fd)?;
    Ok(Some(Slot::Int(count as i32)))
}
/// Native: `ZipEntry.getName() -> String`
pub(crate) fn native_zip_entry_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(heap.get(this_ref)?.fields[0]))
}
/// Native: `ZipEntry.getCompressedSize() -> long`
pub(crate) fn native_zip_entry_get_compressed_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let lo = match fields[1] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let hi = match fields[2] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let val = (i64::from(hi) << 32) | (i64::from(lo) & 0xFFFF_FFFF);
    Ok(Some(Slot::Long(val)))
}
/// Native: `ZipEntry.getSize() -> long`
pub(crate) fn native_zip_entry_get_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let lo = match fields[3] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let hi = match fields[4] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let val = (i64::from(hi) << 32) | (i64::from(lo) & 0xFFFF_FFFF);
    Ok(Some(Slot::Long(val)))
}
/// Native: `ZipEntry.getMethod() -> int`
pub(crate) fn native_zip_entry_get_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(heap.get(this_ref)?.fields[5]))
}
