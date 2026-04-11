fn extract_slot_arg(args: &[Slot], idx: usize) -> Slot {
    args.get(idx).copied().unwrap_or(Slot::Reference(None))
}

#[inline]
fn extract_ref_arg(args: &[Slot], idx: usize) -> VmResult<u64> {
    match args.get(idx) {
        Some(Slot::Reference(Some(r))) => Ok(*r),
        _ => Err(VmError::NullPointerException),
    }
}

#[inline]
fn extract_io_fd(heap: &duke_gc::Heap, obj_ref: u64) -> VmResult<i32> {
    match heap.get(obj_ref)?.fields.first() {
        Some(Slot::Int(id)) => Ok(*id),
        _ => Err(VmError::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

#[inline]
fn extract_io_fd_at(heap: &duke_gc::Heap, obj_ref: u64, idx: usize) -> VmResult<i32> {
    match heap.get(obj_ref)?.fields.get(idx) {
        Some(Slot::Int(id)) => Ok(*id),
        _ => Err(VmError::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

#[inline]
fn extract_int_arg(args: &[Slot], idx: usize) -> VmResult<i32> {
    match args.get(idx) {
        Some(Slot::Int(v)) => Ok(*v),
        _ => Err(VmError::TypeMismatch {
            expected: "Int",
            got: "other",
        }),
    }
}

/// Box a primitive `Slot` into a heap object so it can be stored as `Object` in collections.
/// `Slot::Reference` and `Slot::Long` pad pass through unchanged.
/// `Slot::Int` → `java/lang/Integer`, `Slot::Long` → `java/lang/Long`,
/// `Slot::Double` → `java/lang/Double`, `Slot::Float` → `java/lang/Float`.
fn box_primitive_slot(slot: Slot, heap: &mut duke_gc::Heap) -> Slot {
    match slot {
        Slot::Int(v) => {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            if let Ok(obj) = heap.get_mut(r) {
                obj.fields[0] = Slot::Int(v);
            }
            Slot::Reference(Some(r))
        }
        Slot::Long(v) => {
            let r = heap.allocate("java/lang/Long".to_string(), 1);
            if let Ok(obj) = heap.get_mut(r) {
                obj.fields[0] = Slot::Long(v);
            }
            Slot::Reference(Some(r))
        }
        Slot::Double(v) => {
            let r = heap.allocate("java/lang/Double".to_string(), 1);
            if let Ok(obj) = heap.get_mut(r) {
                obj.fields[0] = Slot::Double(v);
            }
            Slot::Reference(Some(r))
        }
        Slot::Float(v) => {
            let r = heap.allocate("java/lang/Float".to_string(), 1);
            if let Ok(obj) = heap.get_mut(r) {
                obj.fields[0] = Slot::Float(v);
            }
            Slot::Reference(Some(r))
        }
        other => other, // already a reference
    }
}

#[inline]
fn extract_long_arg(args: &[Slot], idx: usize) -> VmResult<i64> {
    match args.get(idx) {
        Some(Slot::Long(v)) => Ok(*v),
        _ => Err(VmError::TypeMismatch {
            expected: "Long",
            got: "other",
        }),
    }
}

#[inline]
fn extract_float_arg(args: &[Slot], idx: usize) -> VmResult<f32> {
    match args.get(idx) {
        Some(Slot::Float(v)) => Ok(*v),
        _ => Err(VmError::TypeMismatch {
            expected: "Float",
            got: "other",
        }),
    }
}

#[inline]
fn extract_double_arg(args: &[Slot], idx: usize) -> VmResult<f64> {
    match args.get(idx) {
        Some(Slot::Double(v)) => Ok(*v),
        _ => Err(VmError::TypeMismatch {
            expected: "Double",
            got: "other",
        }),
    }
}

macro_rules! extract_print_arg {
    ($args:expr, $pat:pat => $expr:expr, $expected:literal) => {
        match $args.get(1) {
            Some($pat) => $expr,
            _ => {
                return Err(VmError::TypeMismatch {
                    expected: $expected,
                    got: "other",
                });
            }
        }
    };
}







fn path_from_string_slot(
    args: &[Slot],
    idx: usize,
    heap: &duke_gc::Heap,
) -> VmResult<std::path::PathBuf> {
    let path_ref = extract_ref_arg(args, idx)?;
    let path = heap
        .get(path_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)?;
    Ok(std::path::PathBuf::from(path))
}

fn string_value_from_ref(heap: &duke_gc::Heap, string_ref: u64) -> VmResult<String> {
    heap.get(string_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)
}

fn file_path_from_ref(file_ref: u64, heap: &duke_gc::Heap) -> VmResult<std::path::PathBuf> {
    let path_ref = match heap.get(file_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    Ok(std::path::PathBuf::from(string_value_from_ref(
        heap, path_ref,
    )?))
}

fn file_path_from_this(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<std::path::PathBuf> {
    let this_ref = extract_ref_arg(args, 0)?;
    file_path_from_ref(this_ref, heap)
}

fn archive_path_from_slot(
    heap: &duke_gc::Heap,
    archive_ref: u64,
    slot_idx: usize,
) -> VmResult<Option<String>> {
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
) -> VmResult<Option<String>> {
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
) -> VmResult<Option<String>> {
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

fn archive_ref_from_slot(
    heap: &duke_gc::Heap,
    obj_ref: u64,
    slot_idx: usize,
) -> VmResult<Option<u64>> {
    match heap.get(obj_ref)?.fields.get(slot_idx).copied() {
        Some(Slot::Reference(Some(r))) => Ok(Some(r)),
        Some(Slot::Reference(None)) | None => Ok(None),
        Some(_) => Err(VmError::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}









fn file_stream_id_from_this(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<i32> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(VmError::JavaException {
            class_name: "java/io/IOException".to_string(),
        }),
    }
}

















// ── Networking natives ────────────────────────────────────────────────────

















// ── ZIP / JAR natives ──────────────────────────────────────────────────











const BOOT_JAR_FILE_ARCHIVE_JAR_FILE_SLOT: usize = 1;
const BOOT_EXPLODED_ARCHIVE_ROOT_DIRECTORY_SLOT: usize = 0;
const BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT: usize = 2;

fn allocate_manifest_from_bytes(heap: &mut duke_gc::Heap, bytes: &[u8]) -> VmResult<u64> {
    let raw_ref = heap.allocate_string(String::from_utf8_lossy(bytes).into_owned());
    let manifest_ref = heap.allocate("java/util/jar/Manifest".to_string(), 1);
    heap.get_mut(manifest_ref)?.fields[0] = Slot::Reference(Some(raw_ref));
    Ok(manifest_ref)
}





















































// ---- List.of / Set.of / Map.of factory methods ----

/// Helper: create an `ArrayList` from a slice of `Slot`s.
fn make_list_from_slots(
    elems: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<u64> {
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    for &elem in elems {
        native_arraylist_add(&[Slot::Reference(Some(list_ref)), elem], heap, out, control)?;
    }
    Ok(list_ref)
}



/// Helper: create a `HashSet` from a slice of `Slot`s.
fn make_set_from_slots(
    elems: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<u64> {
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    for &elem in elems {
        native_hashset_add(&[Slot::Reference(Some(set_ref)), elem], heap, out, control)?;
    }
    Ok(set_ref)
}





// ---- Optional<T> natives ----

















// ---- ArrayList / HashMap bulk operations ----







// ---- LinkedList natives (field layout identical to ArrayList: fields[0]=size, fields[1..]=elements) ----































// ---- HashMap.forEach callback ----





// ---- TreeMap natives (field layout: fields[0]=Int(size), fields[1,2]=k0/v0 sorted by key) ----
// Keys are stored sorted in ascending order for O(n) insert / O(1) first&last.

/// Compare two `TreeMap` keys by natural ordering, supporting String, Integer, Long, and Double keys.
fn compare_treemap_keys(a: Slot, b: Slot, heap: &duke_gc::Heap) -> std::cmp::Ordering {
    let key_ord = |s: Slot| -> Option<KeyOrd> {
        if let Slot::Reference(Some(r)) = s
            && let Ok(obj) = heap.get(r)
        {
            if let Some(sv) = &obj.string_value {
                return Some(KeyOrd::Str(sv.clone()));
            }
            match obj.class_name.as_str() {
                "java/lang/Integer" | "java/lang/Short" | "java/lang/Byte" => {
                    if let Some(Slot::Int(n)) = obj.fields.first() {
                        return Some(KeyOrd::Int(i64::from(*n)));
                    }
                }
                "java/lang/Long" => {
                    if let Some(Slot::Long(n)) = obj.fields.first() {
                        return Some(KeyOrd::Int(*n));
                    }
                }
                "java/lang/Double" | "java/lang/Float" => {
                    if let Some(Slot::Double(n)) = obj.fields.first() {
                        return Some(KeyOrd::Flt(*n));
                    }
                }
                _ => {}
            }
        }
        None
    };
    match (key_ord(a), key_ord(b)) {
        (Some(KeyOrd::Int(x)), Some(KeyOrd::Int(y))) => x.cmp(&y),
        (Some(KeyOrd::Flt(x)), Some(KeyOrd::Flt(y))) => x.total_cmp(&y),
        (Some(KeyOrd::Str(x)), Some(KeyOrd::Str(y))) => x.cmp(&y),
        _ => std::cmp::Ordering::Equal,
    }
}

enum KeyOrd {
    Int(i64),
    Flt(f64),
    Str(String),
}

/// Check if two `TreeMap` keys are equal (same value semantics as `compare_treemap_keys` == Equal).
fn treemap_keys_equal(a: Slot, b: Slot, heap: &duke_gc::Heap) -> bool {
    compare_treemap_keys(a, b, heap) == std::cmp::Ordering::Equal
}























// ---- Stack natives (LIFO backed by ArrayList: push=add, pop=removeLast, peek=peekLast) ----













// ---- TreeSet natives (sorted unique elements, backed by sorted Vec<Slot>) ----
// fields[0] = Int(size), fields[1..] = unique elements in sorted String order



/// Native: `TreeSet.add(E)Z` — inserts in sorted order; returns false if already present.
/// Extract a sortable key from a Slot for `TreeSet` ordering.
/// Returns an `Ordering`-compatible f64 for numeric types, lexicographic for strings.
 // intentional: i64→f64 for sort ordering; precision loss acceptable
fn treeset_slot_sort_key(slot: Slot, heap: &duke_gc::Heap) -> Option<TreeSortKey> {
    match slot {
        Slot::Int(n) => Some(TreeSortKey::Num(f64::from(n))),
        Slot::Long(n) => Some(TreeSortKey::Num(n as f64)),
        Slot::Double(d) => Some(TreeSortKey::Num(d)),
        Slot::Float(f) => Some(TreeSortKey::Num(f64::from(f))),
        Slot::Reference(Some(r)) => {
            let obj = heap.get(r).ok()?;
            match obj.class_name.as_str() {
                "java/lang/Integer" | "java/lang/Long" | "java/lang/Short" | "java/lang/Byte" => {
                    match obj.fields.first() {
                        Some(Slot::Int(n)) => Some(TreeSortKey::Num(f64::from(*n))),
                        Some(Slot::Long(n)) => Some(TreeSortKey::Num(*n as f64)),
                        _ => None,
                    }
                }
                "java/lang/Double" | "java/lang/Float" => match obj.fields.first() {
                    Some(Slot::Double(d)) => Some(TreeSortKey::Num(*d)),
                    Some(Slot::Float(f)) => Some(TreeSortKey::Num(f64::from(*f))),
                    _ => None,
                },
                _ => obj.string_value.clone().map(TreeSortKey::Str),
            }
        }
        _ => None,
    }
}

#[derive(PartialEq)]
enum TreeSortKey {
    Num(f64),
    Str(String),
}

impl TreeSortKey {
    fn less_than(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Num(a), Self::Num(b)) => a < b,
            (Self::Str(a), Self::Str(b)) => a < b,
            _ => false,
        }
    }
}















// ---- Collections.min / max / shuffle ----











// ---- Stream natives ----
// duke/util/Stream: fields[0]=Int(size), fields[1..]=element refs

































/// Helper: allocate a `JoiningCollector` with 3 fields: delimiter, prefix, suffix.
fn make_joining_collector(
    heap: &mut duke_gc::Heap,
    delimiter: &str,
    prefix: &str,
    suffix: &str,
) -> u64 {
    let collector_ref = heap.allocate("duke/util/JoiningCollector".to_string(), 3);
    let delim_ref = heap.allocate_string(delimiter.to_string());
    let prefix_ref = heap.allocate_string(prefix.to_string());
    let suffix_ref = heap.allocate_string(suffix.to_string());
    let obj = heap.get_mut(collector_ref).expect("just allocated");
    obj.fields[0] = Slot::Reference(Some(delim_ref));
    obj.fields[1] = Slot::Reference(Some(prefix_ref));
    obj.fields[2] = Slot::Reference(Some(suffix_ref));
    collector_ref
}

















// ---------------------------------------------------------------------------
// Stream.limit / Stream.skip / Stream.flatMap
// ---------------------------------------------------------------------------







// ---------------------------------------------------------------------------
// IntStream — duke/util/IntStream
// fields[0]=Int(size), fields[1..n]=Int(value) elements (unboxed ints)
// ---------------------------------------------------------------------------

/// Build an `IntStream` heap object from a `Vec<i32>`.
fn make_int_stream(heap: &mut duke_gc::Heap, values: Vec<i32>) -> u64 {
    let n = i32::try_from(values.len()).unwrap_or(0);
    let r = heap.allocate("duke/util/IntStream".to_string(), 1);
    heap.get_mut(r).expect("fresh").fields[0] = Slot::Int(n);
    for v in values {
        heap.get_mut(r).expect("fresh").fields.push(Slot::Int(v));
    }
    r
}

/// Create an `OptionalInt` heap object. `None` = empty, `Some(v)` = present.
fn make_optional_int(heap: &mut duke_gc::Heap, value: Option<i32>) -> u64 {
    let r = heap.allocate("duke/util/OptionalInt".to_string(), 2);
    if let Some(v) = value {
        heap.get_mut(r).expect("fresh").fields[0] = Slot::Int(v);
        heap.get_mut(r).expect("fresh").fields[1] = Slot::Int(1);
    } else {
        heap.get_mut(r).expect("fresh").fields[1] = Slot::Int(0);
    }
    r
}

/// Create a `java/util/Optional` heap object. `None` = empty, `Some(slot)` = present.
fn make_optional(heap: &mut duke_gc::Heap, value: Option<Slot>) -> u64 {
    let r = heap.allocate("java/util/Optional".to_string(), 1);
    heap.get_mut(r).expect("fresh").fields[0] = value.unwrap_or(Slot::Reference(None));
    r
}

/// Extract int elements from an `IntStream` heap object.
fn int_stream_elems(heap: &duke_gc::Heap, ref_: u64) -> Vec<i32> {
    let size = match heap.get(ref_).ok().and_then(|o| o.fields.first().copied()) {
        Some(Slot::Int(n)) => usize::try_from(n).unwrap_or(0),
        _ => 0,
    };
    heap.get(ref_)
        .ok()
        .map(|o| {
            o.fields[1..=size]
                .iter()
                .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
                .collect()
        })
        .unwrap_or_default()
}





















































// ---- ArrayDeque natives ----
// fields[0]=Int(size), fields[1..size]=elements (front at index 1)



















// ---- PriorityQueue natives ----
// fields[0]=Int(size), fields[1..size]=elements; maintained as a min-heap (String key order for Strings, value for boxed ints).















// ---- Comparator natives ----























// ---------------------------------------------------------------------------
// Reflection natives
// ---------------------------------------------------------------------------





















fn allocate_string_backed_object(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    value: String,
) -> VmResult<u64> {
    let obj_ref = heap.allocate(class_name.to_string(), 1);
    let value_ref = heap.allocate_string(value);
    heap.get_mut(obj_ref)?.fields[0] = Slot::Reference(Some(value_ref));
    Ok(obj_ref)
}

fn first_reference_field(heap: &duke_gc::Heap, obj_ref: u64) -> VmResult<Option<u64>> {
    match heap.get(obj_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => Ok(Some(*r)),
        Some(Slot::Reference(None)) => Ok(None),
        _ => Err(VmError::NullPointerException),
    }
}

fn string_backed_object_value(heap: &duke_gc::Heap, obj_ref: u64) -> VmResult<String> {
    let Some(value_ref) = first_reference_field(heap, obj_ref)? else {
        return Err(VmError::NullPointerException);
    };
    string_value_from_ref(heap, value_ref)
}

fn path_to_file_url(path: &std::path::Path) -> String {
    let mut normalized = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        if let Some(stripped) = normalized.strip_prefix("//?/UNC/") {
            normalized = format!("//{stripped}");
        } else if let Some(stripped) = normalized.strip_prefix("//?/") {
            normalized = stripped.to_string();
        }
    }
    if cfg!(windows) && !normalized.starts_with('/') {
        normalized.insert(0, '/');
    }
    format!("file://{normalized}")
}

const fn decode_pct_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) =
                (decode_pct_hex(bytes[i + 1]), decode_pct_hex(bytes[i + 2]))
        {
            out.push(char::from((hi << 4) | lo));
            i += 3;
            continue;
        }
        out.push(char::from(bytes[i]));
        i += 1;
    }
    out
}

fn file_url_to_path(url: &str) -> VmResult<std::path::PathBuf> {
    let Some(rest) = url.strip_prefix("file://") else {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalArgumentException".to_string(),
        });
    };
    let decoded = percent_decode(rest);
    if cfg!(windows) {
        let trimmed = decoded.strip_prefix('/').unwrap_or(decoded.as_str());
        Ok(std::path::PathBuf::from(trimmed.replace('/', "\\")))
    } else {
        Ok(std::path::PathBuf::from(decoded))
    }
}



















fn manifest_attribute_value(manifest_bytes: &[u8], key: &str) -> Option<String> {
    let text = std::str::from_utf8(manifest_bytes).ok()?;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix(key)
            && let Some(value) = value.strip_prefix(':')
        {
            return Some(value.trim().to_string());
        }
    }
    None
}





const BOOT_ARCHIVE_ENTRY_NAME_SLOT: usize = 0;
const BOOT_ARCHIVE_ENTRY_DIRECTORY_SLOT: usize = 1;

#[cfg(test)]
fn boot_archive_entry_name(heap: &duke_gc::Heap, entry_ref: u64) -> VmResult<String> {
    let Some(Slot::Reference(Some(name_ref))) = heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_NAME_SLOT)
        .copied()
    else {
        return Err(VmError::NullPointerException);
    };
    string_value_from_ref(heap, name_ref)
}

fn boot_archive_entry_is_directory_flag(heap: &duke_gc::Heap, entry_ref: u64) -> VmResult<bool> {
    match heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_DIRECTORY_SLOT)
        .copied()
    {
        Some(Slot::Int(value)) => Ok(value != 0),
        _ => Err(VmError::TypeMismatch {
            expected: "Int",
            got: "other",
        }),
    }
}





fn boot_archive_hashset_add_url(
    set_ref: u64,
    url_spec: String,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<()> {
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
) -> VmResult<u64> {
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
) -> VmResult<bool> {
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
        Some(other) => Err(VmError::TypeMismatch {
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

fn open_boot_archive_reader(path: &std::path::Path) -> VmResult<duke_loader::ZipReader> {
    duke_loader::ZipReader::open(path).map_err(|err| match err {
        duke_loader::LoadError::Io { .. } => VmError::JavaException {
            class_name: "java/io/IOException".to_string(),
        },
        _ => VmError::JavaException {
            class_name: "java/util/zip/ZipException".to_string(),
        },
    })
}

fn archive_file_ref_at(heap: &duke_gc::Heap, archive_ref: u64, slot: usize) -> VmResult<u64> {
    match heap.get(archive_ref)?.fields.get(slot).copied() {
        Some(Slot::Reference(Some(file_ref))) => Ok(file_ref),
        Some(Slot::Reference(None)) => Err(VmError::NullPointerException),
        _ => Err(VmError::TypeMismatch {
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



fn list_directory_children_sorted(path: &std::path::Path) -> VmResult<Vec<std::path::PathBuf>> {
    let iter = std::fs::read_dir(path).map_err(|_| VmError::JavaException {
        class_name: "java/io/IOException".to_string(),
    })?;
    let mut children = Vec::new();
    for entry in iter {
        let entry = entry.map_err(|_| VmError::JavaException {
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









fn lookup_reflected_constructor(
    reflected: ReflectedClassInfo,
    parameter_descriptor: &str,
    public_only: bool,
) -> Option<ReflectedMethodInfo> {
    reflected.methods.into_iter().find(|method| {
        method.name == "<init>"
            && (!public_only || method.is_public)
            && descriptor_parameter_part(&method.descriptor) == parameter_descriptor
    })
}

fn reflected_constructors(
    reflected: ReflectedClassInfo,
    public_only: bool,
) -> Vec<ReflectedMethodInfo> {
    reflected
        .methods
        .into_iter()
        .filter(|method| method.name == "<init>" && (!public_only || method.is_public))
        .collect()
}























































// ---------------------------------------------------------------------------
// println overloads (long, float, double, boolean, char, object)
// ---------------------------------------------------------------------------












/// Convert a heap object to its Java display string.
///
/// Checks `string_value` first (handles String/StringBuilder).
/// For boxed primitives, extracts the stored value from `fields[0]`.
/// Falls back to `class_name@hex_ref` for opaque objects.
fn heap_object_to_string(obj: &duke_gc::HeapObject, obj_ref: u64) -> String {
    if let Some(s) = &obj.string_value {
        return s.clone();
    }
    match obj.class_name.as_str() {
        "java/lang/Integer" => {
            if let Some(Slot::Int(v)) = obj.fields.first() {
                return v.to_string();
            }
        }
        "java/lang/Long" => {
            if let Some(Slot::Long(v)) = obj.fields.first() {
                return v.to_string();
            }
        }
        "java/lang/Double" => {
            if let Some(Slot::Double(v)) = obj.fields.first() {
                return format_java_double(*v);
            }
        }
        "java/lang/Float" => {
            if let Some(Slot::Float(v)) = obj.fields.first() {
                return format_java_float(*v);
            }
        }
        "java/lang/Boolean" => {
            return match obj.fields.first() {
                Some(Slot::Int(v)) if *v != 0 => "true".to_string(),
                _ => "false".to_string(),
            };
        }
        "java/lang/Character" => {
            if let Some(Slot::Int(v)) = obj.fields.first()
                && let Some(c) = char::from_u32((*v).cast_unsigned())
            {
                return c.to_string();
            }
        }
        _ => {}
    }
    format!("{}@{:x}", obj.class_name, obj_ref)
}



// ---------------------------------------------------------------------------
// print overloads (long, float, double, boolean, char, object)
// ---------------------------------------------------------------------------













// ---------------------------------------------------------------------------
// System.exit
// ---------------------------------------------------------------------------



static SYSTEM_PROPERTY_OVERRIDES: std::sync::OnceLock<std::sync::Mutex<HashMap<String, String>>> =
    std::sync::OnceLock::new();

fn system_property_overrides() -> &'static std::sync::Mutex<HashMap<String, String>> {
    SYSTEM_PROPERTY_OVERRIDES.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

fn system_property_value(key: &str) -> Option<String> {
    let override_value = system_property_overrides()
        .lock()
        .expect("system property overrides mutex poisoned")
        .get(key)
        .cloned();
    if let Some(value) = override_value {
        return Some(value);
    }
    match key {
        "java.io.tmpdir" => Some(std::env::temp_dir().to_string_lossy().into_owned()),
        "file.separator" => Some(std::path::MAIN_SEPARATOR.to_string()),
        "path.separator" => Some(if cfg!(windows) { ";" } else { ":" }.to_string()),
        "line.separator" => Some(if cfg!(windows) { "\r\n" } else { "\n" }.to_string()),
        "user.dir" => std::env::current_dir()
            .ok()
            .map(|path| path.to_string_lossy().into_owned()),
        "user.home" => std::env::var("USERPROFILE")
            .ok()
            .or_else(|| std::env::var("HOME").ok()),
        "os.name" => Some(
            if cfg!(windows) {
                "Windows"
            } else if cfg!(target_os = "macos") {
                "Mac OS X"
            } else if cfg!(target_os = "linux") {
                "Linux"
            } else {
                std::env::consts::OS
            }
            .to_string(),
        ),
        "os.arch" => Some(std::env::consts::ARCH.to_string()),
        "java.version" => Some("21".to_string()),
        _ => None,
    }
}











fn system_time_to_epoch_millis(now: std::time::SystemTime) -> i64 {
    now.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}



static NANO_TIME_ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn monotonic_nano_time_now() -> i64 {
    let origin = NANO_TIME_ORIGIN.get_or_init(std::time::Instant::now);
    i64::try_from(origin.elapsed().as_nanos()).unwrap_or(i64::MAX)
}



const THREAD_TARGET_SLOT: usize = 0;
const THREAD_ID_SLOT: usize = 1;



























// Dispatch string constants used by the callback-based sort chain.
const COMPARE_TO_METHOD: &str = "compareTo";
const COMPARE_TO_OBJECT_DESC: &str = "(Ljava/lang/Object;)I";
const SORT_COMPARATOR_DESC: &str = "(Ljava/util/Comparator;)V";

/// Maps a `std::cmp::Ordering` to the Java `compareTo` convention: -1 / 0 / 1.
///
/// Used by all boxed-type `compareTo` natives to return a consistent,
/// sign-correct value without relying on `Ordering`'s internal discriminant.
#[inline]
const fn ordering_to_int(o: std::cmp::Ordering) -> i32 {
    match o {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}













// ---- Integer natives ----





























// ---- Integer bit / arithmetic operations ----

























// ---- Long bit / arithmetic operations ----

/// Native: `Long.bitCount(long)` — count number of set bits.
/// Native: `Long.numberOfLeadingZeros(long)` — count leading zero bits.
/// Native: `Long.numberOfTrailingZeros(long)` — count trailing zero bits.

// ---- java.util.Objects natives ----

















// ---- Collections utilities ----























// ---------------------------------------------------------------------------
// Phase 43: Collectors.toSet/toMap, Stream.mapToInt/min/max, IntStream.reduce,
//           Arrays.sort(Object[]), String(char[])/valueOf(char[])
// ---------------------------------------------------------------------------















fn stream_min_max_by_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    want_max: bool,
) -> VmResult<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let cmp_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(cmp_ref)) = cmp_slot else {
        let r = heap.allocate("java/util/Optional".to_string(), 1);
        heap.get_mut(r)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(r))));
    };
    let cmp_class = heap.get(cmp_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let opt_r = heap.allocate("java/util/Optional".to_string(), 1);
    if elems.is_empty() {
        heap.get_mut(opt_r)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(opt_r))));
    }
    let mut best = elems[0];
    for elem in elems.into_iter().skip(1) {
        let cmp_result = ops
            .invoke(
                heap,
                out,
                &cmp_class,
                "compare",
                "(Ljava/lang/Object;Ljava/lang/Object;)I",
                vec![cmp_slot, elem, best],
            )?
            .unwrap_or(Slot::Int(0));
        let cmp_val = match cmp_result {
            Slot::Int(n) => n,
            _ => 0,
        };
        // For min: pick elem if elem < best (cmp_val < 0)
        // For max: pick elem if elem > best (cmp_val > 0)
        if (want_max && cmp_val > 0) || (!want_max && cmp_val < 0) {
            best = elem;
        }
    }
    heap.get_mut(opt_r)?.fields[0] = best;
    Ok(Some(Slot::Reference(Some(opt_r))))
}



fn compare_slots_natural(
    a: Slot,
    b: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> VmResult<i32> {
    match (a, b) {
        (Slot::Reference(Some(ra)), Slot::Reference(Some(_rb))) => {
            let a_class = heap.get(ra)?.class_name.clone();
            let result = ops
                .invoke(
                    heap,
                    out,
                    &a_class,
                    "compareTo",
                    "(Ljava/lang/Object;)I",
                    vec![a, b],
                )?
                .unwrap_or(Slot::Int(0));
            Ok(match result {
                Slot::Int(n) => n,
                _ => 0,
            })
        }
        (Slot::Int(a), Slot::Int(b)) => Ok(a.cmp(&b) as i32),
        _ => Ok(0),
    }
}











// ---- String.valueOf overloads ----













// ---- String.concat ----



/// Formats a single boxed slot value using the given format specifier.
/// Apply width/alignment/flags to an already-formatted value string.
fn apply_format_width(s: String, width: usize, left_align: bool, zero_pad: bool) -> String {
    if s.len() >= width {
        return s;
    }
    let pad = width - s.len();
    if left_align {
        format!("{s}{}", " ".repeat(pad))
    } else if zero_pad {
        // zero-pad: insert zeros after optional sign
        if s.starts_with('-') || s.starts_with('+') {
            let (sign, rest) = s.split_at(1);
            format!("{sign}{}{rest}", "0".repeat(pad))
        } else {
            format!("{}{s}", "0".repeat(pad))
        }
    } else {
        format!("{}{s}", " ".repeat(pad))
    }
}


fn format_arg(
    spec: char,
    flags: &str,
    width: Option<usize>,
    precision: Option<usize>,
    slot: &Slot,
    heap: &duke_gc::Heap,
) -> VmResult<String> {
    let left_align = flags.contains('-');
    let force_sign = flags.contains('+');
    let zero_pad = flags.contains('0') && !left_align;

    let raw = match slot {
        Slot::Reference(None) => {
            if spec == 'b' {
                "false".to_string()
            } else {
                "null".to_string()
            }
        }
        Slot::Reference(Some(r)) => {
            let obj = heap.get(*r)?;
            match spec {
                's' => heap_object_to_string(obj, *r),
                'b' => {
                    // true if non-null Boolean true, else depends
                    if obj.class_name == "java/lang/Boolean" {
                        match obj.fields.first() {
                            Some(Slot::Int(n)) => {
                                if *n != 0 { "true" } else { "false" }.to_string()
                            }
                            _ => "true".to_string(),
                        }
                    } else {
                        "true".to_string() // non-null object → true
                    }
                }
                'c' => {
                    let code = match obj.fields.first() {
                        Some(Slot::Int(n)) => *n,
                        _ => 0,
                    };

                    char::from_u32(code as u32).map_or(String::new(), |c| c.to_string())
                }
                'd' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Int(v)) => i64::from(*v),
                        Some(Slot::Long(v)) => *v,
                        _ => 0,
                    };
                    if force_sign && v >= 0 {
                        format!("+{v}")
                    } else {
                        v.to_string()
                    }
                }
                'o' => match obj.fields.first() {
                    Some(Slot::Int(v)) => format!("{v:o}"),
                    Some(Slot::Long(v)) => format!("{v:o}"),
                    _ => "0".to_string(),
                },
                'f' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Double(v)) => *v,
                        Some(Slot::Float(v)) => f64::from(*v),
                        _ => 0.0,
                    };
                    let s = precision.map_or_else(|| format!("{v:.6}"), |p| format!("{v:.p$}"));
                    if force_sign && v >= 0.0 {
                        format!("+{s}")
                    } else {
                        s
                    }
                }
                'e' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Double(v)) => *v,
                        Some(Slot::Float(v)) => f64::from(*v),
                        _ => 0.0,
                    };
                    let prec = precision.unwrap_or(6);
                    // format in scientific notation matching Java's %e output
                    let s = format_scientific(v, prec, false);
                    if force_sign && v >= 0.0 {
                        format!("+{s}")
                    } else {
                        s
                    }
                }
                'x' => match obj.fields.first() {
                    Some(Slot::Int(v)) => format!("{v:x}"),
                    Some(Slot::Long(v)) => format!("{v:x}"),
                    _ => "0".to_string(),
                },
                'X' => match obj.fields.first() {
                    Some(Slot::Int(v)) => format!("{v:X}"),
                    Some(Slot::Long(v)) => format!("{v:X}"),
                    _ => "0".to_string(),
                },
                _ => String::new(),
            }
        }
        Slot::Int(n) => match spec {
            'd' => {
                if force_sign && *n >= 0 {
                    format!("+{n}")
                } else {
                    n.to_string()
                }
            }
            'b' => "true".to_string(),
            'c' =>
            {

                char::from_u32(*n as u32).map_or(String::new(), |c| c.to_string())
            }
            'o' => format!("{n:o}"),
            'x' => format!("{n:x}"),
            'X' => format!("{n:X}"),
            _ => n.to_string(),
        },
        Slot::Long(n) => match spec {
            'd' => {
                if force_sign && *n >= 0 {
                    format!("+{n}")
                } else {
                    n.to_string()
                }
            }
            'o' => format!("{n:o}"),
            'x' => format!("{n:x}"),
            'X' => format!("{n:X}"),
            _ => n.to_string(),
        },
        Slot::Double(v) => match spec {
            'f' => {
                let s = precision.map_or_else(|| format!("{v:.6}"), |p| format!("{v:.p$}"));
                if force_sign && *v >= 0.0 {
                    format!("+{s}")
                } else {
                    s
                }
            }
            'e' => {
                let prec = precision.unwrap_or(6);
                let s = format_scientific(*v, prec, false);
                if force_sign && *v >= 0.0 {
                    format!("+{s}")
                } else {
                    s
                }
            }
            _ => format!("{v}"),
        },
        _ => String::new(),
    };

    Ok(match width {
        None => raw,
        Some(w) => apply_format_width(raw, w, left_align, zero_pad),
    })
}

/// Format a float in Java-style scientific notation `1.234568e+05`.
fn format_scientific(v: f64, prec: usize, upper: bool) -> String {
    if v == 0.0 {
        let zeros = "0".repeat(prec);
        let e = if upper { 'E' } else { 'e' };
        return format!("0.{zeros}{e}+00");
    }

    let exp = v.abs().log10().floor() as i32;
    let mantissa = v / 10_f64.powi(exp);
    let s = format!("{mantissa:.prec$}");
    let e_char = if upper { 'E' } else { 'e' };
    if exp >= 0 {
        format!("{s}{e_char}+{exp:02}")
    } else {
        format!("{s}{e_char}-{:02}", exp.unsigned_abs())
    }
}



// ---- Extended String natives ----

















// ---- Math natives ----









// ---- Extended Math natives ----























// ---- Extended Math trig / transcendental natives ----





































// ---- System.arraycopy native ----



// ---- HashMap / Map$Entry iteration natives ----











// ---- Long class natives ----





























// ---- Double class natives ----







// ---- Float class native ----









// ---- Boolean class native ----









fn parse_bounded_i32_from_string_arg(
    args: &[Slot],
    heap: &duke_gc::Heap,
    min: i32,
    max: i32,
) -> VmResult<i32> {
    let s = extract_string_arg_value(args, 0, heap)?;
    parse_bounded_i32_with_radix(&s, 10, min, max)
}

fn parse_bounded_i32_from_string_and_radix_args(
    args: &[Slot],
    heap: &duke_gc::Heap,
    min: i32,
    max: i32,
) -> VmResult<i32> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let radix = extract_parse_radix_arg(args, 1)?;
    parse_bounded_i32_with_radix(&s, radix, min, max)
}

fn parse_bounded_i32_with_radix(s: &str, radix: u32, min: i32, max: i32) -> VmResult<i32> {
    let val = i32::from_str_radix(s.trim(), radix).map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    if val < min || val > max {
        return Err(VmError::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    Ok(val)
}

fn parse_i64_from_string_arg(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<i64> {
    let s = extract_string_arg_value(args, 0, heap)?;
    parse_i64_with_radix(&s, 10)
}

fn parse_i64_from_string_and_radix_args(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<i64> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let radix = extract_parse_radix_arg(args, 1)?;
    parse_i64_with_radix(&s, radix)
}

fn parse_i64_with_radix(s: &str, radix: u32) -> VmResult<i64> {
    i64::from_str_radix(s.trim(), radix).map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })
}

fn parse_i32_decode_from_string_arg(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<i32> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let val = parse_i128_decode(&s)?;
    if val < i128::from(i32::MIN) || val > i128::from(i32::MAX) {
        return Err(VmError::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    i32::try_from(val).map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })
}

fn parse_i64_decode_from_string_arg(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<i64> {
    let s = extract_string_arg_value(args, 0, heap)?;
    let val = parse_i128_decode(&s)?;
    if val < i128::from(i64::MIN) || val > i128::from(i64::MAX) {
        return Err(VmError::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    i64::try_from(val).map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })
}

fn parse_i128_decode(s: &str) -> VmResult<i128> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(VmError::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }

    let (negative, sign_len) = match trimmed.as_bytes().first() {
        Some(b'+') => (false, 1),
        Some(b'-') => (true, 1),
        _ => (false, 0),
    };

    let rest = &trimmed[sign_len..];
    let (radix, prefix_len) = if rest.starts_with("0x") || rest.starts_with("0X") {
        (16, 2)
    } else if rest.starts_with('#') {
        (16, 1)
    } else if rest.starts_with('0') && rest.len() > 1 {
        (8, 1)
    } else {
        (10, 0)
    };

    let digits = &rest[prefix_len..];
    if digits.is_empty() || digits.starts_with('+') || digits.starts_with('-') {
        return Err(VmError::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }

    let magnitude = i128::from_str_radix(digits, radix).map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(if negative { -magnitude } else { magnitude })
}

fn extract_string_arg_value(args: &[Slot], index: usize, heap: &duke_gc::Heap) -> VmResult<String> {
    let str_ref = extract_ref_arg(args, index)?;
    Ok(heap.get(str_ref)?.string_value.clone().unwrap_or_default())
}

fn extract_parse_radix_arg(args: &[Slot], index: usize) -> VmResult<u32> {
    let radix = extract_int_arg(args, index)?;
    if !(2..=36).contains(&radix) {
        return Err(VmError::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    Ok(u32::try_from(radix).unwrap_or(0))
}

































/// Execute a `StringConcatFactory` recipe: walk the recipe string, replacing
/// `\u{1}` placeholders with stringified dynamic args from the operand stack.
fn execute_string_concat_recipe(
    recipe: &str,
    dynamic_args: &[Slot],
    arg_types: &[char],
    constants: &[String],
    heap: &mut duke_gc::Heap,
) -> VmResult<Slot> {
    let mut result = String::new();
    let mut dyn_idx = 0;
    let mut const_idx = 0;

    for ch in recipe.chars() {
        match ch {
            '\u{1}' => {
                if dyn_idx < dynamic_args.len() {
                    let type_hint = arg_types.get(dyn_idx).copied().unwrap_or('I');
                    stringify_slot(&dynamic_args[dyn_idx], type_hint, heap, &mut result)?;
                    dyn_idx += 1;
                }
            }
            '\u{2}' => {
                if const_idx < constants.len() {
                    result.push_str(&constants[const_idx]);
                    const_idx += 1;
                }
            }
            other => result.push(other),
        }
    }

    let r = heap.allocate_string(result);
    Ok(Slot::Reference(Some(r)))
}

/// Convert a Slot to its string representation (like Java's String.valueOf).
/// `type_hint` is the JVM type descriptor char: 'Z' for boolean, 'I' for int, etc.
fn stringify_slot(
    slot: &Slot,
    type_hint: char,
    heap: &duke_gc::Heap,
    out: &mut String,
) -> VmResult<()> {
    match slot {
        Slot::Int(v) => {
            if type_hint == 'Z' {
                // JVM boolean: 0 = false, nonzero = true.
                out.push_str(if *v != 0 { "true" } else { "false" });
            } else if type_hint == 'C' {
                // JVM char: render as the Unicode character.
                if let Some(ch) = char::from_u32((*v).cast_unsigned()) {
                    out.push(ch);
                } else {
                    out.push('?');
                }
            } else {
                out.push_str(&v.to_string());
            }
        }
        Slot::Long(v) => out.push_str(&v.to_string()),
        Slot::Float(v) => out.push_str(&format_java_float(*v)),
        Slot::Double(v) => out.push_str(&format_java_double(*v)),
        Slot::Reference(None) => out.push_str("null"),
        Slot::Reference(Some(r)) => {
            out.push_str(&heap_object_to_string(heap.get(*r)?, *r));
        }
        Slot::ReturnAddress(v) => out.push_str(&v.to_string()),
    }
    Ok(())
}

/// Format a float like Java's Float.toString.
fn format_java_float(v: f32) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    let s = format!("{v}");
    if s.contains('.') { s } else { format!("{v}.0") }
}

/// Format a double like Java's Double.toString.
fn format_java_double(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    let s = format!("{v}");
    if s.contains('.') { s } else { format!("{v}.0") }
}

/// Execute a decoded JVM instruction stream.
///
/// # Parameters
/// - `instructions`: output of `duke_bytecode::decode`
/// - `cp`: constant pool from the parsed `ClassFile`
/// - `args`: initial local variable values (method arguments)
/// - `max_stack`: `Code.max_stack` from the class file
/// - `max_locals`: `Code.max_locals` from the class file
///
/// # Returns
/// `Ok(Some(slot))` for value-returning methods, `Ok(None)` for `void`.
///
/// # Errors
/// Returns [`VmError`] on execution faults (division by zero, stack overflow,
/// unimplemented instruction, etc.).
///
/// # Examples
///
/// ```
/// use duke_bytecode::Instruction;
/// use duke_runtime::Slot;
/// use duke_interpreter::execute;
///
/// let instructions = vec![
///     (0, Instruction::Iconst5),
///     (1, Instruction::Ireturn),
/// ];
/// let cp = vec![];
/// let result = execute(&instructions, &cp, vec![], 1, 0).unwrap();
/// assert_eq!(result, Some(Slot::Int(5)));
/// ```
pub fn execute(
    instructions: &[(usize, Instruction)],
    cp: &[Option<CpEntry>],
    args: Vec<Slot>,
    max_stack: u16,
    max_locals: u16,
) -> VmResult<Option<Slot>> {
    // Build PC → instruction-index map for O(1) branch resolution.
    let pc_to_idx: HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(i, &(pc, _))| (pc, i))
        .collect();

    let mut frame = Frame::new(usize::from(max_stack), usize::from(max_locals), args)?;
    let mut idx: usize = 0;
    // Local heap for array objects allocated during single-method execution.
    let mut local_heap: Vec<(String, Vec<Slot>, Option<String>)> = Vec::new();
    let mut string_intern: HashMap<usize, u64> = HashMap::new();

    loop {
        let Some((pc, instr)) = instructions.get(idx) else {
            return Err(VmError::FellOffEnd);
        };
        let pc = *pc;

        // Jump to a PC-relative branch target (offset relative to current `pc`).
        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64).wrapping_add(i64::from($offset)) as usize;
                idx = *pc_to_idx
                    .get(&target)
                    .ok_or(VmError::InvalidBranchTarget { pc: target })?;
                continue;
            }};
        }

        match instr {
            // ----------------------------------------------------------------
            // Constants
            // ----------------------------------------------------------------
            Instruction::Nop => {}
            Instruction::AconstNull => frame.push(Slot::Reference(None))?,
            Instruction::IconstM1 => frame.push(Slot::Int(-1))?,
            Instruction::Iconst0 => frame.push(Slot::Int(0))?,
            Instruction::Iconst1 => frame.push(Slot::Int(1))?,
            Instruction::Iconst2 => frame.push(Slot::Int(2))?,
            Instruction::Iconst3 => frame.push(Slot::Int(3))?,
            Instruction::Iconst4 => frame.push(Slot::Int(4))?,
            Instruction::Iconst5 => frame.push(Slot::Int(5))?,
            Instruction::Lconst0 => frame.push(Slot::Long(0))?,
            Instruction::Lconst1 => frame.push(Slot::Long(1))?,
            Instruction::Fconst0 => frame.push(Slot::Float(0.0))?,
            Instruction::Fconst1 => frame.push(Slot::Float(1.0))?,
            Instruction::Fconst2 => frame.push(Slot::Float(2.0))?,
            Instruction::Dconst0 => frame.push(Slot::Double(0.0))?,
            Instruction::Dconst1 => frame.push(Slot::Double(1.0))?,
            Instruction::Bipush(v) => frame.push(Slot::Int(i32::from(*v)))?,
            Instruction::Sipush(v) => frame.push(Slot::Int(i32::from(*v)))?,

            // ----------------------------------------------------------------
            // Constant pool load
            // ----------------------------------------------------------------
            Instruction::Ldc(raw_idx) => {
                let cp_idx = usize::from(*raw_idx);
                if let Some(CpEntry::String { string_index }) =
                    cp.get(cp_idx).and_then(|e| e.as_ref())
                {
                    let si = string_index.0 as usize;
                    let r = if let Some(&cached) = string_intern.get(&cp_idx) {
                        cached
                    } else {
                        let s = match cp.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        let r = local_heap.len() as u64;
                        local_heap.push(("java/lang/String".to_string(), Vec::new(), Some(s)));
                        string_intern.insert(cp_idx, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    ldc_push(&mut frame, cp, cp_idx)?;
                }
            }
            Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
                let idx_val = usize::from(cp_idx.0);
                if let Some(CpEntry::String { string_index }) =
                    cp.get(idx_val).and_then(|e| e.as_ref())
                {
                    let si = string_index.0 as usize;
                    let r = if let Some(&cached) = string_intern.get(&idx_val) {
                        cached
                    } else {
                        let s = match cp.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        let r = local_heap.len() as u64;
                        local_heap.push(("java/lang/String".to_string(), Vec::new(), Some(s)));
                        string_intern.insert(idx_val, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    ldc_push(&mut frame, cp, idx_val)?;
                }
            }

            // ----------------------------------------------------------------
            // Loads
            // ----------------------------------------------------------------
            Instruction::Iload(i)
            | Instruction::Lload(i)
            | Instruction::Fload(i)
            | Instruction::Dload(i)
            | Instruction::Aload(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }
            Instruction::Iload0
            | Instruction::Lload0
            | Instruction::Fload0
            | Instruction::Dload0
            | Instruction::Aload0 => {
                let s = frame.load_local(0)?;
                frame.push(s)?;
            }
            Instruction::Iload1
            | Instruction::Lload1
            | Instruction::Fload1
            | Instruction::Dload1
            | Instruction::Aload1 => {
                let s = frame.load_local(1)?;
                frame.push(s)?;
            }
            Instruction::Iload2
            | Instruction::Lload2
            | Instruction::Fload2
            | Instruction::Dload2
            | Instruction::Aload2 => {
                let s = frame.load_local(2)?;
                frame.push(s)?;
            }
            Instruction::Iload3
            | Instruction::Lload3
            | Instruction::Fload3
            | Instruction::Dload3
            | Instruction::Aload3 => {
                let s = frame.load_local(3)?;
                frame.push(s)?;
            }
            Instruction::IloadW(i)
            | Instruction::LloadW(i)
            | Instruction::FloadW(i)
            | Instruction::DloadW(i)
            | Instruction::AloadW(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }

            // ----------------------------------------------------------------
            // Stores
            // ----------------------------------------------------------------
            Instruction::Istore(i)
            | Instruction::Lstore(i)
            | Instruction::Fstore(i)
            | Instruction::Dstore(i)
            | Instruction::Astore(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }
            Instruction::Istore0
            | Instruction::Lstore0
            | Instruction::Fstore0
            | Instruction::Dstore0
            | Instruction::Astore0 => {
                let v = frame.pop()?;
                frame.store_local(0, v)?;
            }
            Instruction::Istore1
            | Instruction::Lstore1
            | Instruction::Fstore1
            | Instruction::Dstore1
            | Instruction::Astore1 => {
                let v = frame.pop()?;
                frame.store_local(1, v)?;
            }
            Instruction::Istore2
            | Instruction::Lstore2
            | Instruction::Fstore2
            | Instruction::Dstore2
            | Instruction::Astore2 => {
                let v = frame.pop()?;
                frame.store_local(2, v)?;
            }
            Instruction::Istore3
            | Instruction::Lstore3
            | Instruction::Fstore3
            | Instruction::Dstore3
            | Instruction::Astore3 => {
                let v = frame.pop()?;
                frame.store_local(3, v)?;
            }
            Instruction::IstoreW(i)
            | Instruction::LstoreW(i)
            | Instruction::FstoreW(i)
            | Instruction::DstoreW(i)
            | Instruction::AstoreW(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }

            // ----------------------------------------------------------------
            // Stack manipulation
            // ----------------------------------------------------------------
            Instruction::Pop => {
                frame.pop()?;
            }
            Instruction::Pop2 => {
                frame.pop()?;
                frame.pop()?;
            }
            Instruction::Dup => {
                let v = frame.pop()?;
                frame.push(v)?;
                frame.push(v)?;
            }
            Instruction::DupX1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::DupX2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                let v4 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v4)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Swap => {
                let a = frame.pop()?;
                let b = frame.pop()?;
                frame.push(a)?;
                frame.push(b)?;
            }

            // ----------------------------------------------------------------
            // Integer arithmetic
            // ----------------------------------------------------------------
            Instruction::Iadd => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_add(b)))?;
            }
            Instruction::Isub => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_sub(b)))?;
            }
            Instruction::Imul => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_mul(b)))?;
            }
            Instruction::Idiv => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_div(b)))?;
            }
            Instruction::Irem => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_rem(b)))?;
            }
            Instruction::Ineg => {
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_neg()))?;
            }
            Instruction::Ishl => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shl((s & 0x1F) as u32)))?;
            }
            Instruction::Ishr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shr((s & 0x1F) as u32)))?;
            }
            Instruction::Iushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(((a as u32) >> (s as u32 & 0x1F)) as i32))?;
            }
            Instruction::Iand => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a & b))?;
            }
            Instruction::Ior => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a | b))?;
            }
            Instruction::Ixor => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a ^ b))?;
            }
            Instruction::Iinc { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }
            Instruction::IincW { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }

            // ----------------------------------------------------------------
            // Long arithmetic
            // ----------------------------------------------------------------
            Instruction::Ladd => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_add(b)))?;
            }
            Instruction::Lsub => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_sub(b)))?;
            }
            Instruction::Lmul => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_mul(b)))?;
            }
            Instruction::Ldiv => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_div(b)))?;
            }
            Instruction::Lrem => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_rem(b)))?;
            }
            Instruction::Lneg => {
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_neg()))?;
            }
            Instruction::Lshl => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shl((s & 0x3F) as u32)))?;
            }
            Instruction::Lshr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shr((s & 0x3F) as u32)))?;
            }
            Instruction::Lushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(((a as u64) >> (s as u32 & 0x3F)) as i64))?;
            }
            Instruction::Land => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a & b))?;
            }
            Instruction::Lor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a | b))?;
            }
            Instruction::Lxor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a ^ b))?;
            }
            Instruction::Lcmp => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                let r = match a.cmp(&b) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Float arithmetic
            // ----------------------------------------------------------------
            Instruction::Fadd => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a + b))?;
            }
            Instruction::Fsub => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a - b))?;
            }
            Instruction::Fmul => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a * b))?;
            }
            Instruction::Fdiv => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a / b))?;
            }
            Instruction::Frem => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a % b))?;
            }
            Instruction::Fneg => {
                let a = frame.pop_float()?;
                frame.push(Slot::Float(-a))?;
            }
            Instruction::Fcmpl | Instruction::Fcmpg => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                // Java semantics: exact bit equality for 0 check; NaN handled separately.

                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Fcmpg) {
                    1 // NaN result: Fcmpg pushes 1, Fcmpl pushes -1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Double arithmetic
            // ----------------------------------------------------------------
            Instruction::Dadd => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a + b))?;
            }
            Instruction::Dsub => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a - b))?;
            }
            Instruction::Dmul => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a * b))?;
            }
            Instruction::Ddiv => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a / b))?;
            }
            Instruction::Drem => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a % b))?;
            }
            Instruction::Dneg => {
                let a = frame.pop_double()?;
                frame.push(Slot::Double(-a))?;
            }
            Instruction::Dcmpl | Instruction::Dcmpg => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                // Java semantics: exact bit equality for 0 check; NaN handled separately.

                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Dcmpg) {
                    1 // NaN result: Dcmpg pushes 1, Dcmpl pushes -1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Type conversions
            // ----------------------------------------------------------------
            Instruction::I2l => {
                let v = frame.pop_int()?;
                frame.push(Slot::Long(i64::from(v)))?;
            }
            Instruction::I2f => {
                let v = frame.pop_int()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2d => {
                let v = frame.pop_int()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::L2i => {
                let v = frame.pop_long()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::L2f => {
                let v = frame.pop_long()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::L2d => {
                let v = frame.pop_long()?;
                frame.push(Slot::Double(v as f64))?;
            }
            Instruction::F2i => {
                let v = frame.pop_float()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::F2l => {
                let v = frame.pop_float()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::F2d => {
                let v = frame.pop_float()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::D2i => {
                let v = frame.pop_double()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::D2l => {
                let v = frame.pop_double()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::D2f => {
                let v = frame.pop_double()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2b => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i8)))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as u16)))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i16)))?;
            }

            // ----------------------------------------------------------------
            // Returns
            // ----------------------------------------------------------------
            Instruction::Return => return Ok(None),
            Instruction::Ireturn => return Ok(Some(Slot::Int(frame.pop_int()?))),
            Instruction::Lreturn => return Ok(Some(Slot::Long(frame.pop_long()?))),
            Instruction::Freturn => return Ok(Some(Slot::Float(frame.pop_float()?))),
            Instruction::Dreturn => return Ok(Some(Slot::Double(frame.pop_double()?))),

            // ----------------------------------------------------------------
            // Branches
            // ----------------------------------------------------------------
            Instruction::Goto(offset) => jump!(*offset),
            Instruction::GotoW(offset) => jump!(*offset),

            Instruction::Ifeq(offset) => {
                if frame.pop_int()? == 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifne(offset) => {
                if frame.pop_int()? != 0 {
                    jump!(*offset);
                }
            }
            Instruction::Iflt(offset) => {
                if frame.pop_int()? < 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifge(offset) => {
                if frame.pop_int()? >= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifgt(offset) => {
                if frame.pop_int()? > 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifle(offset) => {
                if frame.pop_int()? <= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifnull(offset) => {
                if matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::Ifnonnull(offset) => {
                if !matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpeq(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpne(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a != b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmplt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a < b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpge(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a >= b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpgt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a > b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmple(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a <= b {
                    jump!(*offset);
                }
            }
            // Reference comparisons
            Instruction::IfAcmpeq(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfAcmpne(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a != b {
                    jump!(*offset);
                }
            }

            // ----------------------------------------------------------------
            // Array allocation
            // ----------------------------------------------------------------
            Instruction::Newarray(array_type) => {
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let class_name = match array_type {
                    ArrayType::Boolean => "[Z",
                    ArrayType::Char => "[C",
                    ArrayType::Float => "[F",
                    ArrayType::Double => "[D",
                    ArrayType::Byte => "[B",
                    ArrayType::Short => "[S",
                    ArrayType::Int => "[I",
                    ArrayType::Long => "[J",
                };
                let init_slot = match array_type {
                    ArrayType::Long => Slot::Long(0),
                    ArrayType::Float => Slot::Float(0.0),
                    ArrayType::Double => Slot::Double(0.0),
                    _ => Slot::Int(0),
                };
                let r = local_heap.len() as u64;
                local_heap.push((
                    class_name.to_string(),
                    vec![init_slot; count as usize],
                    None,
                ));
                frame.push(Slot::Reference(Some(r)))?;
            }
            Instruction::Anewarray(cp_idx) => {
                let element_type = resolve_class_name(cp, usize::from(cp_idx.0))?;
                let array_type = format!("[L{element_type};");
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let r = local_heap.len() as u64;
                local_heap.push((
                    array_type,
                    vec![Slot::Reference(None); count as usize],
                    None,
                ));
                frame.push(Slot::Reference(Some(r)))?;
            }
            Instruction::Arraylength => {
                let r = frame.pop_ref()?;
                let len = local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1
                    .len();
                frame.push(Slot::Int(len as i32))?;
            }

            // ---- Int array ----
            Instruction::Iaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_int()?;
                frame.push(Slot::Int(v))?;
            }
            Instruction::Iastore => {
                let val = frame.pop_int()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Long array ----
            Instruction::Laload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_long()?;
                frame.push(Slot::Long(v))?;
            }
            Instruction::Lastore => {
                let val = frame.pop_long()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Long(val);
            }

            // ---- Float array ----
            Instruction::Faload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_float()?;
                frame.push(Slot::Float(v))?;
            }
            Instruction::Fastore => {
                let val = frame.pop_float()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Float(val);
            }

            // ---- Double array ----
            Instruction::Daload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_double()?;
                frame.push(Slot::Double(v))?;
            }
            Instruction::Dastore => {
                let val = frame.pop_double()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Double(val);
            }

            // ---- Reference array ----
            Instruction::Aaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize];
                frame.push(v)?;
            }
            Instruction::Aastore => {
                let val = frame.pop()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = val;
            }

            // ---- Byte/boolean array (stored as Int, truncated to i8) ----
            Instruction::Baload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as i8);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = i32::from(frame.pop_int()? as i8);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Char array (stored as Int, masked to u16) ----
            Instruction::Caload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as u16);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = i32::from(frame.pop_int()? as u16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Short array (stored as Int, truncated to i16) ----
            Instruction::Saload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as i16);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = i32::from(frame.pop_int()? as i16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Switch ----
            Instruction::Tableswitch {
                default,
                low,
                high,
                offsets,
            } => {
                let key = frame.pop_int()?;
                let offset = if key >= *low && key <= *high {
                    offsets[(key - low) as usize]
                } else {
                    *default
                };
                jump!(offset);
            }
            Instruction::Lookupswitch { default, pairs } => {
                let key = frame.pop_int()?;
                let offset = pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or(*default, |(_, off)| *off);
                jump!(offset);
            }

            // ---- checkcast / instanceof ----
            Instruction::Checkcast(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(slot)?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = resolve_class_name(cp, usize::from(cp_idx.0))?;
                        let actual = local_heap
                            .get(*r as usize)
                            .ok_or(VmError::InvalidRef { address: *r })?
                            .0
                            .clone();
                        if actual == target {
                            frame.push(slot)?;
                        } else {
                            return Err(VmError::ClassCastException {
                                from: actual,
                                to: target,
                            });
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }
            Instruction::Instanceof(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(Slot::Int(0))?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = resolve_class_name(cp, usize::from(cp_idx.0))?;
                        let actual = local_heap
                            .get(*r as usize)
                            .ok_or(VmError::InvalidRef { address: *r })?
                            .0
                            .clone();
                        if actual == target {
                            frame.push(Slot::Int(1))?;
                        } else {
                            frame.push(Slot::Int(0))?;
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }

            // ---- athrow ----
            Instruction::Athrow => {
                let r = frame.pop_ref()?;
                let class_name = local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .0
                    .clone();
                return Err(VmError::JavaException { class_name });
            }

            // ---- monitor (no-op, single-threaded) ----
            Instruction::Monitorenter | Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }

            other => {
                return Err(VmError::Unimplemented {
                    mnemonic: other.mnemonic(),
                });
            }
        }

        idx += 1;
    }
}

/// Ensure a class is initialized. Runs `<clinit>` if present and not yet run.
///
/// Must be called before first active use of a class (new, getstatic, putstatic, invokestatic).
/// `triggered_by` names the class that caused this init (empty string for the entry-point class).
fn ensure_initialized(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    _triggered_by: &str,
) -> VmResult<()> {
    if registry.is_initialized(class_name) {
        return Ok(());
    }
    // Mark as initialized BEFORE running clinit to prevent infinite recursion.
    registry.mark_initialized(class_name);
    initialize_primitive_wrapper_type_field(registry, heap, class_name)?;

    // Check if the class has a <clinit> method.
    let has_clinit = registry.get(class_name).is_ok_and(|ctx| {
        ctx.methods
            .iter()
            .any(|m| m.name == "<clinit>" && m.descriptor == "()V")
    });

    if has_clinit {
        let class_loader = registry.class_loader(class_name).cloned();
        let init_loader = class_loader.as_deref().map_or(loader, |v| v);
        // Run <clinit> by calling it through execute_class.
        #[cfg(feature = "telemetry")]

        let _clinit_start = std::time::Instant::now();
        execute_class(
            registry,
            init_loader,
            heap,
            stdout,
            class_name,
            "<clinit>",
            "()V",
            &[],
        )?;
        #[cfg(feature = "telemetry")]
        registry.telemetry.class_init_dag.record(
            class_name,
            _triggered_by,
            _clinit_start.elapsed().as_nanos() as u64,
        );
    }
    Ok(())
}

fn primitive_wrapper_type_descriptor(class_name: &str) -> Option<&'static str> {
    match class_name {
        "java/lang/Boolean" => Some("Z"),
        "java/lang/Byte" => Some("B"),
        "java/lang/Character" => Some("C"),
        "java/lang/Double" => Some("D"),
        "java/lang/Float" => Some("F"),
        "java/lang/Integer" => Some("I"),
        "java/lang/Long" => Some("J"),
        "java/lang/Short" => Some("S"),
        "java/lang/Void" => Some("V"),
        _ => None,
    }
}

fn initialize_primitive_wrapper_type_field(
    registry: &mut ClassRegistry,
    heap: &mut duke_gc::Heap,
    class_name: &str,
) -> VmResult<()> {
    let Some(descriptor) = primitive_wrapper_type_descriptor(class_name) else {
        return Ok(());
    };
    let type_slot = {
        let ctx = registry.get(class_name)?;
        static_field_idx(ctx, "TYPE")?
    };
    if matches!(
        registry.get(class_name)?.static_fields.get(type_slot),
        Some(Slot::Reference(Some(_)))
    ) {
        return Ok(());
    }
    let class_ref = allocate_class_object(heap, descriptor)?;
    registry.get_mut(class_name)?.static_fields[type_slot] = Slot::Reference(Some(class_ref));
    Ok(())
}

/// Reusable pool of frame backing buffers.
///
/// Eliminates per-call `Vec<Slot>` allocation for locals and operand stack.
/// On a recursive workload the pool reaches steady state after the first
/// call-depth calls; all subsequent frames are pool hits with zero allocation.
///
/// The pool is private to a single `execute_class` invocation.
struct FramePool {
    free: Vec<(Vec<Slot>, Vec<Slot>)>,
}

impl FramePool {
    const fn new() -> Self {
        Self { free: Vec::new() }
    }

    /// Acquire a `(locals_buf, stack_buf)` pair.
    /// Returns a pooled pair if available, otherwise allocates fresh Vecs.
    fn acquire(&mut self) -> (Vec<Slot>, Vec<Slot>) {
        self.free.pop().unwrap_or_default()
    }

    /// Return buffers to the pool.
    /// `stack` must already be empty (guaranteed by `Frame::into_pool_bufs`).
    /// Caps pool at 256 entries to bound memory usage.
    fn release(&mut self, locals: Vec<Slot>, stack: Vec<Slot>) {
        // Cap at 256 entries — typical max call depth is well under 100; anything
        // beyond this is dead weight. Entries dropped here are freed to the allocator.
        if self.free.len() < 256 {
            self.free.push((locals, stack));
        }
    }
}

/// Pre-resolved method dispatch entry — cached on first resolution to eliminate
/// repeated [`ClassRegistry`] `HashMap` lookups on hot call sites.
///
/// Stored in the static dispatch cache (`dispatch_cache`) and the virtual
/// dispatch cache (`vtable_cache`).  All `Arc` fields are cheap to clone
/// (reference-count bump only).
///
/// The caches are keyed as follows:
/// - `dispatch_cache`: `(caller_class, cp_idx)` for `invokestatic`/`invokespecial`
/// - `vtable_cache`: `(caller_class, cp_idx, receiver_runtime_class)` for `invokevirtual`
struct CachedDispatch {
    class_name: String,
    method_idx: usize,
    arg_count: usize,
    /// JVM primitive type chars for each parameter ('I', 'J', 'D', 'F', 'Z', 'B', 'C', 'S', 'L', '[').
    /// Used to assign wide types (J/D) to the correct local variable slots.
    param_types: Vec<char>,
    max_locals: usize,
    max_stack: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: std::sync::Arc<[(usize, Instruction)]>,
}

struct ExecutionState {
    current_class: String,
    method_idx: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: std::sync::Arc<[(usize, Instruction)]>,
    frame: Frame,
    call_stack: Vec<CallFrame>,
    frame_pool: FramePool,
    /// Static dispatch cache for `invokestatic` and `invokespecial`.
    /// Key: (`caller_class`, `cp_idx`) → pre-resolved method data.
    dispatch_cache: HashMap<String, HashMap<u16, CachedDispatch>>,
    /// Polymorphic inline cache for `invokevirtual`.
    /// Key: (`caller_class`, `cp_idx`, `receiver_runtime_class`) → pre-resolved method data.
    vtable_cache: HashMap<String, HashMap<u16, HashMap<String, CachedDispatch>>>,
    idx: usize,
    string_intern: HashMap<usize, u64>,
    #[cfg(feature = "telemetry")]
    current_method: String,
}

struct InterpreterCallbackOps<'a> {
    registry: &'a mut ClassRegistry,
    loader: &'a dyn ClassLoader,
}


fn callback_invoke_registered_lambda(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    class: &str,
    method: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Option<Slot>>> {
    let Some(lambda_info) = registry.get_lambda(class).cloned() else {
        return Ok(None);
    };
    if method != lambda_info.sam_method || descriptor != lambda_info.sam_desc {
        return Ok(None);
    }

    let this_ref = match args.first().copied() {
        Some(Slot::Reference(Some(reference))) => reference,
        Some(Slot::Reference(None)) | None => return Err(VmError::NullPointerException),
        Some(_) => {
            return Err(VmError::TypeMismatch {
                expected: "reference",
                got: "other",
            });
        }
    };
    let lambda_object = heap.get(this_ref)?;
    let mut impl_args =
        Vec::with_capacity(lambda_info.captured_count + args.len().saturating_sub(1));
    for capture_index in 0..lambda_info.captured_count {
        let slot =
            lambda_object
                .fields
                .get(capture_index)
                .copied()
                .ok_or(VmError::Unimplemented {
                    mnemonic: "lambda capture missing",
                })?;
        impl_args.push(slot);
    }
    impl_args.extend_from_slice(&args[1..]);

    let dispatch_class = match lambda_info.impl_kind {
        6 | 7 => lambda_info.impl_class.clone(),
        5 | 9 => match impl_args.first().copied() {
            Some(Slot::Reference(Some(receiver_ref))) => {
                let receiver_class = heap.get(receiver_ref)?.class_name.clone();
                if has_registered_native_override(
                    registry,
                    &receiver_class,
                    &lambda_info.impl_method,
                    &lambda_info.impl_desc,
                ) {
                    receiver_class
                } else if let Some((dispatch_class, _)) = resolve_method_in_hierarchy(
                    registry,
                    loader,
                    &receiver_class,
                    &lambda_info.impl_method,
                    &lambda_info.impl_desc,
                ) {
                    dispatch_class
                } else {
                    lambda_info.impl_class.clone()
                }
            }
            Some(Slot::Reference(None)) | None => return Err(VmError::NullPointerException),
            Some(_) => {
                return Err(VmError::TypeMismatch {
                    expected: "reference",
                    got: "other",
                });
            }
        },
        _ => {
            return Err(VmError::Unimplemented {
                mnemonic: "unsupported lambda impl kind",
            });
        }
    };

    let impl_args = adapt_args_for_impl_desc(&impl_args, &lambda_info.impl_desc, heap);
    let result = execute_class(
        registry,
        loader,
        heap,
        output,
        &dispatch_class,
        &lambda_info.impl_method,
        &lambda_info.impl_desc,
        &impl_args,
    )?;
    let result = autobox_if_needed(result, &lambda_info.impl_desc, &lambda_info.sam_desc, heap)?;
    Ok(Some(result))
}

impl CallbackOps for InterpreterCallbackOps<'_> {
    fn invoke(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
        method: &str,
        descriptor: &str,
        args: Vec<Slot>,
    ) -> VmResult<Option<Slot>> {
        if let Some(result) = callback_invoke_registered_lambda(
            self.registry,
            self.loader,
            heap,
            output,
            class,
            method,
            descriptor,
            &args,
        )? {
            return Ok(result);
        }
        execute_class(
            self.registry,
            self.loader,
            heap,
            output,
            class,
            method,
            descriptor,
            &args,
        )
    }

    fn ensure_loaded(&mut self, class: &str) -> VmResult<()> {
        match self.registry.resolve_loaded_class_key(class) {
            Ok(_) => return Ok(()),
            Err(VmError::ClassNotFound { .. }) => {}
            Err(err) => return Err(err),
        }
        if self.registry.ensure_loaded(class, self.loader)? {
            Ok(())
        } else {
            Err(VmError::ClassNotFound {
                name: class.to_string(),
            })
        }
    }

    fn inspect_class(&mut self, class: &str) -> VmResult<ReflectedClassInfo> {
        inspect_reflected_class(self.registry, self.loader, class)
    }

    fn ensure_class_initialized(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
    ) -> VmResult<()> {
        self.ensure_loaded(class)?;
        ensure_initialized(self.registry, self.loader, heap, output, class, "")
    }

    fn code_source_for_class(&mut self, class: &str) -> VmResult<Option<String>> {
        Ok(self
            .registry
            .code_source_for_class(class)
            .map(ToOwned::to_owned))
    }

    fn ensure_loaded_with_runtime_loader(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: u64,
        class: &str,
    ) -> VmResult<()> {
        if let Some(path) = launched_class_loader_archive_path(self.registry, heap, loader_ref)? {
            if self
                .registry
                .ensure_loaded_with_provenance(class, &path, Some(loader_ref))?
            {
                return Ok(());
            }
            return Err(VmError::ClassNotFound {
                name: class.to_string(),
            });
        }
        self.ensure_loaded(class)
    }

    fn instance_field_slot(&mut self, class: &str, field_name: &str) -> VmResult<usize> {
        field_slot_idx(self.registry, class, field_name)
    }

    fn read_instance_field(
        &mut self,
        heap: &duke_gc::Heap,
        object_ref: u64,
        declaring_class: &str,
        field_name: &str,
    ) -> VmResult<Slot> {
        let actual_class = heap.get(object_ref)?.class_name.clone();
        if !is_assignable_from(
            self.registry,
            self.loader,
            &actual_class,
            declaring_class,
            Some(declaring_class),
        ) {
            return Err(VmError::JavaException {
                class_name: "java/lang/IllegalArgumentException".to_string(),
            });
        }
        let slot = field_slot_idx(self.registry, declaring_class, field_name)?;
        Ok(heap.get(object_ref)?.fields[slot])
    }

    fn write_instance_field(
        &mut self,
        heap: &mut duke_gc::Heap,
        object_ref: u64,
        declaring_class: &str,
        field_name: &str,
        value: Slot,
    ) -> VmResult<()> {
        let actual_class = heap.get(object_ref)?.class_name.clone();
        if !is_assignable_from(
            self.registry,
            self.loader,
            &actual_class,
            declaring_class,
            Some(declaring_class),
        ) {
            return Err(VmError::JavaException {
                class_name: "java/lang/IllegalArgumentException".to_string(),
            });
        }
        let slot = field_slot_idx(self.registry, declaring_class, field_name)?;
        heap.write_field(object_ref, slot, value)?;
        Ok(())
    }

    fn read_static_field(&mut self, class: &str, field_name: &str) -> VmResult<Slot> {
        let slot = {
            let ctx = self.registry.get(class)?;
            static_field_idx(ctx, field_name)?
        };
        Ok(self.registry.get(class)?.static_fields[slot])
    }

    fn write_static_field(&mut self, class: &str, field_name: &str, value: Slot) -> VmResult<()> {
        let slot = {
            let ctx = self.registry.get(class)?;
            static_field_idx(ctx, field_name)?
        };
        self.registry.get_mut(class)?.static_fields[slot] = value;
        Ok(())
    }

    fn runtime_loader_for_class(&mut self, class: &str) -> VmResult<Option<u64>> {
        Ok(self.registry.runtime_loader_for_class(class))
    }

    fn class_key_for_loaded_class(&mut self, class: &str) -> VmResult<String> {
        self.registry.resolve_loaded_class_key(class)
    }

    fn class_key_for_runtime_loader(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: u64,
        class: &str,
    ) -> VmResult<String> {
        if let Some(path) = launched_class_loader_archive_path(self.registry, heap, loader_ref)? {
            return Ok(self.registry.class_key_from_provenance(
                class,
                Some(&path),
                Some(loader_ref),
            ));
        }
        self.class_key_for_loaded_class(class)
    }

    fn class_key_from_source(
        &mut self,
        class: &str,
        source_class: Option<&str>,
    ) -> VmResult<String> {
        Ok(self.registry.class_key_from_source(class, source_class))
    }

    fn allocate_instance(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
    ) -> VmResult<u64> {
        allocate_reflection_instance(self.registry, self.loader, heap, output, class)
    }
}

impl ExecutionState {
    fn new(
        registry: &ClassRegistry,
        class_name: &str,
        #[cfg_attr(not(feature = "telemetry"), allow(unused_variables))] method_name: &str,
        entry_idx: usize,
        args: &[Slot],
    ) -> VmResult<Self> {
        let current_class = class_name.to_string();
        let pc_to_idx = {
            let ctx = registry.get(&current_class)?;
            std::sync::Arc::clone(&ctx.methods[entry_idx].pc_to_idx)
        };
        let frame = {
            let ctx = registry.get(&current_class)?;
            Frame::new(
                usize::from(ctx.methods[entry_idx].max_stack),
                usize::from(ctx.methods[entry_idx].max_locals),
                args.to_vec(),
            )?
        };
        let instructions = {
            let ctx = registry.get(&current_class)?;
            std::sync::Arc::clone(&ctx.methods[entry_idx].instructions)
        };

        Ok(Self {
            current_class,
            method_idx: entry_idx,
            pc_to_idx,
            instructions,
            frame,
            call_stack: Vec::new(),
            frame_pool: FramePool::new(),
            dispatch_cache: HashMap::new(),
            vtable_cache: HashMap::new(),
            idx: 0,
            string_intern: HashMap::new(),
            #[cfg(feature = "telemetry")]
            current_method: method_name.to_string(),
        })
    }
}

#[cfg(feature = "telemetry")]
fn refresh_current_method_name(
    current_method: &mut String,
    registry: &ClassRegistry,
    current_class: &str,
    method_idx: usize,
) {
    *current_method = registry
        .get(current_class)
        .map(|c| {
            c.methods
                .get(method_idx)
                .map(|m| m.name.clone())
                .unwrap_or_default()
        })
        .unwrap_or_default();
}

/// Swap interpreter state to begin executing a callee method.
///
/// All method data is passed pre-resolved so this function performs **zero**
/// [`ClassRegistry`] lookups — eliminating the registry `HashMap` access from every
/// method-dispatch hot path.
/// Maximum Java call stack depth before raising `StackOverflowError`.
const MAX_CALL_DEPTH: usize = 500;


fn activate_method_state(
    frame: &mut Frame,
    method_idx: &mut usize,
    pc_to_idx: &mut std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: &mut std::sync::Arc<[(usize, Instruction)]>,
    current_class: &mut String,
    call_stack: &mut Vec<CallFrame>,
    callee_class: String,
    callee_idx: usize,
    callee_pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    callee_frame: Frame,
    callee_instructions: std::sync::Arc<[(usize, Instruction)]>,
    resume_idx: usize,
    #[cfg(feature = "telemetry")] registry: &ClassRegistry,
    #[cfg(feature = "telemetry")] current_method: &mut String,
) -> VmResult<()> {
    if call_stack.len() >= MAX_CALL_DEPTH {
        return Err(duke_runtime::VmError::JavaException {
            class_name: "java/lang/StackOverflowError".to_string(),
        });
    }
    call_stack.push(CallFrame {
        frame: std::mem::replace(frame, callee_frame),
        method_idx: *method_idx,
        pc_to_idx: std::sync::Arc::clone(pc_to_idx),
        resume_idx,
        class_name: current_class.clone(),
    });
    *method_idx = callee_idx;
    *pc_to_idx = callee_pc_to_idx;
    *current_class = callee_class;
    *instructions = callee_instructions;
    #[cfg(feature = "telemetry")]
    refresh_current_method_name(current_method, registry, current_class, *method_idx);
    Ok(())
}

enum ExecutionOutcome {
    Returned(Option<Slot>),
    ThreadAction(NativeThreadAction),
    /// The thread has exhausted its instruction quantum and should yield so
    /// other threads can make progress.
    Yield,
}

/// Default number of bytecode instructions a thread may execute before
/// yielding the VM lock, enabling fair interleaving of Java threads.
const DEFAULT_THREAD_QUANTUM: usize = 1024;

fn finish_native_call(
    native_control: &mut NativeControl,
    frame: &mut Frame,
    idx: &mut usize,
    result: Option<Slot>,
) -> VmResult<Option<ExecutionOutcome>> {
    if let Some(val) = result {
        frame.push(val)?;
    }
    *idx += 1;
    if let Some(action) = native_control.take() {
        return Ok(Some(ExecutionOutcome::ThreadAction(action)));
    }
    Ok(None)
}


fn prepare_execution_state(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<ExecutionState> {
    let class_name = match registry.resolve_loaded_class_key(class_name) {
        Ok(class_key) => class_key,
        Err(VmError::ClassNotFound { .. }) => {
            if !registry.ensure_loaded(class_name, loader)? {
                return Err(VmError::ClassNotFound {
                    name: class_name.to_string(),
                });
            }
            registry.resolve_loaded_class_key(class_name)?
        }
        Err(err) => return Err(err),
    };
    let entry_idx = {
        let ctx = registry.get(&class_name)?;
        ctx.methods
            .iter()
            .position(|m| m.name == method_name && m.descriptor == descriptor)
            .ok_or_else(|| VmError::MethodNotFound {
                name: format!("{class_name}.{method_name}"),
                descriptor: descriptor.to_string(),
            })?
    };

    let class_loader = registry.class_loader(&class_name).cloned();
    let init_loader = class_loader.as_deref().map_or(loader, |v| v);
    ensure_initialized(registry, init_loader, heap, stdout, &class_name, "")?;
    ExecutionState::new(registry, &class_name, method_name, entry_idx, args)
}

#[cfg(feature = "telemetry")]

const fn instr_name(instr: &duke_bytecode::Instruction) -> &'static str {
    use duke_bytecode::Instruction as I;
    match instr {
        I::Nop => "nop",
        I::AconstNull => "aconst_null",
        I::Iconst0 | I::Iconst1 | I::Iconst2 | I::Iconst3 | I::Iconst4 | I::Iconst5 => "iconst_n",
        I::IconstM1 => "iconst_m1",
        I::Lconst0 | I::Lconst1 => "lconst_n",
        I::Fconst0 | I::Fconst1 | I::Fconst2 => "fconst_n",
        I::Dconst0 | I::Dconst1 => "dconst_n",
        I::Bipush(_) => "bipush",
        I::Sipush(_) => "sipush",
        I::Ldc(_) | I::LdcW(_) | I::Ldc2W(_) => "ldc",
        I::Iload(_) | I::Iload0 | I::Iload1 | I::Iload2 | I::Iload3 | I::IloadW(_) => "iload",
        I::Lload(_) | I::Lload0 | I::Lload1 | I::Lload2 | I::Lload3 | I::LloadW(_) => "lload",
        I::Fload(_) | I::Fload0 | I::Fload1 | I::Fload2 | I::Fload3 | I::FloadW(_) => "fload",
        I::Dload(_) | I::Dload0 | I::Dload1 | I::Dload2 | I::Dload3 | I::DloadW(_) => "dload",
        I::Aload(_) | I::Aload0 | I::Aload1 | I::Aload2 | I::Aload3 | I::AloadW(_) => "aload",
        I::Istore(_) | I::Istore0 | I::Istore1 | I::Istore2 | I::Istore3 | I::IstoreW(_) => {
            "istore"
        }
        I::Lstore(_) | I::Lstore0 | I::Lstore1 | I::Lstore2 | I::Lstore3 | I::LstoreW(_) => {
            "lstore"
        }
        I::Fstore(_) | I::Fstore0 | I::Fstore1 | I::Fstore2 | I::Fstore3 | I::FstoreW(_) => {
            "fstore"
        }
        I::Dstore(_) | I::Dstore0 | I::Dstore1 | I::Dstore2 | I::Dstore3 | I::DstoreW(_) => {
            "dstore"
        }
        I::Astore(_) | I::Astore0 | I::Astore1 | I::Astore2 | I::Astore3 | I::AstoreW(_) => {
            "astore"
        }
        I::Iaload => "iaload",
        I::Laload => "laload",
        I::Faload => "faload",
        I::Daload => "daload",
        I::Aaload => "aaload",
        I::Baload => "baload",
        I::Caload => "caload",
        I::Saload => "saload",
        I::Iastore => "iastore",
        I::Lastore => "lastore",
        I::Fastore => "fastore",
        I::Dastore => "dastore",
        I::Aastore => "aastore",
        I::Bastore => "bastore",
        I::Castore => "castore",
        I::Sastore => "sastore",
        I::Pop => "pop",
        I::Pop2 => "pop2",
        I::Dup => "dup",
        I::DupX1 => "dup_x1",
        I::DupX2 => "dup_x2",
        I::Dup2 => "dup2",
        I::Dup2X1 => "dup2_x1",
        I::Dup2X2 => "dup2_x2",
        I::Swap => "swap",
        I::Iadd => "iadd",
        I::Ladd => "ladd",
        I::Fadd => "fadd",
        I::Dadd => "dadd",
        I::Isub => "isub",
        I::Lsub => "lsub",
        I::Fsub => "fsub",
        I::Dsub => "dsub",
        I::Imul => "imul",
        I::Lmul => "lmul",
        I::Fmul => "fmul",
        I::Dmul => "dmul",
        I::Idiv => "idiv",
        I::Ldiv => "ldiv",
        I::Fdiv => "fdiv",
        I::Ddiv => "ddiv",
        I::Irem => "irem",
        I::Lrem => "lrem",
        I::Frem => "frem",
        I::Drem => "drem",
        I::Ineg => "ineg",
        I::Lneg => "lneg",
        I::Fneg => "fneg",
        I::Dneg => "dneg",
        I::Ishl => "ishl",
        I::Lshl => "lshl",
        I::Ishr => "ishr",
        I::Lshr => "lshr",
        I::Iushr => "iushr",
        I::Lushr => "lushr",
        I::Iand => "iand",
        I::Land => "land",
        I::Ior => "ior",
        I::Lor => "lor",
        I::Ixor => "ixor",
        I::Lxor => "lxor",
        I::Iinc { .. } | I::IincW { .. } => "iinc",
        I::I2l => "i2l",
        I::I2f => "i2f",
        I::I2d => "i2d",
        I::L2i => "l2i",
        I::L2f => "l2f",
        I::L2d => "l2d",
        I::F2i => "f2i",
        I::F2l => "f2l",
        I::F2d => "f2d",
        I::D2i => "d2i",
        I::D2l => "d2l",
        I::D2f => "d2f",
        I::I2b => "i2b",
        I::I2c => "i2c",
        I::I2s => "i2s",
        I::Lcmp => "lcmp",
        I::Fcmpl => "fcmpl",
        I::Fcmpg => "fcmpg",
        I::Dcmpl => "dcmpl",
        I::Dcmpg => "dcmpg",
        I::Ifeq(_) | I::Ifne(_) | I::Iflt(_) | I::Ifge(_) | I::Ifgt(_) | I::Ifle(_) => "if_<cond>",
        I::IfIcmpeq(_)
        | I::IfIcmpne(_)
        | I::IfIcmplt(_)
        | I::IfIcmpge(_)
        | I::IfIcmpgt(_)
        | I::IfIcmple(_) => "if_icmp<cond>",
        I::IfAcmpeq(_) | I::IfAcmpne(_) => "if_acmp<cond>",
        I::Goto(_) | I::GotoW(_) => "goto",
        I::Jsr(_) | I::JsrW(_) => "jsr",
        I::Ret(_) | I::RetW(_) => "ret",
        I::Tableswitch { .. } => "tableswitch",
        I::Lookupswitch { .. } => "lookupswitch",
        I::Ireturn => "ireturn",
        I::Lreturn => "lreturn",
        I::Freturn => "freturn",
        I::Dreturn => "dreturn",
        I::Areturn => "areturn",
        I::Return => "return",
        I::Getstatic(_) => "getstatic",
        I::Putstatic(_) => "putstatic",
        I::Getfield(_) => "getfield",
        I::Putfield(_) => "putfield",
        I::Invokevirtual(_) => "invokevirtual",
        I::Invokespecial(_) => "invokespecial",
        I::Invokestatic(_) => "invokestatic",
        I::Invokeinterface { .. } => "invokeinterface",
        I::Invokedynamic(_) => "invokedynamic",
        I::New(_) => "new",
        I::Newarray(_) => "newarray",
        I::Anewarray(_) => "anewarray",
        I::Arraylength => "arraylength",
        I::Athrow => "athrow",
        I::Checkcast(_) => "checkcast",
        I::Instanceof(_) => "instanceof",
        I::Monitorenter => "monitorenter",
        I::Monitorexit => "monitorexit",
        I::Multianewarray { .. } => "multianewarray",
        I::Ifnull(_) | I::Ifnonnull(_) => "ifnull/nonnull",
    }
}

/// Execute a static method by name within a loaded class context.
///
/// Supports `invokestatic` calls between methods in the same class.
///
/// # Errors
/// Returns [`VmError`] on execution faults or if `method_name`/`descriptor`
/// are not found in `ctx`.
///
/// # Panics
/// Panics on internal invariant violations, such as a `Class` CP entry whose
/// name index refers to a non-`Utf8` entry (indicates a malformed class file).
///
/// # Examples
///
/// ```
/// use std::io::sink;
/// use duke_runtime::Slot;
/// use duke_gc::Heap;
/// use duke_loader::directory::DirectoryLoader;
/// use duke_interpreter::{execute_class, ClassRegistry};
///
/// let mut registry = ClassRegistry::new();
/// let loader = DirectoryLoader::new("my_classes");
/// let mut heap = Heap::new();
/// let mut out = sink();
///
/// // We simulate execution by calling execute_class.
/// // It fails here with a missing class error, but demonstrates the setup.
/// let res = execute_class(&mut registry, &loader, &mut heap, &mut out, "java/lang/Object", "hashCode", "()I", &[]);
/// assert!(res.is_err());
/// ```
pub fn execute_class(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>> {
    // Fast path: if a native handler is registered for this class/method/descriptor,
    // dispatch it directly without requiring a ClassContext in the registry.
    // This handles both Simple natives and Callback natives at the top-level call site.
    match lookup_registered_native_kind(registry, class_name, method_name, descriptor) {
        Some(HandlerKind::Simple(h)) => {
            let mut native_control = NativeControl::default();
            let result = h(args, heap, stdout, &mut native_control)?;
            if native_control.take().is_some() {
                return Err(VmError::Unimplemented {
                    mnemonic: "thread action requires execute_class_to_completion",
                });
            }
            return Ok(result);
        }
        Some(HandlerKind::Callback(h)) => {
            let mut native_control = NativeControl::default();
            let result = {
                let mut callback_ops = InterpreterCallbackOps { registry, loader };
                h(args, heap, stdout, &mut native_control, &mut callback_ops)?
            };
            if native_control.take().is_some() {
                return Err(VmError::Unimplemented {
                    mnemonic: "thread action requires execute_class_to_completion",
                });
            }
            return Ok(result);
        }
        None => {}
    }

    let mut state = prepare_execution_state(
        registry,
        loader,
        heap,
        stdout,
        class_name,
        method_name,
        descriptor,
        args,
    )?;
    match execution::run_execution(&mut state, registry, loader, heap, stdout, true, None)? {
        ExecutionOutcome::Returned(result) => Ok(result),
        ExecutionOutcome::ThreadAction(_) | ExecutionOutcome::Yield => {
            Err(VmError::Unimplemented {
                mnemonic: "thread action requires execute_class_to_completion",
            })
        }
    }
}

struct CompletionVm {
    registry: ClassRegistry,
    heap: duke_gc::Heap,
    output: Vec<u8>,
    live_workers: usize,
}

#[derive(Default)]
struct CompletionRuntime {
    threads: threading::ThreadRuntime,
    handles: HashMap<i32, std::thread::JoinHandle<VmResult<()>>>,
}

fn resolve_thread_entry(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &duke_gc::Heap,
    thread_ref: u64,
) -> VmResult<Option<(String, usize, Vec<Slot>)>> {
    let actual_class = heap.get(thread_ref)?.class_name.clone();
    if actual_class != "java/lang/Thread"
        && let Some((dispatch_class, method_idx)) =
            resolve_method_in_hierarchy(registry, loader, &actual_class, "run", "()V")
    {
        return Ok(Some((
            dispatch_class,
            method_idx,
            vec![Slot::Reference(Some(thread_ref))],
        )));
    }

    let target_ref = match heap.get(thread_ref)?.fields.get(THREAD_TARGET_SLOT) {
        Some(Slot::Reference(Some(target_ref))) => *target_ref,
        _ => return Ok(None),
    };
    let target_class = heap.get(target_ref)?.class_name.clone();
    let (dispatch_class, method_idx) =
        resolve_method_in_hierarchy(registry, loader, &target_class, "run", "()V").ok_or_else(
            || VmError::AbstractMethodError {
                class_name: target_class.clone(),
                method_name: "run".to_string(),
            },
        )?;
    Ok(Some((
        dispatch_class,
        method_idx,
        vec![Slot::Reference(Some(target_ref))],
    )))
}

fn join_java_thread(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    thread_id: i32,
) -> VmResult<()> {
    loop {
        let handle = {
            let mut runtime = runtime.lock().unwrap();
            let is_finished = runtime
                .threads
                .records()
                .iter()
                .find(|record| record.thread_id == thread_id)
                .is_none_or(|record| record.finished);
            if is_finished {
                return Ok(());
            }
            runtime.handles.remove(&thread_id)
        };

        if let Some(handle) = handle {
            return match handle.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            };
        }

        std::thread::yield_now();
    }
}

fn wait_for_all_java_threads(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
) -> VmResult<()> {
    let mut first_error = None;
    loop {
        let handles = {
            let mut runtime = runtime.lock().unwrap();
            if runtime.handles.is_empty() {
                return first_error.unwrap_or(Ok(()));
            }
            runtime
                .handles
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };

        for handle in handles {
            match handle.join() {
                Ok(result) => {
                    if let Err(e) = result {
                        first_error.get_or_insert(Err(e));
                    }
                }
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }
    }
}

fn handle_thread_action(
    action: NativeThreadAction,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> VmResult<()> {
    match action {
        NativeThreadAction::Start { thread_ref } => {
            spawn_java_thread(shared, runtime, loader, thread_ref)
        }
        NativeThreadAction::Sleep(duration) => {
            std::thread::sleep(duration);
            Ok(())
        }
        NativeThreadAction::Join { thread_id } => join_java_thread(runtime, thread_id),
    }
}

fn run_thread_to_completion(
    mut state: ExecutionState,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> VmResult<()> {
    loop {
        let mut shared_guard = shared.lock().unwrap();
        let CompletionVm {
            registry,
            heap,
            output,
            live_workers,
        } = &mut *shared_guard;
        let outcome = execution::run_execution(
            &mut state,
            registry,
            loader.as_ref(),
            heap,
            output,
            *live_workers == 0,
            Some(DEFAULT_THREAD_QUANTUM),
        )?;
        drop(shared_guard);

        match outcome {
            ExecutionOutcome::Returned(_) => return Ok(()),
            ExecutionOutcome::ThreadAction(action) => {
                handle_thread_action(action, shared, runtime, loader)?;
            }
            ExecutionOutcome::Yield => {
                // `std::sync::Mutex` is not fair — the same thread can
                // immediately re-acquire the lock, starving others.  A
                // brief sleep forces the OS scheduler to consider other
                // runnable threads before we loop back.
                std::thread::sleep(std::time::Duration::from_micros(1));
            }
        }
    }
}

fn spawn_java_thread(
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
    thread_ref: u64,
) -> VmResult<()> {
    {
        let mut shared_guard = shared.lock().unwrap();
        let thread = shared_guard.heap.get_mut(thread_ref)?;
        let already_started = matches!(
            thread.fields.get(THREAD_ID_SLOT),
            Some(Slot::Int(thread_id)) if *thread_id >= 0
        );
        drop(shared_guard);
        if already_started {
            return Ok(());
        }
    }

    let thread_id = {
        let mut runtime = runtime.lock().unwrap();
        let thread_id = runtime.threads.allocate_thread_id();
        runtime
            .threads
            .register(threading::ThreadRecord::new(thread_ref, thread_id));
        thread_id
    };

    let entry = {
        let mut shared_guard = shared.lock().unwrap();
        {
            let thread = shared_guard.heap.get_mut(thread_ref)?;
            thread.fields[THREAD_ID_SLOT] = Slot::Int(thread_id);
        }
        let CompletionVm {
            registry,
            heap,
            live_workers,
            ..
        } = &mut *shared_guard;
        let entry = resolve_thread_entry(registry, loader.as_ref(), heap, thread_ref)?;
        if entry.is_some() {
            *live_workers += 1;
        }
        drop(shared_guard);
        entry
    };
    let Some((dispatch_class, method_idx, args)) = entry else {
        let _ = runtime.lock().unwrap().threads.mark_finished(thread_id);
        return Ok(());
    };

    let state = {
        let shared = shared.lock().unwrap();
        ExecutionState::new(&shared.registry, &dispatch_class, "run", method_idx, &args)?
    };

    let shared_clone = std::sync::Arc::clone(shared);
    let runtime_clone = std::sync::Arc::clone(runtime);
    let loader_clone = std::sync::Arc::clone(loader);
    let handle = std::thread::spawn(move || {
        let result = run_thread_to_completion(state, &shared_clone, &runtime_clone, &loader_clone);
        {
            let mut shared = shared_clone.lock().unwrap();
            shared.live_workers = shared.live_workers.saturating_sub(1);
        }
        let _ = runtime_clone
            .lock()
            .unwrap()
            .threads
            .mark_finished_by_java_ref(thread_ref);
        result
    });
    runtime.lock().unwrap().handles.insert(thread_id, handle);
    Ok(())
}

/// Execute a Java entrypoint and keep the VM alive until any spawned worker
/// threads have either finished or been joined.
///
/// # Errors
///
/// Returns `VmError` if class resolution, method dispatch, or bytecode
/// execution fails in any thread.
///
/// # Panics
///
/// Panics if a `Mutex` protecting shared VM state is poisoned by a
/// panicking thread, or if the `Arc` cannot be unwound after all threads
/// have joined.
pub fn execute_class_to_completion<L>(
    registry: &mut ClassRegistry,
    loader: L,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>>
where
    L: ClassLoader + Send + Sync + 'static,
{
    let loader: std::sync::Arc<dyn ClassLoader + Send + Sync> = std::sync::Arc::new(loader);
    if lookup_registered_native_kind(registry, class_name, method_name, descriptor).is_some() {
        return execute_class(
            registry,
            loader.as_ref(),
            heap,
            stdout,
            class_name,
            method_name,
            descriptor,
            args,
        );
    }

    let mut vm = CompletionVm {
        registry: std::mem::take(registry),
        heap: std::mem::replace(heap, duke_gc::Heap::new()),
        output: Vec::new(),
        live_workers: 0,
    };

    let mut state = match prepare_execution_state(
        &mut vm.registry,
        loader.as_ref(),
        &mut vm.heap,
        &mut vm.output,
        class_name,
        method_name,
        descriptor,
        args,
    ) {
        Ok(state) => state,
        Err(err) => {
            *registry = vm.registry;
            *heap = vm.heap;
            return Err(err);
        }
    };

    let shared = std::sync::Arc::new(std::sync::Mutex::new(vm));
    let runtime = std::sync::Arc::new(std::sync::Mutex::new(CompletionRuntime::default()));

    let run_result: VmResult<Option<Slot>> = loop {
        let mut shared_guard = shared.lock().unwrap();
        let CompletionVm {
            registry,
            heap,
            output,
            live_workers,
        } = &mut *shared_guard;
        let outcome = execution::run_execution(
            &mut state,
            registry,
            loader.as_ref(),
            heap,
            output,
            *live_workers == 0,
            Some(DEFAULT_THREAD_QUANTUM),
        )?;
        drop(shared_guard);

        match outcome {
            ExecutionOutcome::Returned(result) => break Ok(result),
            ExecutionOutcome::ThreadAction(action) => {
                handle_thread_action(action, &shared, &runtime, &loader)?;
            }
            ExecutionOutcome::Yield => {
                // See comment in run_thread_to_completion — brief sleep
                // ensures fair scheduling across Java threads.
                std::thread::sleep(std::time::Duration::from_micros(1));
            }
        }
    };

    let wait_result = wait_for_all_java_threads(&runtime);
    let Ok(shared) = std::sync::Arc::try_unwrap(shared) else {
        panic!("completion runtime released shared VM state")
    };
    let Ok(shared) = shared.into_inner() else {
        panic!("shared VM mutex poisoned")
    };
    let flush_result = stdout
        .write_all(&shared.output)
        .map_err(|_| VmError::Unimplemented {
            mnemonic: "output flush",
        });
    *registry = shared.registry;
    *heap = shared.heap;

    match run_result {
        Ok(result) => {
            wait_result?;
            flush_result?;
            Ok(result)
        }
        Err(err) => {
            let _ = wait_result;
            let _ = flush_result;
            Err(err)
        }
    }
}

/// Saved state of a caller frame suspended during a method call.
struct CallFrame {
    frame: Frame,
    method_idx: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    resume_idx: usize,
    /// Class that was executing when this frame was pushed.
    class_name: String,
}

/// Build a [`ClassContext`] from a parsed [`duke_classfile::ClassFile`].
///
/// Decodes all methods with a Code attribute and extracts field metadata.
/// Methods without Code (abstract, native) are silently skipped.
#[must_use]

pub fn build_class_context(cf: &duke_classfile::ClassFile) -> ClassContext {
    use duke_bytecode::decode;
    use duke_classfile::access_flags::{FieldAccessFlags, MethodAccessFlags};
    use duke_classfile::types::{AttributeData, CpEntry};

    // Resolve this_class -> class name string.
    let class_name = {
        let entry = cf
            .constant_pool
            .get(cf.this_class.0 as usize)
            .and_then(|e| e.as_ref());
        if let Some(CpEntry::Class { name_index }) = entry {
            match cf
                .constant_pool
                .get(name_index.0 as usize)
                .and_then(|e| e.as_ref())
            {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => String::new(),
            }
        } else {
            String::new()
        }
    };

    let methods = cf
        .methods
        .iter()
        .filter_map(|m| {
            let name = match cf.constant_pool.get(m.name_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let descriptor = match cf.constant_pool.get(m.descriptor_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let is_native = m.access_flags.contains(MethodAccessFlags::NATIVE);
            let is_abstract = m.access_flags.contains(MethodAccessFlags::ABSTRACT);
            let code = m.attributes.iter().find_map(|a| {
                if let AttributeData::Code(c) = &a.data {
                    Some(c)
                } else {
                    None
                }
            });
            // Native and abstract methods have no Code attribute — emit a
            // stub MethodEntry so they appear in the method table for resolution.
            // The interpreter will dispatch these through NativeRegistry or error
            // on abstract calls.
            let Some(code) = code else {
                if is_native || is_abstract {
                    return Some(MethodEntry {
                        name,
                        descriptor,
                        is_public: m.access_flags.contains(MethodAccessFlags::PUBLIC),
                        is_static: m.access_flags.contains(MethodAccessFlags::STATIC),
                        is_native,
                        is_abstract,
                        instructions: std::sync::Arc::new([]),
                        max_stack: 0,
                        max_locals: 0,
                        exception_table: vec![],
                        pc_to_idx: std::sync::Arc::new(std::collections::HashMap::new()),
                    });
                }
                return None; // no Code and not native/abstract — malformed, skip
            };
            let instructions = decode(&code.code).ok()?;
            let exception_table: Vec<ExceptionEntry> = code
                .exception_table
                .iter()
                .map(|e| {
                    let catch_type = if e.catch_type.0 == 0 {
                        None // catch-all (finally)
                    } else {
                        match cf
                            .constant_pool
                            .get(e.catch_type.0 as usize)
                            .and_then(|x| x.as_ref())
                        {
                            Some(CpEntry::Class { name_index }) => {
                                match cf
                                    .constant_pool
                                    .get(name_index.0 as usize)
                                    .and_then(|x| x.as_ref())
                                {
                                    Some(CpEntry::Utf8(s)) => Some(s.clone()),
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    };
                    ExceptionEntry {
                        start_pc: e.start_pc,
                        end_pc: e.end_pc,
                        handler_pc: e.handler_pc,
                        catch_type,
                    }
                })
                .collect();
            let pc_to_idx_map: std::collections::HashMap<usize, usize> = instructions
                .iter()
                .enumerate()
                .map(|(i, &(pc, _))| (pc, i))
                .collect();
            Some(MethodEntry {
                name,
                descriptor,
                is_public: m.access_flags.contains(MethodAccessFlags::PUBLIC),
                is_static: m.access_flags.contains(MethodAccessFlags::STATIC),
                is_native,
                is_abstract,
                instructions: instructions.into(),
                max_stack: code.max_stack,
                max_locals: code.max_locals,
                exception_table,
                pc_to_idx: std::sync::Arc::new(pc_to_idx_map),
            })
        })
        .collect();

    let mut fields = Vec::new();
    let mut static_fields = Vec::new();
    let mut instance_count = 0usize;

    for f in &cf.fields {
        let name = match cf.constant_pool.get(f.name_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s.clone(),
            _ => continue,
        };
        let descriptor = match cf.constant_pool.get(f.descriptor_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s.clone(),
            _ => continue,
        };
        let is_static = f.access_flags.contains(FieldAccessFlags::STATIC);
        if is_static {
            static_fields.push(default_slot_for_descriptor(&descriptor));
        } else {
            instance_count += 1;
        }
        fields.push(FieldEntry {
            name,
            descriptor,
            is_static,
        });
    }

    // Resolve super_class: if the index is 0, this is java/lang/Object (no super).
    let super_class = if cf.super_class.0 != 0 {
        resolve_class_name(&cf.constant_pool, cf.super_class.0 as usize).ok()
    } else {
        None
    };

    // Resolve directly-implemented interfaces.
    let interfaces: Vec<String> = cf
        .interfaces
        .iter()
        .filter_map(|idx| resolve_class_name(&cf.constant_pool, idx.0 as usize).ok())
        .collect();

    // Extract BootstrapMethods from class-level attributes.
    let bootstrap_methods = cf
        .attributes
        .iter()
        .find_map(|a| {
            if let AttributeData::BootstrapMethods(entries) = &a.data {
                Some(entries.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    ClassContext {
        class_name,
        super_class,
        interfaces,
        constant_pool: cf.constant_pool.clone(),
        methods,
        fields,
        static_fields,
        instance_field_count: instance_count,
        bootstrap_methods,
        load_source: ClassLoadSource::Classfile,
    }
}

/// Resolve a CP Class entry to its name string.
fn resolve_class_name(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Class { name_index }) => {
            match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => Err(VmError::InvalidCpIndex {
                    index: name_index.0 as usize,
                }),
            }
        }
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

fn internal_name_to_binary_name(name: &str) -> String {
    match name {
        "B" => "byte".to_string(),
        "C" => "char".to_string(),
        "D" => "double".to_string(),
        "F" => "float".to_string(),
        "I" => "int".to_string(),
        "J" => "long".to_string(),
        "S" => "short".to_string(),
        "Z" => "boolean".to_string(),
        "V" => "void".to_string(),
        _ => name.replace('/', "."),
    }
}

fn binary_name_to_internal_name(name: &str) -> String {
    name.replace('.', "/")
}

fn cp_utf8_string(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Utf8(s)) => Ok(s.clone()),
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

fn reflected_class_info_from_loader(
    loader: &dyn ClassLoader,
    internal_name: &str,
) -> Option<ReflectedClassInfo> {
    use duke_classfile::access_flags::{FieldAccessFlags, MethodAccessFlags};

    let bytes = loader.find_class(internal_name).ok()?;
    let class_file = duke_classfile::parse(&bytes).ok()?;
    let methods = class_file
        .methods
        .iter()
        .filter_map(|method| {
            let name =
                cp_utf8_string(&class_file.constant_pool, method.name_index.0 as usize).ok()?;
            let descriptor = cp_utf8_string(
                &class_file.constant_pool,
                method.descriptor_index.0 as usize,
            )
            .ok()?;
            Some(ReflectedMethodInfo {
                name,
                descriptor,
                is_public: method.access_flags.contains(MethodAccessFlags::PUBLIC),
                is_static: method.access_flags.contains(MethodAccessFlags::STATIC),
            })
        })
        .collect();
    let fields = class_file
        .fields
        .iter()
        .filter_map(|field| {
            let name =
                cp_utf8_string(&class_file.constant_pool, field.name_index.0 as usize).ok()?;
            let descriptor =
                cp_utf8_string(&class_file.constant_pool, field.descriptor_index.0 as usize)
                    .ok()?;
            Some(ReflectedFieldInfo {
                name,
                descriptor,
                is_public: field.access_flags.contains(FieldAccessFlags::PUBLIC),
                is_static: field.access_flags.contains(FieldAccessFlags::STATIC),
            })
        })
        .collect();
    let super_class = if class_file.super_class.0 != 0 {
        resolve_class_name(&class_file.constant_pool, class_file.super_class.0 as usize).ok()
    } else {
        None
    };
    let interfaces = class_file
        .interfaces
        .iter()
        .filter_map(|idx| resolve_class_name(&class_file.constant_pool, idx.0 as usize).ok())
        .collect();
    Some(ReflectedClassInfo {
        internal_name: internal_name.to_string(),
        binary_name: internal_name_to_binary_name(internal_name),
        super_class,
        interfaces,
        methods,
        fields,
    })
}

fn qualify_reflected_class_info_ancestry(
    registry: &ClassRegistry,
    source_class: &str,
    mut info: ReflectedClassInfo,
) -> ReflectedClassInfo {
    info.super_class = info
        .super_class
        .map(|super_class| registry.class_key_from_source(&super_class, Some(source_class)));
    info.interfaces = info
        .interfaces
        .into_iter()
        .map(|interface| registry.class_key_from_source(&interface, Some(source_class)))
        .collect();
    info
}

fn inspect_reflected_class(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    class: &str,
) -> VmResult<ReflectedClassInfo> {
    let internal_name = registry.internal_name_for_class(class).to_string();
    match registry.resolve_loaded_class_key(class) {
        Ok(class_key) => {
            if let Some(class_loader) = registry.class_loader(&class_key)
                && let Some(info) =
                    reflected_class_info_from_loader(class_loader.as_ref(), &internal_name)
            {
                return Ok(qualify_reflected_class_info_ancestry(
                    registry, &class_key, info,
                ));
            }
            if let Some(info) = reflected_class_info_from_loader(loader, &internal_name) {
                return Ok(qualify_reflected_class_info_ancestry(
                    registry, &class_key, info,
                ));
            }
            let ctx = registry.get(&class_key)?;
            let methods = ctx
                .methods
                .iter()
                .map(|method| ReflectedMethodInfo {
                    name: method.name.clone(),
                    descriptor: method.descriptor.clone(),
                    is_public: method.is_public,
                    is_static: method.is_static,
                })
                .collect();
            let fields = ctx
                .fields
                .iter()
                .map(|field| ReflectedFieldInfo {
                    name: field.name.clone(),
                    descriptor: field.descriptor.clone(),
                    is_public: true,
                    is_static: field.is_static,
                })
                .collect();
            return Ok(ReflectedClassInfo {
                internal_name: internal_name.clone(),
                binary_name: internal_name_to_binary_name(&internal_name),
                super_class: ctx.super_class.clone(),
                interfaces: ctx.interfaces.clone(),
                methods,
                fields,
            });
        }
        Err(VmError::ClassNotFound { .. }) => {}
        Err(err) => return Err(err),
    }

    if !registry.contains(class)
        && let Some(info) = reflected_class_info_from_loader(loader, &internal_name)
    {
        return Ok(info);
    }

    if !registry.contains(class) {
        registry.ensure_loaded(&internal_name, loader)?;
    }
    let class_key = registry.resolve_loaded_class_key(class)?;
    let ctx = registry.get(&class_key)?;
    let methods = ctx
        .methods
        .iter()
        .map(|method| ReflectedMethodInfo {
            name: method.name.clone(),
            descriptor: method.descriptor.clone(),
            is_public: method.is_public,
            is_static: method.is_static,
        })
        .collect();
    let fields = ctx
        .fields
        .iter()
        .map(|field| ReflectedFieldInfo {
            name: field.name.clone(),
            descriptor: field.descriptor.clone(),
            is_public: true,
            is_static: field.is_static,
        })
        .collect();

    Ok(ReflectedClassInfo {
        internal_name: internal_name.clone(),
        binary_name: internal_name_to_binary_name(&internal_name),
        super_class: ctx.super_class.clone(),
        interfaces: ctx.interfaces.clone(),
        methods,
        fields,
    })
}

const REFLECTION_MEMBER_DECLARING_CLASS_FIELD: usize = 0;
const REFLECTION_MEMBER_NAME_FIELD: usize = 1;
const REFLECTION_MEMBER_DESCRIPTOR_FIELD: usize = 2;
const REFLECTION_MEMBER_PUBLIC_FIELD: usize = 3;
const REFLECTION_MEMBER_STATIC_FIELD: usize = 4;
const REFLECTION_MEMBER_ACCESSIBLE_FIELD: usize = 5;

fn class_internal_name_from_key(class_key: &str) -> &str {
    class_key
        .split_once('\0')
        .map_or(class_key, |(internal_name, _)| internal_name)
}

fn allocate_class_object(heap: &mut duke_gc::Heap, class_key: &str) -> VmResult<u64> {
    let class_ref = heap.allocate("java/lang/Class".to_string(), 0);
    heap.get_mut(class_ref)?.string_value = Some(class_key.to_string());
    Ok(class_ref)
}

fn class_key_from_ref(heap: &duke_gc::Heap, class_ref: u64) -> VmResult<String> {
    heap.get(class_ref)?
        .string_value
        .clone()
        .ok_or(VmError::InvalidRef { address: class_ref })
}

fn class_internal_name_from_ref(heap: &duke_gc::Heap, class_ref: u64) -> VmResult<String> {
    Ok(class_internal_name_from_key(&class_key_from_ref(heap, class_ref)?).to_string())
}

fn allocate_reference_array(
    heap: &mut duke_gc::Heap,
    array_class_name: &str,
    elements: &[u64],
) -> VmResult<u64> {
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
) -> VmResult<u64> {
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

fn reflection_member_name_slot(heap: &duke_gc::Heap, member_ref: u64) -> VmResult<Slot> {
    heap.get(member_ref)?
        .fields
        .get(REFLECTION_MEMBER_NAME_FIELD)
        .copied()
        .ok_or(VmError::InvalidRef {
            address: member_ref,
        })
}

fn reflection_member_declaring_class_slot(heap: &duke_gc::Heap, member_ref: u64) -> VmResult<Slot> {
    heap.get(member_ref)?
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied()
        .ok_or(VmError::InvalidRef {
            address: member_ref,
        })
}

fn reflection_member_declaring_class_name_slot(
    heap: &mut duke_gc::Heap,
    member_ref: u64,
) -> VmResult<Slot> {
    let Slot::Reference(Some(class_ref)) =
        reflection_member_declaring_class_slot(heap, member_ref)?
    else {
        return Err(VmError::InvalidRef {
            address: member_ref,
        });
    };
    let class_key = class_key_from_ref(heap, class_ref)?;
    let binary_name = internal_name_to_binary_name(class_internal_name_from_key(&class_key));
    let name_ref = heap.allocate_string(binary_name);
    Ok(Slot::Reference(Some(name_ref)))
}

fn descriptor_class_key_from_source(
    ops: &mut dyn CallbackOps,
    descriptor: &str,
    source_class: Option<&str>,
) -> VmResult<String> {
    match descriptor.as_bytes().first().copied() {
        Some(b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b'V')
            if descriptor.len() == 1 =>
        {
            Ok(descriptor.to_string())
        }
        Some(b'[') => Ok(descriptor.to_string()),
        Some(b'L') if descriptor.ends_with(';') => {
            ops.class_key_from_source(&descriptor[1..descriptor.len() - 1], source_class)
        }
        _ => Err(VmError::TypeMismatch {
            expected: "type descriptor",
            got: "other",
        }),
    }
}

fn descriptor_class_slot_from_source(
    heap: &mut duke_gc::Heap,
    ops: &mut dyn CallbackOps,
    descriptor: &str,
    source_class: Option<&str>,
) -> VmResult<Slot> {
    let class_key = descriptor_class_key_from_source(ops, descriptor, source_class)?;
    let class_ref = allocate_class_object(heap, &class_key)?;
    Ok(Slot::Reference(Some(class_ref)))
}

fn reflection_array_elements(heap: &duke_gc::Heap, args_slot: Slot) -> VmResult<Vec<Slot>> {
    match args_slot {
        Slot::Reference(None) => Ok(Vec::new()),
        Slot::Reference(Some(array_ref)) => Ok(heap.get(array_ref)?.fields.clone()),
        _ => Err(VmError::TypeMismatch {
            expected: "reference array",
            got: "other",
        }),
    }
}

fn class_descriptor_from_class_ref(heap: &duke_gc::Heap, class_ref: u64) -> VmResult<String> {
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    if internal_name.starts_with('[') || internal_name.len() == 1 {
        Ok(internal_name)
    } else {
        Ok(format!("L{internal_name};"))
    }
}

fn parameter_descriptor_from_class_array(heap: &duke_gc::Heap, slot: Slot) -> VmResult<String> {
    let params = reflection_array_elements(heap, slot)?;
    let mut descriptor = String::from("(");
    for param in params {
        let class_ref = match param {
            Slot::Reference(Some(class_ref)) => class_ref,
            Slot::Reference(None) => return Err(VmError::NullPointerException),
            _ => {
                return Err(VmError::TypeMismatch {
                    expected: "reference array",
                    got: "other",
                });
            }
        };
        descriptor.push_str(&class_descriptor_from_class_ref(heap, class_ref)?);
    }
    descriptor.push(')');
    Ok(descriptor)
}

fn descriptor_parameter_part(descriptor: &str) -> &str {
    descriptor
        .find(')')
        .map_or(descriptor, |idx| &descriptor[..=idx])
}

fn lookup_public_reflected_field(
    ops: &mut dyn CallbackOps,
    class: &str,
    field_name: &str,
) -> VmResult<Option<(String, ReflectedFieldInfo)>> {
    lookup_public_reflected_field_inner(ops, class, field_name, &mut HashSet::new())
}

fn lookup_public_reflected_field_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    field_name: &str,
    visited: &mut HashSet<String>,
) -> VmResult<Option<(String, ReflectedFieldInfo)>> {
    if !visited.insert(class.to_string()) {
        return Ok(None);
    }
    let reflected = ops.inspect_class(class)?;
    if let Some(field) = reflected
        .fields
        .iter()
        .find(|field| field.is_public && field.name == field_name)
        .cloned()
    {
        return Ok(Some((class.to_string(), field)));
    }
    if let Some(super_class) = reflected.super_class.clone()
        && let Some(found) =
            lookup_public_reflected_field_inner(ops, &super_class, field_name, visited)?
    {
        return Ok(Some(found));
    }
    for interface in reflected.interfaces {
        if let Some(found) =
            lookup_public_reflected_field_inner(ops, &interface, field_name, visited)?
        {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

fn lookup_public_reflected_method(
    ops: &mut dyn CallbackOps,
    class: &str,
    method_name: &str,
    parameter_descriptor: &str,
) -> VmResult<Option<(String, ReflectedMethodInfo)>> {
    lookup_public_reflected_method_inner(
        ops,
        class,
        method_name,
        parameter_descriptor,
        &mut HashSet::new(),
    )
}

fn lookup_public_reflected_method_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    method_name: &str,
    parameter_descriptor: &str,
    visited: &mut HashSet<String>,
) -> VmResult<Option<(String, ReflectedMethodInfo)>> {
    if !visited.insert(class.to_string()) {
        return Ok(None);
    }
    let reflected = ops.inspect_class(class)?;
    if let Some(method) = reflected
        .methods
        .iter()
        .find(|method| {
            method.is_public
                && method.name != "<init>"
                && method.name != "<clinit>"
                && method.name == method_name
                && descriptor_parameter_part(&method.descriptor) == parameter_descriptor
        })
        .cloned()
    {
        return Ok(Some((class.to_string(), method)));
    }
    if let Some(super_class) = reflected.super_class.clone()
        && let Some(found) = lookup_public_reflected_method_inner(
            ops,
            &super_class,
            method_name,
            parameter_descriptor,
            visited,
        )?
    {
        return Ok(Some(found));
    }
    for interface in reflected.interfaces {
        if let Some(found) = lookup_public_reflected_method_inner(
            ops,
            &interface,
            method_name,
            parameter_descriptor,
            visited,
        )? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

fn collect_public_reflected_fields(
    ops: &mut dyn CallbackOps,
    class: &str,
) -> VmResult<Vec<(String, ReflectedFieldInfo)>> {
    let mut collected = Vec::new();
    collect_public_reflected_fields_inner(
        ops,
        class,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut collected,
    )?;
    Ok(collected)
}

fn collect_public_reflected_fields_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    visited_classes: &mut HashSet<String>,
    seen_fields: &mut HashSet<String>,
    collected: &mut Vec<(String, ReflectedFieldInfo)>,
) -> VmResult<()> {
    if !visited_classes.insert(class.to_string()) {
        return Ok(());
    }
    let reflected = ops.inspect_class(class)?;
    for field in reflected.fields {
        if !field.is_public {
            continue;
        }
        let key = format!("{class}\0{}\0{}", field.name, field.descriptor);
        if seen_fields.insert(key) {
            collected.push((class.to_string(), field));
        }
    }
    if let Some(super_class) = reflected.super_class {
        collect_public_reflected_fields_inner(
            ops,
            &super_class,
            visited_classes,
            seen_fields,
            collected,
        )?;
    }
    for interface in reflected.interfaces {
        collect_public_reflected_fields_inner(
            ops,
            &interface,
            visited_classes,
            seen_fields,
            collected,
        )?;
    }
    Ok(())
}

fn collect_public_reflected_methods(
    ops: &mut dyn CallbackOps,
    class: &str,
) -> VmResult<Vec<(String, ReflectedMethodInfo)>> {
    let mut collected = Vec::new();
    collect_public_reflected_methods_inner(
        ops,
        class,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut collected,
    )?;
    Ok(collected)
}

fn collect_public_reflected_methods_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    visited_classes: &mut HashSet<String>,
    seen_methods: &mut HashSet<String>,
    collected: &mut Vec<(String, ReflectedMethodInfo)>,
) -> VmResult<()> {
    if !visited_classes.insert(class.to_string()) {
        return Ok(());
    }
    let reflected = ops.inspect_class(class)?;
    for method in reflected.methods {
        if !method.is_public || method.name == "<init>" || method.name == "<clinit>" {
            continue;
        }
        let key = format!(
            "{}\0{}",
            method.name,
            descriptor_parameter_part(&method.descriptor)
        );
        if seen_methods.insert(key) {
            collected.push((class.to_string(), method));
        }
    }
    if let Some(super_class) = reflected.super_class {
        collect_public_reflected_methods_inner(
            ops,
            &super_class,
            visited_classes,
            seen_methods,
            collected,
        )?;
    }
    for interface in reflected.interfaces {
        collect_public_reflected_methods_inner(
            ops,
            &interface,
            visited_classes,
            seen_methods,
            collected,
        )?;
    }
    Ok(())
}

struct ReflectedFieldHandle {
    declaring_class_key: String,
    field_name: String,
    descriptor: String,
    is_public: bool,
    is_static: bool,
    is_accessible: bool,
}

fn reflected_field_handle(heap: &duke_gc::Heap, field_ref: u64) -> VmResult<ReflectedFieldHandle> {
    let field_obj = heap.get(field_ref)?;
    let Some(Slot::Reference(Some(declaring_class_ref))) = field_obj
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied()
    else {
        return Err(VmError::InvalidRef { address: field_ref });
    };
    let Some(Slot::Reference(Some(name_ref))) =
        field_obj.fields.get(REFLECTION_MEMBER_NAME_FIELD).copied()
    else {
        return Err(VmError::InvalidRef { address: field_ref });
    };
    let Some(Slot::Reference(Some(descriptor_ref))) = field_obj
        .fields
        .get(REFLECTION_MEMBER_DESCRIPTOR_FIELD)
        .copied()
    else {
        return Err(VmError::InvalidRef { address: field_ref });
    };
    let is_public = matches!(
        field_obj.fields.get(REFLECTION_MEMBER_PUBLIC_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );
    let is_static = matches!(
        field_obj.fields.get(REFLECTION_MEMBER_STATIC_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );
    let is_accessible = matches!(
        field_obj.fields.get(REFLECTION_MEMBER_ACCESSIBLE_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );

    Ok(ReflectedFieldHandle {
        declaring_class_key: class_key_from_ref(heap, declaring_class_ref)?,
        field_name: heap
            .get(name_ref)?
            .string_value
            .clone()
            .ok_or(VmError::NullPointerException)?,
        descriptor: heap
            .get(descriptor_ref)?
            .string_value
            .clone()
            .ok_or(VmError::NullPointerException)?,
        is_public,
        is_static,
        is_accessible,
    })
}

struct ReflectedMethodHandle {
    declaring_class_key: String,
    method_name: String,
    descriptor: String,
    is_public: bool,
    is_static: bool,
    is_accessible: bool,
}

fn reflected_method_handle(
    heap: &duke_gc::Heap,
    method_ref: u64,
) -> VmResult<ReflectedMethodHandle> {
    let method_obj = heap.get(method_ref)?;
    let Some(Slot::Reference(Some(declaring_class_ref))) = method_obj
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied()
    else {
        return Err(VmError::InvalidRef {
            address: method_ref,
        });
    };
    let Some(Slot::Reference(Some(name_ref))) =
        method_obj.fields.get(REFLECTION_MEMBER_NAME_FIELD).copied()
    else {
        return Err(VmError::InvalidRef {
            address: method_ref,
        });
    };
    let Some(Slot::Reference(Some(descriptor_ref))) = method_obj
        .fields
        .get(REFLECTION_MEMBER_DESCRIPTOR_FIELD)
        .copied()
    else {
        return Err(VmError::InvalidRef {
            address: method_ref,
        });
    };
    let is_public = matches!(
        method_obj.fields.get(REFLECTION_MEMBER_PUBLIC_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );
    let is_static = matches!(
        method_obj.fields.get(REFLECTION_MEMBER_STATIC_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );
    let is_accessible = matches!(
        method_obj.fields.get(REFLECTION_MEMBER_ACCESSIBLE_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );

    Ok(ReflectedMethodHandle {
        declaring_class_key: class_key_from_ref(heap, declaring_class_ref)?,
        method_name: heap
            .get(name_ref)?
            .string_value
            .clone()
            .ok_or(VmError::NullPointerException)?,
        descriptor: heap
            .get(descriptor_ref)?
            .string_value
            .clone()
            .ok_or(VmError::NullPointerException)?,
        is_public,
        is_static,
        is_accessible,
    })
}

fn build_reflection_invoke_args(
    heap: &duke_gc::Heap,
    target_slot: Slot,
    descriptor: &str,
    invoke_arg_slots: Vec<Slot>,
    is_static: bool,
) -> VmResult<Vec<Slot>> {
    let arg_types = parse_arg_types(descriptor);
    if arg_types.len() != invoke_arg_slots.len() {
        return Err(VmError::TypeMismatch {
            expected: "matching reflective argument count",
            got: "different count",
        });
    }

    let mut invoke_args = Vec::with_capacity(arg_types.len() + usize::from(!is_static));
    if !is_static {
        match target_slot {
            Slot::Reference(Some(_)) => invoke_args.push(target_slot),
            _ => return Err(VmError::NullPointerException),
        }
    }
    for (descriptor, arg) in arg_types.iter().copied().zip(invoke_arg_slots) {
        invoke_args.push(unbox_reflection_argument(heap, descriptor, arg)?);
        if matches!(descriptor, 'J' | 'D') {
            invoke_args.push(Slot::Int(0));
        }
    }
    Ok(invoke_args)
}

fn unbox_reflection_argument(heap: &duke_gc::Heap, descriptor: char, arg: Slot) -> VmResult<Slot> {
    match descriptor {
        'L' | '[' => match arg {
            Slot::Reference(_) => Ok(arg),
            _ => Err(VmError::TypeMismatch {
                expected: "reference",
                got: "other",
            }),
        },
        'B' | 'C' | 'I' | 'S' | 'Z' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(VmError::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Int(value)) => Ok(Slot::Int(*value)),
                _ => Err(VmError::TypeMismatch {
                    expected: "boxed int-like primitive",
                    got: "other",
                }),
            }
        }
        'J' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(VmError::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Long(value)) => Ok(Slot::Long(*value)),
                _ => Err(VmError::TypeMismatch {
                    expected: "boxed long",
                    got: "other",
                }),
            }
        }
        'F' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(VmError::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Float(value)) => Ok(Slot::Float(*value)),
                _ => Err(VmError::TypeMismatch {
                    expected: "boxed float",
                    got: "other",
                }),
            }
        }
        'D' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(VmError::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Double(value)) => Ok(Slot::Double(*value)),
                _ => Err(VmError::TypeMismatch {
                    expected: "boxed double",
                    got: "other",
                }),
            }
        }
        _ => Err(VmError::Unimplemented {
            mnemonic: "reflection primitive unboxing",
        }),
    }
}

fn descriptor_return_type(descriptor: &str) -> char {
    descriptor
        .split_once(')')
        .and_then(|(_, ret)| ret.chars().next())
        .unwrap_or('V')
}

fn method_return_descriptor(descriptor: &str) -> &str {
    descriptor.split_once(')').map_or("V", |(_, ret)| ret)
}

fn box_reflection_return_value(
    heap: &mut duke_gc::Heap,
    return_type: char,
    result: Option<Slot>,
) -> VmResult<Slot> {
    match return_type {
        'V' => Ok(Slot::Reference(None)),
        'L' | '[' => Ok(result.unwrap_or(Slot::Reference(None))),
        'B' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "byte result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Byte".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'C' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "char result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Character".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'D' => {
            let Some(Slot::Double(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "double result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Double".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Double(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'F' => {
            let Some(Slot::Float(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "float result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Float".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Float(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'I' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "int result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'J' => {
            let Some(Slot::Long(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "long result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Long".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Long(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'S' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "short result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Short".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'Z' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "boolean result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Boolean".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        _ => Err(VmError::Unimplemented {
            mnemonic: "reflection primitive boxing",
        }),
    }
}

/// Push a constant pool value onto the frame's operand stack.
fn ldc_push(frame: &mut Frame, cp: &[Option<CpEntry>], idx: usize) -> VmResult<()> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Integer(v)) => frame.push(Slot::Int(*v)),
        Some(CpEntry::Float(v)) => frame.push(Slot::Float(*v)),
        Some(CpEntry::Long(v)) => frame.push(Slot::Long(*v)),
        Some(CpEntry::Double(v)) => frame.push(Slot::Double(*v)),
        _ => Err(VmError::InvalidCpIndex { index: idx }),
    }
}

/// Check if `from` is a subtype of `to` (i.e., `from` can be assigned where `to` is expected).
///
/// Walks the class hierarchy from `from` upward through superclasses.
/// Returns `true` if `to` is found in the chain, or if `to` is `"java/lang/Object"`.
/// Return `true` if a reference of type `from` can be used where `to` is expected.
///
/// BFS over the full type graph (superclass + all implemented interfaces at each
/// level), so `String instanceof Comparable` resolves correctly.
fn is_assignable_from(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    from: &str,
    to: &str,
    target_source_class: Option<&str>,
) -> bool {
    let from_key = if registry.contains(from) {
        from.to_string()
    } else {
        registry.class_key_from_source(from, Some(from))
    };
    let to_key = if registry.contains(to) {
        to.to_string()
    } else {
        registry.class_key_from_source(to, target_source_class)
    };
    let from_internal = registry.internal_name_for_class(&from_key);
    let to_internal = registry.internal_name_for_class(&to_key);
    if from_key == to_key || to_internal == "java/lang/Object" {
        return true;
    }
    // Lambda proxies implement their SAM interface (and transitively java/lang/Object).
    if from_key.starts_with("$$Lambda$")
        && let Some(lambda_info) = registry.get_lambda(&from_key)
        && (lambda_info.sam_interface == to_key || lambda_info.sam_interface == to_internal)
    {
        return true;
    }
    // Arrays implement Cloneable and Serializable; everything else is Object.
    if from_internal.starts_with('[') {
        return matches!(to_internal, "java/lang/Cloneable" | "java/io/Serializable");
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    queue.push_back(from_key);

    while let Some(current) = queue.pop_front() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if !registry.contains(&current) {
            let _ = registry.ensure_loaded_from(&current, Some(from), loader);
        }
        let (super_class, interfaces) = match registry.get(&current) {
            Ok(ctx) => (ctx.super_class.clone(), ctx.interfaces.clone()),
            Err(_) => continue,
        };
        if let Some(sc) = super_class {
            if sc == to_key {
                return true;
            }
            queue.push_back(sc);
        }
        for iface in interfaces {
            if iface == to_key {
                return true;
            }
            queue.push_back(iface);
        }
    }
    false
}

/// Search a method's exception table for a handler matching the given pc and exception class.
///
/// Uses hierarchy-aware type checking: a `catch(Exception)` will match a thrown
/// `RuntimeException` because `RuntimeException` is a subclass of `Exception`.
fn find_exception_handler(
    exception_table: &[ExceptionEntry],
    pc: usize,
    class_name: &str,
    handler_class: &str,
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
) -> Option<u16> {
    exception_table.iter().find_map(|entry| {
        let in_range = pc >= entry.start_pc as usize && pc < entry.end_pc as usize;
         // match is clearer with &mut registry
        let type_matches = match &entry.catch_type {
            None => true, // catch-all (finally)
            Some(ct) => is_assignable_from(registry, loader, class_name, ct, Some(handler_class)),
        };
        if in_range && type_matches {
            Some(entry.handler_pc)
        } else {
            None
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MethodHierarchyLookup {
    Bytecode(String, usize),
    NativeOverride,
    Missing,
}

fn has_registered_native_override(
    registry: &ClassRegistry,
    class_name: &str,
    method_name: &str,
    method_desc: &str,
) -> bool {
    lookup_registered_native_kind(registry, class_name, method_name, method_desc).is_some()
}

fn lookup_registered_native_kind(
    registry: &ClassRegistry,
    class_name: &str,
    method_name: &str,
    method_desc: &str,
) -> Option<HandlerKind> {
    registry
        .natives()
        .get_kind(class_name, method_name, method_desc)
        .or_else(|| {
            let internal_name = registry.internal_name_for_class(class_name);
            (internal_name != class_name)
                .then(|| {
                    registry
                        .natives()
                        .get_kind(internal_name, method_name, method_desc)
                })
                .flatten()
        })
}

/// Walk the class hierarchy to find a bytecode method by name and descriptor,
/// stopping early if an exact native override owns that slot.
fn resolve_method_in_hierarchy_lookup(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    start_class: &str,
    method_name: &str,
    method_desc: &str,
) -> MethodHierarchyLookup {
    let mut current = if registry.contains(start_class) {
        start_class.to_string()
    } else {
        registry.class_key_from_source(start_class, Some(start_class))
    };
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return MethodHierarchyLookup::Missing; // circular — bail
        }
        if has_registered_native_override(registry, &current, method_name, method_desc) {
            return MethodHierarchyLookup::NativeOverride;
        }
        if !registry.contains(&current) {
            let _ = registry.ensure_loaded_from(&current, Some(start_class), loader);
            current = registry.class_key_from_source(&current, Some(start_class));
        }
        match registry.get(&current) {
            Ok(ctx) => {
                if let Some(idx) = ctx
                    .methods
                    .iter()
                    .position(|m| m.name == method_name && m.descriptor == method_desc)
                {
                    let method = &ctx.methods[idx];
                    if method.is_native {
                        // Declared native in classfile — dispatch through NativeRegistry
                        return MethodHierarchyLookup::NativeOverride;
                    }
                    if !method.is_abstract {
                        return MethodHierarchyLookup::Bytecode(current, idx);
                    }
                    // Abstract method — keep walking to find a concrete implementation
                }
                match &ctx.super_class {
                    Some(s) => current.clone_from(s),
                    None => return MethodHierarchyLookup::Missing,
                }
            }
            Err(_) => return MethodHierarchyLookup::Missing,
        }
    }
}

/// Walk the class hierarchy to find a method by name and descriptor.
/// Returns `(class_name_where_found, method_index)` or `None`.
fn resolve_method_in_hierarchy(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    start_class: &str,
    method_name: &str,
    method_desc: &str,
) -> Option<(String, usize)> {
    match resolve_method_in_hierarchy_lookup(
        registry,
        loader,
        start_class,
        method_name,
        method_desc,
    ) {
        MethodHierarchyLookup::Bytecode(class_name, method_idx) => Some((class_name, method_idx)),
        MethodHierarchyLookup::NativeOverride | MethodHierarchyLookup::Missing => None,
    }
}

/// Resolve a constant pool Methodref to (`class_name`, `method_name`, `descriptor`).
fn resolve_methodref(cp: &[Option<CpEntry>], idx: usize) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(
            CpEntry::Methodref {
                class_index,
                name_and_type_index,
            }
            | CpEntry::InterfaceMethodref {
                class_index,
                name_and_type_index,
            },
        ) => {
            let class_name = match cp.get(class_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    }
                }
                _ => return Err(VmError::InvalidMethodref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    };
                    Ok((class_name, name, desc))
                }
                _ => Err(VmError::InvalidMethodref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidMethodref { index: idx }),
    }
}

/// Count argument slots in a JVM method descriptor like `(ILjava/lang/String;[I)V`.
fn parse_arg_count(descriptor: &str) -> usize {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut count = 0;
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => count += 1,
            '[' => {
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
                count += 1;
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                count += 1;
            }
            _ => {}
        }
    }
    count
}

/// Resolve a constant pool Fieldref to (`class_name`, `field_name`, descriptor).
fn resolve_fieldref(cp: &[Option<CpEntry>], idx: usize) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Fieldref {
            class_index,
            name_and_type_index,
        }) => {
            let class_name = match cp.get(class_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    }
                }
                _ => return Err(VmError::InvalidFieldref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    Ok((class_name, name, desc))
                }
                _ => Err(VmError::InvalidFieldref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidFieldref { index: idx }),
    }
}

/// Resolve a `MethodHandle` CP entry to (`reference_kind`, `class_name`, `method_name`, descriptor).
fn resolve_method_handle(
    cp: &[Option<CpEntry>],
    cp_idx: usize,
) -> VmResult<(u8, String, String, String)> {
    let (kind, ref_idx) = match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::MethodHandle {
            reference_kind,
            reference_index,
        }) => (*reference_kind, reference_index.0 as usize),
        _ => return Err(VmError::InvalidCpIndex { index: cp_idx }),
    };
    let (class_name, method_name, descriptor) = resolve_methodref(cp, ref_idx)?;
    Ok((kind, class_name, method_name, descriptor))
}

/// Resolve a `NameAndType` CP entry to (name, descriptor).
fn resolve_name_and_type(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<(String, String)> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::NameAndType {
            name_index,
            descriptor_index,
        }) => {
            let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(VmError::InvalidCpIndex {
                        index: name_index.0 as usize,
                    });
                }
            };
            let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(VmError::InvalidCpIndex {
                        index: descriptor_index.0 as usize,
                    });
                }
            };
            Ok((name, desc))
        }
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Resolve a CP String entry to its UTF-8 content. Also handles bare Utf8 entries.
fn resolve_cp_string(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::String { string_index }) => {
            match cp.get(string_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => Err(VmError::InvalidCpIndex {
                    index: string_index.0 as usize,
                }),
            }
        }
        Some(CpEntry::Utf8(s)) => Ok(s.clone()),
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Parse argument type descriptors from a JVM method descriptor like `(IZLjava/lang/String;)V`.
/// Returns a Vec of single-char type codes: 'I', 'Z', 'L' (for object refs), '[' (for arrays), etc.
fn parse_arg_types(descriptor: &str) -> Vec<char> {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut types = Vec::with_capacity(params.len());
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => types.push(c),
            '[' => {
                // Skip array dimensions and element type.
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
                types.push('[');
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                types.push('L');
            }
            _ => {}
        }
    }
    types
}

/// Expand method arguments to JVM local variable slot layout.
///
/// In the JVM spec, `long` (`J`) and `double` (`D`) parameters each occupy
/// *two* local variable slots. The second slot is a phantom placeholder so
/// that all subsequent parameters are addressed at the correct index.
/// For example, a static `(JJ)J` method uses `lload_0` / `lload_2`; without
/// expansion, Duke would pack both longs at indices 0 and 1, causing
/// `lload_2` to see `Slot::Int(0)`.
///
/// If the descriptor contains no wide types this is a zero-copy clone.
/// Adapts `args` to match the parameter types in `descriptor`:
/// - Inserts a padding `Slot::Int(0)` after each Long/Double slot.
/// - Unboxes `Slot::Reference(Some(r))` → `Slot::Int/Long/Float/Double` when
///   the corresponding descriptor param type is `I`, `J`, `F`, or `D`.
///   This handles method references like `Integer::sum` used as `BinaryOperator<Integer>`.
fn adapt_args_for_impl_desc(args: &[Slot], descriptor: &str, heap: &duke_gc::Heap) -> Vec<Slot> {
    let param_types = parse_arg_types(descriptor);
    if param_types.is_empty() || args.is_empty() {
        return args.to_vec();
    }
    let mut result = Vec::with_capacity(args.len() + 4);
    for (slot, &type_char) in args.iter().zip(param_types.iter()) {
        let adapted = match (slot, type_char) {
            // Unbox Reference → int
            (Slot::Reference(Some(r)), 'I' | 'B' | 'S' | 'C' | 'Z') => heap
                .get(*r)
                .ok()
                .and_then(|obj| obj.fields.first().copied())
                .filter(|s| matches!(s, Slot::Int(_)))
                .unwrap_or(*slot),
            // Unbox Reference → long
            (Slot::Reference(Some(r)), 'J') => heap
                .get(*r)
                .ok()
                .and_then(|obj| obj.fields.first().copied())
                .filter(|s| matches!(s, Slot::Long(_)))
                .unwrap_or(*slot),
            // Unbox Reference → double
            (Slot::Reference(Some(r)), 'D') => heap
                .get(*r)
                .ok()
                .and_then(|obj| obj.fields.first().copied())
                .filter(|s| matches!(s, Slot::Double(_)))
                .unwrap_or(*slot),
            // Unbox Reference → float
            (Slot::Reference(Some(r)), 'F') => heap
                .get(*r)
                .ok()
                .and_then(|obj| obj.fields.first().copied())
                .filter(|s| matches!(s, Slot::Float(_)))
                .unwrap_or(*slot),
            _ => *slot,
        };
        result.push(adapted);
        if type_char == 'J' || type_char == 'D' {
            result.push(Slot::Int(0)); // wide padding
        }
    }
    // Preserve trailing args beyond descriptor param count (captures).
    if args.len() > param_types.len() {
        result.extend_from_slice(&args[param_types.len()..]);
    }
    result
}

fn expand_args_for_desc(args: &[Slot], descriptor: &str) -> Vec<Slot> {
    let param_types = parse_arg_types(descriptor);
    let wide_count = param_types
        .iter()
        .filter(|&&c| c == 'J' || c == 'D')
        .count();
    if wide_count == 0 || args.is_empty() {
        return args.to_vec();
    }
    let mut result = Vec::with_capacity(args.len() + wide_count);
    for (slot, &type_char) in args.iter().zip(param_types.iter()) {
        result.push(*slot);
        if type_char == 'J' || type_char == 'D' {
            result.push(Slot::Int(0));
        }
    }
    // Preserve any trailing args that exceed the descriptor param count (captures, etc.)
    if args.len() > param_types.len() {
        result.extend_from_slice(&args[param_types.len()..]);
    }
    result
}

/// Pop args from the operand stack and store them in the locals array.
///
/// Wide types (J = long, D = double) occupy two local variable slots in the JVM.
/// For each such type, the value is stored at `local_idx` and `local_idx + 1` is
/// left as the default padding (`Slot::Int(0)`).
///
/// For methods without any wide-type params, this behaves identically to the
/// previous simple sequential assignment.
fn pop_typed_args_into_locals(
    param_types: &[char],
    frame: &mut Frame,
    locals: &mut [Slot],
    start_idx: usize,
) -> VmResult<()> {
    let count = param_types.len();
    let mut args = vec![Slot::Int(0); count];
    for i in (0..count).rev() {
        args[i] = frame.pop()?;
    }
    let mut local_idx = start_idx;
    for (slot, &tc) in args.iter().zip(param_types.iter()) {
        if local_idx < locals.len() {
            locals[local_idx] = *slot;
        }
        local_idx += 1;
        if tc == 'J' || tc == 'D' {
            local_idx += 1; // wide type: skip the padding slot
        }
    }
    Ok(())
}

/// Return the first character of the return type portion of a method descriptor.
fn desc_return_char(desc: &str) -> Option<char> {
    desc.split_once(')').and_then(|(_, ret)| ret.chars().next())
}

/// Autobox a primitive return value when the SAM descriptor expects a reference.
///
/// This is needed for bound method references like `s::length` where the impl
/// returns `int` but the SAM interface (`Supplier.get()`) returns `Object`.
fn autobox_if_needed(
    result: Option<Slot>,
    impl_desc: &str,
    sam_desc: &str,
    heap: &mut duke_gc::Heap,
) -> VmResult<Option<Slot>> {
    let Some(slot) = result else {
        return Ok(None);
    };
    if !matches!(desc_return_char(sam_desc), Some('L' | '[')) {
        return Ok(Some(slot));
    }
    match (desc_return_char(impl_desc), slot) {
        (Some('I'), Slot::Int(v)) => {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Int(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        (Some('Z'), Slot::Int(v)) => {
            let r = heap.allocate("java/lang/Boolean".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Int(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        (Some('J'), Slot::Long(v)) => {
            let r = heap.allocate("java/lang/Long".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Long(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        (Some('D'), Slot::Double(v)) => {
            let r = heap.allocate("java/lang/Double".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Double(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        (Some('F'), Slot::Float(v)) => {
            let r = heap.allocate("java/lang/Float".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Float(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        _ => Ok(Some(slot)),
    }
}

fn parse_arg_descriptors(descriptor: &str) -> Vec<String> {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut descriptors = Vec::with_capacity(params.len());
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => descriptors.push(c.to_string()),
            '[' => {
                let mut desc = String::from("[");
                while chars.peek() == Some(&'[') {
                    desc.push(chars.next().unwrap_or('['));
                }
                if chars.peek() == Some(&'L') {
                    desc.push(chars.next().unwrap_or('L'));
                    for c2 in chars.by_ref() {
                        desc.push(c2);
                        if c2 == ';' {
                            break;
                        }
                    }
                } else if let Some(elem) = chars.next() {
                    desc.push(elem);
                }
                descriptors.push(desc);
            }
            'L' => {
                let mut desc = String::from("L");
                for c2 in chars.by_ref() {
                    desc.push(c2);
                    if c2 == ';' {
                        break;
                    }
                }
                descriptors.push(desc);
            }
            _ => {}
        }
    }
    descriptors
}

/// Index of a named instance field within ctx.fields (non-static only).
/// Return the correct default [`Slot`] for a field with the given JVM descriptor.
///
/// Per JVMS §2.3/2.4: numeric types default to 0, reference/array types to null.
#[inline]
fn default_slot_for_descriptor(desc: &str) -> Slot {
    match desc.chars().next() {
        Some('J') => Slot::Long(0),
        Some('F') => Slot::Float(0.0),
        Some('D') => Slot::Double(0.0),
        Some('L' | '[') => Slot::Reference(None),
        _ => Slot::Int(0), // I, Z, B, C, S
    }
}

/// reference-typed fields (`L…;` / `[…`), which must be `Reference(None)`.
/// Sum a class's instance fields across its full superclass chain.
fn total_instance_field_count(registry: &ClassRegistry, class_name: &str) -> usize {
    let mut count = registry
        .get(class_name)
        .map(|c| c.instance_field_count)
        .unwrap_or(0);
    let mut sc = registry
        .get(class_name)
        .ok()
        .and_then(|c| c.super_class.clone());
    while let Some(ref s) = sc {
        match registry.get(s) {
            Ok(sctx) => {
                count += sctx.instance_field_count;
                sc = sctx.super_class.clone();
            }
            Err(_) => break,
        }
    }
    count
}

/// Allocate a heap exception object for a native-thrown Java exception.
///
/// Native handlers currently surface Java exceptions as class names. Catch
/// blocks need an object reference on the operand stack, so we materialize a
/// minimal heap object of that class before routing through exception-table
/// dispatch.
fn materialize_java_exception_object(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    class_name: &str,
) -> VmResult<u64> {
    registry.ensure_loaded(class_name, loader)?;
    let exc_ref = heap.allocate(
        class_name.to_string(),
        total_instance_field_count(registry, class_name),
    );
    init_object_fields(registry, heap, exc_ref, class_name);
    Ok(exc_ref)
}

fn allocate_reflection_instance(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    class: &str,
) -> VmResult<u64> {
    if !registry.contains(class) {
        registry.ensure_loaded(class, loader)?;
    }
    let class_key = registry.resolve_loaded_class_key(class)?;
    ensure_initialized(registry, loader, heap, output, &class_key, &class_key)?;
    let object_ref = heap.allocate(
        class_key.clone(),
        total_instance_field_count(registry, &class_key),
    );
    init_object_fields(registry, heap, object_ref, &class_key);
    Ok(object_ref)
}

/// After allocating an object on the heap, initialize each field slot to the
/// JVM-spec default for its descriptor.
///
/// `heap.allocate` sets all slots to `Slot::Int(0)`, which is wrong for
/// reference-typed fields (`L...;` / `[...]`), which must be `Reference(None)`.
/// This function walks the full class hierarchy (Object-first) and writes the
/// correct default into every slot that differs from `Int(0)`.
fn init_object_fields(
    registry: &ClassRegistry,
    heap: &mut duke_gc::Heap,
    obj_ref: u64,
    class_name: &str,
) {
    // Collect hierarchy: class_name → … → root
    let mut chain: Vec<String> = Vec::new();
    let mut cur = Some(class_name.to_string());
    while let Some(cls) = cur {
        if let Ok(ctx) = registry.get(&cls) {
            let sc = ctx.super_class.clone();
            chain.push(cls);
            cur = sc;
        } else {
            break;
        }
    }
    chain.reverse(); // Object-first

    let mut slot_idx = 0usize;
    for cls in &chain {
        if let Ok(ctx) = registry.get(cls) {
            for field in ctx.fields.iter().filter(|f| !f.is_static) {
                let default = default_slot_for_descriptor(&field.descriptor);
                // Only write non-Int-zero defaults (avoids an unnecessary mut borrow).
                if !matches!(default, Slot::Int(0))
                    && let Ok(obj) = heap.get_mut(obj_ref)
                    && slot_idx < obj.fields.len()
                {
                    obj.fields[slot_idx] = default;
                }
                slot_idx += 1;
            }
        }
    }
}

/// Compute the absolute slot index of a named instance field within a heap
/// object whose class is `target_class` (or any subclass of it).
///
/// JVM `Fieldref` entries name the access class (often a subclass), not
/// necessarily the declaring class. This function walks the full hierarchy
/// from the root (Object) down to `target_class`, searching each class for
/// the field and accumulating the running slot offset as it goes.
///
/// Layout: root fields occupy the lowest-numbered slots; each subclass
/// appends its fields immediately after its superclass's fields.
fn field_slot_idx(registry: &ClassRegistry, target_class: &str, name: &str) -> VmResult<usize> {
    let mut current = target_class;

    while let Ok(ctx) = registry.get(current) {
        if let Some(local_idx) = ctx
            .fields
            .iter()
            .filter(|f| !f.is_static)
            .position(|f| f.name == name)
        {
            let super_fields = ctx
                .super_class
                .as_deref()
                .map_or(0, |sc| total_instance_field_count(registry, sc));
            return Ok(super_fields + local_idx);
        }

        if let Some(ref sc) = ctx.super_class {
            current = sc;
        } else {
            break;
        }
    }

    Err(VmError::InvalidFieldref { index: 0 })
}

/// Index of a named static field within `ctx.static_fields`.
fn static_field_idx(ctx: &ClassContext, name: &str) -> VmResult<usize> {
    ctx.fields
        .iter()
        .filter(|f| f.is_static)
        .position(|f| f.name == name)
        .ok_or(VmError::InvalidFieldref { index: 0 })
}

// ---------------------------------------------------------------------------
// StringBuilder natives
// ---------------------------------------------------------------------------





















// ---- StringBuilder extended operations ----

















// Character natives
// ---------------------------------------------------------------------------

/// Helper: extract a `char` from a `Slot::Int` argument.
fn slot_to_char(slot: &Slot) -> VmResult<char> {
    match slot {
        Slot::Int(v) => Ok(char::from_u32((*v).cast_unsigned()).unwrap_or('\0')),
        _ => Err(VmError::TypeMismatch {
            expected: "Int (char)",
            got: "other",
        }),
    }
}





















// ---------------------------------------------------------------------------
// ArrayList natives
// ---------------------------------------------------------------------------













fn collection_elements_from_ref(heap: &duke_gc::Heap, collection_ref: u64) -> VmResult<Vec<Slot>> {
    let collection = heap.get(collection_ref)?;
    match collection.class_name.as_str() {
        "java/util/ArrayList" | "java/util/HashSet" => {
            let size = match collection.fields.first() {
                Some(Slot::Int(size)) if *size >= 0 => usize::try_from(*size).unwrap_or(0),
                _ => 0,
            };
            Ok(collection
                .fields
                .iter()
                .skip(1)
                .take(size)
                .copied()
                .collect())
        }
        _ => Err(VmError::TypeMismatch {
            expected: "java/util/Collection",
            got: "other",
        }),
    }
}

fn allocate_reference_array_from_slots(
    heap: &mut duke_gc::Heap,
    array_class_name: &str,
    elements: &[Slot],
) -> VmResult<u64> {
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





/// Native: `ArrayList.sort(Comparator)V` — sorts in-place using insertion sort,
/// calling `compareTo` on each element pair via the interpreter callback.
///
/// Only null Comparator (natural ordering via `compareTo`) is supported.
pub(crate) fn array_list_sort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // args[0] = ArrayList ref, args[1] = Comparator (null = natural ordering)
    let list_ref = extract_ref_arg(args, 0)?;

    // args[1] = optional Comparator ref (null = natural ordering via compareTo)
    let comparator = args.get(1).copied();

    // Fix 2: guard against a negative size stored in fields[0].
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(VmError::NegativeArraySize { size: *n }),
        _ => return Ok(None),
    };

    if size <= 1 {
        return Ok(None);
    }

    // Collect element refs (fields[1..=size]).
    let mut elems: Vec<u64> = (1..=size)
        .filter_map(|i| match heap.get(list_ref).ok()?.fields.get(i) {
            Some(Slot::Reference(Some(r))) => Some(*r),
            _ => None,
        })
        .collect();

    // Fix 3: malformed list → InvalidRef, not silent Ok(None).
    if elems.len() != size {
        return Err(VmError::InvalidRef { address: list_ref });
    }

    // Insertion sort — O(n²), correct, easy to verify.
    for i in 1..elems.len() {
        let mut key = elems[i];
        let mut j = i;
        while j > 0 {
            let receiver = elems[j - 1];
            let cmp = if let Some(Slot::Reference(Some(comp_ref))) = comparator {
                let comp_class = heap.get(comp_ref)?.class_name.clone();
                ops.invoke(
                    heap,
                    output,
                    &comp_class,
                    "compare",
                    "(Ljava/lang/Object;Ljava/lang/Object;)I",
                    vec![
                        Slot::Reference(Some(comp_ref)),
                        Slot::Reference(Some(receiver)),
                        Slot::Reference(Some(key)),
                    ],
                )?
            } else {
                let class_name = heap.get(receiver)?.class_name.clone();
                ops.invoke(
                    heap,
                    output,
                    &class_name,
                    COMPARE_TO_METHOD,
                    COMPARE_TO_OBJECT_DESC,
                    vec![Slot::Reference(Some(receiver)), Slot::Reference(Some(key))],
                )?
            };

            // Patch stale young-gen refs if a minor GC fired during the callback.
            // Guarded by `has_pending_forwards` so the common (no-GC) path pays
            // only one bool check instead of O(n) HashMap probes.
            if heap.has_pending_forwards() {
                for elem in &mut elems {
                    let mut slot = Slot::Reference(Some(*elem));
                    heap.apply_forward(&mut slot);
                    if let Slot::Reference(Some(r)) = slot {
                        *elem = r;
                    }
                }
                // Re-read key after forwarding patch (it lives outside elems
                // during the innermost loop iteration).
                let mut key_slot = Slot::Reference(Some(key));
                heap.apply_forward(&mut key_slot);
                if let Slot::Reference(Some(r)) = key_slot {
                    key = r;
                }
            }

            // Fix 4: explicit error on non-Int compareTo return.
            match cmp {
                Some(Slot::Int(n)) if n <= 0 => break,
                Some(Slot::Int(_)) => {} // n > 0, keep shifting
                _ => {
                    return Err(VmError::TypeMismatch {
                        expected: "Int",
                        got: "other",
                    });
                }
            }
            elems[j] = elems[j - 1];
            j -= 1;
        }
        elems[j] = key;
    }

    // Write sorted elements back using the write barrier.
    for (i, &r) in elems.iter().enumerate() {
        heap.write_field(list_ref, i + 1, Slot::Reference(Some(r)))?;
    }

    Ok(None)
}

// ---------------------------------------------------------------------------
// Collections natives
// ---------------------------------------------------------------------------



// ---------------------------------------------------------------------------
// ArrayListIterator natives
// ---------------------------------------------------------------------------









// ---- ArrayList extended methods ----



















// ---- HashMap extended methods ----







// ---- Double.isNaN ----





// ---- Arrays natives ----













// ---------------------------------------------------------------------------
// Phase 44: Arrays.copyOfRange, List.subList, Comparator.reversed,
//           Collections.binarySearch, String.intern, ArrayList.removeIf
// ---------------------------------------------------------------------------

















// ---- Arrays.asList ----



// ---- String extended operations (Java 11+) ----























// ---------------------------------------------------------------------------
// java.util.Random — 48-bit LCG (same multiplier/addend as Java's java.util.Random)
// fields[0] = Long(seed as i64); all bit-level casts are intentional.
// ---------------------------------------------------------------------------


const RANDOM_MULTIPLIER: u64 = 25_214_903_917; // 0x5DEECE66D — Java's LCG multiplier
const RANDOM_ADDEND: u64 = 0xB;
const RANDOM_MASK: u64 = (1u64 << 48) - 1;

/// Advance the LCG and return `bits` high bits of the new state.
const fn random_next(seed: u64, bits: u32) -> (u64, i32) {
    let new_seed = seed
        .wrapping_mul(RANDOM_MULTIPLIER)
        .wrapping_add(RANDOM_ADDEND)
        & RANDOM_MASK;
    let value = (new_seed >> (48 - bits)) as i32; // intentional truncation to bit pattern
    (new_seed, value)
}



/// Native: `Random.<init>(J)V` — seed with explicit long value.
/// Retrieve and advance seed from `fields[0]`, returning new seed and `bits` high bits.
fn random_step(heap: &mut duke_gc::Heap, this_ref: u64, bits: u32) -> VmResult<(u64, i32)> {
    let old_seed = match heap.get(this_ref)?.fields.first().copied() {
        Some(Slot::Long(v)) => v as u64,
        _ => 0,
    };
    let (new_seed, value) = random_next(old_seed, bits);
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(new_seed as i64); // safe: < 2^48
    Ok((new_seed, value))
}



/// Native: `Random.nextInt(I)I` — bounded random int [0, bound).

// ---------------------------------------------------------------------------
// java.util.regex.Pattern / Matcher
// Pattern: string_value = regex string.
// Matcher: fields[0]=Pattern ref, fields[1]=input ref, fields[2]=pos,
//          fields[3]=match_start (-1=no match), fields[4]=match_end;
//          string_value = last matched text.
// ---------------------------------------------------------------------------

/// Helper: compile a regex from a pattern string.
/// Returns `Err` with `JavaException` on bad pattern.
fn compile_java_regex(pattern: &str) -> VmResult<regex::Regex> {
    regex::Regex::new(pattern).map_err(|e| duke_runtime::VmError::JavaException {
        class_name: format!("java/util/regex/PatternSyntaxException: {e}"),
    })
}





























// ---------------------------------------------------------------------------
// Optional extensions (callback-based)
// ---------------------------------------------------------------------------











// ---------------------------------------------------------------------------
// HashMap extensions: compute, merge
// ---------------------------------------------------------------------------

/// Helper: find key index in `HashMap` fields (`fields[0]`=size, `fields[1,3,5..]`=keys, `fields[2,4,6..]`=vals).
fn hashmap_find_key(fields: &[Slot], key: Slot, heap: &duke_gc::Heap) -> Option<usize> {
    let size = match fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return None,
    };
    for i in 0..size {
        let key_slot = fields.get(1 + i * 2)?;
        if slots_equal(key_slot, &key, heap) {
            return Some(1 + i * 2);
        }
    }
    None
}





// ---------------------------------------------------------------------------
// java.lang.StringBuffer — mutable string, delegates to StringBuilder internals
// (string_value field used as buffer, same as StringBuilder)
// ---------------------------------------------------------------------------











// ---------------------------------------------------------------------------
// java.util.StringJoiner
// fields[0] = Reference(delimiter), fields[1] = Reference(prefix), fields[2] = Reference(suffix)
// fields[3] = Reference(emptyValue), fields[4..] = added elements
// instance_field_count = 4 (slots 0-3 pre-allocated)
// ---------------------------------------------------------------------------









/// Helper: read a string from a slot (returns "" for null).
fn slot_to_string(slot: Slot, heap: &duke_gc::Heap) -> String {
    match slot {
        Slot::Reference(Some(r)) => heap
            .get(r)
            .ok()
            .and_then(|o| o.string_value.clone())
            .unwrap_or_default(),
        _ => String::new(),
    }
}





// ---------------------------------------------------------------------------
// HashMap natives
// ---------------------------------------------------------------------------

fn uses_first_field_value_equality(class_name: &str) -> bool {
    matches!(
        class_name,
        "java/lang/Boolean"
            | "java/lang/Byte"
            | "java/lang/Character"
            | "java/lang/Double"
            | "java/lang/Float"
            | "java/lang/Integer"
            | "java/lang/Long"
            | "java/lang/Short"
    )
}

/// Semantic equality for `HashMap` keys: reference identity by default, with value
/// semantics for strings, class mirrors, and boxed primitive wrappers.
fn slots_equal(a: &Slot, b: &Slot, heap: &duke_gc::Heap) -> bool {
    match (a, b) {
        (Slot::Reference(None), Slot::Reference(None)) => true,
        (Slot::Reference(Some(ra)), Slot::Reference(Some(rb))) => {
            if ra == rb {
                return true;
            }
            let Ok(oa) = heap.get(*ra) else { return false };
            let Ok(ob) = heap.get(*rb) else { return false };
            if oa.class_name != ob.class_name {
                return false;
            }
            match oa.class_name.as_str() {
                "java/lang/String" | "java/lang/Class" => oa.string_value == ob.string_value,
                class_name if uses_first_field_value_equality(class_name) => {
                    oa.fields.first() == ob.fields.first()
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// Native: `HashMap.<init>()V` — initialises size counter at fields\[0\] to 0.
fn find_hashmap_entry_index(fields: &[Slot], key: &Slot, heap: &duke_gc::Heap) -> Option<usize> {
    (1..fields.len())
        .step_by(2)
        .find(|&i| i + 1 < fields.len() && slots_equal(&fields[i], key, heap))
}

















// ---------------------------------------------------------------------------
// HashSet natives
// ---------------------------------------------------------------------------



fn find_hashset_entry_index(
    fields: &[Slot],
    element: &Slot,
    heap: &duke_gc::Heap,
) -> Option<usize> {
    fields
        .iter()
        .skip(1)
        .position(|field| slots_equal(field, element, heap))
        .map(|idx| idx + 1)
}































const PROCESS_ID_FIELD: usize = 0;
const PROCESS_STDIN_FIELD: usize = 1;
const PROCESS_STDOUT_FIELD: usize = 2;
const PROCESS_STDERR_FIELD: usize = 3;

fn string_array_from_slot(slot: Slot, heap: &duke_gc::Heap) -> VmResult<Vec<String>> {
    let Slot::Reference(Some(array_ref)) = slot else {
        return Err(VmError::NullPointerException);
    };
    let elements = heap.get(array_ref)?.fields.clone();
    elements
        .into_iter()
        .map(|element| match element {
            Slot::Reference(Some(string_ref)) => string_value_from_ref(heap, string_ref),
            _ => Err(VmError::NullPointerException),
        })
        .collect()
}

fn optional_file_path_from_slot(
    slot: Slot,
    heap: &duke_gc::Heap,
) -> VmResult<Option<std::path::PathBuf>> {
    match slot {
        Slot::Reference(Some(file_ref)) => Ok(Some(file_path_from_ref(file_ref, heap)?)),
        Slot::Reference(None) => Ok(None),
        _ => Err(VmError::NullPointerException),
    }
}

fn allocate_process_impl(
    heap: &mut duke_gc::Heap,
    ids: duke_gc::SpawnedProcessIds,
) -> VmResult<Option<Slot>> {
    let process_ref = heap.allocate("java/lang/ProcessImpl".to_string(), 4);
    let process_obj = heap.get_mut(process_ref)?;
    process_obj.fields[PROCESS_ID_FIELD] = Slot::Int(ids.process_id);
    process_obj.fields[PROCESS_STDIN_FIELD] = Slot::Int(ids.stdin_id);
    process_obj.fields[PROCESS_STDOUT_FIELD] = Slot::Int(ids.stdout_id);
    process_obj.fields[PROCESS_STDERR_FIELD] = Slot::Int(ids.stderr_id);
    Ok(Some(Slot::Reference(Some(process_ref))))
}

fn spawn_process_impl(
    heap: &mut duke_gc::Heap,
    command: &[String],
    cwd: Option<&std::path::Path>,
) -> VmResult<Option<Slot>> {
    let ids = heap.spawn_host_process(command, cwd)?;
    allocate_process_impl(heap, ids)
}

fn process_field_id_from_this(
    args: &[Slot],
    heap: &duke_gc::Heap,
    field_idx: usize,
) -> VmResult<i32> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.get(field_idx) {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(VmError::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

fn allocate_process_stream(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    handle_id: i32,
) -> VmResult<Option<Slot>> {
    let stream_ref = heap.allocate(class_name.to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(handle_id);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

























// ---------------------------------------------------------------------------
// GC root gathering
// ---------------------------------------------------------------------------

/// Collect all live Slot values from the interpreter's current execution state.
/// The GC uses these as the root set for reachability analysis.
fn gather_roots(
    frame: &duke_runtime::Frame,
    call_stack: &[CallFrame],
    registry: &ClassRegistry,
) -> Vec<duke_runtime::Slot> {
    let mut roots = Vec::new();
    roots.extend(frame.slots());
    for cf in call_stack {
        roots.extend(cf.frame.slots());
    }
    for ctx in registry.all_classes() {
        roots.extend(ctx.static_fields.iter().copied());
    }
    roots
}

/// Apply GC forwarding pointers to all live interpreter slots after a minor
/// collection. Must be called immediately after `heap.collect()` returns so
/// that stale young-gen references are updated to their new locations.
fn patch_forwarded_slots(
    frame: &mut duke_runtime::Frame,
    call_stack: &mut [CallFrame],
    registry: &mut ClassRegistry,
    heap: &duke_gc::Heap,
) {
    for slot in frame.slots_mut() {
        heap.apply_forward(slot);
    }
    for cf in call_stack.iter_mut() {
        for slot in cf.frame.slots_mut() {
            heap.apply_forward(slot);
        }
    }
    for ctx in registry.all_classes_mut() {
        for slot in &mut ctx.static_fields {
            heap.apply_forward(slot);
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 48: Stream.generate/iterate/concat/empty
// ---------------------------------------------------------------------------









// ---------------------------------------------------------------------------
// Phase 46: Stream.takeWhile/dropWhile (Java 9)
// ---------------------------------------------------------------------------





// ---------------------------------------------------------------------------
// Phase 45: ArrayList.forEach, Stream.sorted(Comparator), Arrays.toString,
//           HashMap.replace, Collections.swap/unmodifiableMap,
//           Collectors.partitioningBy, IntStream.sorted
// ---------------------------------------------------------------------------





















// ---------------------------------------------------------------------------
// Phase 49: Comparator.thenComparing, Predicate combinators, Function combinators,
//           Stream.mapToLong, Stream.mapToDouble
// ---------------------------------------------------------------------------





/// Helper: dispatch `comparator.compare(a, b)` via ops.invoke.
fn invoke_comparator(
    comparator: Slot,
    a: Slot,
    b: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> VmResult<i32> {
    let Slot::Reference(Some(cmp_ref)) = comparator else {
        return Ok(0);
    };
    let cmp_class = heap.get(cmp_ref)?.class_name.clone();
    let res = ops
        .invoke(
            heap,
            out,
            &cmp_class,
            "compare",
            "(Ljava/lang/Object;Ljava/lang/Object;)I",
            vec![comparator, a, b],
        )?
        .unwrap_or(Slot::Int(0));
    Ok(match res {
        Slot::Int(n) => n,
        _ => 0,
    })
}













/// Helper: dispatch `predicate.test(elem)` via ops.invoke, returns bool.
fn invoke_predicate_test(
    predicate: Slot,
    elem: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> VmResult<bool> {
    let Slot::Reference(Some(pred_ref)) = predicate else {
        return Ok(false);
    };
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let res = ops
        .invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![predicate, elem],
        )?
        .unwrap_or(Slot::Int(0));
    Ok(matches!(res, Slot::Int(n) if n != 0))
}









fn invoke_consumer_accept(
    consumer: Slot,
    arg: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> VmResult<()> {
    let Slot::Reference(Some(c_ref)) = consumer else {
        return Ok(());
    };
    let c_class = heap.get(c_ref)?.class_name.clone();
    ops.invoke(
        heap,
        out,
        &c_class,
        "accept",
        "(Ljava/lang/Object;)V",
        vec![consumer, arg],
    )?;
    Ok(())
}





/// Helper: dispatch `function.apply(input)` via ops.invoke.
fn invoke_function_apply(
    function: Slot,
    input: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> VmResult<Slot> {
    let Slot::Reference(Some(fn_ref)) = function else {
        return Ok(Slot::Reference(None));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    Ok(ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![function, input],
        )?
        .unwrap_or(Slot::Reference(None)))
}







/// Allocates a `duke/util/LongStream` with `fields[0]=Int(size), fields[1..n]=Long(value)`.
fn make_long_stream(heap: &mut duke_gc::Heap, values: Vec<i64>) -> u64 {
    let r = heap.allocate("duke/util/LongStream".to_string(), 1);
    if let Ok(obj) = heap.get_mut(r) {
        obj.fields[0] = Slot::Int(i32::try_from(values.len()).unwrap_or(0));
        for v in values {
            obj.fields.push(Slot::Long(v));
        }
    }
    r
}





/// Allocates a `duke/util/DoubleStream` with `fields[0]=Int(size), fields[1..n]=Double(value)`.
fn make_double_stream(heap: &mut duke_gc::Heap, values: Vec<f64>) -> u64 {
    let r = heap.allocate("duke/util/DoubleStream".to_string(), 1);
    if let Ok(obj) = heap.get_mut(r) {
        obj.fields[0] = Slot::Int(i32::try_from(values.len()).unwrap_or(0));
        for v in values {
            obj.fields.push(Slot::Double(v));
        }
    }
    r
}



// ---------------------------------------------------------------------------
// Phase 51: LongStream full ops, DoubleStream full ops,
//           IntStream.asLongStream/asDoubleStream, Collectors.summingInt/averagingInt
// ---------------------------------------------------------------------------

// ---- Helper extractors ----

/// Extract long elements from a `duke/util/LongStream`.
fn long_stream_elems(heap: &duke_gc::Heap, ref_: u64) -> Vec<i64> {
    let size = match heap.get(ref_).ok().and_then(|o| o.fields.first().copied()) {
        Some(Slot::Int(n)) => usize::try_from(n).unwrap_or(0),
        _ => 0,
    };
    heap.get(ref_)
        .ok()
        .map(|o| {
            o.fields[1..=size]
                .iter()
                .filter_map(|s| {
                    if let Slot::Long(n) = s {
                        Some(*n)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Extract double elements from a `duke/util/DoubleStream`.
fn double_stream_elems(heap: &duke_gc::Heap, ref_: u64) -> Vec<f64> {
    let size = match heap.get(ref_).ok().and_then(|o| o.fields.first().copied()) {
        Some(Slot::Int(n)) => usize::try_from(n).unwrap_or(0),
        _ => 0,
    };
    heap.get(ref_)
        .ok()
        .map(|o| {
            o.fields[1..=size]
                .iter()
                .filter_map(|s| {
                    if let Slot::Double(d) = s {
                        Some(*d)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Allocate an `OptionalLong`: `fields[0]=Long(value)`, `fields[1]=Int(present)`.
fn make_optional_long(heap: &mut duke_gc::Heap, value: Option<i64>) -> u64 {
    let r = heap.allocate("duke/util/OptionalLong".to_string(), 2);
    if let Ok(obj) = heap.get_mut(r) {
        if let Some(v) = value {
            obj.fields[0] = Slot::Long(v);
            obj.fields[1] = Slot::Int(1);
        } else {
            obj.fields[1] = Slot::Int(0);
        }
    }
    r
}

/// Allocate an `OptionalDouble` (for LongStream/DoubleStream average/min/max).
fn make_optional_double_val(heap: &mut duke_gc::Heap, value: Option<f64>) -> u64 {
    let r = heap.allocate("duke/util/OptionalDouble".to_string(), 2);
    if let Ok(obj) = heap.get_mut(r) {
        if let Some(v) = value {
            obj.fields[0] = Slot::Double(v);
            obj.fields[1] = Slot::Int(1);
        } else {
            obj.fields[1] = Slot::Int(0);
        }
    }
    r
}

// ---- LongStream static factories ----







// ---- LongStream terminal ops ----



















// ---- LongStream intermediate ops ----







// ---- LongStream.mapToInt / mapToDouble ----



// ---- DoubleStream static factories ----





// ---- DoubleStream terminal ops ----













// ---- DoubleStream intermediate ops ----





// ---- IntStream.asLongStream / asDoubleStream ----





// ---- OptionalLong ----





// ---- Collectors.summingInt / averagingInt ----





// ---------------------------------------------------------------------------
// Phase 53: IntStream/LongStream terminal ops, Comparator.comparingLong,
//           Optional.or / ifPresentOrElse, Collectors.toUnmodifiable*
// ---------------------------------------------------------------------------























// ---------------------------------------------------------------------------
// Phase 58: Comparator.comparingDouble, Map.copyOf/entry/ofEntries,
//           Collections.singletonMap/singletonSet/unmodifiableSet
// ---------------------------------------------------------------------------

















// ---------------------------------------------------------------------------
// Phase 59: Stream.flatMapToInt/Long/Double, Collectors.toMap (3-arg),
//           forEach on HashSet/TreeSet/TreeMap/LinkedList/LinkedHashMap/PriorityQueue
// ---------------------------------------------------------------------------





















// ---------------------------------------------------------------------------
// Phase 60: IntStream/LongStream/DoubleStream takeWhile/dropWhile,
//           Integer/Long/Double compare/max/min,
//           TreeMap.keySet/values/getOrDefault, TreeSet.stream
// ---------------------------------------------------------------------------















































// ---------------------------------------------------------------------------
// Phase 54: IntStream/LongStream/DoubleStream limit/skip,
//           IntStream/LongStream flatMap, Collectors.mapping
// ---------------------------------------------------------------------------





















// ---------------------------------------------------------------------------
// Phase 55: Complete DoubleStream, LongStream gaps, Collectors.summingLong/averagingDouble
// ---------------------------------------------------------------------------

































// ---------------------------------------------------------------------------
// Phase 56: Collectors.minBy/maxBy, summingDouble, averagingLong,
//           toUnmodifiableMap, collectingAndThen
// ---------------------------------------------------------------------------















// ---------------------------------------------------------------------------
// Phase 57: Collectors.reducing, Stream.iterate predicate, Optional.stream,
//           ArrayDeque completion
// ---------------------------------------------------------------------------



































// ---------------------------------------------------------------------------
// Phase 62: java.time (LocalDate, LocalDateTime, Instant, Duration, Period)
// ---------------------------------------------------------------------------

/// Convert (year, month, day) to a proleptic Gregorian epoch day count.
/// Day 0 = 1970-01-01.  Howard Hinnant's branchless algorithm.
fn ymd_to_epoch_days(year: i32, month: u32, day: u32) -> i32 {
    let (y, m, d) = (year as i64, month as i64, day as i64);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400); // year of era [0, 399]
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // day of era [0, 146096]
    (era * 146_097 + doe - 719_468) as i32
}

/// Convert a proleptic Gregorian epoch day to (year, month, day).
fn epoch_days_to_ymd(epoch_days: i32) -> (i32, u32, u32) {
    let z = epoch_days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097); // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

// ---- LocalDate layout: fields[0] = Slot::Int(epoch_days) ----





























/// Helper: days in a given month of a given year (handles leap years).
const fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30, // months 4,6,9,11 + any invalid input
    }
}

// ---- Duration layout: fields[0]=Slot::Long(seconds), fields[1]=Slot::Int(nanos_adj) ----



























// ---- Period layout: fields[0]=years(Int), fields[1]=months(Int), fields[2]=days(Int) ----



















// ---- Instant layout: fields[0]=Slot::Long(epoch_seconds), fields[1]=Slot::Int(nanos_adj) ----













// ---------------------------------------------------------------------------
// Phase 63: java.time.LocalDateTime
// Layout: fields[0]=epoch_days(Int), fields[1]=hour(Int),
//         fields[2]=minute(Int), fields[3]=second(Int), fields[4]=nano(Int)
// ---------------------------------------------------------------------------

































// ---------------------------------------------------------------------------
// Phase 64: String.indent, StringBuilder.setCharAt, Collections.disjoint,
//           HashMap.computeIfPresent
// ---------------------------------------------------------------------------









// ---------------------------------------------------------------------------
// Phase 117: Map.remove(k,v), Collections.emptyList (immutable), Stream.concat,
//            Map.replace, Integer.sum, IntStream.mapToObj, Optional.map,
//            UnmodifiableSet
// ---------------------------------------------------------------------------
