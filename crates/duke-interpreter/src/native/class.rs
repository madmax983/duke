pub(crate) fn native_class_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let name_ref = heap.allocate_string(internal_name_to_binary_name(&internal_name));
    Ok(Some(Slot::Reference(Some(name_ref))))
}

pub(crate) fn native_class_get_package_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let package_name = if internal_name.starts_with('[') {
        String::new()
    } else {
        internal_name
            .rsplit_once('/')
            .map_or_else(String::new, |(package, _)| package.replace('/', "."))
    };
    let package_ref = heap.allocate_string(package_name);
    Ok(Some(Slot::Reference(Some(package_ref))))
}

/// Native: `Class.desiredAssertionStatus()` - Duke currently runs with assertions disabled.
pub(crate) fn native_class_desired_assertion_status(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(0)))
}

pub(crate) fn native_class_for_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let binary_name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .ok_or(Error::NullPointerException)?;
    let internal_name = binary_name_to_internal_name(&binary_name);
    match ops.ensure_loaded(&internal_name) {
        Ok(()) => {
            let class_key = ops.class_key_for_loaded_class(&internal_name)?;
            let class_ref = allocate_class_object(heap, &class_key)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        Err(Error::ClassNotFound { .. }) => Err(Error::JavaException {
            class_name: "java/lang/ClassNotFoundException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_class_for_name_with_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let binary_name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .ok_or(Error::NullPointerException)?;
    let internal_name = binary_name_to_internal_name(&binary_name);
    let (load_result, class_key) = match args.get(2) {
        Some(Slot::Reference(Some(loader_ref))) => (
            ops.ensure_loaded_with_runtime_loader(heap, *loader_ref, &internal_name),
            ops.class_key_for_runtime_loader(heap, *loader_ref, &internal_name)?,
        ),
        Some(Slot::Reference(None)) | None => (
            ops.ensure_loaded(&internal_name),
            ops.class_key_for_loaded_class(&internal_name)?,
        ),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    match load_result {
        Ok(()) => {
            let class_ref = allocate_class_object(heap, &class_key)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        Err(Error::ClassNotFound { .. }) => Err(Error::JavaException {
            class_name: "java/lang/ClassNotFoundException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_class_get_class_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    Ok(Some(Slot::Reference(
        ops.runtime_loader_for_class(&class_key)?,
    )))
}

fn allocate_resource_url(heap: &mut duke_gc::Heap, url: String) -> Result<u64> {
    allocate_string_backed_object(heap, "java/net/URL", url)
}

fn allocate_resource_enumeration(
    heap: &mut duke_gc::Heap,
    resources: Vec<duke_loader::LocatedResource>,
) -> Result<u64> {
    let enum_ref = heap.allocate(
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

fn lookup_class_resource(
    args: &[Slot],
    heap: &duke_gc::Heap,
    ops: &mut dyn CallbackOps,
) -> Result<(String, String, String, Option<duke_loader::LocatedResource>)> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let class_internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let loader_ref = ops.runtime_loader_for_class(&class_key)?;
    let requested_name = string_arg(args, 1, heap)?;
    let base = class_resource_base(&class_internal_name);
    let classpath = classpath_debug_label(loader_ref);
    let Some(resolved_name) = resolve_class_resource_name(&class_internal_name, &requested_name) else {
        log_resource_lookup_miss(&requested_name, &base, &classpath);
        return Ok((requested_name, base, classpath, None));
    };
    let resource = ops.find_resource_entry(heap, loader_ref, &resolved_name)?;
    if resource.is_none() {
        log_resource_lookup_miss(&resolved_name, &base, &classpath);
    }
    Ok((resolved_name, base, classpath, resource))
}

fn lookup_class_loader_resource(
    args: &[Slot],
    heap: &duke_gc::Heap,
    ops: &mut dyn CallbackOps,
) -> Result<(String, String, String, Option<duke_loader::LocatedResource>)> {
    let loader_ref = extract_ref_arg(args, 0)?;
    let requested_name = string_arg(args, 1, heap)?;
    let base = "<class-loader>".to_string();
    let classpath = classpath_debug_label(Some(loader_ref));
    let Some(resolved_name) = normalize_resource_name(&requested_name) else {
        log_resource_lookup_miss(&requested_name, &base, &classpath);
        return Ok((requested_name, base, classpath, None));
    };
    let resource = ops.find_resource_entry(heap, Some(loader_ref), &resolved_name)?;
    if resource.is_none() {
        log_resource_lookup_miss(&resolved_name, &base, &classpath);
    }
    Ok((resolved_name, base, classpath, resource))
}

pub(crate) fn native_class_get_resource_as_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let (_, _, _, resource) = lookup_class_resource(args, heap, ops)?;
    let Some(resource) = resource else {
        return Ok(Some(Slot::Reference(None)));
    };
    let stream_ref = allocate_resource_input_stream(heap, resource.bytes)?;
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

pub(crate) fn native_class_loader_get_resource_as_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let (_, _, _, resource) = lookup_class_loader_resource(args, heap, ops)?;
    let Some(resource) = resource else {
        return Ok(Some(Slot::Reference(None)));
    };
    let stream_ref = allocate_resource_input_stream(heap, resource.bytes)?;
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

pub(crate) fn native_class_get_resource(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let (_, _, _, resource) = lookup_class_resource(args, heap, ops)?;
    let Some(resource) = resource else {
        return Ok(Some(Slot::Reference(None)));
    };
    let url_ref = allocate_resource_url(heap, resource.url)?;
    Ok(Some(Slot::Reference(Some(url_ref))))
}

pub(crate) fn native_class_loader_get_resource(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let (_, _, _, resource) = lookup_class_loader_resource(args, heap, ops)?;
    let Some(resource) = resource else {
        return Ok(Some(Slot::Reference(None)));
    };
    let url_ref = allocate_resource_url(heap, resource.url)?;
    Ok(Some(Slot::Reference(Some(url_ref))))
}

pub(crate) fn native_class_loader_get_resources(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let loader_ref = extract_ref_arg(args, 0)?;
    let requested_name = string_arg(args, 1, heap)?;
    let classpath = classpath_debug_label(Some(loader_ref));
    let Some(resolved_name) = normalize_resource_name(&requested_name) else {
        log_resource_lookup_miss(&requested_name, "<class-loader>", &classpath);
        let enum_ref = allocate_resource_enumeration(heap, Vec::new())?;
        return Ok(Some(Slot::Reference(Some(enum_ref))));
    };
    let resources = ops.find_resource_entries(heap, Some(loader_ref), &resolved_name)?;
    if resources.is_empty() {
        log_resource_lookup_miss(&resolved_name, "<class-loader>", &classpath);
    }
    let enum_ref = allocate_resource_enumeration(heap, resources)?;
    Ok(Some(Slot::Reference(Some(enum_ref))))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_class_loader_register_as_parallel_capable(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Int(1)))
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

fn first_reference_field(heap: &duke_gc::Heap, obj_ref: u64) -> Result<Option<u64>> {
    match heap.get(obj_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => Ok(Some(*r)),
        Some(Slot::Reference(None)) => Ok(None),
        _ => Err(Error::NullPointerException),
    }
}

fn string_backed_object_value(heap: &duke_gc::Heap, obj_ref: u64) -> Result<String> {
    let Some(value_ref) = first_reference_field(heap, obj_ref)? else {
        return Err(Error::NullPointerException);
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

fn file_url_to_path(url: &str) -> Result<std::path::PathBuf> {
    let Some(rest) = url.strip_prefix("file://") else {
        return Err(Error::JavaException {
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

const RESOURCE_STREAM_BYTES_FIELD: usize = 0;
const RESOURCE_STREAM_CURSOR_FIELD: usize = 1;
const RESOURCE_STREAM_CLOSED_FIELD: usize = 2;

const RESOURCE_ENUM_INDEX_FIELD: usize = 0;
const RESOURCE_ENUM_COUNT_FIELD: usize = 1;
const RESOURCE_ENUM_VALUES_START: usize = 2;

fn resource_lookup_trace_enabled() -> bool {
    std::env::var("RUST_LOG").is_ok_and(|value| {
        let lower = value.to_ascii_lowercase();
        lower.contains("debug") || lower.contains("trace")
    })
}

fn log_resource_lookup_miss(resolved_name: &str, base: &str, classpath: &str) {
    if resource_lookup_trace_enabled() {
        eprintln!(
            "resource lookup miss: name={resolved_name} base={base} classpath={classpath}"
        );
    }
}

fn normalize_resource_name(name: &str) -> Option<String> {
    if name.is_empty()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains(':')
    {
        return None;
    }
    let mut normalized = String::with_capacity(name.len());
    for (idx, component) in name.split(['/', '\\']).enumerate() {
        if component.is_empty() || component == "." || component == ".." {
            return None;
        }
        if idx > 0 {
            normalized.push('/');
        }
        normalized.push_str(component);
    }
    Some(normalized)
}

fn resolve_class_resource_name(class_internal_name: &str, name: &str) -> Option<String> {
    if let Some(absolute) = name.strip_prefix('/') {
        return normalize_resource_name(absolute);
    }

    let mut resolved = String::new();
    if let Some((package, _)) = class_internal_name.rsplit_once('/') {
        resolved.push_str(package);
        resolved.push('/');
    }
    resolved.push_str(name);
    normalize_resource_name(&resolved)
}

fn class_resource_base(class_internal_name: &str) -> String {
    class_internal_name
        .rsplit_once('/')
        .map_or_else(|| "<default-package>".to_string(), |(package, _)| package.to_string())
}

fn classpath_debug_label(loader_ref: Option<u64>) -> String {
    loader_ref.map_or_else(|| "bootstrap".to_string(), |loader| format!("loader:{loader}"))
}

fn string_arg(args: &[Slot], idx: usize, heap: &duke_gc::Heap) -> Result<String> {
    let string_ref = extract_ref_arg(args, idx)?;
    string_value_from_ref(heap, string_ref)
}

fn allocate_resource_input_stream(heap: &mut duke_gc::Heap, bytes: Vec<u8>) -> Result<u64> {
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

fn resource_stream_array_ref(heap: &duke_gc::Heap, stream_ref: u64) -> Result<u64> {
    match heap.get(stream_ref)?.fields.get(RESOURCE_STREAM_BYTES_FIELD) {
        Some(Slot::Reference(Some(array_ref))) => Ok(*array_ref),
        _ => Err(Error::InvalidRef { address: stream_ref }),
    }
}

fn resource_stream_is_closed(heap: &duke_gc::Heap, stream_ref: u64) -> Result<bool> {
    Ok(matches!(
        heap.get(stream_ref)?.fields.get(RESOURCE_STREAM_CLOSED_FIELD),
        Some(Slot::Int(value)) if *value != 0
    ))
}

fn resource_stream_ensure_open(heap: &duke_gc::Heap, stream_ref: u64) -> Result<()> {
    if resource_stream_is_closed(heap, stream_ref)? {
        push_pending_java_exception_message("java/io/IOException", "Stream closed".to_string());
        return Err(Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        });
    }
    Ok(())
}

fn resource_stream_cursor(heap: &duke_gc::Heap, stream_ref: u64) -> Result<usize> {
    match heap.get(stream_ref)?.fields.get(RESOURCE_STREAM_CURSOR_FIELD) {
        Some(Slot::Int(value)) if *value >= 0 => Ok(usize::try_from(*value).unwrap_or(usize::MAX)),
        _ => Ok(0),
    }
}

fn resource_stream_set_cursor(heap: &mut duke_gc::Heap, stream_ref: u64, cursor: usize) -> Result<()> {
    heap.write_field(
        stream_ref,
        RESOURCE_STREAM_CURSOR_FIELD,
        Slot::Int(i32::try_from(cursor).unwrap_or(i32::MAX)),
    )
}

fn resource_stream_available_bytes(heap: &duke_gc::Heap, stream_ref: u64) -> Result<usize> {
    let array_ref = resource_stream_array_ref(heap, stream_ref)?;
    let len = heap.get(array_ref)?.fields.len();
    Ok(len.saturating_sub(resource_stream_cursor(heap, stream_ref)?))
}

fn read_resource_bytes_from_jar_spec(spec: &str) -> Result<Vec<u8>> {
    let Some((container, entry_name)) = spec.rsplit_once("!/") else {
        return Err(Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        });
    };

    if container.starts_with("file://") && container.contains("!/") {
        let Some((outer_url, nested_entry_name)) = container.rsplit_once("!/") else {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
        };
        let outer_path = file_url_to_path(outer_url)?;
        let nested_bytes = duke_loader::ZipReader::open(&outer_path)
            .map_err(|_| Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            })?
            .read_entry(nested_entry_name)
            .map_err(|_| Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            })?;
        return duke_loader::ZipReader::from_bytes(nested_bytes)
            .map_err(|_| Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            })?
            .read_entry(entry_name)
            .map_err(|_| Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
    }

    let jar_path = file_url_to_path(container)?;
    duke_loader::ZipReader::open(&jar_path)
        .map_err(|_| Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        })?
        .read_entry(entry_name)
        .map_err(|_| Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        })
}

fn read_resource_bytes_from_url_spec(spec: &str) -> Result<Vec<u8>> {
    if let Some(jar_spec) = spec.strip_prefix("jar:") {
        return read_resource_bytes_from_jar_spec(jar_spec);
    }
    if spec.starts_with("file://") {
        let path = file_url_to_path(spec)?;
        return std::fs::read(path).map_err(|_| Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        });
    }
    Err(Error::JavaException {
        class_name: "java/io/IOException".to_string(),
    })
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

pub(crate) fn native_class_get_protection_domain(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let pd_ref = heap.allocate("java/security/ProtectionDomain".to_string(), 1);
    let code_source_slot = if let Some(path) = ops.code_source_for_class(&class_key)? {
        let url_ref = allocate_string_backed_object(
            heap,
            "java/net/URL",
            path_to_file_url(std::path::Path::new(&path)),
        )?;
        let code_source_ref = heap.allocate("java/security/CodeSource".to_string(), 1);
        heap.get_mut(code_source_ref)?.fields[0] = Slot::Reference(Some(url_ref));
        Slot::Reference(Some(code_source_ref))
    } else {
        Slot::Reference(None)
    };
    heap.get_mut(pd_ref)?.fields[0] = code_source_slot;
    Ok(Some(Slot::Reference(Some(pd_ref))))
}

pub(crate) fn native_protection_domain_get_code_source(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Err(Error::NullPointerException),
    }
}

pub(crate) fn native_code_source_get_location(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Err(Error::NullPointerException),
    }
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
        _ => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
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
        _ => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
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

pub(crate) fn native_resource_input_stream_read(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    resource_stream_ensure_open(heap, this_ref)?;
    let array_ref = resource_stream_array_ref(heap, this_ref)?;
    let cursor = resource_stream_cursor(heap, this_ref)?;
    let bytes = &heap.get(array_ref)?.fields;
    if cursor >= bytes.len() {
        return Ok(Some(Slot::Int(-1)));
    }
    let next = match bytes.get(cursor) {
        Some(Slot::Int(value)) => *value,
        _ => 0,
    };
    resource_stream_set_cursor(heap, this_ref, cursor + 1)?;
    Ok(Some(Slot::Int(next)))
}

pub(crate) fn native_resource_input_stream_read_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let array_ref = extract_ref_arg(args, 1)?;
    let len = heap.get(array_ref)?.fields.len();
    let len_i32 = i32::try_from(len).unwrap_or(i32::MAX);
    native_resource_input_stream_read_bytes_slice(
        &[
            Slot::Reference(Some(this_ref)),
            Slot::Reference(Some(array_ref)),
            Slot::Int(0),
            Slot::Int(len_i32),
        ],
        heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
    )
}

pub(crate) fn native_resource_input_stream_read_bytes_slice(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target_ref = extract_ref_arg(args, 1)?;
    let offset = extract_int_arg(args, 2)?;
    let len = extract_int_arg(args, 3)?;
    resource_stream_ensure_open(heap, this_ref)?;

    let target_len = heap.get(target_ref)?.fields.len();
    if offset < 0 || len < 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    let len = usize::try_from(len).unwrap_or(usize::MAX);
    if offset > target_len || len > target_len.saturating_sub(offset) {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    if len == 0 {
        return Ok(Some(Slot::Int(0)));
    }

    let source_ref = resource_stream_array_ref(heap, this_ref)?;
    let cursor = resource_stream_cursor(heap, this_ref)?;
    let source = heap.get(source_ref)?.fields.clone();
    if cursor >= source.len() {
        return Ok(Some(Slot::Int(-1)));
    }
    let available = source.len() - cursor;
    let read_len = available.min(len);
    {
        let target = heap.get_mut(target_ref)?;
        target.fields[offset..offset + read_len].copy_from_slice(&source[cursor..cursor + read_len]);
    }
    resource_stream_set_cursor(heap, this_ref, cursor + read_len)?;
    Ok(Some(Slot::Int(i32::try_from(read_len).unwrap_or(i32::MAX))))
}

pub(crate) fn native_resource_input_stream_available(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    resource_stream_ensure_open(heap, this_ref)?;
    let available = resource_stream_available_bytes(heap, this_ref)?;
    Ok(Some(Slot::Int(i32::try_from(available).unwrap_or(i32::MAX))))
}

pub(crate) fn native_resource_input_stream_skip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let requested = extract_long_arg(args, 1)?;
    resource_stream_ensure_open(heap, this_ref)?;
    if requested <= 0 {
        return Ok(Some(Slot::Long(0)));
    }
    let available = resource_stream_available_bytes(heap, this_ref)?;
    let skipped = available.min(usize::try_from(requested).unwrap_or(usize::MAX));
    let cursor = resource_stream_cursor(heap, this_ref)?;
    resource_stream_set_cursor(heap, this_ref, cursor + skipped)?;
    Ok(Some(Slot::Long(i64::try_from(skipped).unwrap_or(i64::MAX))))
}

pub(crate) fn native_resource_input_stream_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.write_field(this_ref, RESOURCE_STREAM_CLOSED_FIELD, Slot::Int(1))?;
    Ok(None)
}

pub(crate) fn native_resource_enumeration_has_more_elements(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let index = match fields.get(RESOURCE_ENUM_INDEX_FIELD) {
        Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
        _ => 0,
    };
    let count = match fields.get(RESOURCE_ENUM_COUNT_FIELD) {
        Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(index < count))))
}

pub(crate) fn native_resource_enumeration_next_element(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (index, count, slot) = {
        let enumeration = heap.get(this_ref)?;
        let index = match enumeration.fields.get(RESOURCE_ENUM_INDEX_FIELD) {
            Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
            _ => 0,
        };
        let count = match enumeration.fields.get(RESOURCE_ENUM_COUNT_FIELD) {
            Some(Slot::Int(value)) if *value > 0 => usize::try_from(*value).unwrap_or(0),
            _ => 0,
        };
        let slot = enumeration
            .fields
            .get(RESOURCE_ENUM_VALUES_START + index)
            .copied()
            .unwrap_or(Slot::Reference(None));
        (index, count, slot)
    };
    if index >= count {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    heap.write_field(
        this_ref,
        RESOURCE_ENUM_INDEX_FIELD,
        Slot::Int(i32::try_from(index + 1).unwrap_or(i32::MAX)),
    )?;
    Ok(Some(slot))
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
    native_arraylist_init(
        &[Slot::Reference(Some(path_list_ref))],
        heap,
        out,
        control,
    )?;
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
            let class_key = ops.class_key_for_runtime_loader(heap, this_ref, &internal_name)?;
            let class_ref = allocate_class_object(heap, &class_key)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        Err(Error::ClassNotFound { .. }) => Err(Error::JavaException {
            class_name: "java/lang/ClassNotFoundException".to_string(),
        }),
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

pub(crate) fn native_path_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let uri_ref = extract_ref_arg(args, 0)?;
    let uri = string_backed_object_value(heap, uri_ref)?;
    let path = file_url_to_path(&uri)?;
    let path_ref = allocate_string_backed_object(
        heap,
        "java/nio/file/Path",
        path.to_string_lossy().to_string(),
    )?;
    Ok(Some(Slot::Reference(Some(path_ref))))
}

pub(crate) fn native_path_to_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .ok_or(Error::NullPointerException)?;
    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    heap.get_mut(file_ref)?.fields[0] = path_slot;
    Ok(Some(Slot::Reference(Some(file_ref))))
}

pub(crate) fn native_paths_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let first_ref = extract_ref_arg(args, 0)?;
    let mut path = std::path::PathBuf::from(string_value_from_ref(heap, first_ref)?);
    let more_slot = extract_slot_arg(args, 1);
    match more_slot {
        Slot::Reference(Some(array_ref)) => {
            let segments = heap.get(array_ref)?.fields.clone();
            for segment in segments {
                let Slot::Reference(Some(segment_ref)) = segment else {
                    return Err(Error::NullPointerException);
                };
                path.push(string_value_from_ref(heap, segment_ref)?);
            }
        }
        Slot::Reference(None) => {}
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    }
    let path_ref = allocate_string_backed_object(
        heap,
        "java/nio/file/Path",
        path.to_string_lossy().into_owned(),
    )?;
    Ok(Some(Slot::Reference(Some(path_ref))))
}

pub(crate) fn native_class_get_declared_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let method_name = string_value_from_ref(heap, name_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 2))?;

    let Some(method) = reflected.methods.into_iter().find(|method| {
        method.name == method_name
            && descriptor_parameter_part(&method.descriptor) == parameter_descriptor
    }) else {
        return Err(Error::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let method_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Method",
        &class_key,
        &method.name,
        &method.descriptor,
        method.is_public,
        method.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(method_ref))))
}

pub(crate) fn native_class_get_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let method_name = string_value_from_ref(heap, name_ref)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 2))?;

    let Some((declaring_class, method)) =
        lookup_public_reflected_method(ops, &class_key, &method_name, &parameter_descriptor)?
    else {
        return Err(Error::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let method_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Method",
        &declaring_class,
        &method.name,
        &method.descriptor,
        method.is_public,
        method.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(method_ref))))
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

pub(crate) fn native_class_get_declared_field(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let field_name = string_value_from_ref(heap, name_ref)?;
    let reflected = ops.inspect_class(&class_key)?;

    let Some(field) = reflected
        .fields
        .into_iter()
        .find(|field| field.name == field_name)
    else {
        return Err(Error::JavaException {
            class_name: "java/lang/NoSuchFieldException".to_string(),
        });
    };

    let field_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Field",
        &class_key,
        &field.name,
        &field.descriptor,
        field.is_public,
        field.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(field_ref))))
}

pub(crate) fn native_class_get_declared_constructor(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 1))?;

    let Some(constructor) = lookup_reflected_constructor(reflected, &parameter_descriptor, false)
    else {
        return Err(Error::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let constructor_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Constructor",
        &class_key,
        &constructor.name,
        &constructor.descriptor,
        constructor.is_public,
        constructor.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(constructor_ref))))
}

pub(crate) fn native_class_get_field(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let field_name = string_value_from_ref(heap, name_ref)?;

    let Some((declaring_class, field)) =
        lookup_public_reflected_field(ops, &class_key, &field_name)?
    else {
        return Err(Error::JavaException {
            class_name: "java/lang/NoSuchFieldException".to_string(),
        });
    };

    let field_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Field",
        &declaring_class,
        &field.name,
        &field.descriptor,
        field.is_public,
        field.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(field_ref))))
}

pub(crate) fn native_class_get_constructor(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 1))?;

    let Some(constructor) = lookup_reflected_constructor(reflected, &parameter_descriptor, true)
    else {
        return Err(Error::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let constructor_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Constructor",
        &class_key,
        &constructor.name,
        &constructor.descriptor,
        constructor.is_public,
        constructor.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(constructor_ref))))
}

pub(crate) fn native_class_new_instance(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let constructors = reflected_constructors(reflected, false);
    let Some(constructor) = constructors
        .iter()
        .find(|constructor| constructor.descriptor == "()V")
        .cloned()
    else {
        return Err(Error::JavaException {
            class_name: "java/lang/InstantiationException".to_string(),
        });
    };
    if !constructor.is_public {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let instance_ref = ops.allocate_instance(heap, output, &class_key)?;
    match ops.invoke(
        heap,
        output,
        &class_key,
        "<init>",
        &constructor.descriptor,
        vec![Slot::Reference(Some(instance_ref))],
    ) {
        Ok(_) => Ok(Some(Slot::Reference(Some(instance_ref)))),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_class_get_declared_methods(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let method_refs = reflected
        .methods
        .into_iter()
        .filter(|method| method.name != "<init>" && method.name != "<clinit>")
        .map(|method| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Method",
                &class_key,
                &method.name,
                &method.descriptor,
                method.is_public,
                method.is_static,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Method;", &method_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_declared_constructors(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let constructor_refs = reflected_constructors(reflected, false)
        .into_iter()
        .map(|constructor| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Constructor",
                &class_key,
                &constructor.name,
                &constructor.descriptor,
                constructor.is_public,
                constructor.is_static,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let array_ref =
        allocate_reference_array(heap, "[Ljava/lang/reflect/Constructor;", &constructor_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_methods(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let method_refs = collect_public_reflected_methods(ops, &class_key)?
        .into_iter()
        .map(|(declaring_class, method)| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Method",
                &declaring_class,
                &method.name,
                &method.descriptor,
                method.is_public,
                method.is_static,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Method;", &method_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_constructors(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let constructor_refs = reflected_constructors(reflected, true)
        .into_iter()
        .map(|constructor| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Constructor",
                &class_key,
                &constructor.name,
                &constructor.descriptor,
                constructor.is_public,
                constructor.is_static,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let array_ref =
        allocate_reference_array(heap, "[Ljava/lang/reflect/Constructor;", &constructor_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_declared_fields(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let field_refs = reflected
        .fields
        .into_iter()
        .map(|field| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Field",
                &class_key,
                &field.name,
                &field.descriptor,
                field.is_public,
                field.is_static,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Field;", &field_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_fields(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let field_refs = collect_public_reflected_fields(ops, &class_key)?
        .into_iter()
        .map(|(declaring_class, field)| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Field",
                &declaring_class,
                &field.name,
                &field.descriptor,
                field.is_public,
                field.is_static,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Field;", &field_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

const ANNOTATION_PROXY_PREFIX: &str = "duke/annotation/AnnotationProxy:";

pub(crate) fn annotation_proxy_type(class_name: &str) -> Option<&str> {
    class_name.strip_prefix(ANNOTATION_PROXY_PREFIX)
}

fn find_annotation<'a>(
    annotations: &'a [ReflectedAnnotation],
    requested_type: &str,
) -> Option<&'a ReflectedAnnotation> {
    let requested_type = class_internal_name_from_key(requested_type);
    annotations
        .iter()
        .find(|annotation| annotation.type_name == requested_type)
}

fn annotation_element_values(
    annotation: &ReflectedAnnotation,
    ops: &mut dyn CallbackOps,
) -> Result<Vec<(String, String, ReflectedAnnotationValue)>> {
    let annotation_info = ops.inspect_class(&annotation.type_name)?;
    let mut values = Vec::new();
    for method in annotation_info
        .methods
        .into_iter()
        .filter(|method| method.descriptor.starts_with("()"))
    {
        let explicit = annotation
            .elements
            .iter()
            .find(|element| element.name == method.name)
            .map(|element| element.value.clone());
        let Some(value) = explicit.or_else(|| method.annotation_default.clone()) else {
            continue;
        };
        values.push((
            method.name,
            method_return_descriptor(&method.descriptor).to_string(),
            value,
        ));
    }
    Ok(values)
}

fn array_element_descriptor(array_descriptor: &str) -> &str {
    array_descriptor.strip_prefix('[').unwrap_or("Ljava/lang/Object;")
}

fn materialize_annotation_const(
    heap: &mut duke_gc::Heap,
    descriptor: &str,
    value: &ReflectedAnnotationConst,
) -> Slot {
    match value {
        ReflectedAnnotationConst::String(value) => {
            Slot::Reference(Some(heap.allocate_string(value.clone())))
        }
        ReflectedAnnotationConst::Long(value) => Slot::Long(*value),
        ReflectedAnnotationConst::Float(value) => Slot::Float(*value),
        ReflectedAnnotationConst::Double(value) => Slot::Double(*value),
        ReflectedAnnotationConst::Boolean(value) => Slot::Int(i32::from(*value)),
        ReflectedAnnotationConst::Byte(value)
        | ReflectedAnnotationConst::Char(value)
        | ReflectedAnnotationConst::Int(value)
        | ReflectedAnnotationConst::Short(value) => {
            if descriptor == "Z" {
                Slot::Int(i32::from(*value != 0))
            } else {
                Slot::Int(*value)
            }
        }
    }
}

fn materialize_annotation_value(
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    descriptor: &str,
    value: &ReflectedAnnotationValue,
) -> Result<Slot> {
    match value {
        ReflectedAnnotationValue::Const(value) => {
            Ok(materialize_annotation_const(heap, descriptor, value))
        }
        ReflectedAnnotationValue::Class(class_key) => {
            let class_ref = allocate_class_object(heap, class_key)?;
            Ok(Slot::Reference(Some(class_ref)))
        }
        ReflectedAnnotationValue::Enum {
            type_name,
            const_name,
        } => {
            ops.ensure_class_initialized(heap, output, type_name)?;
            ops.read_static_field(type_name, const_name)
        }
        ReflectedAnnotationValue::Annotation(annotation) => {
            let annotation_ref = allocate_annotation_proxy(heap, output, ops, annotation)?;
            Ok(Slot::Reference(Some(annotation_ref)))
        }
        ReflectedAnnotationValue::Array(values) => {
            let element_descriptor = array_element_descriptor(descriptor);
            let slots = values
                .iter()
                .map(|value| {
                    materialize_annotation_value(heap, output, ops, element_descriptor, value)
                })
                .collect::<Result<Vec<_>>>()?;
            let array_ref = allocate_reference_array_from_slots(heap, descriptor, &slots)?;
            Ok(Slot::Reference(Some(array_ref)))
        }
    }
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
        let value_slot = materialize_annotation_value(heap, output, ops, &descriptor, &value)?;
        fields.push(Slot::Reference(Some(name_ref)));
        fields.push(Slot::Reference(Some(descriptor_ref)));
        fields.push(value_slot);
    }

    let proxy_ref = heap.allocate(
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
    let array_ref =
        allocate_reference_array(heap, "[Ljava/lang/annotation/Annotation;", &annotation_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn annotation_proxy_element_slot(
    heap: &duke_gc::Heap,
    proxy_ref: u64,
    method_name: &str,
    descriptor: &str,
) -> Result<Option<Slot>> {
    let proxy = heap.get(proxy_ref)?;
    if annotation_proxy_type(&proxy.class_name).is_none() {
        return Ok(None);
    }
    if method_name == "annotationType" && descriptor == "()Ljava/lang/Class;" {
        return Ok(proxy.fields.first().copied());
    }
    for chunk in proxy.fields[1..].chunks(3) {
        let [Slot::Reference(Some(name_ref)), Slot::Reference(Some(desc_ref)), value] = chunk
        else {
            continue;
        };
        if string_value_from_ref(heap, *name_ref)? == method_name
            && string_value_from_ref(heap, *desc_ref)? == method_return_descriptor(descriptor)
        {
            return Ok(Some(*value));
        }
    }
    Ok(None)
}

fn annotations_for_reflected_method(
    heap: &duke_gc::Heap,
    method_ref: u64,
    ops: &mut dyn CallbackOps,
) -> Result<Vec<ReflectedAnnotation>> {
    let method = reflected_method_handle(heap, method_ref)?;
    Ok(ops
        .inspect_class(&method.declaring_class_key)?
        .methods
        .into_iter()
        .find(|candidate| {
            let name_matches = candidate.name == method.method_name;
            let descriptor_matches = candidate.descriptor == method.descriptor;
            name_matches && descriptor_matches
        })
        .map_or_else(Vec::new, |method| method.annotations))
}

fn annotations_for_reflected_field(
    heap: &duke_gc::Heap,
    field_ref: u64,
    ops: &mut dyn CallbackOps,
) -> Result<Vec<ReflectedAnnotation>> {
    let field = reflected_field_handle(heap, field_ref)?;
    Ok(ops
        .inspect_class(&field.declaring_class_key)?
        .fields
        .into_iter()
        .find(|candidate| {
            let name_matches = candidate.name == field.field_name;
            let descriptor_matches = candidate.descriptor == field.descriptor;
            name_matches && descriptor_matches
        })
        .map_or_else(Vec::new, |field| field.annotations))
}

pub(crate) fn native_class_get_annotations(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let annotations = ops.inspect_class(&class_key)?.annotations;
    allocate_annotation_array(heap, out, ops, &annotations)
}

pub(crate) fn native_class_get_declared_annotations(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_class_get_annotations(args, heap, out, control, ops)
}

pub(crate) fn native_class_get_annotation(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let annotation_type_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let requested_type = class_key_from_ref(heap, annotation_type_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    match find_annotation(&reflected.annotations, &requested_type) {
        Some(annotation) => {
            let annotation_ref = allocate_annotation_proxy(heap, out, ops, annotation)?;
            Ok(Some(Slot::Reference(Some(annotation_ref))))
        }
        None => Ok(Some(Slot::Reference(None))),
    }
}
