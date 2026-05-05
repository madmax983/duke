fn properties_illegal_argument() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    }
}
fn properties_io_exception() -> Error {
    Error::JavaException {
        class_name: "java/io/IOException".to_string(),
    }
}
fn properties_stream_id_from_slot(slot: Slot, heap: &duke_gc::Heap) -> Result<i32> {
    let Slot::Reference(Some(stream_ref)) = slot else {
        return Err(Error::NullPointerException);
    };
    match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(properties_io_exception()),
    }
}
fn properties_entry_index(
    fields: &[Slot],
    key: &Slot,
    heap: &duke_gc::Heap,
) -> Option<usize> {
    (PROPERTIES_ENTRIES_START..fields.len())
        .step_by(2)
        .find(|&idx| idx + 1 < fields.len() && slots_equal(&fields[idx], key, heap))
}
fn properties_local_entries(
    heap: &duke_gc::Heap,
    props_ref: u64,
) -> Result<Vec<(Slot, Slot)>> {
    let fields = heap.get(props_ref)?.fields.clone();
    let mut entries = Vec::new();
    let mut idx = PROPERTIES_ENTRIES_START;
    while idx + 1 < fields.len() {
        entries.push((fields[idx], fields[idx + 1]));
        idx += 2;
    }
    Ok(entries)
}
fn properties_local_string_entries(
    heap: &duke_gc::Heap,
    props_ref: u64,
) -> Result<Vec<(String, String)>> {
    let mut entries = Vec::new();
    for (key_slot, value_slot) in properties_local_entries(heap, props_ref)? {
        let Some(key) = string_from_slot(heap, key_slot) else {
            continue;
        };
        let Some(value) = string_from_slot(heap, value_slot) else {
            continue;
        };
        entries.push((key, value));
    }
    Ok(entries)
}
fn properties_put_slots(
    heap: &mut duke_gc::Heap,
    props_ref: u64,
    key: Slot,
    value: Slot,
) -> Result<Slot> {
    ensure_properties_layout(heap, props_ref)?;
    let entry_idx = {
        let fields = &heap.get(props_ref)?.fields;
        properties_entry_index(fields, &key, heap)
    };
    if let Some(idx) = entry_idx {
        let old = heap.get(props_ref)?.fields[idx + 1];
        heap.get_mut(props_ref)?.fields[idx + 1] = value;
        return Ok(old);
    }
    let props = heap.get_mut(props_ref)?;
    match props.fields.get_mut(PROPERTIES_SIZE_FIELD) {
        Some(Slot::Int(size)) => *size += 1,
        _ => props.fields[PROPERTIES_SIZE_FIELD] = Slot::Int(1),
    }
    props.fields.push(key);
    props.fields.push(value);
    Ok(Slot::Reference(None))
}
fn properties_put_string_pair(
    heap: &mut duke_gc::Heap,
    props_ref: u64,
    key: String,
    value: String,
) -> Result<()> {
    let key_slot = Slot::Reference(Some(heap.allocate_string(key)));
    let value_slot = Slot::Reference(Some(heap.allocate_string(value)));
    properties_put_slots(heap, props_ref, key_slot, value_slot)?;
    Ok(())
}
fn properties_get_property_slot(
    heap: &duke_gc::Heap,
    props_ref: u64,
    key: Slot,
) -> Result<Slot> {
    let fields = heap.get(props_ref)?.fields.clone();
    if let Some(idx) = properties_entry_index(&fields, &key, heap) {
        let value = fields[idx + 1];
        if slot_is_java_string(heap, value) {
            return Ok(value);
        }
    }
    match fields.get(PROPERTIES_DEFAULTS_FIELD).copied() {
        Some(Slot::Reference(Some(defaults_ref))) => {
            properties_get_property_slot(heap, defaults_ref, key)
        }
        _ => Ok(Slot::Reference(None)),
    }
}
fn properties_push_unique_name(heap: &duke_gc::Heap, names: &mut Vec<Slot>, key: Slot) {
    if !slot_is_java_string(heap, key) {
        return;
    }
    if names.iter().any(|existing| slots_equal(existing, &key, heap)) {
        return;
    }
    names.push(key);
}
fn properties_collect_name_slots(
    heap: &duke_gc::Heap,
    props_ref: u64,
    names: &mut Vec<Slot>,
) -> Result<()> {
    let fields = heap.get(props_ref)?.fields.clone();
    if let Some(Slot::Reference(Some(defaults_ref))) = fields
        .get(PROPERTIES_DEFAULTS_FIELD)
    {
        properties_collect_name_slots(heap, *defaults_ref, names)?;
    }
    let mut idx = PROPERTIES_ENTRIES_START;
    while idx + 1 < fields.len() {
        let key = fields[idx];
        let value = fields[idx + 1];
        if slot_is_java_string(heap, value) {
            properties_push_unique_name(heap, names, key);
        }
        idx += 2;
    }
    Ok(())
}
pub(crate) fn native_properties_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    init_properties_with_defaults(heap, this_ref, Slot::Reference(None))?;
    Ok(None)
}
pub(crate) fn native_properties_init_defaults(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let defaults = extract_slot_arg(args, 1);
    init_properties_with_defaults(heap, this_ref, defaults)?;
    Ok(None)
}
pub(crate) fn native_properties_set_property(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key_ref = extract_ref_arg(args, 1)?;
    let value_ref = extract_ref_arg(args, 2)?;
    let _key = string_value_from_ref(heap, key_ref)?;
    let _value = string_value_from_ref(heap, value_ref)?;
    let old = properties_put_slots(
        heap,
        this_ref,
        Slot::Reference(Some(key_ref)),
        Slot::Reference(Some(value_ref)),
    )?;
    Ok(Some(old))
}
pub(crate) fn native_properties_get_property(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key_ref = extract_ref_arg(args, 1)?;
    let _key = string_value_from_ref(heap, key_ref)?;
    Ok(
        Some(
            properties_get_property_slot(heap, this_ref, Slot::Reference(Some(key_ref)))?,
        ),
    )
}
pub(crate) fn native_properties_get_property_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key_ref = extract_ref_arg(args, 1)?;
    let default = extract_slot_arg(args, 2);
    let _key = string_value_from_ref(heap, key_ref)?;
    let value = properties_get_property_slot(
        heap,
        this_ref,
        Slot::Reference(Some(key_ref)),
    )?;
    if matches!(value, Slot::Reference(None)) {
        Ok(Some(default))
    } else {
        Ok(Some(value))
    }
}
pub(crate) fn native_properties_load(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let file_id = properties_stream_id_from_slot(extract_slot_arg(args, 1), heap)?;
    let mut bytes = Vec::new();
    loop {
        let next = heap.read_host_file_byte(file_id)?;
        if next < 0 {
            break;
        }
        let byte = u8::try_from(next).map_err(|_| properties_io_exception())?;
        bytes.push(byte);
    }
    for (key, value) in parse_properties_bytes(&bytes)? {
        properties_put_string_pair(heap, this_ref, key, value)?;
    }
    Ok(None)
}
pub(crate) fn native_properties_store(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let file_id = properties_stream_id_from_slot(extract_slot_arg(args, 1), heap)?;
    let comment_slot = extract_slot_arg(args, 2);
    let comment = match comment_slot {
        Slot::Reference(Some(comment_ref)) => {
            Some(string_value_from_ref(heap, comment_ref)?)
        }
        Slot::Reference(None) => None,
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let rendered = render_properties_store(heap, this_ref, comment.as_deref())?;
    write_iso_8859_1_ascii(heap, file_id, &rendered)?;
    Ok(None)
}
pub(crate) fn native_properties_property_names(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let mut names = Vec::new();
    properties_collect_name_slots(heap, this_ref, &mut names)?;
    let enum_ref = heap
        .allocate(
            "duke/util/PropertiesEnumeration".to_string(),
            PROPERTIES_ENUM_NAMES_START + names.len(),
        );
    heap.write_field(enum_ref, PROPERTIES_ENUM_INDEX_FIELD, Slot::Int(0))?;
    heap.write_field(
        enum_ref,
        PROPERTIES_ENUM_COUNT_FIELD,
        Slot::Int(i32::try_from(names.len()).unwrap_or(i32::MAX)),
    )?;
    for (idx, name) in names.iter().copied().enumerate() {
        heap.write_field(enum_ref, PROPERTIES_ENUM_NAMES_START + idx, name)?;
    }
    Ok(Some(Slot::Reference(Some(enum_ref))))
}
pub(crate) fn native_properties_string_property_names(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let mut names = Vec::new();
    properties_collect_name_slots(heap, this_ref, &mut names)?;
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    for name in names {
        native_hashset_add(&[Slot::Reference(Some(set_ref)), name], heap, out, control)?;
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}
pub(crate) fn native_properties_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.get(PROPERTIES_SIZE_FIELD) {
        Some(Slot::Int(size)) => *size,
        _ => 0,
    };
    Ok(Some(Slot::Int(size)))
}
pub(crate) fn native_properties_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let is_empty = matches!(
        heap.get(this_ref) ?.fields.get(PROPERTIES_SIZE_FIELD), Some(Slot::Int(0)) | None
    );
    Ok(Some(Slot::Int(i32::from(is_empty))))
}
pub(crate) fn native_properties_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fields = &heap.get(this_ref)?.fields;
    Ok(Some(Slot::Int(i32::from(properties_entry_index(fields, &key, heap).is_some()))))
}
pub(crate) fn native_properties_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    ensure_properties_layout(heap, this_ref)?;
    let props = heap.get_mut(this_ref)?;
    props.fields[PROPERTIES_SIZE_FIELD] = Slot::Int(0);
    props.fields.truncate(PROPERTIES_ENTRIES_START);
    Ok(None)
}
pub(crate) fn native_properties_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let idx_opt = {
        let fields = &heap.get(this_ref)?.fields;
        properties_entry_index(fields, &key, heap)
    };
    if let Some(idx) = idx_opt {
        let old = heap.get(this_ref)?.fields[idx + 1];
        let props = heap.get_mut(this_ref)?;
        let last_value_idx = props.fields.len() - 1;
        let last_key_idx = props.fields.len() - 2;
        props.fields.swap(idx + 1, last_value_idx);
        props.fields.swap(idx, last_key_idx);
        props.fields.truncate(props.fields.len() - 2);
        if let Some(Slot::Int(size)) = props.fields.get_mut(PROPERTIES_SIZE_FIELD) {
            *size -= 1;
        }
        Ok(Some(old))
    } else {
        Ok(Some(Slot::Reference(None)))
    }
}
pub(crate) fn native_properties_key_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    for (key, _) in properties_local_entries(heap, this_ref)? {
        native_hashset_add(&[Slot::Reference(Some(set_ref)), key], heap, out, control)?;
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}
pub(crate) fn native_properties_values(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    for (_, value) in properties_local_entries(heap, this_ref)? {
        native_arraylist_add(
            &[Slot::Reference(Some(list_ref)), value],
            heap,
            out,
            control,
        )?;
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}
pub(crate) fn native_properties_entry_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    for (key, value) in properties_local_entries(heap, this_ref)? {
        let entry_ref = heap.allocate("java/util/Map$Entry".to_string(), 2);
        {
            let entry = heap.get_mut(entry_ref)?;
            entry.fields[0] = key;
            entry.fields[1] = value;
        }
        native_hashset_add(
            &[Slot::Reference(Some(set_ref)), Slot::Reference(Some(entry_ref))],
            heap,
            out,
            control,
        )?;
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}
pub(crate) fn native_properties_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let mut rendered = String::from("{");
    let mut first = true;
    for (key, value) in properties_local_entries(heap, this_ref)? {
        if first {
            first = false;
        } else {
            rendered.push_str(", ");
        }
        rendered.push_str(&slot_to_string(key, heap));
        rendered.push('=');
        rendered.push_str(&slot_to_string(value, heap));
    }
    rendered.push('}');
    let string_ref = heap.allocate_string(rendered);
    Ok(Some(Slot::Reference(Some(string_ref))))
}
pub(crate) fn native_properties_enum_has_more_elements(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let index = match fields.get(PROPERTIES_ENUM_INDEX_FIELD) {
        Some(Slot::Int(index)) => *index,
        _ => 0,
    };
    let count = match fields.get(PROPERTIES_ENUM_COUNT_FIELD) {
        Some(Slot::Int(count)) => *count,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(index < count))))
}
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_properties_enum_next_element(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (index, count) = {
        let fields = &heap.get(this_ref)?.fields;
        let index = match fields.get(PROPERTIES_ENUM_INDEX_FIELD) {
            Some(Slot::Int(index)) => *index,
            _ => 0,
        };
        let count = match fields.get(PROPERTIES_ENUM_COUNT_FIELD) {
            Some(Slot::Int(count)) => *count,
            _ => 0,
        };
        (index, count)
    };
    if index < 0 || index >= count {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let slot_idx = PROPERTIES_ENUM_NAMES_START + index as usize;
    let value = heap
        .get(this_ref)?
        .fields
        .get(slot_idx)
        .copied()
        .unwrap_or(Slot::Reference(None));
    heap.get_mut(this_ref)?.fields[PROPERTIES_ENUM_INDEX_FIELD] = Slot::Int(index + 1);
    Ok(Some(value))
}
