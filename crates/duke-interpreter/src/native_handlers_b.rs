// ---------------------------------------------------------------------------
// StringBuilder natives
// ---------------------------------------------------------------------------

/// Native: `StringBuilder.<init>()V` — initialise empty buffer.
pub(crate) fn native_sb_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    obj.string_value = Some(String::new());
    Ok(None)
}

/// Native: `StringBuilder.<init>(Ljava/lang/String;)V` — init with string.
pub(crate) fn native_sb_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let init_str = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone().unwrap_or_default(),
        _ => String::new(),
    };
    let obj = heap.get_mut(this_ref)?;
    obj.string_value = Some(init_str);
    Ok(None)
}

/// Native: `StringBuilder.append(Ljava/lang/String;)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let append_str = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone().unwrap_or_default(),
        _ => "null".to_string(),
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&append_str);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(I)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_int_arg(args, 1)?;
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(J)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_long_arg(args, 1)?;
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(D)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_double_arg(args, 1)?;
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(F)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_float_arg(args, 1)?;
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(Z)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v != 0,
        _ => false,
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(if val { "true" } else { "false" });
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(C)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = match args.get(1) {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('\0'),
        _ => '\0',
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push(val);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.toString()Ljava/lang/String;`
pub(crate) fn native_sb_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let content = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(content);
    Ok(Some(Slot::Reference(Some(r))))
}

// ---- StringBuilder extended operations ----

/// Native: `StringBuilder.insert(int, String)StringBuilder` — inserts string at index.
pub(crate) fn native_sb_insert_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let offset = extract_int_arg(args, 1)?;
    let s = match args.get(2) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone().unwrap_or_default(),
        Some(Slot::Reference(None)) | None => "null".to_string(),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "String",
                got: "other",
            });
        }
    };
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    let byte_idx = offset
        .try_into()
        .ok()
        .and_then(|i: usize| buf.char_indices().nth(i).map(|(b, _)| b))
        .unwrap_or(buf.len());
    buf.insert_str(byte_idx, &s);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.insert(int, char)StringBuilder` — inserts char at index.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_sb_insert_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let offset = extract_int_arg(args, 1)?;
    let ch = char::from_u32(extract_int_arg(args, 2)? as u32).unwrap_or('\0');
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    let byte_idx = offset
        .try_into()
        .ok()
        .and_then(|i: usize| buf.char_indices().nth(i).map(|(b, _)| b))
        .unwrap_or(buf.len());
    buf.insert(byte_idx, ch);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.delete(int, int)StringBuilder` — removes chars in [start, end).
pub(crate) fn native_sb_delete(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let start = extract_int_arg(args, 1)?;
    let end = extract_int_arg(args, 2)?;
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    let start_byte = usize::try_from(start)
        .ok()
        .and_then(|i| buf.char_indices().nth(i).map(|(b, _)| b))
        .unwrap_or(buf.len());
    let end_byte = usize::try_from(end)
        .ok()
        .and_then(|i| buf.char_indices().nth(i).map(|(b, _)| b))
        .unwrap_or(buf.len());
    buf.drain(start_byte..end_byte);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.deleteCharAt(int)StringBuilder` — removes single char at index.
pub(crate) fn native_sb_delete_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let index = extract_int_arg(args, 1)?;
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    if let Some(i) = usize::try_from(index)
        .ok()
        .and_then(|i| buf.char_indices().nth(i).map(|(b, _)| b))
    {
        buf.remove(i);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.reverse()StringBuilder` — reverses the character sequence.
pub(crate) fn native_sb_reverse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    *buf = buf.chars().rev().collect();
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.charAt(int)C` — returns char at given index.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_sb_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let index = extract_int_arg(args, 1)?;
    let ch = heap
        .get(this_ref)?
        .string_value
        .as_deref()
        .and_then(|s| usize::try_from(index).ok().and_then(|i| s.chars().nth(i)))
        .unwrap_or('\0');
    Ok(Some(Slot::Int(ch as i32)))
}

/// Native: `StringBuilder.setLength(int)V` — truncates or pads with null chars.
pub(crate) fn native_sb_set_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let new_len = usize::try_from(extract_int_arg(args, 1)?).unwrap_or(0);
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    let char_count = buf.chars().count();
    if new_len < char_count {
        if let Some((byte_idx, _)) = buf.char_indices().nth(new_len) {
            buf.truncate(byte_idx);
        }
    } else if new_len > char_count {
        buf.extend(std::iter::repeat_n('\0', new_len - char_count));
    }
    Ok(None)
}

/// Native: `StringBuilder.length()I`
pub(crate) fn native_sb_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let len = heap
        .get(this_ref)?
        .string_value
        .as_ref()
        .map_or(0, String::len);
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}

// Character natives
// ---------------------------------------------------------------------------

/// Helper: extract a `char` from a `Slot::Int` argument.
fn slot_to_char(slot: &Slot) -> Result<char> {
    match slot {
        Slot::Int(v) => Ok(char::from_u32((*v).cast_unsigned()).unwrap_or('\0')),
        _ => Err(Error::TypeMismatch {
            expected: "Int (char)",
            got: "other",
        }),
    }
}

/// Native: `Character.isDigit(C)Z`
pub(crate) fn native_char_is_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_ascii_digit()))))
}

/// Native: `Character.isLetter(C)Z`
pub(crate) fn native_char_is_letter(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphabetic()))))
}

/// Native: `Character.isWhitespace(C)Z`
pub(crate) fn native_char_is_whitespace(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_whitespace()))))
}

/// Native: `Character.isUpperCase(C)Z`
pub(crate) fn native_char_is_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_uppercase()))))
}

/// Native: `Character.isLowerCase(C)Z`
pub(crate) fn native_char_is_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_lowercase()))))
}

/// Native: `Character.toUpperCase(C)C`
pub(crate) fn native_char_to_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    let upper = ch.to_uppercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(upper as i32)))
}

/// Native: `Character.toLowerCase(C)C`
pub(crate) fn native_char_to_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    let lower = ch.to_lowercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(lower as i32)))
}

/// Native: `Character.isLetterOrDigit(C)Z`
pub(crate) fn native_char_is_letter_or_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(Error::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphanumeric()))))
}

/// Native: `Character.valueOf(C)Ljava/lang/Character;` — box a char.
pub(crate) fn native_char_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Int (char)",
                got: "other",
            });
        }
    };
    let r = heap.allocate("java/lang/Character".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Character.charValue()C` — unbox Character to char.
pub(crate) fn native_char_charvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

// ---------------------------------------------------------------------------
// ArrayList natives
// ---------------------------------------------------------------------------

/// Native: `ArrayList.<init>()V` — initializes with size=0.
pub(crate) fn native_arraylist_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ArrayList.<init>(Collection)V` — copies elements from another `ArrayList`/collection.
pub(crate) fn native_arraylist_init_from_collection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Reference(Some(src_ref))) = args.get(1).copied() else {
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
        return Ok(None);
    };
    // Copy size and elements from source collection (ArrayList layout: fields[0]=size, fields[1..]=elems)
    let src_fields = heap.get(src_ref)?.fields.clone();
    let src_size = match src_fields.first() {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(src_size);
    for elem in src_fields
        .into_iter()
        .skip(1)
        .take(usize::try_from(src_size).unwrap_or(0))
    {
        heap.get_mut(this_ref)?.fields.push(elem);
    }
    Ok(None)
}

/// Native: `ArrayList.add(Object)Z` — appends element, returns true.
pub(crate) fn native_arraylist_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(Error::NullPointerException), // shouldn't happen; init sets fields[0]=Int(0)
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1))) // boolean true
}

/// Native: `ArrayList.get(I)Object` — returns element at index.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)? as usize;
    let obj = heap.get(this_ref)?;
    obj.fields.get(idx + 1).map_or_else(
        || {
            Err(Error::JavaException {
                class_name: "java/lang/ArrayIndexOutOfBoundsException".to_string(),
            })
        },
        |slot| Ok(Some(*slot)),
    )
}

/// Native: `ArrayList.size()I`
pub(crate) fn native_arraylist_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(Slot::Int(sz)) => Ok(Some(Slot::Int(*sz))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `ArrayList.iterator()Iterator` — creates an `ArrayListIterator`.
pub(crate) fn native_arraylist_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    // fields[0]=list_ref, fields[1]=cursor, fields[2]=last_returned (-1 = none)
    let iter_ref = heap.allocate("duke/util/ArrayListIterator".to_string(), 3);
    {
        let iter_obj = heap.get_mut(iter_ref)?;
        iter_obj.fields[0] = Slot::Reference(Some(this_ref));
        iter_obj.fields[1] = Slot::Int(0);
        iter_obj.fields[2] = Slot::Int(-1);
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
}

fn collection_elements_from_ref(heap: &duke_gc::Heap, collection_ref: u64) -> Result<Vec<Slot>> {
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
        _ => Err(Error::TypeMismatch {
            expected: "java/util/Collection",
            got: "other",
        }),
    }
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

/// Native: `Collection.toArray()` — copies Duke-backed collection elements into `Object[]`.
pub(crate) fn native_collection_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elements = collection_elements_from_ref(heap, this_ref)?;
    let array_ref = allocate_reference_array_from_slots(heap, "[Ljava/lang/Object;", &elements)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

/// Native: `Collection.toArray(Object[])` — preserves the requested array type.
pub(crate) fn native_collection_to_array_with_seed_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let seed_array_ref = extract_ref_arg(args, 1)?;
    let elements = collection_elements_from_ref(heap, this_ref)?;
    let seed_class_name = heap.get(seed_array_ref)?.class_name.clone();
    let seed_len = heap.get(seed_array_ref)?.fields.len();

    if seed_len < elements.len() {
        let new_array_ref = allocate_reference_array_from_slots(heap, &seed_class_name, &elements)?;
        return Ok(Some(Slot::Reference(Some(new_array_ref))));
    }

    let seed_array = heap.get_mut(seed_array_ref)?;
    for slot in &mut seed_array.fields {
        *slot = Slot::Reference(None);
    }
    for (idx, element) in elements.iter().enumerate() {
        seed_array.fields[idx] = *element;
    }
    Ok(Some(Slot::Reference(Some(seed_array_ref))))
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
) -> Result<Option<Slot>> {
    // args[0] = ArrayList ref, args[1] = Comparator (null = natural ordering)
    let list_ref = extract_ref_arg(args, 0)?;

    // args[1] = optional Comparator ref (null = natural ordering via compareTo)
    let comparator = args.get(1).copied();

    // Fix 2: guard against a negative size stored in fields[0].
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(Error::NegativeArraySize { size: *n }),
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
        return Err(Error::InvalidRef { address: list_ref });
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
                    return Err(Error::TypeMismatch {
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

/// Native: `Collections.sort(List)V` — delegates to the list's sort(null) method.
///
/// `Collections.sort(list)` is compiled by javac as
/// `invokestatic java/util/Collections.sort:(Ljava/util/List;)V`.
/// We forward to the runtime class's `sort(Comparator=null)`, which for an
/// `ArrayList` performs the insertion-sort-with-compareTo callback.
pub(crate) fn native_collections_sort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    // args[0] = List ref
    let list_ref = extract_ref_arg(args, 0)?;
    // Dispatch on the actual runtime class so any List implementation works.
    let class_name = heap.get(list_ref)?.class_name.clone();
    ops.invoke(
        heap,
        output,
        &class_name,
        "sort",
        SORT_COMPARATOR_DESC,
        vec![Slot::Reference(Some(list_ref)), Slot::Reference(None)],
    )?;
    Ok(None)
}

// ---------------------------------------------------------------------------
// ServiceLoader natives
// ---------------------------------------------------------------------------

const SERVICE_LOADER_SERVICE_CLASS_FIELD: usize = 0;
const SERVICE_LOADER_LOADER_FIELD: usize = 1;
const SERVICE_LOADER_COUNT_FIELD: usize = 2;
const SERVICE_LOADER_PROVIDERS_START: usize = 3;

const SERVICE_ITER_SERVICE_CLASS_FIELD: usize = 0;
const SERVICE_ITER_LOADER_FIELD: usize = 1;
const SERVICE_ITER_INDEX_FIELD: usize = 2;
const SERVICE_ITER_COUNT_FIELD: usize = 3;
const SERVICE_ITER_PROVIDERS_START: usize = 4;

fn read_resource_enumeration_bytes(
    enum_ref: u64,
    heap: &mut duke_gc::Heap,
) -> Result<Vec<Vec<u8>>> {
    let mut files = Vec::new();
    loop {
        let has_more = match native_resource_enumeration_has_more_elements(
            &[Slot::Reference(Some(enum_ref))],
            heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
        )? {
            Some(Slot::Int(value)) => value != 0,
            _ => false,
        };
        if !has_more {
            break;
        }
        let url_slot = native_resource_enumeration_next_element(
            &[Slot::Reference(Some(enum_ref))],
            heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
        )?
        .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(url_ref)) = url_slot else {
            return Err(Error::NullPointerException);
        };
        let spec = string_backed_object_value(heap, url_ref)?;
        files.push(read_resource_bytes_from_url_spec(&spec)?);
    }
    Ok(files)
}

fn service_configuration_files_via_resources(
    heap: &mut duke_gc::Heap,
    ops: &mut dyn CallbackOps,
    loader_ref: Option<u64>,
    service_binary_name: &str,
) -> Result<Vec<Vec<u8>>> {
    let resource_name = format!("META-INF/services/{service_binary_name}");
    let enum_ref = if let Some(loader_ref) = loader_ref {
        let resource_name_ref = heap.allocate_string(resource_name);
        let enum_slot = native_class_loader_get_resources(
            &[
                Slot::Reference(Some(loader_ref)),
                Slot::Reference(Some(resource_name_ref)),
            ],
            heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
            ops,
        )?
        .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(enum_ref)) = enum_slot else {
            return Ok(Vec::new());
        };
        enum_ref
    } else {
        let resources = ops.find_resource_entries(heap, None, &resource_name)?;
        allocate_resource_enumeration(heap, resources)?
    };
    read_resource_enumeration_bytes(enum_ref, heap)
}

fn parse_service_provider_names(files: Vec<Vec<u8>>) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for bytes in files {
        let text = String::from_utf8(bytes).map_err(|_| Error::JavaException {
            class_name: "java/util/ServiceConfigurationError".to_string(),
        })?;
        for line in text.lines() {
            let live = line
                .split_once('#')
                .map_or(line, |(before, _)| before)
                .trim();
            if !live.is_empty() {
                names.push(live.to_string());
            }
        }
    }
    Ok(names)
}

fn service_loader_cause_type(err: &Error) -> &'static str {
    match err {
        Error::ClassNotFound { .. } => "java.lang.ClassNotFoundException",
        Error::JavaException { class_name } => match class_name.as_str() {
            "java/lang/ClassNotFoundException" => "java.lang.ClassNotFoundException",
            "java/lang/IllegalAccessException" => "java.lang.IllegalAccessException",
            "java/lang/InstantiationException" => "java.lang.InstantiationException",
            "java/lang/NoSuchMethodException" => "java.lang.NoSuchMethodException",
            _ => "java.lang.RuntimeException",
        },
        Error::InstantiationError { .. } => "java.lang.InstantiationException",
        Error::NullPointerException => "java.lang.NullPointerException",
        _ => "duke_runtime.Error",
    }
}

fn service_configuration_error(provider_name: &str, cause_type: &str) -> Error {
    push_pending_java_exception_message(
        "java/util/ServiceConfigurationError",
        format!("{provider_name}: provider construction failed due to {cause_type}"),
    );
    Error::JavaException {
        class_name: "java/util/ServiceConfigurationError".to_string(),
    }
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
    let files = service_configuration_files_via_resources(heap, ops, loader_ref, &service_binary_name)?;
    let provider_names = parse_service_provider_names(files)?;
    let loader_ref = heap.allocate(
        "java/util/ServiceLoader".to_string(),
        SERVICE_LOADER_PROVIDERS_START + provider_names.len(),
    );
    {
        let service_loader = heap.get_mut(loader_ref)?;
        service_loader.fields[SERVICE_LOADER_SERVICE_CLASS_FIELD] =
            Slot::Reference(Some(service_class_ref));
        service_loader.fields[SERVICE_LOADER_LOADER_FIELD] = loader_slot;
        service_loader.fields[SERVICE_LOADER_COUNT_FIELD] =
            Slot::Int(i32::try_from(provider_names.len()).unwrap_or(i32::MAX));
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

/// Native: `ArrayListIterator.<init>` — no-op; fields set directly by `native_arraylist_iterator`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_arraylist_iter_init(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}

/// Native: `ArrayListIterator.hasNext()Z`
pub(crate) fn native_arraylist_iter_hasnext(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_obj = heap.get(this_ref)?;
    let list_ref = match iter_obj.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Int(0))),
    };
    let cursor = match iter_obj.fields.get(1) {
        Some(Slot::Int(i)) => *i,
        _ => 0,
    };
    let list_size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(cursor < list_size))))
}

/// Native: `ArrayListIterator.next()Object` — returns element at cursor, advances cursor.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (list_ref, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let lr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(Error::NullPointerException),
        };
        let c = match iter_obj.fields.get(1) {
            Some(Slot::Int(i)) => *i,
            _ => 0,
        };
        (lr, c)
    };
    let element = {
        let list_obj = heap.get(list_ref)?;
        match list_obj.fields.get(cursor as usize + 1) {
            Some(slot) => *slot,
            None => {
                return Err(Error::JavaException {
                    class_name: "java/util/NoSuchElementException".to_string(),
                });
            }
        }
    };
    let iter_obj = heap.get_mut(this_ref)?;
    iter_obj.fields[1] = Slot::Int(cursor + 1);
    iter_obj.fields[2] = Slot::Int(cursor); // record last-returned index
    Ok(Some(element))
}

/// Native: `ArrayListIterator.remove()V` — removes the last element returned by `next()`.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_iter_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (list_ref, last, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let lr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(Error::NullPointerException),
        };
        let last = match iter_obj.fields.get(2) {
            Some(Slot::Int(i)) => *i,
            _ => -1,
        };
        let cursor = match iter_obj.fields.get(1) {
            Some(Slot::Int(i)) => *i,
            _ => 0,
        };
        (lr, last, cursor)
    };
    if last < 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalStateException".to_string(),
        });
    }
    // Remove from backing list: shift elements left, decrement size.
    let list_size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    let remove_idx = last as usize + 1; // +1 because fields[0] is size
    let list_obj = heap.get_mut(list_ref)?;
    let new_size = (list_size - 1) as usize;
    list_obj.fields[0] = Slot::Int(list_size - 1);
    list_obj.fields.remove(remove_idx);
    list_obj.fields.push(Slot::Int(0)); // pad to keep capacity stable
    // Adjust cursor: removed element was before cursor, so decrement.
    if last < cursor {
        let iter_obj = heap.get_mut(this_ref)?;
        iter_obj.fields[1] = Slot::Int(cursor - 1);
    }
    // Reset last-returned sentinel.
    heap.get_mut(this_ref)?.fields[2] = Slot::Int(-1);
    let _ = new_size; // used implicitly
    Ok(None)
}

// ---- ArrayList extended methods ----

/// Native: `ArrayList.remove(I)Object` — removes element at index, returns it.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_remove_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)?;
    if idx < 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let idx = idx as usize;
    let len = heap.get(this_ref)?.fields.len();
    // fields[0]=size, elements start at 1; idx is 0-based element index → field index = idx+1
    if idx + 1 >= len {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let removed = heap.get(this_ref)?.fields[idx + 1];
    let obj = heap.get_mut(this_ref)?;
    obj.fields.remove(idx + 1);
    if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
        *sz -= 1;
    }
    Ok(Some(removed))
}

/// Native: `ArrayList.remove(Object)Z` — removes first occurrence, returns true if found.
pub(crate) fn native_arraylist_remove_obj(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let mut found = None;
    for (i, slot) in heap.get(this_ref)?.fields.iter().enumerate().skip(1) {
        if slots_equal(slot, &target, heap) {
            found = Some(i);
            break;
        }
    }
    if let Some(field_idx) = found {
        let obj = heap.get_mut(this_ref)?;
        obj.fields.remove(field_idx);
        if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
            *sz -= 1;
        }
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `ArrayList.contains(Object)Z` — returns 1 if element is present.
pub(crate) fn native_arraylist_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let mut found = false;
    for slot in heap.get(this_ref)?.fields.iter().skip(1) {
        if slots_equal(slot, &target, heap) {
            found = true;
            break;
        }
    }
    Ok(Some(Slot::Int(i32::from(found))))
}

/// Native: `ArrayList.clear()V` — removes all elements.
pub(crate) fn native_arraylist_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields.truncate(1);
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ArrayList.isEmpty()Z` — returns 1 if size is 0.
pub(crate) fn native_arraylist_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let is_empty = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz == 0,
        _ => true,
    };
    Ok(Some(Slot::Int(i32::from(is_empty))))
}

/// Native: `ArrayList.set(I,Object)Object` — replaces element at index, returns old value.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)?;
    let value = extract_slot_arg(args, 2);
    if idx < 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let field_idx = idx as usize + 1;
    let old = *heap
        .get(this_ref)?
        .fields
        .get(field_idx)
        .ok_or_else(|| Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        })?;
    heap.get_mut(this_ref)?.fields[field_idx] = value;
    Ok(Some(old))
}

/// Native: `ArrayList.indexOf(Object)I` — returns first index of element, or -1.
pub(crate) fn native_arraylist_index_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let len = heap.get(this_ref)?.fields.len();
    for i in 1..len {
        let slot = heap.get(this_ref)?.fields[i];
        if slots_equal(&slot, &target, heap) {
            return Ok(Some(Slot::Int(i32::try_from(i - 1).unwrap_or(i32::MAX))));
        }
    }
    Ok(Some(Slot::Int(-1)))
}

/// Native: `ArrayList.lastIndexOf(Object)I` — last occurrence, or -1.
pub(crate) fn native_arraylist_last_index_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let len = heap.get(this_ref)?.fields.len();
    if len > 1 {
        for i in (1..len).rev() {
            let slot = heap.get(this_ref)?.fields[i];
            if slots_equal(&slot, &target, heap) {
                return Ok(Some(Slot::Int(i32::try_from(i - 1).unwrap_or(i32::MAX))));
            }
        }
    }
    Ok(Some(Slot::Int(-1)))
}

/// Native: `ArrayList.add(I,Object)V` — inserts element at index, shifting others right.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_arraylist_add_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = extract_int_arg(args, 1)?;
    let element = extract_slot_arg(args, 2);
    if idx < 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let field_idx = idx as usize + 1;
    let obj = heap.get_mut(this_ref)?;
    let len = obj.fields.len();
    if field_idx > len {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    obj.fields.insert(field_idx, element);
    if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
        *sz += 1;
    }
    Ok(None)
}

// ---- HashMap extended methods ----

/// Native: `HashMap.putIfAbsent(K,V)Object` — inserts only if key is absent; returns existing or null.
pub(crate) fn native_hashmap_put_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let val = extract_slot_arg(args, 2);
    let i_opt = find_hashmap_entry_index(&heap.get(this_ref)?.fields, &key, heap);
    if let Some(i) = i_opt {
        // Key already present — return existing value.
        return Ok(Some(heap.get(this_ref)?.fields[i + 1]));
    }
    // Key absent — insert and return null.
    let obj = heap.get_mut(this_ref)?;
    if let Some(Slot::Int(sz)) = obj.fields.first_mut() {
        *sz += 1;
    }
    obj.fields.push(key);
    obj.fields.push(val);
    Ok(Some(Slot::Reference(None)))
}

/// Native: `HashMap.clear()V` — removes all entries.
pub(crate) fn native_hashmap_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields.truncate(1);
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `HashMap.containsValue(Object)Z` — returns 1 if any entry has this value.
pub(crate) fn native_hashmap_contains_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let mut i = 2usize;
    loop {
        // Evaluate len inside the loop since heap access can't be held across slots_equal
        let len = heap.get(this_ref)?.fields.len();
        if i >= len {
            break;
        }
        let slot = heap.get(this_ref)?.fields[i];
        if slots_equal(&slot, &target, heap) {
            return Ok(Some(Slot::Int(1)));
        }
        i += 2;
    }
    Ok(Some(Slot::Int(0)))
}

// ---- Double.isNaN ----

/// Native: `Double.isNaN(D)Z` — returns 1 if value is NaN.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_isnan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Double(v)) => Ok(Some(Slot::Int(i32::from(v.is_nan())))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `Double.compareTo(Object)` — compares two boxed Doubles.
pub(crate) fn native_double_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let double_val = |s: &Slot| -> Result<f64> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Double(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => double_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = double_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    // Use total_cmp: implements Java's total order where NaN > +∞ > … > -∞.
    Ok(Some(Slot::Int(ordering_to_int(a.total_cmp(&b)))))
}

// ---- Arrays natives ----

/// Native: `Arrays.fill(int[], int)` — fills all elements with val.
pub(crate) fn native_arrays_fill_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let val = match args.get(1) {
        Some(Slot::Int(v)) => Slot::Int(*v),
        _ => Slot::Int(0),
    };
    let obj = heap.get_mut(arr_ref)?;
    for slot in &mut obj.fields {
        *slot = val;
    }
    Ok(None)
}

/// Native: `Arrays.fill(Object[], Object)` — fills all elements with val.
pub(crate) fn native_arrays_fill_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let val = extract_slot_arg(args, 1);
    let obj = heap.get_mut(arr_ref)?;
    for slot in &mut obj.fields {
        *slot = val;
    }
    Ok(None)
}

/// Native: `Arrays.copyOf(int[], int)` — copies to new int[] of given length.
pub(crate) fn native_arrays_copyof_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let new_len = match args.get(1) {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(Error::NegativeArraySize { size: *n }),
        _ => 0,
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[I".to_string(), new_len);
    let dst = heap.get_mut(dst_ref)?;
    for i in 0..new_len {
        dst.fields[i] = src_fields.get(i).copied().unwrap_or(Slot::Int(0));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.copyOf(Object[], int)` — copies to new Object[] of given length.
pub(crate) fn native_arrays_copyof_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let new_len = match args.get(1) {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(Error::NegativeArraySize { size: *n }),
        _ => 0,
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[Ljava/lang/Object;".to_string(), new_len);
    let dst = heap.get_mut(dst_ref)?;
    for i in 0..new_len {
        dst.fields[i] = src_fields.get(i).copied().unwrap_or(Slot::Reference(None));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.sort(int[])` — sorts fields in place.
pub(crate) fn native_arrays_sort_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(arr_ref)?;
    obj.fields.sort_by(|a, b| match (a, b) {
        (Slot::Int(x), Slot::Int(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    });
    Ok(None)
}

/// Native: `Arrays.equals(int[], int[])boolean` — element-wise equality.
pub(crate) fn native_arrays_equals_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a_ref = extract_ref_arg(args, 0)?;
    let b_ref = extract_ref_arg(args, 1)?;
    let a_len = heap.get(a_ref)?.fields.len();
    let b_len = heap.get(b_ref)?.fields.len();
    if a_len != b_len {
        return Ok(Some(Slot::Int(0)));
    }
    for i in 0..a_len {
        let x = heap.get(a_ref)?.fields[i];
        let y = heap.get(b_ref)?.fields[i];
        match (x, y) {
            (Slot::Int(a), Slot::Int(b)) if a == b => {}
            _ => return Ok(Some(Slot::Int(0))),
        }
    }
    Ok(Some(Slot::Int(1)))
}

// ---------------------------------------------------------------------------
// Phase 44: Arrays.copyOfRange, List.subList, Comparator.reversed,
//           Collections.binarySearch, String.intern, ArrayList.removeIf
// ---------------------------------------------------------------------------

/// Native: `Arrays.copyOfRange(int[], int, int)int[]` — slice of int array, zero-padded.
pub(crate) fn native_arrays_copy_of_range_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let from = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let to = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let new_len = to.saturating_sub(from);
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[I".to_string(), new_len);
    for i in 0..new_len {
        heap.get_mut(dst_ref)?.fields[i] =
            src_fields.get(from + i).copied().unwrap_or(Slot::Int(0));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.copyOfRange(Object[], int, int)Object[]` — slice of reference array.
pub(crate) fn native_arrays_copy_of_range_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let from = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let to = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let new_len = to.saturating_sub(from);
    let (src_class, src_fields) = {
        let obj = heap.get(src_ref)?;
        (obj.class_name.clone(), obj.fields.clone())
    };
    let dst_ref = heap.allocate(src_class, new_len);
    for i in 0..new_len {
        heap.get_mut(dst_ref)?.fields[i] = src_fields
            .get(from + i)
            .copied()
            .unwrap_or(Slot::Reference(None));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `ArrayList.subList(int, int)List` — returns a new `ArrayList` with the sub-range.
pub(crate) fn native_arraylist_sub_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let from = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let to = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let new_len = to.saturating_sub(from);
    // ArrayList layout: fields[0]=size, fields[1..]=elements
    let src_elems: Vec<Slot> = {
        let fields = &heap.get(this_ref)?.fields;
        fields
            .iter()
            .skip(1 + from)
            .take(new_len)
            .copied()
            .collect()
    };
    let sub_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    heap.get_mut(sub_ref)?.fields[0] = Slot::Int(i32::try_from(new_len).unwrap_or(0));
    for elem in src_elems {
        heap.get_mut(sub_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(sub_ref))))
}

/// Native: `ArrayList.removeIf(Predicate)Z` — removes all elements where predicate returns true.
pub(crate) fn native_arraylist_remove_if(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let mut kept = Vec::with_capacity(elems.len());
    let mut removed = false;
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![fn_slot, elem],
            )?
            .unwrap_or(Slot::Int(0));
        match result {
            Slot::Int(1) => {
                removed = true;
            } // predicate true → remove
            _ => kept.push(elem),
        }
    }
    let new_size = i32::try_from(kept.len()).unwrap_or(0);
    heap.get_mut(this_ref)?.fields.truncate(1);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    for elem in kept {
        heap.get_mut(this_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Int(i32::from(removed))))
}

/// Native: `Comparator.reversed()Comparator` — wraps comparator to invert ordering.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_reversed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let delegate = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/ReversedComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = delegate;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ReversedComparator.compare(a, b)I` — inverts delegate comparison.
pub(crate) fn native_reversed_comparator_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let delegate = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(del_ref)) = delegate else {
        return Ok(Some(Slot::Int(0)));
    };
    let del_class = heap.get(del_ref)?.class_name.clone();
    let result = ops
        .invoke(
            heap,
            out,
            &del_class,
            "compare",
            "(Ljava/lang/Object;Ljava/lang/Object;)I",
            vec![delegate, a, b],
        )?
        .unwrap_or(Slot::Int(0));
    let cmp = match result {
        Slot::Int(n) => n,
        _ => 0,
    };
    Ok(Some(Slot::Int(-cmp)))
}

/// Native: `Collections.binarySearch(List, T)I` — binary search on sorted `ArrayList`.
pub(crate) fn native_collections_binary_search(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(list_ref)?.fields[1..=size].to_vec();
    let mut lo: i64 = 0;
    let mut hi: i64 = i64::try_from(elems.len()).unwrap_or(0) - 1;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2; // avoid overflow via i64 midpoint
        let mid_elem = elems[usize::try_from(mid).unwrap_or(0)];
        let cmp = compare_slots_natural(mid_elem, key, heap, out, ops)?;
        match cmp.cmp(&0) {
            std::cmp::Ordering::Equal => {
                return Ok(Some(Slot::Int(i32::try_from(mid).unwrap_or(0))));
            }
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid - 1,
        }
    }
    // Return -(insertion point) - 1
    let insertion = i32::try_from(lo).unwrap_or(0);
    Ok(Some(Slot::Int(-(insertion + 1))))
}

/// Native: `String.intern()String` — returns canonical string (identity for our heap strings).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_string_intern(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // In our interpreter, string equality is by value already; intern = identity.
    Ok(Some(extract_slot_arg(args, 0)))
}

// ---- Arrays.asList ----

/// Native: `Arrays.asList(Object[])List` — wraps a reference array as an `ArrayList`.
pub(crate) fn native_arrays_as_list(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let arr_len = heap.get(arr_ref)?.fields.len();
    // Create a new ArrayList (1 field slot for size counter) and populate it.
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(&[Slot::Reference(Some(list_ref))], heap, out, control)?;
    for i in 0..arr_len {
        let elem = extract_field_arg(heap, arr_ref, i)?;
        native_arraylist_add(&[Slot::Reference(Some(list_ref)), elem], heap, out, control)?;
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}

// ---- String extended operations (Java 11+) ----

/// Native: `String.strip()String` — removes leading and trailing Unicode whitespace.
pub(crate) fn native_string_strip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.trim().to_owned());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.stripLeading()String` — removes leading Unicode whitespace.
pub(crate) fn native_string_strip_leading(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.trim_start().to_owned());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.stripTrailing()String` — removes trailing Unicode whitespace.
pub(crate) fn native_string_strip_trailing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.trim_end().to_owned());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.isBlank()Z` — true if empty or all whitespace.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_string_is_blank(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let blank = heap.get(this_ref).map_or(true, |o| {
        o.string_value
            .as_deref()
            .is_none_or(|s| s.chars().all(char::is_whitespace))
    });
    Ok(Some(Slot::Int(i32::from(blank))))
}

/// Native: `String.repeat(int)String` — repeats this string n times.
pub(crate) fn native_string_repeat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let n = usize::try_from(extract_int_arg(args, 1)?.max(0)).unwrap_or(0);
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();

    let max_size = 1024 * 1024 * 128; // 128 MB max string size
    if n.checked_mul(s.len()).is_none_or(|len| len > max_size) {
        return Err(duke_runtime::Error::JavaException {
            class_name: "java/lang/OutOfMemoryError".to_string(),
        });
    }

    let r = heap.allocate_string(s.repeat(n));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.formatted(Object[])String` — instance alias for `String.format(this, args)`.
pub(crate) fn native_string_formatted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // args[0] = this (the format string), args[1] = Object[] varargs
    let fmt_ref = extract_ref_arg(args, 0)?;
    let arr_slot = extract_slot_arg(args, 1);
    native_string_format(
        &[Slot::Reference(Some(fmt_ref)), arr_slot],
        heap,
        out,
        control,
    )
}

/// Native: `String.join(CharSequence, CharSequence[])String` — joins array elements with delimiter.
pub(crate) fn native_string_join(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let delim_ref = extract_ref_arg(args, 0)?;
    let delim = heap
        .get(delim_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    // args[1] can be an Object[] array (varargs) or a single Iterable (ArrayList)
    let parts: Vec<String> = match args.get(1) {
        Some(Slot::Reference(Some(arr_ref))) => {
            let obj = heap.get(*arr_ref)?;
            if obj.class_name.starts_with('[') {
                // It's an array — fields are the elements.
                let len = obj.fields.len();
                let mut result = Vec::with_capacity(len);
                let slots: Vec<Slot> = obj.fields.clone();
                let _ = obj;
                for slot in slots {
                    let s = match slot {
                        Slot::Reference(Some(r)) => heap
                            .get(r)?
                            .string_value
                            .clone()
                            .unwrap_or_else(|| "null".to_string()),
                        Slot::Reference(None) => "null".to_string(),
                        Slot::Int(n) => n.to_string(),
                        Slot::Long(n) => n.to_string(),
                        other => format!("{other:?}"),
                    };
                    result.push(s);
                }
                result
            } else {
                // ArrayList or similar — fields[0]=size, fields[1..]=elements
                let size_val = match obj.fields.first() {
                    Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                    _ => 0,
                };
                let elems: Vec<Slot> =
                    obj.fields[1..=size_val.min(obj.fields.len().saturating_sub(1))].to_vec();
                let _ = obj;
                let mut result = Vec::with_capacity(size_val);
                for slot in elems {
                    let s = match slot {
                        Slot::Reference(Some(r)) => heap
                            .get(r)?
                            .string_value
                            .clone()
                            .unwrap_or_else(|| "null".to_string()),
                        Slot::Reference(None) => "null".to_string(),
                        Slot::Int(n) => n.to_string(),
                        Slot::Long(n) => n.to_string(),
                        other => format!("{other:?}"),
                    };
                    result.push(s);
                }
                result
            }
        }
        _ => Vec::new(),
    };

    // 👺 Havoc: Check for OOM!
    #[allow(clippy::manual_saturating_arithmetic)]
    let mut total_len = parts.len().saturating_sub(1).checked_mul(delim.len()).unwrap_or(usize::MAX);
    for part in &parts {
        #[allow(clippy::manual_saturating_arithmetic)]
        { total_len = total_len.checked_add(part.len()).unwrap_or(usize::MAX); }
    }
    let max_size = 1024 * 1024 * 128; // 128 MB limit
    if total_len > max_size {
        return Err(duke_runtime::Error::JavaException {
            class_name: "java/lang/OutOfMemoryError".to_string(),
        });
    }

    let mut joined = String::with_capacity(total_len);
    if let Some((first, rest)) = parts.split_first() {
        joined.push_str(first);
        for part in rest {
            joined.push_str(&delim);
            joined.push_str(part);
        }
    }

    let r = heap.allocate_string(joined);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.indexOf(int)I` — finds first occurrence of char (as Unicode code point).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn native_string_index_of_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let ch = char::from_u32(extract_int_arg(args, 1)? as u32).unwrap_or('\0');
    let idx = heap
        .get(this_ref)?
        .string_value
        .as_deref()
        .and_then(|s| s.char_indices().find(|(_, c)| *c == ch).map(|(i, _)| i))
        .and_then(|byte_pos| {
            heap.get(this_ref).ok().and_then(|o| {
                o.string_value
                    .as_deref()
                    .map(|s| s[..byte_pos].chars().count())
            })
        });
    let result = idx.and_then(|i| i32::try_from(i).ok()).unwrap_or(-1);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.lastIndexOf(String)I` — finds last occurrence of substring.
pub(crate) fn native_string_last_index_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let sub_ref = extract_ref_arg(args, 1)?;
    let this_str = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let sub_str = heap.get(sub_ref)?.string_value.clone().unwrap_or_default();
    let result = this_str
        .rfind(sub_str.as_str())
        .and_then(|byte_pos| i32::try_from(this_str[..byte_pos].chars().count()).ok())
        .unwrap_or(-1);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.codePointAt(I)I` — returns the Unicode code point at the given index.
pub(crate) fn native_string_code_point_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = usize::try_from(extract_int_arg(args, 1)?).unwrap_or(0);
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let cp = s
        .chars()
        .nth(idx)
        .map_or(0_i32, |c| i32::try_from(u32::from(c)).unwrap_or(0));
    Ok(Some(Slot::Int(cp)))
}

/// Native: `String.lines()Stream` — splits on newlines, wraps in Stream.
pub(crate) fn native_string_lines(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let n = lines.len();
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), n + 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(i32::try_from(n).unwrap_or(0));
    for (i, line) in lines.iter().enumerate() {
        let s_ref = heap.allocate_string((*line).to_string());
        heap.get_mut(stream_ref)?.fields[i + 1] = Slot::Reference(Some(s_ref));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

// ---------------------------------------------------------------------------
// java.util.Random — 48-bit LCG (same multiplier/addend as Java's java.util.Random)
// fields[0] = Long(seed as i64); all bit-level casts are intentional.
// ---------------------------------------------------------------------------

#[allow(clippy::unreadable_literal)]
const RANDOM_MULTIPLIER: u64 = 25_214_903_917; // 0x5DEECE66D — Java's LCG multiplier
const RANDOM_ADDEND: u64 = 0xB;
const RANDOM_MASK: u64 = (1u64 << 48) - 1;

/// Advance the LCG and return `bits` high bits of the new state.
#[allow(clippy::cast_possible_truncation)]
const fn random_next(seed: u64, bits: u32) -> (u64, i32) {
    let new_seed = seed
        .wrapping_mul(RANDOM_MULTIPLIER)
        .wrapping_add(RANDOM_ADDEND)
        & RANDOM_MASK;
    let value = (new_seed >> (48 - bits)) as i32; // intentional truncation to bit pattern
    (new_seed, value)
}

/// Native: `Random.<init>()V` — seed from current time.
#[allow(clippy::unnecessary_wraps, clippy::cast_possible_wrap)]
pub(crate) fn native_random_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let nanos = u64::from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos()),
    );
    let initial = (nanos ^ RANDOM_MULTIPLIER) & RANDOM_MASK;
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(initial as i64); // safe: mask ensures < 2^48
    Ok(None)
}

/// Native: `Random.<init>(J)V` — seed with explicit long value.
#[allow(
    clippy::unnecessary_wraps,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
pub(crate) fn native_random_init_seed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let seed = match args.get(1).copied() {
        Some(Slot::Long(v)) => v as u64,
        Some(Slot::Int(v)) => v as u64,
        _ => 0,
    };
    let initial = (seed ^ RANDOM_MULTIPLIER) & RANDOM_MASK;
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(initial as i64); // safe: < 2^48
    Ok(None)
}

/// Retrieve and advance seed from `fields[0]`, returning new seed and `bits` high bits.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
fn random_step(heap: &mut duke_gc::Heap, this_ref: u64, bits: u32) -> Result<(u64, i32)> {
    let old_seed = match heap.get(this_ref)?.fields.first().copied() {
        Some(Slot::Long(v)) => v as u64,
        _ => 0,
    };
    let (new_seed, value) = random_next(old_seed, bits);
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(new_seed as i64); // safe: < 2^48
    Ok((new_seed, value))
}

/// Native: `Random.nextInt()I` — full-range random int.
pub(crate) fn native_random_next_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, v) = random_step(heap, this_ref, 32)?;
    Ok(Some(Slot::Int(v)))
}

/// Native: `Random.nextInt(I)I` — bounded random int [0, bound).
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]
pub(crate) fn native_random_next_int_bound(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let bound = extract_int_arg(args, 1)?;
    if bound <= 0 {
        return Err(duke_runtime::Error::JavaException {
            class_name: "java/lang/IllegalArgumentException".to_string(),
        });
    }
    let bound_u = bound as u32;
    // Java rejection-sampling to avoid modulo bias
    loop {
        let (_, bits) = random_step(heap, this_ref, 31)?;
        let bits_u = bits as u32; // bits from next(31) are always non-negative
        let val = bits_u % bound_u;
        if bits_u.wrapping_sub(val).wrapping_add(bound_u - 1) < u32::MAX {
            return Ok(Some(Slot::Int(val as i32))); // val < bound <= i32::MAX
        }
    }
}

/// Native: `Random.nextLong()J` — 64-bit random long (two 32-bit calls).
pub(crate) fn native_random_next_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, hi) = random_step(heap, this_ref, 32)?;
    let (_, lo) = random_step(heap, this_ref, 32)?;
    let v = (i64::from(hi) << 32) + i64::from(lo);
    Ok(Some(Slot::Long(v)))
}

/// Native: `Random.nextDouble()D` — uniform [0.0, 1.0).
#[allow(clippy::cast_precision_loss)]
pub(crate) fn native_random_next_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, hi) = random_step(heap, this_ref, 26)?;
    let (_, lo) = random_step(heap, this_ref, 27)?;
    // Java spec: ((long)(next(26)) << 27) + next(27)) / (double)(1L << 53)
    let combined = (i64::from(hi) << 27) + i64::from(lo);
    let v = combined as f64 / (1u64 << 53) as f64;
    Ok(Some(Slot::Double(v)))
}

/// Native: `Random.nextFloat()F` — uniform [0.0, 1.0) as float.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn native_random_next_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, bits) = random_step(heap, this_ref, 24)?;
    // next(24) is non-negative, safe to cast to f32
    let v = bits as f32 / (1u32 << 24) as f32;
    Ok(Some(Slot::Float(v)))
}

/// Native: `Random.nextBoolean()Z` — random boolean.
pub(crate) fn native_random_next_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, v) = random_step(heap, this_ref, 1)?;
    Ok(Some(Slot::Int(v)))
}

fn byte_array_from_ref(heap: &duke_gc::Heap, array_ref: u64) -> Result<Vec<u8>> {
    let arr = heap.get(array_ref)?;
    let mut bytes = Vec::with_capacity(arr.fields.len());
    for slot in &arr.fields {
        let Slot::Int(v) = slot else {
            return Err(Error::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        };
        bytes.push(v.to_le_bytes()[0]);
    }
    Ok(bytes)
}

fn alloc_byte_array(heap: &mut duke_gc::Heap, bytes: &[u8]) -> u64 {
    let array_ref = heap.allocate("[B".to_string(), bytes.len());
    if let Ok(array) = heap.get_mut(array_ref) {
        for (idx, byte) in bytes.iter().copied().enumerate() {
            array.fields[idx] = Slot::Int(i32::from(byte));
        }
    }
    array_ref
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Base64Variant {
    Standard,
    Mime,
    Url,
}

impl Base64Variant {
    const fn field_value(self) -> i32 {
        match self {
            Self::Standard => 0,
            Self::Mime => 1,
            Self::Url => 2,
        }
    }

    const fn from_field(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Standard),
            1 => Some(Self::Mime),
            2 => Some(Self::Url),
            _ => None,
        }
    }

    const fn alphabet(self) -> &'static [u8; 64] {
        match self {
            Self::Standard | Self::Mime => {
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
            }
            Self::Url => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
        }
    }
}

fn invalid_base64_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    }
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

fn base64_variant_arg(args: &[Slot], heap: &duke_gc::Heap) -> Result<Base64Variant> {
    let coder_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Int(value)) = heap.get(coder_ref)?.fields.first() else {
        return Err(Error::TypeMismatch {
            expected: "Base64 variant",
            got: "other",
        });
    };
    Base64Variant::from_field(*value).ok_or_else(invalid_base64_error)
}

fn encode_base64(input: &[u8], variant: Base64Variant) -> String {
    let alphabet = variant.alphabet();
    let mut out = Vec::with_capacity(input.len().div_ceil(3) * 4);
    let mut line_len = 0_usize;

    for chunk in input.chunks(3) {
        let first = u32::from(chunk[0]);
        let second = chunk.get(1).copied().map_or(0, u32::from);
        let third = chunk.get(2).copied().map_or(0, u32::from);
        let triple = (first << 16) | (second << 8) | third;
        let encoded = [
            alphabet[((triple >> 18) & 0x3f) as usize],
            alphabet[((triple >> 12) & 0x3f) as usize],
            if chunk.len() > 1 {
                alphabet[((triple >> 6) & 0x3f) as usize]
            } else {
                b'='
            },
            if chunk.len() > 2 {
                alphabet[(triple & 0x3f) as usize]
            } else {
                b'='
            },
        ];

        for byte in encoded {
            if variant == Base64Variant::Mime && line_len == 76 {
                out.extend_from_slice(b"\r\n");
                line_len = 0;
            }
            out.push(byte);
            line_len += 1;
        }
    }

    out.into_iter().map(char::from).collect()
}

fn base64_decode_value(byte: u8, variant: Base64Variant) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' if variant != Base64Variant::Url => Some(62),
        b'/' if variant != Base64Variant::Url => Some(63),
        b'-' if variant == Base64Variant::Url => Some(62),
        b'_' if variant == Base64Variant::Url => Some(63),
        _ => None,
    }
}

fn filtered_base64_input(input: &[u8], variant: Base64Variant) -> Vec<u8> {
    input
        .iter()
        .copied()
        .filter(|byte| {
            variant != Base64Variant::Mime || !matches!(byte, b'\r' | b'\n' | b' ' | b'\t')
        })
        .collect()
}

fn require_base64_value(byte: u8, variant: Base64Variant) -> Result<u8> {
    base64_decode_value(byte, variant).ok_or_else(invalid_base64_error)
}

fn push_base64_triplet(out: &mut Vec<u8>, values: [u8; 4]) {
    out.push((values[0] << 2) | (values[1] >> 4));
    out.push(((values[1] & 0x0f) << 4) | (values[2] >> 2));
    out.push(((values[2] & 0x03) << 6) | values[3]);
}

fn decode_base64(input: &[u8], variant: Base64Variant) -> Result<Vec<u8>> {
    let bytes = filtered_base64_input(input, variant);
    if bytes.is_empty() {
        return Ok(Vec::new());
    }

    let mut first_padding = None;
    for (idx, byte) in bytes.iter().copied().enumerate() {
        if byte == b'=' {
            first_padding.get_or_insert(idx);
        } else if first_padding.is_some() || base64_decode_value(byte, variant).is_none() {
            return Err(invalid_base64_error());
        }
    }

    let data_len = first_padding.unwrap_or(bytes.len());
    if let Some(first_padding_idx) = first_padding {
        let pad_count = bytes.len() - first_padding_idx;
        let data_remainder = data_len % 4;
        if pad_count > 2
            || !bytes.len().is_multiple_of(4)
            || (pad_count == 1 && data_remainder != 3)
            || (pad_count == 2 && data_remainder != 2)
        {
            return Err(invalid_base64_error());
        }
    } else if data_len % 4 == 1 {
        return Err(invalid_base64_error());
    }

    let mut out = Vec::with_capacity((data_len / 4) * 3 + 2);
    let mut idx = 0;
    while idx + 4 <= data_len {
        let values = [
            require_base64_value(bytes[idx], variant)?,
            require_base64_value(bytes[idx + 1], variant)?,
            require_base64_value(bytes[idx + 2], variant)?,
            require_base64_value(bytes[idx + 3], variant)?,
        ];
        push_base64_triplet(&mut out, values);
        idx += 4;
    }

    match data_len - idx {
        0 => {}
        2 => {
            let first = require_base64_value(bytes[idx], variant)?;
            let second = require_base64_value(bytes[idx + 1], variant)?;
            out.push((first << 2) | (second >> 4));
        }
        3 => {
            let first = require_base64_value(bytes[idx], variant)?;
            let second = require_base64_value(bytes[idx + 1], variant)?;
            let third = require_base64_value(bytes[idx + 2], variant)?;
            out.push((first << 2) | (second >> 4));
            out.push(((second & 0x0f) << 4) | (third >> 2));
        }
        _ => return Err(invalid_base64_error()),
    }

    Ok(out)
}

pub(crate) fn native_base64_get_encoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let encoder_ref =
        allocate_base64_coder(heap, "java/util/Base64$Encoder", Base64Variant::Standard)?;
    Ok(Some(Slot::Reference(Some(encoder_ref))))
}

pub(crate) fn native_base64_get_mime_encoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let encoder_ref =
        allocate_base64_coder(heap, "java/util/Base64$Encoder", Base64Variant::Mime)?;
    Ok(Some(Slot::Reference(Some(encoder_ref))))
}

pub(crate) fn native_base64_get_url_encoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let encoder_ref = allocate_base64_coder(heap, "java/util/Base64$Encoder", Base64Variant::Url)?;
    Ok(Some(Slot::Reference(Some(encoder_ref))))
}

pub(crate) fn native_base64_get_decoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let decoder_ref =
        allocate_base64_coder(heap, "java/util/Base64$Decoder", Base64Variant::Standard)?;
    Ok(Some(Slot::Reference(Some(decoder_ref))))
}

pub(crate) fn native_base64_get_mime_decoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let decoder_ref =
        allocate_base64_coder(heap, "java/util/Base64$Decoder", Base64Variant::Mime)?;
    Ok(Some(Slot::Reference(Some(decoder_ref))))
}

pub(crate) fn native_base64_get_url_decoder(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let decoder_ref = allocate_base64_coder(heap, "java/util/Base64$Decoder", Base64Variant::Url)?;
    Ok(Some(Slot::Reference(Some(decoder_ref))))
}

pub(crate) fn native_base64_encoder_encode_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let variant = base64_variant_arg(args, heap)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    let string_ref = heap.allocate_string(encode_base64(&input, variant));
    Ok(Some(Slot::Reference(Some(string_ref))))
}

pub(crate) fn native_base64_encoder_encode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let variant = base64_variant_arg(args, heap)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    let encoded = encode_base64(&input, variant);
    let array_ref = alloc_byte_array(heap, &encoded.into_bytes());
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_base64_decoder_decode_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let variant = base64_variant_arg(args, heap)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = string_value_from_ref(heap, input_ref)?;
    let decoded = decode_base64(input.as_bytes(), variant)?;
    let array_ref = alloc_byte_array(heap, &decoded);
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_base64_decoder_decode_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let variant = base64_variant_arg(args, heap)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    let decoded = decode_base64(&input, variant)?;
    let array_ref = alloc_byte_array(heap, &decoded);
    Ok(Some(Slot::Reference(Some(array_ref))))
}

const DUKE_SECURITY_PROVIDER: &str = "DUKE";
const MESSAGE_DIGEST_SERVICE_TYPE: &str = "MessageDigest";
const MESSAGE_DIGEST_ALGORITHMS: [&str; 3] = ["SHA-256", "SHA-1", "MD5"];
const MESSAGE_DIGEST_ALGORITHM_FIELD: usize = 0;
const MESSAGE_DIGEST_BUFFER_FIELD: usize = 1;
const PROVIDER_SERVICE_PROVIDER_FIELD: usize = 0;
const PROVIDER_SERVICE_TYPE_FIELD: usize = 1;
const PROVIDER_SERVICE_ALGORITHM_FIELD: usize = 2;

fn canonical_digest_algorithm(algorithm: &str) -> Option<&'static str> {
    let compact: String = algorithm
        .bytes()
        .filter(|byte| !matches!(byte, b'-' | b'_' | b' ' | b'\t' | b'\r' | b'\n'))
        .map(|byte| char::from(byte.to_ascii_uppercase()))
        .collect();
    match compact.as_str() {
        "SHA256" => Some("SHA-256"),
        "SHA1" => Some("SHA-1"),
        "MD5" => Some("MD5"),
        _ => None,
    }
}

fn require_digest_algorithm(algorithm: &str) -> Result<&'static str> {
    canonical_digest_algorithm(algorithm).ok_or_else(|| Error::JavaException {
        class_name: "java/security/NoSuchAlgorithmException".to_string(),
    })
}

const fn is_duke_provider_name(name: &str) -> bool {
    name.eq_ignore_ascii_case(DUKE_SECURITY_PROVIDER)
}

const fn is_message_digest_service_type(service_type: &str) -> bool {
    service_type.eq_ignore_ascii_case(MESSAGE_DIGEST_SERVICE_TYPE)
}

fn compute_message_digest(algorithm: &str, bytes: &[u8]) -> Result<Vec<u8>> {
    use sha1::Digest as _;
    let digest = match require_digest_algorithm(algorithm)? {
        "SHA-256" => sha2::Sha256::digest(bytes).to_vec(),
        "SHA-1" => sha1::Sha1::digest(bytes).to_vec(),
        "MD5" => md5::Md5::digest(bytes).to_vec(),
        _ => unreachable!("canonical digest algorithm must be supported"),
    };
    Ok(digest)
}

fn message_digest_algorithm(heap: &duke_gc::Heap, digest_ref: u64) -> Result<String> {
    let (string_algorithm, field_algorithm) = {
        let digest = heap.get(digest_ref)?;
        (
            digest.string_value.clone(),
            digest.fields.get(MESSAGE_DIGEST_ALGORITHM_FIELD).copied(),
        )
    };
    if let Some(algorithm) = string_algorithm {
        return Ok(algorithm);
    }
    match field_algorithm {
        Some(Slot::Reference(Some(algorithm_ref))) => string_value_from_ref(heap, algorithm_ref),
        _ => Ok(String::new()),
    }
}

fn message_digest_buffer(heap: &duke_gc::Heap, digest_ref: u64) -> Result<Vec<u8>> {
    match heap
        .get(digest_ref)?
        .fields
        .get(MESSAGE_DIGEST_BUFFER_FIELD)
        .copied()
    {
        Some(Slot::Reference(Some(buffer_ref))) => byte_array_from_ref(heap, buffer_ref),
        _ => Ok(Vec::new()),
    }
}

fn write_message_digest_buffer(
    heap: &mut duke_gc::Heap,
    digest_ref: u64,
    bytes: &[u8],
) -> Result<()> {
    let buffer_slot = if bytes.is_empty() {
        Slot::Reference(None)
    } else {
        Slot::Reference(Some(alloc_byte_array(heap, bytes)))
    };
    let digest = heap.get_mut(digest_ref)?;
    if digest.fields.len() <= MESSAGE_DIGEST_BUFFER_FIELD {
        digest
            .fields
            .resize(MESSAGE_DIGEST_BUFFER_FIELD + 1, Slot::Reference(None));
    }
    digest.fields[MESSAGE_DIGEST_BUFFER_FIELD] = buffer_slot;
    Ok(())
}

fn append_message_digest_buffer(
    heap: &mut duke_gc::Heap,
    digest_ref: u64,
    bytes: &[u8],
) -> Result<()> {
    let mut buffer = message_digest_buffer(heap, digest_ref)?;
    buffer.extend_from_slice(bytes);
    write_message_digest_buffer(heap, digest_ref, &buffer)
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
        service.fields[PROVIDER_SERVICE_PROVIDER_FIELD] = Slot::Reference(Some(provider_ref));
        service.fields[PROVIDER_SERVICE_TYPE_FIELD] = Slot::Reference(Some(type_ref));
        service.fields[PROVIDER_SERVICE_ALGORITHM_FIELD] = Slot::Reference(Some(algorithm_ref));
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
            &[
                Slot::Reference(Some(set_ref)),
                Slot::Reference(Some(algorithm_ref)),
            ],
            heap,
            out,
            control,
        )?;
    }
    Ok(set_ref)
}

/// Native: `MessageDigest.getInstance(String)MessageDigest` — creates a digest for supported algorithms.
pub(crate) fn native_message_digest_get_instance(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let algorithm_ref = extract_ref_arg(args, 0)?;
    let algorithm = string_value_from_ref(heap, algorithm_ref)?;
    let canonical = require_digest_algorithm(&algorithm)?;
    let canonical_ref = heap.allocate_string(canonical.to_string());
    let digest_ref = heap.allocate("java/security/MessageDigest".to_string(), 2);
    let digest = heap.get_mut(digest_ref)?;
    digest.string_value = Some(canonical.to_string());
    digest.fields[MESSAGE_DIGEST_ALGORITHM_FIELD] = Slot::Reference(Some(canonical_ref));
    digest.fields[MESSAGE_DIGEST_BUFFER_FIELD] = Slot::Reference(None);
    Ok(Some(Slot::Reference(Some(digest_ref))))
}

/// Native: `MessageDigest.getInstance(String,String)MessageDigest`.
pub(crate) fn native_message_digest_get_instance_provider_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = extract_ref_arg(args, 1)?;
    let provider_name = string_value_from_ref(heap, provider_ref)?;
    if !is_duke_provider_name(&provider_name) {
        return Err(Error::JavaException {
            class_name: "java/security/NoSuchProviderException".to_string(),
        });
    }
    native_message_digest_get_instance(&[extract_slot_arg(args, 0)], heap, out, control)
}

/// Native: `MessageDigest.getInstance(String,Provider)MessageDigest`.
pub(crate) fn native_message_digest_get_instance_provider(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = extract_ref_arg(args, 1)?;
    let provider_name = heap
        .get(provider_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    if !is_duke_provider_name(&provider_name) {
        return Err(Error::JavaException {
            class_name: "java/security/NoSuchAlgorithmException".to_string(),
        });
    }
    native_message_digest_get_instance(&[extract_slot_arg(args, 0)], heap, out, control)
}

/// Native: `MessageDigest.getAlgorithm()String`.
pub(crate) fn native_message_digest_get_algorithm(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let digest_ref = extract_ref_arg(args, 0)?;
    let algorithm = message_digest_algorithm(heap, digest_ref)?;
    let algorithm_ref = heap.allocate_string(algorithm);
    Ok(Some(Slot::Reference(Some(algorithm_ref))))
}

/// Native: `MessageDigest.getProvider()Provider`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_message_digest_get_provider(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = allocate_duke_provider(heap);
    Ok(Some(Slot::Reference(Some(provider_ref))))
}

/// Native: `MessageDigest.update(byte)V`.
pub(crate) fn native_message_digest_update_byte(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let digest_ref = extract_ref_arg(args, 0)?;
    let byte = extract_int_arg(args, 1)?.to_le_bytes()[0];
    append_message_digest_buffer(heap, digest_ref, &[byte])?;
    Ok(None)
}

/// Native: `MessageDigest.update(byte[])V`.
pub(crate) fn native_message_digest_update_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let digest_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    append_message_digest_buffer(heap, digest_ref, &input)?;
    Ok(None)
}

/// Native: `MessageDigest.update(byte[],int,int)V`.
pub(crate) fn native_message_digest_update_bytes_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let digest_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let offset = extract_int_arg(args, 2)?;
    let len = extract_int_arg(args, 3)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    if offset < 0 || len < 0 {
        return Err(Error::ArrayIndexOutOfBounds {
            index: offset.min(len),
            length: input.len(),
        });
    }
    let start = usize::try_from(offset).map_err(|_| Error::ArrayIndexOutOfBounds {
        index: offset,
        length: input.len(),
    })?;
    let count = usize::try_from(len).map_err(|_| Error::ArrayIndexOutOfBounds {
        index: len,
        length: input.len(),
    })?;
    let Some(end) = start.checked_add(count) else {
        return Err(Error::ArrayIndexOutOfBounds {
            index: offset.saturating_add(len),
            length: input.len(),
        });
    };
    if end > input.len() {
        return Err(Error::ArrayIndexOutOfBounds {
            index: offset.saturating_add(len),
            length: input.len(),
        });
    }
    append_message_digest_buffer(heap, digest_ref, &input[start..end])?;
    Ok(None)
}

/// Native: `MessageDigest.digest([B)[B` — one-shot digest of provided bytes.
pub(crate) fn native_message_digest_digest_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let digest_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let algorithm = message_digest_algorithm(heap, digest_ref)?;
    let mut input = message_digest_buffer(heap, digest_ref)?;
    input.extend(byte_array_from_ref(heap, input_ref)?);
    let output = compute_message_digest(&algorithm, &input)?;
    write_message_digest_buffer(heap, digest_ref, &[])?;
    let out_ref = alloc_byte_array(heap, &output);
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `MessageDigest.digest()[B` — digest buffered bytes and reset state.
pub(crate) fn native_message_digest_digest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let digest_ref = extract_ref_arg(args, 0)?;
    let algorithm = message_digest_algorithm(heap, digest_ref)?;
    let input = message_digest_buffer(heap, digest_ref)?;
    let output = compute_message_digest(&algorithm, &input)?;
    write_message_digest_buffer(heap, digest_ref, &[])?;
    let out_ref = alloc_byte_array(heap, &output);
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `Provider.getName()String`.
pub(crate) fn native_provider_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = extract_ref_arg(args, 0)?;
    let name = heap
        .get(provider_ref)?
        .string_value
        .clone()
        .unwrap_or_else(|| DUKE_SECURITY_PROVIDER.to_string());
    let name_ref = heap.allocate_string(name);
    Ok(Some(Slot::Reference(Some(name_ref))))
}

/// Native: `Provider.getService(String,String)Provider.Service`.
pub(crate) fn native_provider_get_service(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = extract_ref_arg(args, 0)?;
    let type_ref = extract_ref_arg(args, 1)?;
    let algorithm_ref = extract_ref_arg(args, 2)?;
    let provider_name = heap
        .get(provider_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let service_type = string_value_from_ref(heap, type_ref)?;
    let algorithm = string_value_from_ref(heap, algorithm_ref)?;
    let Some(canonical) = canonical_digest_algorithm(&algorithm) else {
        return Ok(Some(Slot::Reference(None)));
    };
    if !is_duke_provider_name(&provider_name) || !is_message_digest_service_type(&service_type) {
        return Ok(Some(Slot::Reference(None)));
    }
    let service_ref =
        allocate_provider_service(heap, provider_ref, MESSAGE_DIGEST_SERVICE_TYPE, canonical);
    Ok(Some(Slot::Reference(Some(service_ref))))
}

/// Native: `Provider.Service.getAlgorithm()String`.
pub(crate) fn native_provider_service_get_algorithm(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let service_ref = extract_ref_arg(args, 0)?;
    Ok(Some(extract_field_arg(
        heap,
        service_ref,
        PROVIDER_SERVICE_ALGORITHM_FIELD,
    )?))
}

/// Native: `Provider.Service.getType()String`.
pub(crate) fn native_provider_service_get_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let service_ref = extract_ref_arg(args, 0)?;
    Ok(Some(extract_field_arg(
        heap,
        service_ref,
        PROVIDER_SERVICE_TYPE_FIELD,
    )?))
}

/// Native: `Provider.Service.getProvider()Provider`.
pub(crate) fn native_provider_service_get_provider(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let service_ref = extract_ref_arg(args, 0)?;
    Ok(Some(extract_field_arg(
        heap,
        service_ref,
        PROVIDER_SERVICE_PROVIDER_FIELD,
    )?))
}

/// Native: `Security.getProvider(String)Provider`.
pub(crate) fn native_security_get_provider(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let name = string_value_from_ref(heap, name_ref)?;
    if !is_duke_provider_name(&name) {
        return Ok(Some(Slot::Reference(None)));
    }
    let provider_ref = allocate_duke_provider(heap);
    Ok(Some(Slot::Reference(Some(provider_ref))))
}

/// Native: `Security.getProviders()Provider[]`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_security_get_providers(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = allocate_duke_provider(heap);
    let provider_array_ref = heap.allocate("[Ljava/security/Provider;".to_string(), 1);
    if let Ok(providers) = heap.get_mut(provider_array_ref) {
        providers.fields[0] = Slot::Reference(Some(provider_ref));
    }
    Ok(Some(Slot::Reference(Some(provider_array_ref))))
}

/// Native: `Security.getAlgorithms(String)Set`.
pub(crate) fn native_security_get_algorithms(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let service_type_ref = extract_ref_arg(args, 0)?;
    let service_type = string_value_from_ref(heap, service_type_ref)?;
    let algorithms = if is_message_digest_service_type(&service_type) {
        &MESSAGE_DIGEST_ALGORITHMS[..]
    } else {
        &[][..]
    };
    let set_ref = allocate_algorithm_set(heap, out, control, algorithms)?;
    Ok(Some(Slot::Reference(Some(set_ref))))
}

/// Native: `SecureRandom.nextBytes([B)V` — fills target array using host CSPRNG.
pub(crate) fn native_secure_random_next_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let array_ref = extract_ref_arg(args, 1)?;
    let mut bytes = vec![0_u8; heap.get(array_ref)?.fields.len()];
    getrandom::fill(&mut bytes).map_err(|_| Error::JavaException {
        class_name: "java/lang/InternalError".to_string(),
    })?;
    let arr = heap.get_mut(array_ref)?;
    for (idx, byte) in bytes.iter().copied().enumerate() {
        arr.fields[idx] = Slot::Int(i32::from(byte));
    }
    Ok(None)
}

/// Native: `SecureRandom.generateSeed(I)[B` — returns a fresh random byte array.
pub(crate) fn native_secure_random_generate_seed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let len = usize::try_from(extract_int_arg(args, 1)?.max(0)).unwrap_or(0);
    let mut bytes = vec![0_u8; len];
    getrandom::fill(&mut bytes).map_err(|_| Error::JavaException {
        class_name: "java/lang/InternalError".to_string(),
    })?;
    let out_ref = alloc_byte_array(heap, &bytes);
    Ok(Some(Slot::Reference(Some(out_ref))))
}

const UUID_MSB_FIELD: usize = 0;
const UUID_LSB_FIELD: usize = 1;

fn uuid_bits_from_object(obj: &duke_gc::HeapObject) -> Option<(i64, i64)> {
    match (
        obj.fields.get(UUID_MSB_FIELD),
        obj.fields.get(UUID_LSB_FIELD),
    ) {
        (Some(Slot::Long(msb)), Some(Slot::Long(lsb))) => Some((*msb, *lsb)),
        _ => None,
    }
}

fn uuid_bits_from_ref(heap: &duke_gc::Heap, uuid_ref: u64) -> Result<(i64, i64)> {
    uuid_bits_from_object(heap.get(uuid_ref)?).ok_or(Error::TypeMismatch {
        expected: "UUID fields",
        got: "other",
    })
}

fn write_uuid_bits(heap: &mut duke_gc::Heap, uuid_ref: u64, msb: i64, lsb: i64) -> Result<()> {
    let uuid = heap.get_mut(uuid_ref)?;
    if uuid.fields.len() <= UUID_LSB_FIELD {
        uuid.fields.resize(UUID_LSB_FIELD + 1, Slot::Long(0));
    }
    uuid.fields[UUID_MSB_FIELD] = Slot::Long(msb);
    uuid.fields[UUID_LSB_FIELD] = Slot::Long(lsb);
    Ok(())
}

fn allocate_uuid(heap: &mut duke_gc::Heap, msb: i64, lsb: i64) -> Result<u64> {
    let uuid_ref = heap.allocate("java/util/UUID".to_string(), 2);
    write_uuid_bits(heap, uuid_ref, msb, lsb)?;
    Ok(uuid_ref)
}

fn uuid_bits_from_bytes(bytes: &[u8; 16]) -> (i64, i64) {
    let mut msb_bytes = [0_u8; 8];
    let mut lsb_bytes = [0_u8; 8];
    msb_bytes.copy_from_slice(&bytes[..8]);
    lsb_bytes.copy_from_slice(&bytes[8..]);
    (i64::from_be_bytes(msb_bytes), i64::from_be_bytes(lsb_bytes))
}

fn uuid_invalid_format() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    }
}

fn uuid_hex_nibble(byte: u8) -> Option<u64> {
    match byte {
        b'0'..=b'9' => Some(u64::from(byte - b'0')),
        b'a'..=b'f' => Some(u64::from(byte - b'a' + 10)),
        b'A'..=b'F' => Some(u64::from(byte - b'A' + 10)),
        _ => None,
    }
}

fn parse_uuid_hex_u64(bytes: &[u8]) -> Result<u64> {
    let mut value = 0_u64;
    for byte in bytes {
        let nibble = uuid_hex_nibble(*byte).ok_or_else(uuid_invalid_format)?;
        value = (value << 4) | nibble;
    }
    Ok(value)
}

fn parse_uuid_string(text: &str) -> Result<(i64, i64)> {
    let bytes = text.as_bytes();
    if bytes.len() != 36
        || bytes[8] != b'-'
        || bytes[13] != b'-'
        || bytes[18] != b'-'
        || bytes[23] != b'-'
    {
        return Err(uuid_invalid_format());
    }
    let msb = (parse_uuid_hex_u64(&bytes[0..8])? << 32)
        | (parse_uuid_hex_u64(&bytes[9..13])? << 16)
        | parse_uuid_hex_u64(&bytes[14..18])?;
    let lsb = (parse_uuid_hex_u64(&bytes[19..23])? << 48)
        | parse_uuid_hex_u64(&bytes[24..36])?;
    Ok((msb.cast_signed(), lsb.cast_signed()))
}

fn uuid_to_string(msb: i64, lsb: i64) -> String {
    let msb = msb.cast_unsigned();
    let lsb = lsb.cast_unsigned();
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (msb >> 32) & 0xffff_ffff,
        (msb >> 16) & 0xffff,
        msb & 0xffff,
        (lsb >> 48) & 0xffff,
        lsb & 0xffff_ffff_ffff
    )
}

/// Native: `UUID.<init>(long,long)V` — stores the two canonical 64-bit halves.
pub(crate) fn native_uuid_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let msb = extract_long_arg(args, 1)?;
    let lsb = extract_long_arg(args, 2)?;
    write_uuid_bits(heap, this_ref, msb, lsb)?;
    Ok(None)
}

/// Native: `UUID.randomUUID()UUID` — RFC 4122 version-4 UUID from host CSPRNG.
pub(crate) fn native_uuid_random_uuid(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| Error::JavaException {
        class_name: "java/lang/InternalError".to_string(),
    })?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let (msb, lsb) = uuid_bits_from_bytes(&bytes);
    let uuid_ref = allocate_uuid(heap, msb, lsb)?;
    Ok(Some(Slot::Reference(Some(uuid_ref))))
}

/// Native: `UUID.nameUUIDFromBytes(byte[])UUID` — RFC 4122 version-3 MD5 UUID.
pub(crate) fn native_uuid_name_uuid_from_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let input_ref = extract_ref_arg(args, 0)?;
    let input = byte_array_from_ref(heap, input_ref)?;
    let digest = compute_message_digest("MD5", &input)?;
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let (msb, lsb) = uuid_bits_from_bytes(&bytes);
    let uuid_ref = allocate_uuid(heap, msb, lsb)?;
    Ok(Some(Slot::Reference(Some(uuid_ref))))
}

/// Native: `UUID.fromString(String)UUID` — parses canonical 8-4-4-4-12 UUID text.
pub(crate) fn native_uuid_from_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let text_ref = extract_ref_arg(args, 0)?;
    let text = string_value_from_ref(heap, text_ref)?;
    let (msb, lsb) = parse_uuid_string(&text)?;
    let uuid_ref = allocate_uuid(heap, msb, lsb)?;
    Ok(Some(Slot::Reference(Some(uuid_ref))))
}

/// Native: `UUID.getMostSignificantBits()long`.
pub(crate) fn native_uuid_get_most_significant_bits(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (msb, _) = uuid_bits_from_ref(heap, this_ref)?;
    Ok(Some(Slot::Long(msb)))
}

/// Native: `UUID.getLeastSignificantBits()long`.
pub(crate) fn native_uuid_get_least_significant_bits(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, lsb) = uuid_bits_from_ref(heap, this_ref)?;
    Ok(Some(Slot::Long(lsb)))
}

/// Native: `UUID.version()int`.
pub(crate) fn native_uuid_version(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (msb, _) = uuid_bits_from_ref(heap, this_ref)?;
    let version = i32::try_from((msb.cast_unsigned() >> 12) & 0x0f)
        .expect("UUID version nibble fits in i32");
    Ok(Some(Slot::Int(version)))
}

/// Native: `UUID.variant()int`.
pub(crate) fn native_uuid_variant(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (_, lsb) = uuid_bits_from_ref(heap, this_ref)?;
    let lsb = lsb.cast_unsigned();
    let variant = if (lsb >> 63) == 0 {
        0
    } else if (lsb >> 62) == 0b10 {
        2
    } else if (lsb >> 61) == 0b110 {
        6
    } else {
        7
    };
    Ok(Some(Slot::Int(variant)))
}

/// Native: `UUID.toString()String`.
pub(crate) fn native_uuid_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (msb, lsb) = uuid_bits_from_ref(heap, this_ref)?;
    let string_ref = heap.allocate_string(uuid_to_string(msb, lsb));
    Ok(Some(Slot::Reference(Some(string_ref))))
}

/// Native: `UUID.equals(Object)boolean`.
pub(crate) fn native_uuid_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    let this_bits = uuid_bits_from_ref(heap, this_ref)?;
    let other = heap.get(other_ref)?;
    let equal = other.class_name == "java/util/UUID"
        && uuid_bits_from_object(other).is_some_and(|other_bits| other_bits == this_bits);
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `UUID.hashCode()int`.
pub(crate) fn native_uuid_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (msb, lsb) = uuid_bits_from_ref(heap, this_ref)?;
    let hilo = msb.cast_unsigned() ^ lsb.cast_unsigned();
    let high = u32::try_from(hilo >> 32).expect("upper 32 bits fit in u32");
    let low = u32::try_from(hilo & 0xffff_ffff).expect("lower 32 bits fit in u32");
    Ok(Some(Slot::Int((high ^ low).cast_signed())))
}

/// Native: `UUID.compareTo(UUID)int`.
pub(crate) fn native_uuid_compare_to(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let this_bits = uuid_bits_from_ref(heap, this_ref)?;
    let other_bits = uuid_bits_from_ref(heap, other_ref)?;
    let ordering = this_bits
        .0
        .cmp(&other_bits.0)
        .then_with(|| this_bits.1.cmp(&other_bits.1));
    Ok(Some(Slot::Int(match ordering {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    })))
}

/// Native: version-1 UUID accessors are intentionally deferred for v3/v4 UUIDs.
pub(crate) fn native_uuid_unsupported_version1_accessor(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Err(Error::JavaException {
        class_name: "java/lang/UnsupportedOperationException".to_string(),
    })
}

// ---------------------------------------------------------------------------
// java.util.regex.Pattern / Matcher
// Pattern: string_value = regex string.
// Matcher: fields[0]=Pattern ref, fields[1]=input ref, fields[2]=pos,
//          fields[3]=match_start (-1=no match), fields[4]=match_end;
//          string_value = last matched text.
// ---------------------------------------------------------------------------

const PATTERN_UNIX_LINES: i32 = 1;
const PATTERN_CASE_INSENSITIVE: i32 = 2;
const PATTERN_COMMENTS: i32 = 4;
const PATTERN_MULTILINE: i32 = 8;
const PATTERN_LITERAL: i32 = 16;
const PATTERN_DOTALL: i32 = 32;
const PATTERN_UNICODE_CASE: i32 = 64;
const PATTERN_CANON_EQ: i32 = 128;
const PATTERN_UNICODE_CHARACTER_CLASS: i32 = 256;
const PATTERN_SUPPORTED_FLAGS: i32 = PATTERN_UNIX_LINES
    | PATTERN_CASE_INSENSITIVE
    | PATTERN_COMMENTS
    | PATTERN_MULTILINE
    | PATTERN_LITERAL
    | PATTERN_DOTALL
    | PATTERN_UNICODE_CASE
    | PATTERN_CANON_EQ
    | PATTERN_UNICODE_CHARACTER_CLASS;

const PATTERN_FLAGS_FIELD: usize = 0;

const MATCHER_PATTERN_FIELD: usize = 0;
const MATCHER_INPUT_FIELD: usize = 1;
const MATCHER_POS_FIELD: usize = 2;
const MATCHER_MATCH_START_FIELD: usize = 3;
const MATCHER_MATCH_END_FIELD: usize = 4;
const MATCHER_APPEND_POS_FIELD: usize = 5;
const MATCHER_FIELD_COUNT: usize = 6;

#[derive(Clone, Copy)]
enum MatcherGroup<'a> {
    Index(usize),
    Name(&'a str),
}

fn regex_java_exception(class_name: &str, message: impl Into<String>) -> Error {
    push_pending_java_exception_message(class_name, message.into());
    Error::JavaException {
        class_name: class_name.to_string(),
    }
}

fn regex_pattern_syntax_error(message: impl Into<String>) -> Error {
    regex_java_exception("java/util/regex/PatternSyntaxException", message)
}

fn regex_illegal_argument(message: impl Into<String>) -> Error {
    regex_java_exception("java/lang/IllegalArgumentException", message)
}

fn regex_illegal_state(message: impl Into<String>) -> Error {
    regex_java_exception("java/lang/IllegalStateException", message)
}

fn regex_index_out_of_bounds(message: impl Into<String>) -> Error {
    regex_java_exception("java/lang/IndexOutOfBoundsException", message)
}

fn validate_pattern_flags(flags: i32) -> Result<()> {
    if flags & !PATTERN_SUPPORTED_FLAGS != 0 {
        return Err(regex_illegal_argument(format!("Unknown regex flags: {flags}")));
    }
    if flags & PATTERN_CANON_EQ != 0 {
        return Err(regex_pattern_syntax_error(
            "CANON_EQ is not supported by Duke's regex engine",
        ));
    }
    Ok(())
}

fn translate_java_named_groups(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    let mut chars = pattern.chars();
    while let Some(ch) = chars.next() {
        if ch == '(' {
            let mut probe = chars.clone();
            if probe.next() == Some('?') && probe.next() == Some('<') {
                match probe.next() {
                    Some('=' | '!') | None => out.push(ch),
                    Some(_) => {
                        out.push_str("(?P<");
                        chars.next();
                        chars.next();
                    }
                }
            } else {
                out.push(ch);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn copy_group_name(chars: &[char], start: usize, out: &mut String) -> Option<usize> {
    let mut idx = start;
    while idx < chars.len() {
        let ch = chars[idx];
        out.push(ch);
        idx += 1;
        if ch == '>' {
            return Some(idx);
        }
    }
    None
}

fn expand_ascii_case_insensitive(pattern: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::with_capacity(pattern.len());
    let mut idx = 0;
    let mut escaped = false;
    let mut in_class = false;
    while idx < chars.len() {
        let ch = chars[idx];
        if escaped {
            out.push(ch);
            escaped = false;
            idx += 1;
            continue;
        }
        if ch == '\\' {
            out.push(ch);
            escaped = true;
            idx += 1;
            continue;
        }
        if !in_class && ch == '(' && chars.get(idx + 1) == Some(&'?') {
            if chars.get(idx + 2) == Some(&'P') && chars.get(idx + 3) == Some(&'<') {
                out.push_str("(?P<");
                if let Some(next_idx) = copy_group_name(&chars, idx + 4, &mut out) {
                    idx = next_idx;
                    continue;
                }
            } else if chars.get(idx + 2) == Some(&'<')
                && !matches!(chars.get(idx + 3), Some('=' | '!') | None)
            {
                out.push_str("(?<");
                if let Some(next_idx) = copy_group_name(&chars, idx + 3, &mut out) {
                    idx = next_idx;
                    continue;
                }
            }
        }
        match ch {
            '[' => {
                in_class = true;
                out.push(ch);
            }
            ']' if in_class => {
                in_class = false;
                out.push(ch);
            }
            _ if !in_class && ch.is_ascii_alphabetic() => {
                out.push('[');
                out.push(ch.to_ascii_lowercase());
                out.push(ch.to_ascii_uppercase());
                out.push(']');
            }
            _ => out.push(ch),
        }
        idx += 1;
    }
    out
}

/// Helper: compile a regex from a pattern string.
/// Returns `Err` with `JavaException` on bad pattern.
fn compile_java_regex(pattern: &str) -> Result<regex::Regex> {
    compile_java_regex_with_flags(pattern, 0)
}

fn compile_java_regex_with_flags(pattern: &str, flags: i32) -> Result<regex::Regex> {
    validate_pattern_flags(flags)?;
    let mut source = if flags & PATTERN_LITERAL != 0 {
        regex::escape(pattern)
    } else {
        translate_java_named_groups(pattern)
    };
    let ascii_case_insensitive =
        flags & PATTERN_CASE_INSENSITIVE != 0 && flags & PATTERN_UNICODE_CASE == 0;
    if ascii_case_insensitive {
        source = expand_ascii_case_insensitive(&source);
    }

    let mut builder = regex::RegexBuilder::new(&source);
    builder
        .case_insensitive(flags & PATTERN_CASE_INSENSITIVE != 0 && !ascii_case_insensitive)
        .multi_line(flags & PATTERN_MULTILINE != 0)
        .dot_matches_new_line(flags & PATTERN_DOTALL != 0)
        .ignore_whitespace(flags & PATTERN_COMMENTS != 0)
        .unicode(true);
    builder.build().map_err(|e| regex_pattern_syntax_error(e.to_string()))
}

fn pattern_text_and_flags(heap: &duke_gc::Heap, pat_ref: u64) -> Result<(String, i32)> {
    let pat = heap.get(pat_ref)?;
    let pattern_str = pat.string_value.clone().unwrap_or_default();
    let flags = match pat.fields.get(PATTERN_FLAGS_FIELD).copied() {
        Some(Slot::Int(flags)) => flags,
        _ => 0,
    };
    Ok((pattern_str, flags))
}

fn allocate_pattern(heap: &mut duke_gc::Heap, pattern_str: String, flags: i32) -> Result<u64> {
    let pat_ref = heap.allocate("java/util/regex/Pattern".to_string(), 1);
    let pat = heap.get_mut(pat_ref)?;
    pat.fields[PATTERN_FLAGS_FIELD] = Slot::Int(flags);
    pat.string_value = Some(pattern_str);
    Ok(pat_ref)
}

/// Native: `Pattern.compile(String)Pattern` — static factory.
pub(crate) fn native_pattern_compile(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let pat_str_ref = extract_ref_arg(args, 0)?;
    let pattern_str = heap
        .get(pat_str_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    // Validate the regex eagerly so we fail here not at match time.
    compile_java_regex(&pattern_str)?;
    let pat_ref = allocate_pattern(heap, pattern_str, 0)?;
    Ok(Some(Slot::Reference(Some(pat_ref))))
}

/// Native: `Pattern.compile(String,int)Pattern` — static factory with flags.
pub(crate) fn native_pattern_compile_flags(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let pat_str_ref = extract_ref_arg(args, 0)?;
    let flags = extract_int_arg(args, 1)?;
    let pattern_str = heap
        .get(pat_str_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    compile_java_regex_with_flags(&pattern_str, flags)?;
    let pat_ref = allocate_pattern(heap, pattern_str, flags)?;
    Ok(Some(Slot::Reference(Some(pat_ref))))
}

/// Native: `Pattern.matcher(CharSequence)Matcher` — creates a Matcher.
pub(crate) fn native_pattern_matcher(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let pat_ref = extract_ref_arg(args, 0)?;
    let input_slot = extract_slot_arg(args, 1);
    // fields: pattern, input, next find position, last match start/end, append position
    let m_ref = heap.allocate("java/util/regex/Matcher".to_string(), MATCHER_FIELD_COUNT);
    heap.get_mut(m_ref)?.fields[MATCHER_PATTERN_FIELD] = Slot::Reference(Some(pat_ref));
    heap.get_mut(m_ref)?.fields[MATCHER_INPUT_FIELD] = input_slot;
    reset_matcher_fields(heap, m_ref)?;
    Ok(Some(Slot::Reference(Some(m_ref))))
}

/// Native: `Pattern.matches(String,CharSequence)Z` — static full-string match.
pub(crate) fn native_pattern_matches_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let pat_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re
        .find(&input)
        .is_some_and(|m| m.start() == 0 && m.end() == input.len());
    Ok(Some(Slot::Int(i32::from(result))))
}

fn regex_split_parts(re: &regex::Regex, input: &str, limit: i32) -> Vec<String> {
    if limit > 0 {
        return re
            .splitn(input, usize::try_from(limit).unwrap_or(usize::MAX))
            .map(str::to_string)
            .collect();
    }
    let mut parts: Vec<String> = re.split(input).map(str::to_string).collect();
    if limit == 0 {
        while parts.last().is_some_and(String::is_empty) {
            parts.pop();
        }
    }
    parts
}

fn alloc_string_array_from_parts(heap: &mut duke_gc::Heap, parts: &[String]) -> Result<u64> {
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), parts.len());
    for (idx, part) in parts.iter().enumerate() {
        let str_ref = heap.allocate_string(part.clone());
        heap.get_mut(arr_ref)?.fields[idx] = Slot::Reference(Some(str_ref));
    }
    Ok(arr_ref)
}

fn pattern_split_impl(args: &[Slot], heap: &mut duke_gc::Heap, limit: i32) -> Result<Option<Slot>> {
    let pat_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let parts = regex_split_parts(&re, &input, limit);
    let arr_ref = alloc_string_array_from_parts(heap, &parts)?;
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `Pattern.split(CharSequence)String[]`.
pub(crate) fn native_pattern_split(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    pattern_split_impl(args, heap, 0)
}

/// Native: `Pattern.split(CharSequence,int)String[]`.
pub(crate) fn native_pattern_split_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    pattern_split_impl(args, heap, extract_int_arg(args, 2)?)
}

fn matcher_pattern_input(heap: &duke_gc::Heap, m_ref: u64) -> Result<Option<(u64, u64)>> {
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
        return Ok(None);
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
        return Ok(None);
    };
    Ok(Some((pat_ref, input_ref)))
}

fn matcher_pattern_input_text(
    heap: &duke_gc::Heap,
    m_ref: u64,
) -> Result<Option<(String, i32, String)>> {
    let Some((pat_ref, input_ref)) = matcher_pattern_input(heap, m_ref)? else {
        return Ok(None);
    };
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    Ok(Some((pattern_str, flags, input)))
}

fn matcher_field_int(heap: &duke_gc::Heap, m_ref: u64, field: usize, default: i32) -> Result<i32> {
    Ok(match heap.get(m_ref)?.fields.get(field).copied() {
        Some(Slot::Int(value)) => value,
        _ => default,
    })
}

fn set_matcher_no_match(heap: &mut duke_gc::Heap, m_ref: u64) -> Result<()> {
    let matcher = heap.get_mut(m_ref)?;
    matcher.fields[MATCHER_MATCH_START_FIELD] = Slot::Int(-1);
    matcher.fields[MATCHER_MATCH_END_FIELD] = Slot::Int(0);
    matcher.string_value = None;
    Ok(())
}

fn reset_matcher_fields(heap: &mut duke_gc::Heap, m_ref: u64) -> Result<()> {
    let matcher = heap.get_mut(m_ref)?;
    matcher.fields[MATCHER_POS_FIELD] = Slot::Int(0);
    matcher.fields[MATCHER_MATCH_START_FIELD] = Slot::Int(-1);
    matcher.fields[MATCHER_MATCH_END_FIELD] = Slot::Int(0);
    matcher.fields[MATCHER_APPEND_POS_FIELD] = Slot::Int(0);
    matcher.string_value = None;
    Ok(())
}

fn next_find_pos(input: &str, start: usize, end: usize) -> usize {
    if start != end || end >= input.len() {
        return end;
    }
    input[end..]
        .chars()
        .next()
        .map_or(end, |ch| end + ch.len_utf8())
}

fn store_matcher_match(
    heap: &mut duke_gc::Heap,
    m_ref: u64,
    input: &str,
    start: usize,
    end: usize,
) -> Result<()> {
    let next_pos = next_find_pos(input, start, end);
    let start_i32 = i32::try_from(start).unwrap_or(i32::MAX);
    let end_i32 = i32::try_from(end).unwrap_or(i32::MAX);
    let next_i32 = i32::try_from(next_pos).unwrap_or(i32::MAX);
    let matcher = heap.get_mut(m_ref)?;
    matcher.fields[MATCHER_POS_FIELD] = Slot::Int(next_i32);
    matcher.fields[MATCHER_MATCH_START_FIELD] = Slot::Int(start_i32);
    matcher.fields[MATCHER_MATCH_END_FIELD] = Slot::Int(end_i32);
    matcher.string_value = Some(input[start..end].to_string());
    Ok(())
}

fn last_match_bounds(heap: &duke_gc::Heap, m_ref: u64) -> Result<(usize, usize)> {
    let start = matcher_field_int(heap, m_ref, MATCHER_MATCH_START_FIELD, -1)?;
    let end = matcher_field_int(heap, m_ref, MATCHER_MATCH_END_FIELD, 0)?;
    if start < 0 {
        return Err(regex_illegal_state("No match available"));
    }
    Ok((
        usize::try_from(start).unwrap_or(0),
        usize::try_from(end.max(0)).unwrap_or(0),
    ))
}

fn matcher_group_bounds(
    heap: &duke_gc::Heap,
    m_ref: u64,
    group: MatcherGroup<'_>,
) -> Result<Option<(usize, usize)>> {
    let (match_start, match_end) = last_match_bounds(heap, m_ref)?;
    let Some((pattern_str, flags, input)) = matcher_pattern_input_text(heap, m_ref)? else {
        return Err(regex_illegal_state("No match available"));
    };
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let Some(caps) = re.captures_at(&input, match_start) else {
        return Err(regex_illegal_state("No match available"));
    };
    let Some(whole) = caps.get(0) else {
        return Err(regex_illegal_state("No match available"));
    };
    if whole.start() != match_start || whole.end() != match_end {
        return Err(regex_illegal_state("No match available"));
    }
    match group {
        MatcherGroup::Index(index) => {
            if index >= caps.len() {
                return Err(regex_index_out_of_bounds(format!("No group {index}")));
            }
            Ok(caps.get(index).map(|m| (m.start(), m.end())))
        }
        MatcherGroup::Name(name) => {
            if !re.capture_names().flatten().any(|candidate| candidate == name) {
                return Err(regex_illegal_argument(format!(
                    "No group with name <{name}>"
                )));
            }
            Ok(caps.name(name).map(|m| (m.start(), m.end())))
        }
    }
}

fn matcher_group_text(
    heap: &duke_gc::Heap,
    m_ref: u64,
    group: MatcherGroup<'_>,
) -> Result<Option<String>> {
    let Some((_, _, input)) = matcher_pattern_input_text(heap, m_ref)? else {
        return Err(regex_illegal_state("No match available"));
    };
    Ok(matcher_group_bounds(heap, m_ref, group)?.map(|(start, end)| input[start..end].to_string()))
}

/// Native: `Matcher.find()Z` — finds next match; advances position.
pub(crate) fn native_matcher_find(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
        return Ok(Some(Slot::Int(0)));
    };
    let input_slot = fields
        .get(MATCHER_INPUT_FIELD)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(input_ref)) = input_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let pos = match fields.get(MATCHER_POS_FIELD).copied() {
        Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        _ => 0,
    };
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    if let Some(m) = re.find_at(&input, pos.min(input.len())) {
        store_matcher_match(heap, m_ref, &input, m.start(), m.end())?;
        Ok(Some(Slot::Int(1)))
    } else {
        set_matcher_no_match(heap, m_ref)?;
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `Matcher.matches()Z` — full-string match (resets position).
pub(crate) fn native_matcher_matches(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
        return Ok(Some(Slot::Int(0)));
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
        return Ok(Some(Slot::Int(0)));
    };
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let matched = re
        .find(&input)
        .filter(|m| m.start() == 0 && m.end() == input.len());
    if let Some(m) = matched {
        store_matcher_match(heap, m_ref, &input, m.start(), m.end())?;
    } else {
        set_matcher_no_match(heap, m_ref)?;
    }
    Ok(Some(Slot::Int(i32::from(matched.is_some()))))
}

/// Native: `Matcher.group()String` — returns text of last match.
pub(crate) fn native_matcher_group(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let Some(matched) = matcher_group_text(heap, m_ref, MatcherGroup::Index(0))? else {
        return Ok(Some(Slot::Reference(None)));
    };
    let r = heap.allocate_string(matched);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Matcher.group(int)String` — returns the nth capture group from the last match.
pub(crate) fn native_matcher_group_n(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Int(n)) if n >= 0 => usize::try_from(n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(regex_index_out_of_bounds(format!("No group {n}"))),
        _ => 0,
    };
    if n == 0 {
        // group(0) == group() — full match
        return native_matcher_group(args, heap, out, control);
    }
    let Some(group) = matcher_group_text(heap, m_ref, MatcherGroup::Index(n))? else {
        return Ok(Some(Slot::Reference(None)));
    };
    let s = heap.allocate_string(group);
    Ok(Some(Slot::Reference(Some(s))))
}

/// Native: `Matcher.group(String)String` — returns a named capture group.
pub(crate) fn native_matcher_group_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let Some(group) = matcher_group_text(heap, m_ref, MatcherGroup::Name(&name))? else {
        return Ok(Some(Slot::Reference(None)));
    };
    let s = heap.allocate_string(group);
    Ok(Some(Slot::Reference(Some(s))))
}

/// Native: `Matcher.groupCount()I` — number of capturing groups, excluding group 0.
pub(crate) fn native_matcher_group_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let Some((pattern_str, flags, _)) = matcher_pattern_input_text(heap, m_ref)? else {
        return Ok(Some(Slot::Int(0)));
    };
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let count = re.captures_len().saturating_sub(1);
    Ok(Some(Slot::Int(i32::try_from(count).unwrap_or(i32::MAX))))
}

/// Native: `Matcher.start()I` — start index of last match.
pub(crate) fn native_matcher_start(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let start = matcher_group_bounds(heap, m_ref, MatcherGroup::Index(0))?
        .map_or(-1, |(start, _)| i32::try_from(start).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(start)))
}

/// Native: `Matcher.start(String)I` — start index of a named capture group.
pub(crate) fn native_matcher_start_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let start = matcher_group_bounds(heap, m_ref, MatcherGroup::Name(&name))?
        .map_or(-1, |(start, _)| i32::try_from(start).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(start)))
}

/// Native: `Matcher.end()I` — exclusive end index of last match.
pub(crate) fn native_matcher_end(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let end = matcher_group_bounds(heap, m_ref, MatcherGroup::Index(0))?
        .map_or(-1, |(_, end)| i32::try_from(end).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(end)))
}

/// Native: `Matcher.end(String)I` — exclusive end index of a named capture group.
pub(crate) fn native_matcher_end_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let end = matcher_group_bounds(heap, m_ref, MatcherGroup::Name(&name))?
        .map_or(-1, |(_, end)| i32::try_from(end).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(end)))
}

/// Native: `Matcher.reset()Matcher` — reset state against the current input.
pub(crate) fn native_matcher_reset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    reset_matcher_fields(heap, m_ref)?;
    Ok(Some(Slot::Reference(Some(m_ref))))
}

/// Native: `Matcher.reset(CharSequence)Matcher` — reset state against new input.
pub(crate) fn native_matcher_reset_input(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let input_slot = extract_slot_arg(args, 1);
    heap.get_mut(m_ref)?.fields[MATCHER_INPUT_FIELD] = input_slot;
    reset_matcher_fields(heap, m_ref)?;
    Ok(Some(Slot::Reference(Some(m_ref))))
}

fn append_to_string_builder(heap: &mut duke_gc::Heap, builder_ref: u64, text: &str) -> Result<()> {
    let builder = heap.get_mut(builder_ref)?;
    builder
        .string_value
        .get_or_insert_with(String::new)
        .push_str(text);
    Ok(())
}

/// Native: `Matcher.appendReplacement(StringBuilder,String)Matcher`.
pub(crate) fn native_matcher_append_replacement_sb(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let builder_ref = extract_ref_arg(args, 1)?;
    let replacement_ref = extract_ref_arg(args, 2)?;
    let replacement = heap
        .get(replacement_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let (match_start, match_end) = last_match_bounds(heap, m_ref)?;
    let append_pos = usize::try_from(matcher_field_int(
        heap,
        m_ref,
        MATCHER_APPEND_POS_FIELD,
        0,
    )?)
    .unwrap_or(0);
    let Some((pattern_str, flags, input)) = matcher_pattern_input_text(heap, m_ref)? else {
        return Err(regex_illegal_state("No match available"));
    };
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let Some(caps) = re.captures_at(&input, match_start) else {
        return Err(regex_illegal_state("No match available"));
    };
    let Some(whole) = caps.get(0) else {
        return Err(regex_illegal_state("No match available"));
    };
    if whole.start() != match_start || whole.end() != match_end {
        return Err(regex_illegal_state("No match available"));
    }
    let mut expanded = String::new();
    caps.expand(&replacement, &mut expanded);
    let safe_append_pos = append_pos.min(match_start);
    append_to_string_builder(heap, builder_ref, &input[safe_append_pos..match_start])?;
    append_to_string_builder(heap, builder_ref, &expanded)?;
    heap.get_mut(m_ref)?.fields[MATCHER_APPEND_POS_FIELD] =
        Slot::Int(i32::try_from(match_end).unwrap_or(i32::MAX));
    Ok(Some(Slot::Reference(Some(m_ref))))
}

/// Native: `Matcher.appendTail(StringBuilder)StringBuilder`.
pub(crate) fn native_matcher_append_tail_sb(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let builder_ref = extract_ref_arg(args, 1)?;
    let append_pos = usize::try_from(matcher_field_int(
        heap,
        m_ref,
        MATCHER_APPEND_POS_FIELD,
        0,
    )?)
    .unwrap_or(0);
    let Some((_, _, input)) = matcher_pattern_input_text(heap, m_ref)? else {
        return Ok(Some(Slot::Reference(Some(builder_ref))));
    };
    let safe_append_pos = append_pos.min(input.len());
    append_to_string_builder(heap, builder_ref, &input[safe_append_pos..])?;
    heap.get_mut(m_ref)?.fields[MATCHER_APPEND_POS_FIELD] =
        Slot::Int(i32::try_from(input.len()).unwrap_or(i32::MAX));
    Ok(Some(Slot::Reference(Some(builder_ref))))
}

/// Native: `Matcher.replaceAll(String)String` — replace all matches.
pub(crate) fn native_matcher_replace_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let repl_ref = extract_ref_arg(args, 1)?;
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let repl = heap.get(repl_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let result = re.replace_all(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Matcher.replaceFirst(String)String` — replace first match.
pub(crate) fn native_matcher_replace_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let repl_ref = extract_ref_arg(args, 1)?;
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let repl = heap.get(repl_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let result = re.replace(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.matches(String)Z` — full-string regex match.
pub(crate) fn native_string_matches_regex(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let pat_ref = extract_ref_arg(args, 1)?;
    let input = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let matched = re
        .find(&input)
        .is_some_and(|m| m.start() == 0 && m.end() == input.len());
    Ok(Some(Slot::Int(i32::from(matched))))
}

/// Native: `String.replaceAll(String,String)String` — regex replace all.
pub(crate) fn native_string_replace_all_regex(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let pat_ref = extract_ref_arg(args, 1)?;
    let repl_ref = extract_ref_arg(args, 2)?;
    let input = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let repl = heap.get(repl_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re.replace_all(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.replaceFirst(String,String)String` — regex replace first.
pub(crate) fn native_string_replace_first_regex(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let pat_ref = extract_ref_arg(args, 1)?;
    let repl_ref = extract_ref_arg(args, 2)?;
    let input = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let repl = heap.get(repl_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re.replace(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Optional extensions (callback-based)
// ---------------------------------------------------------------------------

/// Native: `Optional.map(Function)Optional` — maps value if present.
pub(crate) fn native_optional_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
    let result_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if matches!(value, Slot::Reference(None)) {
        // empty — propagate empty
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    }
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mapped = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, value],
    )?;
    heap.get_mut(result_ref)?.fields[0] = mapped.unwrap_or(Slot::Reference(None));
    Ok(Some(Slot::Reference(Some(result_ref))))
}

/// Native: `Optional.filter(Predicate)Optional` — keeps value only if predicate passes.
pub(crate) fn native_optional_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
    let result_ref = heap.allocate("java/util/Optional".to_string(), 1);
    if matches!(value, Slot::Reference(None)) {
        heap.get_mut(result_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(result_ref))));
    }
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        heap.get_mut(result_ref)?.fields[0] = value;
        return Ok(Some(Slot::Reference(Some(result_ref))));
    };
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let test_result = ops.invoke(
        heap,
        out,
        &pred_class,
        "test",
        "(Ljava/lang/Object;)Z",
        vec![pred_slot, value],
    )?;
    let passes = matches!(test_result, Some(Slot::Int(n)) if n != 0);
    let stored = if passes { value } else { Slot::Reference(None) };
    heap.get_mut(result_ref)?.fields[0] = stored;
    Ok(Some(Slot::Reference(Some(result_ref))))
}

/// Native: `Optional.flatMap(Function)Optional` — maps value to Optional if present, flattens.
pub(crate) fn native_optional_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
    if matches!(value, Slot::Reference(None)) {
        let empty_ref = heap.allocate("java/util/Optional".to_string(), 1);
        heap.get_mut(empty_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(empty_ref))));
    }
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        let empty_ref = heap.allocate("java/util/Optional".to_string(), 1);
        heap.get_mut(empty_ref)?.fields[0] = Slot::Reference(None);
        return Ok(Some(Slot::Reference(Some(empty_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    // The function returns an Optional — return it directly (flat, not wrapped again)
    let result = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, value],
    )?;
    Ok(Some(result.unwrap_or(Slot::Reference(None))))
}

/// Native: `Optional.ifPresent(Consumer)V` — invokes consumer if value is present.
pub(crate) fn native_optional_if_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
    if let (Slot::Reference(Some(_)), Slot::Reference(Some(consumer_ref))) = (value, consumer_slot)
    {
        let consumer_class = heap.get(consumer_ref)?.class_name.clone();
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, value],
        )?;
    }
    Ok(None)
}

/// Native: `Optional.orElseGet(Supplier)Object` — calls supplier if empty.
pub(crate) fn native_optional_or_else_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let supplier_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
    if !matches!(value, Slot::Reference(None)) {
        return Ok(Some(value));
    }
    let Slot::Reference(Some(supplier_ref)) = supplier_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let supplier_class = heap.get(supplier_ref)?.class_name.clone();
    let result = ops.invoke(
        heap,
        out,
        &supplier_class,
        "get",
        "()Ljava/lang/Object;",
        vec![supplier_slot],
    )?;
    Ok(Some(result.unwrap_or(Slot::Reference(None))))
}

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

/// Native: `HashMap.compute(K, BiFunction)V` — compute new value from old (possibly null).
pub(crate) fn native_hashmap_compute(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(None)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    // Find old value
    let fields = &heap.get(this_ref)?.fields;
    let old_value = hashmap_find_key(fields, key, heap)
        .and_then(|ki| fields.get(ki + 1).copied())
        .unwrap_or(Slot::Reference(None));
    // Call BiFunction.apply(key, oldValue)
    let new_value = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, key, old_value],
    )?;
    // null return means remove the key
    let is_null = matches!(new_value, None | Some(Slot::Reference(None)));
    let new_val = new_value.unwrap_or(Slot::Reference(None));
    let fields2 = &heap.get(this_ref)?.fields;
    if let Some(ki) = hashmap_find_key(fields2, key, heap) {
        if is_null {
            // Remove the key-value pair (swap-remove style)
            let fields3 = &mut heap.get_mut(this_ref)?.fields;
            fields3.remove(ki + 1);
            fields3.remove(ki);
            let size = match fields3.first() {
                Some(Slot::Int(n)) => *n,
                _ => 0,
            };
            if let Some(first) = fields3.first_mut() {
                *first = Slot::Int(size - 1);
            }
        } else {
            heap.get_mut(this_ref)?.fields[ki + 1] = new_val;
        }
    } else if !is_null {
        let size = match heap.get(this_ref)?.fields.first().copied() {
            Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
            _ => 0,
        };
        heap.get_mut(this_ref)?.fields.push(key);
        heap.get_mut(this_ref)?.fields.push(new_val);
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
    }
    Ok(Some(new_val))
}

/// Native: `HashMap.merge(K, V, BiFunction)V` — put V if absent, else merge with `BiFunction`.
pub(crate) fn native_hashmap_merge(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let new_val_slot = extract_slot_arg(args, 2);
    let fn_slot = extract_slot_arg(args, 3);
    let fields = &heap.get(this_ref)?.fields;
    let old_ki = hashmap_find_key(fields, key, heap);
    if let Some(ki) = old_ki {
        let old_value = fields.get(ki + 1).copied().unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(fn_ref)) = fn_slot else {
            return Ok(Some(old_value));
        };
        let fn_class = heap.get(fn_ref)?.class_name.clone();
        let merged = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, old_value, new_val_slot],
        )?;
        let merged_raw = merged.unwrap_or(Slot::Reference(None));
        // Box primitive results so the stored value is always a Reference (matches Java generics)
        let merged_val = box_primitive_slot(merged_raw, heap);
        heap.get_mut(this_ref)?.fields[ki + 1] = merged_val;
        Ok(Some(merged_val))
    } else {
        // Key absent — insert new value
        let size = match heap.get(this_ref)?.fields.first().copied() {
            Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
            _ => 0,
        };
        heap.get_mut(this_ref)?.fields.push(key);
        heap.get_mut(this_ref)?.fields.push(new_val_slot);
        heap.get_mut(this_ref)?.fields[0] = Slot::Int(i32::try_from(size + 1).unwrap_or(i32::MAX));
        Ok(Some(new_val_slot))
    }
}

// ---------------------------------------------------------------------------
// java.lang.StringBuffer — mutable string, delegates to StringBuilder internals
// (string_value field used as buffer, same as StringBuilder)
// ---------------------------------------------------------------------------

/// Native: `StringBuffer.<init>()V` — empty buffer.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stringbuffer_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.string_value = Some(String::new());
    Ok(None)
}

/// Native: `StringBuffer.<init>(Ljava/lang/String;)V` — init with string.
pub(crate) fn native_stringbuffer_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = match args.get(1).copied() {
        Some(Slot::Reference(Some(r))) => heap.get(r)?.string_value.clone().unwrap_or_default(),
        _ => String::new(),
    };
    heap.get_mut(this_ref)?.string_value = Some(s);
    Ok(None)
}

/// Native: `StringBuffer.append(...)StringBuffer` — append any type; returns `this`.
pub(crate) fn native_stringbuffer_append(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let frag = match extract_slot_arg(args, 1) {
        Slot::Reference(Some(r)) => heap
            .get(r)?
            .string_value
            .clone()
            .unwrap_or_else(|| "null".to_string()),
        Slot::Reference(None) => "null".to_string(),
        Slot::Int(n) => n.to_string(),
        Slot::Long(n) => n.to_string(),
        Slot::Double(d) => format!("{d}"),
        Slot::Float(f) => format!("{f}"),
        Slot::ReturnAddress(_) => String::new(),
    };
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    buf.push_str(&frag);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuffer.toString()String`.
pub(crate) fn native_stringbuffer_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `StringBuffer.length()I`.
pub(crate) fn native_stringbuffer_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let len = heap
        .get(this_ref)?
        .string_value
        .as_deref()
        .unwrap_or("")
        .len();
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}

// ---------------------------------------------------------------------------
// java.util.StringJoiner
// fields[0] = Reference(delimiter), fields[1] = Reference(prefix), fields[2] = Reference(suffix)
// fields[3] = Reference(emptyValue), fields[4..] = added elements
// instance_field_count = 4 (slots 0-3 pre-allocated)
// ---------------------------------------------------------------------------

/// Native: `StringJoiner.<init>(CharSequence)V` — delimiter only.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stringjoiner_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let delim_slot = extract_slot_arg(args, 1);
    let empty_ref = heap.allocate_string(String::new());
    let prefix_ref = heap.allocate_string(String::new());
    let suffix_ref = heap.allocate_string(String::new());
    heap.get_mut(this_ref)?.fields[0] = delim_slot;
    heap.get_mut(this_ref)?.fields[1] = Slot::Reference(Some(prefix_ref));
    heap.get_mut(this_ref)?.fields[2] = Slot::Reference(Some(suffix_ref));
    heap.get_mut(this_ref)?.fields[3] = Slot::Reference(Some(empty_ref));
    Ok(None)
}

/// Native: `StringJoiner.<init>(CharSequence,CharSequence,CharSequence)V` — delim + prefix + suffix.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stringjoiner_init_prefix_suffix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let delim_slot = extract_slot_arg(args, 1);
    let prefix_slot = extract_slot_arg(args, 2);
    let suffix_slot = extract_slot_arg(args, 3);
    let empty_ref = heap.allocate_string(String::new());
    heap.get_mut(this_ref)?.fields[0] = delim_slot;
    heap.get_mut(this_ref)?.fields[1] = prefix_slot;
    heap.get_mut(this_ref)?.fields[2] = suffix_slot;
    heap.get_mut(this_ref)?.fields[3] = Slot::Reference(Some(empty_ref));
    Ok(None)
}

/// Native: `StringJoiner.add(CharSequence)StringJoiner` — append element.
pub(crate) fn native_stringjoiner_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    heap.get_mut(this_ref)?.fields.push(elem);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringJoiner.setEmptyValue(CharSequence)StringJoiner`.
pub(crate) fn native_stringjoiner_set_empty_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let empty_slot = extract_slot_arg(args, 1);
    heap.get_mut(this_ref)?.fields[3] = empty_slot;
    Ok(Some(Slot::Reference(Some(this_ref))))
}

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

/// Native: `StringJoiner.toString()String` — builds the joined result.
///
/// ⚡ Bolt Optimization: Eliminated intermediate `Vec<String>` allocation and format
/// macro overhead by appending directly to a single String buffer.
pub(crate) fn native_stringjoiner_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    // elements start at index 4
    let elems: Vec<Slot> = fields.get(4..).map(<[Slot]>::to_vec).unwrap_or_default();
    if elems.is_empty() {
        let empty_slot = fields.get(3).copied().unwrap_or(Slot::Reference(None));
        let prefix = slot_to_string(
            fields.get(1).copied().unwrap_or(Slot::Reference(None)),
            heap,
        );
        let suffix = slot_to_string(
            fields.get(2).copied().unwrap_or(Slot::Reference(None)),
            heap,
        );
        let empty = slot_to_string(empty_slot, heap);
        // If prefix+suffix are both empty, return emptyValue; otherwise prefix+suffix
        let result = if prefix.is_empty() && suffix.is_empty() {
            empty
        } else {
            format!("{prefix}{suffix}")
        };
        let r = heap.allocate_string(result);
        return Ok(Some(Slot::Reference(Some(r))));
    }
    let delim = slot_to_string(
        fields.first().copied().unwrap_or(Slot::Reference(None)),
        heap,
    );
    let prefix = slot_to_string(
        fields.get(1).copied().unwrap_or(Slot::Reference(None)),
        heap,
    );
    let suffix = slot_to_string(
        fields.get(2).copied().unwrap_or(Slot::Reference(None)),
        heap,
    );
    // ⚡ Bolt: Eliminate intermediate Vec<String> allocation and format! macro overhead
    // by appending directly to a single String buffer.
    let mut result = String::new();
    result.push_str(&prefix);
    for (i, elem) in elems.iter().enumerate() {
        if i > 0 {
            result.push_str(&delim);
        }
        result.push_str(&slot_to_string(*elem, heap));
    }
    result.push_str(&suffix);
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `StringJoiner.length()I` — length of the `toString()` result.
pub(crate) fn native_stringjoiner_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // Reuse toString and measure
    let result = native_stringjoiner_tostring(args, heap, out, control)?;
    let len = match result {
        Some(Slot::Reference(Some(r))) => heap.get(r)?.string_value.as_deref().unwrap_or("").len(),
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
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
                "java/util/UUID" => uuid_bits_from_object(oa) == uuid_bits_from_object(ob),
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

pub(crate) fn native_hashmap_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.is_empty() {
        obj.fields.push(Slot::Int(0));
    } else {
        obj.fields[0] = Slot::Int(0);
    }
    Ok(None)
}

/// Native: `HashMap.put(Object, Object)Object` — inserts or updates a key-value pair.
/// Returns the old value if the key was already present, or null if it is new.
pub(crate) fn native_hashmap_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let val = extract_slot_arg(args, 2);
    // We just find the index, without taking fields ownership yet
    let i_opt = find_hashmap_entry_index(&heap.get(this_ref)?.fields, &key, heap);

    if let Some(i) = i_opt {
        let old = heap.get(this_ref)?.fields[i + 1];
        heap.get_mut(this_ref)?.fields[i + 1] = val;
        return Ok(Some(old));
    }

    // New key — append pair and bump size.
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(Error::NullPointerException),
    }
    obj.fields.push(key);
    obj.fields.push(val);
    Ok(Some(Slot::Reference(None)))
}

/// Native: `HashMap.get(Object)Object` — returns value for key, or null if absent.
pub(crate) fn native_hashmap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fields = &heap.get(this_ref)?.fields;

    Ok(Some(
        find_hashmap_entry_index(fields, &key, heap)
            .map_or(Slot::Reference(None), |i| fields[i + 1]),
    ))
}

/// Native: `HashMap.containsKey(Object)Z` — returns 1 if key present, 0 otherwise.
pub(crate) fn native_hashmap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let fields = &heap.get(this_ref)?.fields;

    if find_hashmap_entry_index(fields, &key, heap).is_some() {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashMap.size()I` — returns entry count from fields\[0\].
pub(crate) fn native_hashmap_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Ok(Some(Slot::Int(*n))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `HashMap.remove(Object)Object` — removes a key-value pair, returns old value or null.
/// Uses swap-remove (swaps target pair with last pair) for O(1) deletion.
pub(crate) fn native_hashmap_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let i_opt = find_hashmap_entry_index(&heap.get(this_ref)?.fields, &key, heap);

    if let Some(i) = i_opt {
        let old_val = heap.get(this_ref)?.fields[i + 1];
        let obj = heap.get_mut(this_ref)?;
        let last_val_idx = obj.fields.len() - 1;
        let last_key_idx = obj.fields.len() - 2;
        obj.fields.swap(i + 1, last_val_idx);
        obj.fields.swap(i, last_key_idx);
        obj.fields.truncate(obj.fields.len() - 2);
        match obj.fields.first_mut() {
            Some(Slot::Int(sz)) => *sz -= 1,
            _ => return Err(Error::NullPointerException),
        }
        Ok(Some(old_val))
    } else {
        Ok(Some(Slot::Reference(None)))
    }
}

/// Native: `HashMap.isEmpty()Z` — returns 1 if size == 0, else 0.
pub(crate) fn native_hashmap_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) => Ok(Some(Slot::Int(1))),
        Some(Slot::Int(_)) => Ok(Some(Slot::Int(0))),
        _ => Ok(Some(Slot::Int(1))),
    }
}

/// Native: `HashMap.getOrDefault(Object, Object)Object` — returns value for key, or default if absent.
pub(crate) fn native_hashmap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let default = extract_slot_arg(args, 2);
    let fields = &heap.get(this_ref)?.fields;

    Ok(Some(
        find_hashmap_entry_index(fields, &key, heap).map_or(default, |i| fields[i + 1]),
    ))
}

// ---------------------------------------------------------------------------
// java.util.Properties natives
// ---------------------------------------------------------------------------

const PROPERTIES_SIZE_FIELD: usize = 0;
const PROPERTIES_DEFAULTS_FIELD: usize = 1;
const PROPERTIES_ENTRIES_START: usize = 2;
const PROPERTIES_ENUM_INDEX_FIELD: usize = 0;
const PROPERTIES_ENUM_COUNT_FIELD: usize = 1;
const PROPERTIES_ENUM_NAMES_START: usize = 2;
const PROPERTIES_STORE_TIMESTAMP: &str = "1970-01-01T00:00:00Z";

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

fn ensure_properties_layout(heap: &mut duke_gc::Heap, props_ref: u64) -> Result<()> {
    let props = heap.get_mut(props_ref)?;
    if props.fields.len() < PROPERTIES_ENTRIES_START {
        props
            .fields
            .resize(PROPERTIES_ENTRIES_START, Slot::Reference(None));
    }
    Ok(())
}

fn init_properties_with_defaults(
    heap: &mut duke_gc::Heap,
    props_ref: u64,
    defaults: Slot,
) -> Result<()> {
    ensure_properties_layout(heap, props_ref)?;
    let props = heap.get_mut(props_ref)?;
    props.fields[PROPERTIES_SIZE_FIELD] = Slot::Int(0);
    props.fields[PROPERTIES_DEFAULTS_FIELD] = defaults;
    props.fields.truncate(PROPERTIES_ENTRIES_START);
    Ok(())
}

fn properties_entry_index(fields: &[Slot], key: &Slot, heap: &duke_gc::Heap) -> Option<usize> {
    (PROPERTIES_ENTRIES_START..fields.len())
        .step_by(2)
        .find(|&idx| idx + 1 < fields.len() && slots_equal(&fields[idx], key, heap))
}

fn slot_is_java_string(heap: &duke_gc::Heap, slot: Slot) -> bool {
    let Slot::Reference(Some(string_ref)) = slot else {
        return false;
    };
    heap.get(string_ref).is_ok_and(|obj| {
        obj.class_name == "java/lang/String" && obj.string_value.is_some()
    })
}

fn string_from_slot(heap: &duke_gc::Heap, slot: Slot) -> Option<String> {
    let Slot::Reference(Some(string_ref)) = slot else {
        return None;
    };
    heap.get(string_ref)
        .ok()
        .and_then(|obj| obj.string_value.clone())
}

fn properties_local_entries(heap: &duke_gc::Heap, props_ref: u64) -> Result<Vec<(Slot, Slot)>> {
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
    if let Some(Slot::Reference(Some(defaults_ref))) = fields.get(PROPERTIES_DEFAULTS_FIELD) {
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

const fn is_properties_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\u{000c}')
}

fn split_properties_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if matches!(chars.peek(), Some('\n')) {
                    chars.next();
                }
                lines.push(std::mem::take(&mut current));
            }
            '\n' => lines.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    lines.push(current);
    lines
}

fn has_odd_trailing_backslashes(line: &str) -> bool {
    let mut count = 0usize;
    for ch in line.chars().rev() {
        if ch != '\\' {
            break;
        }
        count += 1;
    }
    count % 2 == 1
}

fn logical_properties_lines(text: &str) -> Vec<String> {
    let mut logical = Vec::new();
    let mut pending = String::new();
    let mut continuing = false;

    for line in split_properties_lines(text) {
        let mut piece = if continuing {
            line.trim_start_matches(is_properties_whitespace).to_string()
        } else {
            line
        };

        if continuing && piece.is_empty() {
            continue;
        }

        if has_odd_trailing_backslashes(&piece) {
            piece.pop();
            pending.push_str(&piece);
            continuing = true;
        } else {
            pending.push_str(&piece);
            logical.push(std::mem::take(&mut pending));
            continuing = false;
        }
    }

    if continuing || !pending.is_empty() {
        logical.push(pending);
    }
    logical
}

fn hex_value(ch: char) -> Option<u32> {
    match ch {
        '0'..='9' => Some(u32::from(ch) - u32::from('0')),
        'a'..='f' => Some(u32::from(ch) - u32::from('a') + 10),
        'A'..='F' => Some(u32::from(ch) - u32::from('A') + 10),
        _ => None,
    }
}

fn unescape_property_text(raw: &str) -> Result<String> {
    let mut result = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        let Some(escaped) = chars.next() else {
            result.push('\\');
            break;
        };

        match escaped {
            'n' => result.push('\n'),
            'r' => result.push('\r'),
            't' => result.push('\t'),
            'f' => result.push('\u{000c}'),
            'u' => {
                let mut code = 0_u32;
                for _ in 0..4 {
                    let Some(hex) = chars.next().and_then(hex_value) else {
                        return Err(properties_illegal_argument());
                    };
                    code = (code << 4) | hex;
                }
                let Some(decoded) = char::from_u32(code) else {
                    return Err(properties_illegal_argument());
                };
                result.push(decoded);
            }
            other => result.push(other),
        }
    }
    Ok(result)
}

fn parse_property_logical_line(line: &str) -> Result<Option<(String, String)>> {
    let chars: Vec<char> = line.chars().collect();
    let mut idx = 0usize;
    while idx < chars.len() && is_properties_whitespace(chars[idx]) {
        idx += 1;
    }
    if idx >= chars.len() || matches!(chars[idx], '#' | '!') {
        return Ok(None);
    }

    let key_start = idx;
    let mut escaped = false;
    while idx < chars.len() {
        let ch = chars[idx];
        if escaped {
            escaped = false;
            idx += 1;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            idx += 1;
            continue;
        }
        if matches!(ch, '=' | ':') || is_properties_whitespace(ch) {
            break;
        }
        idx += 1;
    }

    let key_end = idx;
    let value_start = if idx < chars.len() {
        if is_properties_whitespace(chars[idx]) {
            while idx < chars.len() && is_properties_whitespace(chars[idx]) {
                idx += 1;
            }
            if idx < chars.len() && matches!(chars[idx], '=' | ':') {
                idx += 1;
            }
        } else {
            idx += 1;
        }
        while idx < chars.len() && is_properties_whitespace(chars[idx]) {
            idx += 1;
        }
        idx
    } else {
        chars.len()
    };

    let raw_key: String = chars[key_start..key_end].iter().collect();
    let raw_value: String = chars[value_start..].iter().collect();
    Ok(Some((
        unescape_property_text(&raw_key)?,
        unescape_property_text(&raw_value)?,
    )))
}

fn parse_properties_bytes(bytes: &[u8]) -> Result<Vec<(String, String)>> {
    let text: String = bytes.iter().map(|byte| char::from(*byte)).collect();
    let mut entries = Vec::new();
    for line in logical_properties_lines(&text) {
        if let Some((key, value)) = parse_property_logical_line(&line)? {
            entries.push((key, value));
        }
    }
    Ok(entries)
}

fn push_u16_escape(out: &mut String, unit: u16) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push('\\');
    out.push('u');
    for shift in [12, 8, 4, 0] {
        let idx = usize::from((unit >> shift) & 0x000f);
        out.push(char::from(HEX[idx]));
    }
}

fn push_unicode_escape(out: &mut String, ch: char) {
    let mut encoded = [0_u16; 2];
    for unit in ch.encode_utf16(&mut encoded).iter().copied() {
        push_u16_escape(out, unit);
    }
}

fn push_escaped_key_char(out: &mut String, ch: char) {
    match ch {
        '\\' => out.push_str("\\\\"),
        '=' => out.push_str("\\="),
        ':' => out.push_str("\\:"),
        ' ' => out.push_str("\\ "),
        '#' => out.push_str("\\#"),
        '!' => out.push_str("\\!"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        '\u{000c}' => out.push_str("\\f"),
        _ if !ch.is_ascii() || u32::from(ch) < 0x20 || u32::from(ch) > 0x7e => {
            push_unicode_escape(out, ch);
        }
        _ => out.push(ch),
    }
}

fn escape_property_key(key: &str) -> String {
    let mut result = String::new();
    for ch in key.chars() {
        push_escaped_key_char(&mut result, ch);
    }
    result
}

fn escape_property_value(value: &str) -> String {
    let mut result = String::new();
    for (idx, ch) in value.chars().enumerate() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{000c}' => result.push_str("\\f"),
            ' ' if idx == 0 => result.push_str("\\ "),
            _ if !ch.is_ascii() || u32::from(ch) < 0x20 || u32::from(ch) > 0x7e => {
                push_unicode_escape(&mut result, ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

fn escape_property_comment(comment: &str) -> String {
    let mut result = String::new();
    for ch in comment.chars() {
        match ch {
            '\n' | '\r' => {}
            _ if !ch.is_ascii() || u32::from(ch) < 0x20 || u32::from(ch) > 0x7e => {
                push_unicode_escape(&mut result, ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

fn append_store_comment(out: &mut String, comment: &str) {
    for line in split_properties_lines(comment) {
        if line.is_empty() {
            out.push_str("#\n");
        } else {
            out.push_str("# ");
            out.push_str(&escape_property_comment(&line));
            out.push('\n');
        }
    }
}

fn render_properties_store(heap: &duke_gc::Heap, props_ref: u64, comment: Option<&str>) -> Result<String> {
    let mut output = String::new();
    if let Some(comment_text) = comment {
        append_store_comment(&mut output, comment_text);
    }
    output.push_str("# ");
    output.push_str(PROPERTIES_STORE_TIMESTAMP);
    output.push('\n');

    for (key, value) in properties_local_string_entries(heap, props_ref)? {
        output.push_str(&escape_property_key(&key));
        output.push('=');
        output.push_str(&escape_property_value(&value));
        output.push('\n');
    }
    Ok(output)
}

fn write_iso_8859_1_ascii(heap: &mut duke_gc::Heap, file_id: i32, text: &str) -> Result<()> {
    for byte in text.bytes() {
        heap.write_host_file_byte(file_id, i32::from(byte))?;
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
    Ok(Some(properties_get_property_slot(
        heap,
        this_ref,
        Slot::Reference(Some(key_ref)),
    )?))
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
    let value = properties_get_property_slot(heap, this_ref, Slot::Reference(Some(key_ref)))?;
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
        Slot::Reference(Some(comment_ref)) => Some(string_value_from_ref(heap, comment_ref)?),
        Slot::Reference(None) => None,
        _ => return Err(Error::TypeMismatch { expected: "Reference", got: "other" }),
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
    let enum_ref = heap.allocate(
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
        heap.get(this_ref)?.fields.get(PROPERTIES_SIZE_FIELD),
        Some(Slot::Int(0)) | None
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
    Ok(Some(Slot::Int(i32::from(
        properties_entry_index(fields, &key, heap).is_some(),
    ))))
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
        native_arraylist_add(&[Slot::Reference(Some(list_ref)), value], heap, out, control)?;
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
            &[
                Slot::Reference(Some(set_ref)),
                Slot::Reference(Some(entry_ref)),
            ],
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

// ---------------------------------------------------------------------------
// ConcurrentHashMap natives
// ---------------------------------------------------------------------------

fn concurrent_hashmap_lock(
    heap: &mut duke_gc::Heap,
    this_ref: u64,
) -> Result<std::sync::Arc<std::sync::Mutex<()>>> {
    let obj = heap.get_mut(this_ref)?;
    if let Some(payload) = &obj.atomic_payload {
        return match payload {
            duke_gc::AtomicPayload::ConcurrentMapLock(lock) => Ok(std::sync::Arc::clone(lock)),
            duke_gc::AtomicPayload::Int(_)
            | duke_gc::AtomicPayload::Long(_)
            | duke_gc::AtomicPayload::Bool(_)
            | duke_gc::AtomicPayload::Reference(_)
            | duke_gc::AtomicPayload::ReentrantLock(_)
            | duke_gc::AtomicPayload::Condition(_)
            | duke_gc::AtomicPayload::ReadWriteLock(_)
            | duke_gc::AtomicPayload::ReadWriteLockView { .. }
            | duke_gc::AtomicPayload::Executor(_)
            | duke_gc::AtomicPayload::CountDownLatch(_)
            | duke_gc::AtomicPayload::Semaphore(_)
            | duke_gc::AtomicPayload::CyclicBarrier(_) => Err(Error::TypeMismatch {
                expected: "concurrent map lock",
                got: "other",
            }),
        };
    }

    let lock = std::sync::Arc::new(std::sync::Mutex::new(()));
    obj.atomic_payload = Some(duke_gc::AtomicPayload::ConcurrentMapLock(
        std::sync::Arc::clone(&lock),
    ));
    Ok(lock)
}

fn concurrent_hashmap_guard(
    lock: &std::sync::Arc<std::sync::Mutex<()>>,
) -> std::sync::MutexGuard<'_, ()> {
    lock.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

const fn require_chm_non_null(slot: Slot) -> Result<Slot> {
    if matches!(slot, Slot::Reference(None)) {
        Err(Error::NullPointerException)
    } else {
        Ok(slot)
    }
}

fn chm_non_null_arg(args: &[Slot], idx: usize) -> Result<Slot> {
    require_chm_non_null(extract_slot_arg(args, idx))
}

fn chm_entry_snapshot(heap: &duke_gc::Heap, map_ref: u64) -> Result<Vec<(Slot, Slot)>> {
    let fields = heap.get(map_ref)?.fields.clone();
    let mut entries = Vec::with_capacity(fields.len().saturating_sub(1) / 2);
    let mut i = 1usize;
    while i + 1 < fields.len() {
        entries.push((fields[i], fields[i + 1]));
        i += 2;
    }
    Ok(entries)
}

pub(crate) fn native_concurrent_hashmap_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let _lock = concurrent_hashmap_lock(heap, this_ref)?;
    native_hashmap_init(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_init_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_concurrent_hashmap_init(args, heap, out, control)?;
    let this_ref = extract_ref_arg(args, 0)?;
    let source_ref = extract_ref_arg(args, 1)?;
    native_concurrent_hashmap_put_all(
        &[
            Slot::Reference(Some(this_ref)),
            Slot::Reference(Some(source_ref)),
        ],
        heap,
        out,
        control,
    )
}

pub(crate) fn native_concurrent_hashmap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_get(&[Slot::Reference(Some(this_ref)), key], heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let default = extract_slot_arg(args, 2);
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_get_or_default(
        &[Slot::Reference(Some(this_ref)), key, default],
        heap,
        out,
        control,
    )
}

pub(crate) fn native_concurrent_hashmap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_contains_key(&[Slot::Reference(Some(this_ref)), key], heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_contains_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = chm_non_null_arg(args, 1)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_contains_value(&[Slot::Reference(Some(this_ref)), value], heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_size(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_is_empty(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let value = chm_non_null_arg(args, 2)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_put(
        &[Slot::Reference(Some(this_ref)), key, value],
        heap,
        out,
        control,
    )
}

pub(crate) fn native_concurrent_hashmap_put_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let value = chm_non_null_arg(args, 2)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_put_if_absent(
        &[Slot::Reference(Some(this_ref)), key, value],
        heap,
        out,
        control,
    )
}

pub(crate) fn native_concurrent_hashmap_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_remove(&[Slot::Reference(Some(this_ref)), key], heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_remove_key_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let expected = chm_non_null_arg(args, 2)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_remove_key_value(
        &[Slot::Reference(Some(this_ref)), key, expected],
        heap,
        out,
        control,
    )
}

pub(crate) fn native_concurrent_hashmap_replace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let value = chm_non_null_arg(args, 2)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_replace(
        &[Slot::Reference(Some(this_ref)), key, value],
        heap,
        out,
        control,
    )
}

pub(crate) fn native_concurrent_hashmap_replace_key_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let expected = chm_non_null_arg(args, 2)?;
    let replacement = chm_non_null_arg(args, 3)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    let fields = heap.get(this_ref)?.fields.clone();
    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let actual = fields[i + 1];
        if slots_equal(&actual, &expected, heap) {
            heap.get_mut(this_ref)?.fields[i + 1] = replacement;
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

pub(crate) fn native_concurrent_hashmap_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_clear(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_put_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let source_ref = extract_ref_arg(args, 1)?;
    let entries = chm_entry_snapshot(heap, source_ref)?;
    for (key, value) in &entries {
        require_chm_non_null(*key)?;
        require_chm_non_null(*value)?;
    }

    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    for (key, value) in entries {
        native_hashmap_put(
            &[Slot::Reference(Some(this_ref)), key, value],
            heap,
            out,
            control,
        )?;
    }
    Ok(None)
}

pub(crate) fn native_concurrent_hashmap_key_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_key_set(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_values(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_values(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_entry_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let _guard = concurrent_hashmap_guard(&lock);
    native_hashmap_entry_set(args, heap, out, control)
}

pub(crate) fn native_concurrent_hashmap_compute_if_absent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_slot = Slot::Reference(Some(fn_ref));
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    {
        let _guard = concurrent_hashmap_guard(&lock);
        let fields = &heap.get(this_ref)?.fields;
        if let Some(i) = find_hashmap_entry_index(fields, &key, heap) {
            return Ok(Some(fields[i + 1]));
        }
    }

    let computed = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, key],
    )?;
    let Some(value) = computed else {
        return Ok(Some(Slot::Reference(None)));
    };
    if matches!(value, Slot::Reference(None)) {
        return Ok(Some(Slot::Reference(None)));
    }

    let _guard = concurrent_hashmap_guard(&lock);
    let fields = &heap.get(this_ref)?.fields;
    if let Some(i) = find_hashmap_entry_index(fields, &key, heap) {
        return Ok(Some(fields[i + 1]));
    }
    native_hashmap_put(
        &[Slot::Reference(Some(this_ref)), key, value],
        heap,
        out,
        &mut NativeControl::default(),
    )?;
    Ok(Some(value))
}

pub(crate) fn native_concurrent_hashmap_compute_if_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_slot = Slot::Reference(Some(fn_ref));
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let old_value = {
        let _guard = concurrent_hashmap_guard(&lock);
        let fields = &heap.get(this_ref)?.fields;
        match find_hashmap_entry_index(fields, &key, heap) {
            Some(i) => fields[i + 1],
            None => return Ok(Some(Slot::Reference(None))),
        }
    };

    let new_value = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, key, old_value],
    )?;
    let _guard = concurrent_hashmap_guard(&lock);
    match new_value {
        Some(value) if !matches!(value, Slot::Reference(None)) => {
            native_hashmap_put(
                &[Slot::Reference(Some(this_ref)), key, value],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            Ok(Some(value))
        }
        _ => {
            native_hashmap_remove(
                &[Slot::Reference(Some(this_ref)), key],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            Ok(Some(Slot::Reference(None)))
        }
    }
}

pub(crate) fn native_concurrent_hashmap_compute(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_slot = Slot::Reference(Some(fn_ref));
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let old_value = {
        let _guard = concurrent_hashmap_guard(&lock);
        let fields = &heap.get(this_ref)?.fields;
        find_hashmap_entry_index(fields, &key, heap)
            .map_or(Slot::Reference(None), |i| fields[i + 1])
    };

    let new_value = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, key, old_value],
    )?;
    let _guard = concurrent_hashmap_guard(&lock);
    match new_value {
        Some(value) if !matches!(value, Slot::Reference(None)) => {
            native_hashmap_put(
                &[Slot::Reference(Some(this_ref)), key, value],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            Ok(Some(value))
        }
        _ => {
            native_hashmap_remove(
                &[Slot::Reference(Some(this_ref)), key],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            Ok(Some(Slot::Reference(None)))
        }
    }
}

pub(crate) fn native_concurrent_hashmap_merge(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = chm_non_null_arg(args, 1)?;
    let value = chm_non_null_arg(args, 2)?;
    let fn_ref = extract_ref_arg(args, 3)?;
    let fn_slot = Slot::Reference(Some(fn_ref));
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let old_value = {
        let _guard = concurrent_hashmap_guard(&lock);
        let fields = &heap.get(this_ref)?.fields;
        if let Some(i) = find_hashmap_entry_index(fields, &key, heap) {
            Some(fields[i + 1])
        } else {
            native_hashmap_put(
                &[Slot::Reference(Some(this_ref)), key, value],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            return Ok(Some(value));
        }
    };
    let Some(old_value) = old_value else {
        return Ok(Some(value));
    };

    let merged = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![fn_slot, old_value, value],
    )?;
    let _guard = concurrent_hashmap_guard(&lock);
    match merged {
        Some(merged_value) if !matches!(merged_value, Slot::Reference(None)) => {
            let merged_value = box_primitive_slot(merged_value, heap);
            native_hashmap_put(
                &[Slot::Reference(Some(this_ref)), key, merged_value],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            Ok(Some(merged_value))
        }
        _ => {
            native_hashmap_remove(
                &[Slot::Reference(Some(this_ref)), key],
                heap,
                out,
                &mut NativeControl::default(),
            )?;
            Ok(Some(Slot::Reference(None)))
        }
    }
}

pub(crate) fn native_concurrent_hashmap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let consumer_ref = extract_ref_arg(args, 1)?;
    let consumer_slot = Slot::Reference(Some(consumer_ref));
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    let lock = concurrent_hashmap_lock(heap, this_ref)?;
    let entries = {
        let _guard = concurrent_hashmap_guard(&lock);
        chm_entry_snapshot(heap, this_ref)?
    };
    for (key, value) in entries {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;Ljava/lang/Object;)V",
            vec![consumer_slot, key, value],
        )?;
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// HashSet natives
// ---------------------------------------------------------------------------

pub(crate) fn native_set_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;

    match extract_slot_arg(args, 0) {
        Slot::Reference(Some(array_ref)) => {
            let elements = heap.get(array_ref)?.fields.clone();
            for element in elements {
                native_hashset_add(
                    &[Slot::Reference(Some(set_ref)), element],
                    heap,
                    out,
                    control,
                )?;
            }
        }
        Slot::Reference(None) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    }

    Ok(Some(Slot::Reference(Some(set_ref))))
}

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

pub(crate) fn native_hashset_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.is_empty() {
        obj.fields.push(Slot::Int(0));
    } else {
        obj.fields[0] = Slot::Int(0);
    }
    Ok(None)
}

pub(crate) fn native_hashset_init_from_collection(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let mut this_ref = extract_ref_arg(args, 0)?;
    let collection_ref = extract_ref_arg(args, 1)?;
    native_hashset_init(&[Slot::Reference(Some(this_ref))], heap, output, control)?;

    let collection_class = heap.get(collection_ref)?.class_name.clone();
    let elements: Vec<Slot> = if collection_class == "java/util/HashSet" {
        heap.get(collection_ref)?
            .fields
            .iter()
            .skip(1)
            .copied()
            .collect()
    } else {
        let array_slot = ops.invoke(
            heap,
            output,
            &collection_class,
            "toArray",
            "()[Ljava/lang/Object;",
            vec![Slot::Reference(Some(collection_ref))],
        )?;
        let Some(Slot::Reference(Some(array_ref))) = array_slot else {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        };
        patch_forwarded_ref_if_needed(heap, &mut this_ref);
        heap.get(array_ref)?.fields.clone()
    };

    for element in elements {
        native_hashset_add(
            &[Slot::Reference(Some(this_ref)), element],
            heap,
            output,
            control,
        )?;
    }
    Ok(None)
}

/// Native: `HashSet.add(Object)Z` — adds element if not already present.
/// Returns 1 if added, 0 if element was already in the set.
pub(crate) fn native_hashset_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    // fields[0] = size, fields[1..] = elements
    if find_hashset_entry_index(&heap.get(this_ref)?.fields, &element, heap).is_some() {
        return Ok(Some(Slot::Int(0))); // duplicate
    }

    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(Error::NullPointerException),
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1)))
}

/// Native: `HashSet.contains(Object)Z` — returns 1 if element is present, 0 otherwise.
pub(crate) fn native_hashset_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let fields = &heap.get(this_ref)?.fields;

    if find_hashset_entry_index(fields, &element, heap).is_some() {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashSet.remove(Object)Z` — removes element if present, returns 1 if removed, 0 if absent.
/// Uses swap-remove (swaps target with last element) for O(1) deletion.
pub(crate) fn native_hashset_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let element = extract_slot_arg(args, 1);
    let i_opt = find_hashset_entry_index(&heap.get(this_ref)?.fields, &element, heap);

    if let Some(i) = i_opt {
        let obj = heap.get_mut(this_ref)?;
        let last_idx = obj.fields.len() - 1;
        obj.fields.swap(i, last_idx);
        obj.fields.truncate(obj.fields.len() - 1);
        match obj.fields.first_mut() {
            Some(Slot::Int(sz)) => *sz -= 1,
            _ => return Err(Error::NullPointerException),
        }
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashSet.size()I` — returns element count from fields\[0\].
pub(crate) fn native_hashset_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Ok(Some(Slot::Int(*n))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `HashSet.isEmpty()Z` — returns 1 if size == 0, else 0.
pub(crate) fn native_hashset_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) => Ok(Some(Slot::Int(1))),
        Some(Slot::Int(_)) => Ok(Some(Slot::Int(0))),
        _ => Ok(Some(Slot::Int(1))),
    }
}

/// Native: `HashSet.iterator()Iterator` — creates a `HashSetIterator`.
pub(crate) fn native_hashset_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_ref = heap.allocate("duke/util/HashSetIterator".to_string(), 2);
    {
        let iter_obj = heap.get_mut(iter_ref)?;
        iter_obj.fields[0] = Slot::Reference(Some(this_ref));
        iter_obj.fields[1] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
}

/// Native: `HashSetIterator.<init>` — no-op; fields are set by `native_hashset_iterator`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_hashset_iter_init(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}

/// Native: `HashSetIterator.hasNext()Z`
pub(crate) fn native_hashset_iter_hasnext(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let iter_obj = heap.get(this_ref)?;
    let set_ref = match iter_obj.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Int(0))),
    };
    let cursor = match iter_obj.fields.get(1) {
        Some(Slot::Int(i)) => *i,
        _ => 0,
    };
    let set_size = match heap.get(set_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(cursor < set_size))))
}

/// Native: `HashSetIterator.next()Object` — returns element at cursor, advances cursor.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_hashset_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (set_ref, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let sr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(Error::NullPointerException),
        };
        let c = match iter_obj.fields.get(1) {
            Some(Slot::Int(i)) => *i,
            _ => 0,
        };
        (sr, c)
    };
    let element = {
        let set_obj = heap.get(set_ref)?;
        match set_obj.fields.get(cursor as usize + 1) {
            Some(slot) => *slot,
            None => {
                return Err(Error::JavaException {
                    class_name: "java/util/NoSuchElementException".to_string(),
                });
            }
        }
    };
    heap.get_mut(this_ref)?.fields[1] = Slot::Int(cursor + 1);
    Ok(Some(element))
}

/// Native: `HashSet.stream()Stream` — wraps elements into a `duke/util/Stream`.
pub(crate) fn native_hashset_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(i32::try_from(size).unwrap_or(0));
    for elem in elems {
        heap.get_mut(stream_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `LinkedList.stream()Stream` — wraps elements into a `duke/util/Stream`.
pub(crate) fn native_linked_list_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_stream(args, heap, out, control)
}

/// Native: `Collections.nCopies(int, Object)List` — returns a list of N copies of an element.
pub(crate) fn native_collections_n_copies(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let n = usize::try_from(extract_int_arg(args, 0)?.max(0)).unwrap_or(0);
    let elem = extract_slot_arg(args, 1);
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    heap.get_mut(list_ref)?.fields[0] = Slot::Int(i32::try_from(n).unwrap_or(0));
    for _ in 0..n {
        heap.get_mut(list_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(list_ref))))
}

/// Native: `String.chars()IntStream` — returns char code points as an `IntStream`.
pub(crate) fn native_string_chars(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let values: Vec<i32> = s.chars().map(|c| c as i32).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

const PROCESS_ID_FIELD: usize = 0;
const PROCESS_STDIN_FIELD: usize = 1;
const PROCESS_STDOUT_FIELD: usize = 2;
const PROCESS_STDERR_FIELD: usize = 3;

fn string_array_from_slot(slot: Slot, heap: &duke_gc::Heap) -> Result<Vec<String>> {
    let Slot::Reference(Some(array_ref)) = slot else {
        return Err(Error::NullPointerException);
    };
    let elements = heap.get(array_ref)?.fields.clone();
    elements
        .into_iter()
        .map(|element| match element {
            Slot::Reference(Some(string_ref)) => string_value_from_ref(heap, string_ref),
            _ => Err(Error::NullPointerException),
        })
        .collect()
}

fn optional_file_path_from_slot(
    slot: Slot,
    heap: &duke_gc::Heap,
) -> Result<Option<std::path::PathBuf>> {
    match slot {
        Slot::Reference(Some(file_ref)) => Ok(Some(file_path_from_ref(file_ref, heap)?)),
        Slot::Reference(None) => Ok(None),
        _ => Err(Error::NullPointerException),
    }
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

fn spawn_process_impl(
    heap: &mut duke_gc::Heap,
    command: &[String],
    cwd: Option<&std::path::Path>,
) -> Result<Option<Slot>> {
    let ids = heap.spawn_host_process(command, cwd)?;
    allocate_process_impl(heap, ids)
}

fn process_field_id_from_this(
    args: &[Slot],
    heap: &duke_gc::Heap,
    field_idx: usize,
) -> Result<i32> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.get(field_idx) {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(Error::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
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

pub(crate) fn native_process_builder_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let command_slot = extract_slot_arg(args, 1);
    let builder_obj = heap.get_mut(this_ref)?;
    if builder_obj.fields.len() < 2 {
        return Err(Error::InvalidRef { address: this_ref });
    }
    builder_obj.fields[0] = command_slot;
    builder_obj.fields[1] = Slot::Reference(None);
    Ok(None)
}

pub(crate) fn native_process_builder_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let directory_slot = extract_slot_arg(args, 1);
    let builder_obj = heap.get_mut(this_ref)?;
    if builder_obj.fields.len() < 2 {
        return Err(Error::InvalidRef { address: this_ref });
    }
    builder_obj.fields[1] = directory_slot;
    Ok(Some(Slot::Reference(Some(this_ref))))
}

pub(crate) fn native_process_builder_start(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let builder_obj = heap.get(this_ref)?;
    let command_slot = builder_obj
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let directory_slot = builder_obj
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let command = string_array_from_slot(command_slot, heap)?;
    let cwd = optional_file_path_from_slot(directory_slot, heap)?;
    spawn_process_impl(heap, &command, cwd.as_deref())
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
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

pub(crate) fn native_process_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stdout_id = process_field_id_from_this(args, heap, PROCESS_STDOUT_FIELD)?;
    allocate_process_stream(heap, "duke/process/ProcessInputStream", stdout_id)
}

pub(crate) fn native_process_get_error_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stderr_id = process_field_id_from_this(args, heap, PROCESS_STDERR_FIELD)?;
    allocate_process_stream(heap, "duke/process/ProcessErrorStream", stderr_id)
}

pub(crate) fn native_process_get_output_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stdin_id = process_field_id_from_this(args, heap, PROCESS_STDIN_FIELD)?;
    allocate_process_stream(heap, "duke/process/ProcessOutputStream", stdin_id)
}

pub(crate) fn native_process_wait_for(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    Ok(Some(Slot::Int(heap.wait_host_process(process_id)?)))
}

pub(crate) fn native_process_exit_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    let Some(exit_code) = heap.try_host_process_exit_value(process_id)? else {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalThreadStateException".into(),
        });
    };
    Ok(Some(Slot::Int(exit_code)))
}

pub(crate) fn native_process_destroy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    heap.destroy_host_process(process_id)?;
    Ok(None)
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

/// Native: `Stream.generate(Supplier)Stream` — returns a `duke/util/GeneratorStream` sentinel.
/// Materialised into a real Stream when `.limit(N)` is called.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_generate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let supplier = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/GeneratorStream".to_string(), 1);
    heap.get_mut(r)?.fields[0] = supplier;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Stream.iterate(seed, UnaryOperator)Stream` — returns a `duke/util/IteratorStream`.
/// Materialised into a real Stream when `.limit(N)` is called.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_iterate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let seed = extract_slot_arg(args, 0);
    let fn_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/IteratorStream".to_string(), 2);
    heap.get_mut(r)?.fields[0] = seed;
    heap.get_mut(r)?.fields[1] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Stream.concat(Stream, Stream)Stream` — concatenates two eager streams.
pub(crate) fn native_stream_concat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a_ref = extract_ref_arg(args, 0)?;
    let b_ref = extract_ref_arg(args, 1)?;
    let a_size = match heap.get(a_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let b_size = match heap.get(b_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let a_elems: Vec<Slot> = heap.get(a_ref)?.fields[1..=a_size].to_vec();
    let b_elems: Vec<Slot> = heap.get(b_ref)?.fields[1..=b_size].to_vec();
    let total = a_size + b_size;
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(total).unwrap_or(0));
    for elem in a_elems.into_iter().chain(b_elems) {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `Stream.empty()Stream` — returns a zero-element stream.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_stream_empty(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Phase 46: Stream.takeWhile/dropWhile (Java 9)
// ---------------------------------------------------------------------------

/// Native: `Stream.takeWhile(Predicate)Stream` — keeps prefix while predicate holds.
pub(crate) fn native_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept: Vec<Slot> = Vec::new();
    for elem in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![Slot::Reference(Some(pred_ref)), elem],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(elem);
        } else {
            break;
        }
    }
    let new_size = i32::try_from(kept.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in kept {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

/// Native: `Stream.dropWhile(Predicate)Stream` — drops prefix while predicate holds, keeps rest.
pub(crate) fn native_stream_drop_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(None)));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut dropping = true;
    let mut kept: Vec<Slot> = Vec::new();
    for elem in elems {
        if dropping {
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(Ljava/lang/Object;)Z",
                vec![Slot::Reference(Some(pred_ref)), elem],
            )?;
            if matches!(result, Some(Slot::Int(n)) if n != 0) {
                continue;
            }
            dropping = false;
        }
        kept.push(elem);
    }
    let new_size = i32::try_from(kept.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in kept {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

// ---------------------------------------------------------------------------
// Phase 45: ArrayList.forEach, Stream.sorted(Comparator), Arrays.toString,
//           HashMap.replace, Collections.swap/unmodifiableMap,
//           Collectors.partitioningBy, IntStream.sorted
// ---------------------------------------------------------------------------

/// Native: `ArrayList.forEach(Consumer)V` — invokes consumer.accept(elem) for each element.
pub(crate) fn native_arraylist_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(consumer_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(None);
    };
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(list_ref)?.fields[1..=size].to_vec();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for elem in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![Slot::Reference(Some(consumer_ref)), elem],
        )?;
    }
    Ok(None)
}

/// Native: `Stream.sorted(Comparator)Stream` — sorts stream elements using the given comparator.
pub(crate) fn native_stream_sorted_comparator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    // If no comparator provided, fall back to natural-order sort.
    let Slot::Reference(Some(comp_ref)) = extract_slot_arg(args, 1)
    else {
        return native_stream_sorted(args, heap, out, control, ops);
    };
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    // Insertion sort using the provided comparator.
    for i in 1..elems.len() {
        let mut j = i;
        while j > 0 {
            let comp_class = heap.get(comp_ref)?.class_name.clone();
            let cmp = ops.invoke(
                heap,
                out,
                &comp_class,
                "compare",
                "(Ljava/lang/Object;Ljava/lang/Object;)I",
                vec![Slot::Reference(Some(comp_ref)), elems[j - 1], elems[j]],
            )?;
            if matches!(cmp, Some(Slot::Int(n)) if n > 0) {
                elems.swap(j - 1, j);
                j -= 1;
            } else {
                break;
            }
        }
    }
    let new_size = i32::try_from(elems.len()).unwrap_or(0);
    let new_stream = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(new_size);
    for elem in elems {
        heap.get_mut(new_stream)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

/// Native: `Arrays.toString(int[])String` — formats as `[1, 2, 3]`.
pub(crate) fn native_arrays_to_string_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    use std::fmt::Write as FmtWrite;
    let arr_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(arr_ref)?.fields.clone();

    // ⚡ Bolt: Eliminate intermediate Vec<String> allocation, format! macro overhead,
    // and .join() by appending directly to a single String buffer.
    let mut result = String::with_capacity(fields.len() * 4 + 2);
    result.push('[');
    for (i, s) in fields.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        match s {
            Slot::Int(n) => { let _ = write!(result, "{n}"); },
            _ => result.push('0'),
        }
    }
    result.push(']');

    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Arrays.toString(Object[])String` — formats as `[a, b, c]`.
pub(crate) fn native_arrays_to_string_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    use std::fmt::Write as FmtWrite;
    let arr_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(arr_ref)?.fields.clone();

    // ⚡ Bolt: Eliminate intermediate Vec<String> allocation, format! macro overhead,
    // and .join() by appending directly to a single String buffer.
    let mut result = String::with_capacity(fields.len() * 8 + 2);
    result.push('[');
    for (i, s) in fields.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        match s {
            Slot::Reference(Some(r)) => {
                let part = heap
                    .get(*r)
                    .ok()
                    .and_then(|o| o.string_value.clone())
                    .unwrap_or_else(|| "null".to_string());
                result.push_str(&part);
            }
            Slot::Reference(None) => result.push_str("null"),
            Slot::Int(n) => {
                let _ = write!(result, "{n}");
            }
            _ => result.push('?'),
        }
    }
    result.push(']');

    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `HashMap.replace(Object, Object)Object` — updates value for existing key,
/// returns the old value or null if key was absent.
pub(crate) fn native_hashmap_replace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let new_val = extract_slot_arg(args, 2);
    let i_opt = find_hashmap_entry_index(&heap.get(this_ref)?.fields, &key, heap);
    if let Some(i) = i_opt {
        let old = heap.get(this_ref)?.fields[i + 1];
        heap.get_mut(this_ref)?.fields[i + 1] = new_val;
        Ok(Some(old))
    } else {
        Ok(Some(Slot::Reference(None)))
    }
}

/// Native: `Collections.swap(List, int, int)V` — swaps elements at indices i and j.
pub(crate) fn native_collections_swap(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let i = match args.get(1) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(None),
    };
    let j = match args.get(2) {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => return Ok(None),
    };
    // fields[0] = size, elements at fields[1..=size]
    let fi = i + 1;
    let fj = j + 1;
    let fields = heap.get(list_ref)?.fields.clone();
    let len = fields.len();
    if fi < len && fj < len {
        let vi = fields[fi];
        let vj = fields[fj];
        heap.get_mut(list_ref)?.fields[fi] = vj;
        heap.get_mut(list_ref)?.fields[fj] = vi;
    }
    Ok(None)
}

/// Native: `Collections.unmodifiableMap(Map)Map` — identity stub (we have no mutation checks).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let src_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Reference(None))),
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let src_class = heap.get(src_ref)?.class_name.clone();
    // Copy data into an UnmodifiableMap wrapper (same layout as HashMap)
    let r = heap.allocate("java/util/UnmodifiableMap".to_string(), src_fields.len());
    let r_fields = &mut heap.get_mut(r)?.fields;
    *r_fields = src_fields;
    drop(src_class);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.partitioningBy(Predicate)Collector` — returns a sentinel collector.
pub(crate) fn native_collectors_partitioning_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let pred = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/PartitioningByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = pred;
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_collectors_partitioning_by_downstream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let pred = extract_slot_arg(args, 0);
    let downstream = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/PartitioningByDownstreamCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = pred;
    heap.get_mut(r)?.fields[1] = downstream;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `IntStream.sorted()IntStream` — returns a new sorted `IntStream`.
pub(crate) fn native_int_stream_sorted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let mut vals: Vec<i32> = heap.get(stream_ref)?.fields[1..=size]
        .iter()
        .filter_map(|s| if let Slot::Int(n) = s { Some(*n) } else { None })
        .collect();
    vals.sort_unstable();
    let new_stream = heap.allocate("duke/util/IntStream".to_string(), 1);
    heap.get_mut(new_stream)?.fields[0] = Slot::Int(i32::try_from(vals.len()).unwrap_or(0));
    for v in vals {
        heap.get_mut(new_stream)?.fields.push(Slot::Int(v));
    }
    Ok(Some(Slot::Reference(Some(new_stream))))
}

// ---------------------------------------------------------------------------
// Phase 49: Comparator.thenComparing, Predicate combinators, Function combinators,
//           Stream.mapToLong, Stream.mapToDouble
// ---------------------------------------------------------------------------

/// Native: `Comparator.thenComparing(Comparator)Comparator` — chains two comparators.
/// Stores primary in `fields[0]`, secondary in `fields[1]`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_then_comparing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let primary = extract_slot_arg(args, 0);
    let secondary = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ThenComparingComparator".to_string(), 2);
    heap.get_mut(r)?.fields[0] = primary;
    heap.get_mut(r)?.fields[1] = secondary;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ThenComparingComparator.compare(O,O)I` — runs primary then secondary.
pub(crate) fn native_then_comparing_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let primary = extract_first_field_arg(heap, this_ref)?;
    let secondary = extract_field_arg(heap, this_ref, 1)?;
    // Invoke primary.compare(a, b)
    let result = invoke_comparator(primary, a, b, heap, out, ops)?;
    if result != 0 {
        return Ok(Some(Slot::Int(result)));
    }
    // Tie-break with secondary
    let result2 = invoke_comparator(secondary, a, b, heap, out, ops)?;
    Ok(Some(Slot::Int(result2)))
}

/// Helper: dispatch `comparator.compare(a, b)` via ops.invoke.
fn invoke_comparator(
    comparator: Slot,
    a: Slot,
    b: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<i32> {
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

/// Native: `Predicate.and(Predicate)Predicate` — logical AND of two predicates.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_predicate_and(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let left = extract_slot_arg(args, 0);
    let right = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/AndPredicate".to_string(), 2);
    heap.get_mut(r)?.fields[0] = left;
    heap.get_mut(r)?.fields[1] = right;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `AndPredicate.test(O)Z` — both predicates must return true.
pub(crate) fn native_and_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let left = extract_first_field_arg(heap, this_ref)?;
    let right = extract_field_arg(heap, this_ref, 1)?;
    let la = invoke_predicate_test(left, elem, heap, out, ops)?;
    if !la {
        return Ok(Some(Slot::Int(0)));
    }
    let rb = invoke_predicate_test(right, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(rb))))
}

/// Native: `Predicate.or(Predicate)Predicate` — logical OR of two predicates.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_predicate_or(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let left = extract_slot_arg(args, 0);
    let right = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/OrPredicate".to_string(), 2);
    heap.get_mut(r)?.fields[0] = left;
    heap.get_mut(r)?.fields[1] = right;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `OrPredicate.test(O)Z` — either predicate returning true is sufficient.
pub(crate) fn native_or_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let left = extract_first_field_arg(heap, this_ref)?;
    let right = extract_field_arg(heap, this_ref, 1)?;
    let la = invoke_predicate_test(left, elem, heap, out, ops)?;
    if la {
        return Ok(Some(Slot::Int(1)));
    }
    let rb = invoke_predicate_test(right, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(rb))))
}

/// Native: `Predicate.negate()Predicate` — logical NOT of a predicate.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_predicate_negate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let original = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/NegatedPredicate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = original;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `NegatedPredicate.test(O)Z` — inverts the wrapped predicate.
pub(crate) fn native_negated_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let original = extract_first_field_arg(heap, this_ref)?;
    let result = invoke_predicate_test(original, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(!result))))
}

/// Helper: dispatch `predicate.test(elem)` via ops.invoke, returns bool.
fn invoke_predicate_test(
    predicate: Slot,
    elem: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<bool> {
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

/// Native: `Function.andThen(Function)Function` — `f.andThen(g)` = `g(f(x))`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_function_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let first = extract_slot_arg(args, 0);
    let second = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/AndThenFunction".to_string(), 2);
    heap.get_mut(r)?.fields[0] = first;
    heap.get_mut(r)?.fields[1] = second;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `AndThenFunction.apply(O)O` — applies first then second.
pub(crate) fn native_and_then_function_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let input = extract_slot_arg(args, 1);
    let first = extract_first_field_arg(heap, this_ref)?;
    let second = extract_field_arg(heap, this_ref, 1)?;
    let mid = invoke_function_apply(first, input, heap, out, ops)?;
    invoke_function_apply(second, mid, heap, out, ops).map(Some)
}

/// Native: `Consumer.andThen(Consumer)Consumer` — chains two consumers sequentially.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_consumer_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let first = extract_slot_arg(args, 0);
    let second = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/AndThenConsumer".to_string(), 2);
    heap.get_mut(r)?.fields[0] = first;
    heap.get_mut(r)?.fields[1] = second;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `AndThenConsumer.accept(O)V` — runs first then second consumer.
pub(crate) fn native_and_then_consumer_accept(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let arg = extract_slot_arg(args, 1);
    let first = extract_first_field_arg(heap, this_ref)?;
    let second = extract_field_arg(heap, this_ref, 1)?;
    invoke_consumer_accept(first, arg, heap, out, ops)?;
    invoke_consumer_accept(second, arg, heap, out, ops)?;
    Ok(None)
}

fn invoke_consumer_accept(
    consumer: Slot,
    arg: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<()> {
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

/// Native: `Function.compose(Function)Function` — `f.compose(g)` = `f(g(x))`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_function_compose(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let outer = extract_slot_arg(args, 0);
    let inner = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ComposeFunction".to_string(), 2);
    heap.get_mut(r)?.fields[0] = outer;
    heap.get_mut(r)?.fields[1] = inner;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ComposeFunction.apply(O)O` — applies inner then outer.
pub(crate) fn native_compose_function_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let input = extract_slot_arg(args, 1);
    let outer = extract_first_field_arg(heap, this_ref)?;
    let inner = extract_field_arg(heap, this_ref, 1)?;
    let mid = invoke_function_apply(inner, input, heap, out, ops)?;
    invoke_function_apply(outer, mid, heap, out, ops).map(Some)
}

/// Helper: dispatch `function.apply(input)` via ops.invoke.
fn invoke_function_apply(
    function: Slot,
    input: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<Slot> {
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

/// Native: `BiFunction.andThen(Function)BiFunction` — returns `BiFunctionAndThen` proxy.
pub(crate) fn native_bifunction_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bifunction = extract_slot_arg(args, 0);
    let after = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/BiFunctionAndThen".to_string(), 2);
    heap.get_mut(r)?.fields[0] = bifunction;
    heap.get_mut(r)?.fields[1] = after;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `BiFunctionAndThen.apply(Object,Object)Object` — calls wrapped bifunction then after.
pub(crate) fn native_bifunction_and_then_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let bifunction = extract_first_field_arg(heap, this_ref)?;
    let after = extract_field_arg(heap, this_ref, 1)?;
    let Slot::Reference(Some(bf_ref)) = bifunction else {
        return Ok(Some(Slot::Reference(None)));
    };
    let bf_class = heap.get(bf_ref)?.class_name.clone();
    let mid = ops
        .invoke(
            heap,
            out,
            &bf_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![bifunction, a, b],
        )?
        .unwrap_or(Slot::Reference(None));
    Ok(Some(invoke_function_apply(after, mid, heap, out, ops)?))
}

/// Native: `Stream.mapToLong(ToLongFunction)LongStream` — maps each element via `applyAsLong`.
pub(crate) fn native_stream_map_to_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let mut values = Vec::with_capacity(elems.len());
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(Ljava/lang/Object;)J",
                vec![fn_slot, elem],
            )?
            .unwrap_or(Slot::Long(0));
        let v = match result {
            Slot::Long(n) => n,
            Slot::Int(n) => i64::from(n),
            _ => 0,
        };
        values.push(v);
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
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

/// Native: `LongStream.sum()J` — sums all elements.
pub(crate) fn native_long_stream_sum(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let sum: i64 = heap.get(stream_ref)?.fields[1..=size]
        .iter()
        .map(|s| match s {
            Slot::Long(n) => *n,
            Slot::Int(n) => i64::from(*n),
            _ => 0,
        })
        .sum();
    Ok(Some(Slot::Long(sum)))
}

/// Native: `Stream.mapToDouble(ToDoubleFunction)DoubleStream` — maps each element via `applyAsDouble`.
pub(crate) fn native_stream_map_to_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = stream_elements(heap, stream_ref)?;
    let mut values = Vec::with_capacity(elems.len());
    for elem in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(Ljava/lang/Object;)D",
                vec![fn_slot, elem],
            )?
            .unwrap_or(Slot::Double(0.0));
        let v = match result {
            Slot::Double(d) => d,
            Slot::Float(f) => f64::from(f),
            Slot::Int(n) => f64::from(n),
            _ => 0.0,
        };
        values.push(v);
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, values,
    )))))
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

/// Native: `DoubleStream.sum()D` — sums all elements.
pub(crate) fn native_double_stream_sum(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let sum: f64 = stream_elements(heap, stream_ref)?
        .iter()
        .map(|s| match s {
            Slot::Double(d) => *d,
            Slot::Float(f) => f64::from(*f),
            Slot::Int(n) => f64::from(*n),
            _ => 0.0,
        })
        .sum();
    Ok(Some(Slot::Double(sum)))
}

// ---------------------------------------------------------------------------
// Phase 51: LongStream full ops, DoubleStream full ops,
//           IntStream.asLongStream/asDoubleStream, Collectors.summingInt/averagingInt
// ---------------------------------------------------------------------------

// ---- Helper extractors ----

/// Extracts stream payload slots from `fields[1..=size]`, safely handling empty streams and
/// malformed `size` headers.
fn stream_elements(heap: &duke_gc::Heap, stream_ref: u64) -> Result<Vec<Slot>> {
    let obj = heap.get(stream_ref)?;
    let declared_size = match obj.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let actual_size = declared_size.min(obj.fields.len().saturating_sub(1));
    if actual_size == 0 {
        return Ok(Vec::new());
    }
    Ok(obj.fields[1..=actual_size].to_vec())
}

/// Extract long elements from a `duke/util/LongStream`.
fn long_stream_elems(heap: &duke_gc::Heap, ref_: u64) -> Vec<i64> {
    stream_elements(heap, ref_)
        .map(|elems| {
            elems.into_iter()
                .filter_map(|s| {
                    if let Slot::Long(n) = s {
                        Some(n)
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
    stream_elements(heap, ref_)
        .map(|elems| {
            elems.into_iter()
                .filter_map(|s| {
                    if let Slot::Double(d) = s {
                        Some(d)
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

/// Native: `LongStream.of(long[])LongStream` — from a long[] vararg array.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let values: Vec<i64> = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| {
            if let Slot::Long(n) = s {
                Some(*n)
            } else {
                None
            }
        })
        .collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}

/// Native: `LongStream.range(long,long)LongStream` — half-open range [start, end).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let start = match args.first().copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let end = match args.get(1).copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let values: Vec<i64> = (start..end).collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}

/// Native: `LongStream.rangeClosed(long,long)LongStream` — inclusive range [start, end].
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_range_closed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let start = match args.first().copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let end = match args.get(1).copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let values: Vec<i64> = (start..=end).collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}

// ---- LongStream terminal ops ----

/// Native: `LongStream.count()J`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match heap.get(r)?.fields.first() {
        Some(Slot::Int(n)) => i64::from(*n),
        _ => 0,
    };
    Ok(Some(Slot::Long(n)))
}

/// Native: `LongStream.min()OptionalLong`
pub(crate) fn native_long_stream_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let opt = make_optional_long(heap, elems.into_iter().min());
    Ok(Some(Slot::Reference(Some(opt))))
}

/// Native: `LongStream.max()OptionalLong`
pub(crate) fn native_long_stream_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let opt = make_optional_long(heap, elems.into_iter().max());
    Ok(Some(Slot::Reference(Some(opt))))
}

/// Native: `LongStream.average()OptionalDouble`
pub(crate) fn native_long_stream_average(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let opt = if elems.is_empty() {
        None
    } else {
        #[allow(clippy::cast_precision_loss)]
        Some(elems.iter().sum::<i64>() as f64 / elems.len() as f64)
    };
    let opt_ref = make_optional_double_val(heap, opt);
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `LongStream.toArray()long[]`
pub(crate) fn native_long_stream_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let arr_ref = heap.allocate("[J".to_string(), elems.len());
    for (i, v) in elems.into_iter().enumerate() {
        heap.get_mut(arr_ref)?.fields[i] = Slot::Long(v);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `LongStream.sorted()LongStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_sorted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let mut elems = long_stream_elems(heap, r);
    elems.sort_unstable();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, elems)))))
}

/// Native: `LongStream.distinct()LongStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_distinct(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let mut seen = std::collections::HashSet::new();
    let elems: Vec<i64> = long_stream_elems(heap, r)
        .into_iter()
        .filter(|v| seen.insert(*v))
        .collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, elems)))))
}

/// Native: `LongStream.reduce(long, LongBinaryOperator)long`
pub(crate) fn native_long_stream_reduce_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let identity = match args.get(1).copied() {
        Some(Slot::Long(n)) => n,
        _ => 0,
    };
    let fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Long(identity)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = long_stream_elems(heap, r);
    let mut acc = identity;
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(JJ)J",
                vec![fn_slot, Slot::Long(acc), Slot::Long(v)],
            )?
            .unwrap_or(Slot::Long(0));
        acc = match result {
            Slot::Long(n) => n,
            Slot::Int(n) => i64::from(n),
            _ => acc,
        };
    }
    Ok(Some(Slot::Long(acc)))
}

/// Native: `LongStream.boxed()Stream` — boxes each long into `java/lang/Long`.
pub(crate) fn native_long_stream_boxed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for v in elems {
        let boxed_ref = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(boxed_ref)?.fields[0] = Slot::Long(v);
        heap.get_mut(stream_ref)?
            .fields
            .push(Slot::Reference(Some(boxed_ref)));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

// ---- LongStream intermediate ops ----

/// Native: `LongStream.filter(LongPredicate)LongStream`
pub(crate) fn native_long_stream_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(J)Z",
            vec![pred_slot, Slot::Long(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, kept)))))
}

/// Native: `LongStream.map(LongUnaryOperator)LongStream`
pub(crate) fn native_long_stream_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(J)J",
            vec![fn_slot, Slot::Long(v)],
        )?;
        result.push(match r {
            Some(Slot::Long(n)) => n,
            Some(Slot::Int(n)) => i64::from(n),
            _ => 0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}

/// Native: `LongStream.forEach(LongConsumer)V`
pub(crate) fn native_long_stream_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(None);
    };
    let elems = long_stream_elems(heap, r);
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for v in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(J)V",
            vec![consumer_slot, Slot::Long(v)],
        )?;
    }
    Ok(None)
}

// ---- LongStream.mapToInt / mapToDouble ----

/// Native: `LongStream.mapToInt(LongToIntFunction)IntStream`
pub(crate) fn native_long_stream_map_to_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(J)I",
            vec![fn_slot, Slot::Long(v)],
        )?;
        result.push(match r {
            Some(Slot::Int(n)) => n,
            _ => 0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}

// ---- DoubleStream static factories ----

/// Native: `DoubleStream.of(double[])DoubleStream` — from a double[] vararg array.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = extract_ref_arg(args, 0)?;
    let values: Vec<f64> = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| {
            if let Slot::Double(d) = s {
                Some(*d)
            } else {
                None
            }
        })
        .collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, values,
    )))))
}

/// Native: `DoubleStream.of(double)DoubleStream` — single-element factory.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_of_single(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = match args.first() {
        Some(Slot::Double(d)) => *d,
        Some(Slot::Float(f)) => f64::from(*f),
        _ => 0.0,
    };
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap,
        vec![v],
    )))))
}

// ---- DoubleStream terminal ops ----

/// Native: `DoubleStream.count()J`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match heap.get(r)?.fields.first() {
        Some(Slot::Int(n)) => i64::from(*n),
        _ => 0,
    };
    Ok(Some(Slot::Long(n)))
}

/// Native: `DoubleStream.min()OptionalDouble`
pub(crate) fn native_double_stream_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let min = elems.iter().copied().reduce(f64::min);
    let opt_ref = make_optional_double_val(heap, min);
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `DoubleStream.max()OptionalDouble`
pub(crate) fn native_double_stream_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let max = elems.iter().copied().reduce(f64::max);
    let opt_ref = make_optional_double_val(heap, max);
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `DoubleStream.average()OptionalDouble`
pub(crate) fn native_double_stream_average(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let opt = if elems.is_empty() {
        None
    } else {
        #[allow(clippy::cast_precision_loss)]
        Some(elems.iter().sum::<f64>() / elems.len() as f64)
    };
    let opt_ref = make_optional_double_val(heap, opt);
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `DoubleStream.toArray()double[]`
pub(crate) fn native_double_stream_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let arr_ref = heap.allocate("[D".to_string(), elems.len());
    for (i, v) in elems.into_iter().enumerate() {
        heap.get_mut(arr_ref)?.fields[i] = Slot::Double(v);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `DoubleStream.sorted()DoubleStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_sorted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let mut elems = double_stream_elems(heap, r);
    elems.sort_by(f64::total_cmp);
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, elems)))))
}

// ---- DoubleStream intermediate ops ----

/// Native: `DoubleStream.filter(DoublePredicate)DoubleStream`
pub(crate) fn native_double_stream_filter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(D)Z",
            vec![pred_slot, Slot::Double(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, kept)))))
}

/// Native: `DoubleStream.map(DoubleUnaryOperator)DoubleStream`
pub(crate) fn native_double_stream_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(D)D",
            vec![fn_slot, Slot::Double(v)],
        )?;
        result.push(match r {
            Some(Slot::Double(d)) => d,
            Some(Slot::Float(f)) => f64::from(f),
            Some(Slot::Int(n)) => f64::from(n),
            _ => 0.0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, result,
    )))))
}

// ---- IntStream.asLongStream / asDoubleStream ----

/// Native: `IntStream.asLongStream()LongStream` — widens each int to long.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_as_long_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let values: Vec<i64> = int_stream_elems(heap, r)
        .into_iter()
        .map(i64::from)
        .collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, values)))))
}

/// Native: `IntStream.asDoubleStream()DoubleStream` — widens each int to double.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_as_double_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let values: Vec<f64> = int_stream_elems(heap, r)
        .into_iter()
        .map(f64::from)
        .collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, values,
    )))))
}

// ---- OptionalLong ----

/// Native: `OptionalLong.getAsLong()J`
pub(crate) fn native_optional_long_get_as_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    if !present {
        return Err(Error::MethodNotFound {
            name: "OptionalLong.getAsLong on empty".to_string(),
            descriptor: String::new(),
        });
    }
    Ok(Some(
        heap.get(r)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Long(0)),
    ))
}

/// Native: `OptionalLong.isPresent()Z`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_long_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}

// ---- Collectors.summingInt / averagingInt ----

/// Native: `Collectors.summingInt(ToIntFunction)Collector` — returns a `SummingIntCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_summing_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/SummingIntCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.averagingInt(ToIntFunction)Collector` — returns an `AveragingIntCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_averaging_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/AveragingIntCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Phase 53: IntStream/LongStream terminal ops, Comparator.comparingLong,
//           Optional.or / ifPresentOrElse, Collectors.toUnmodifiable*
// ---------------------------------------------------------------------------

/// Native: `IntStream.findFirst()OptionalInt`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_find_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = int_stream_elems(heap, r);
    let opt = make_optional_int(heap, elems.into_iter().next());
    Ok(Some(Slot::Reference(Some(opt))))
}

/// Native: `IntStream.anyMatch(IntPredicate)Z`
pub(crate) fn native_int_stream_any_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(I)Z",
            vec![pred_slot, Slot::Int(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `IntStream.allMatch(IntPredicate)Z`
pub(crate) fn native_int_stream_all_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(I)Z",
            vec![pred_slot, Slot::Int(v)],
        )?;
        if !matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `IntStream.noneMatch(IntPredicate)Z`
pub(crate) fn native_int_stream_none_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(I)Z",
            vec![pred_slot, Slot::Int(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `IntStream.mapToLong(IntToLongFunction)LongStream`
pub(crate) fn native_int_stream_map_to_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(I)J",
            vec![fn_slot, Slot::Int(v)],
        )?;
        result.push(match r {
            Some(Slot::Long(n)) => n,
            Some(Slot::Int(n)) => i64::from(n),
            _ => 0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}

/// Native: `LongStream.findFirst()OptionalLong`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_find_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = long_stream_elems(heap, r);
    let opt = make_optional_long(heap, elems.into_iter().next());
    Ok(Some(Slot::Reference(Some(opt))))
}

/// Native: `LongStream.anyMatch(LongPredicate)Z`
pub(crate) fn native_long_stream_any_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(J)Z",
            vec![pred_slot, Slot::Long(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `LongStream.allMatch(LongPredicate)Z`
pub(crate) fn native_long_stream_all_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(J)Z",
            vec![pred_slot, Slot::Long(v)],
        )?;
        if !matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `LongStream.noneMatch(LongPredicate)Z`
pub(crate) fn native_long_stream_none_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(J)Z",
            vec![pred_slot, Slot::Long(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `Comparator.comparingLong(ToLongFunction)Comparator` — wraps key extractor.
/// Creates a `duke/util/ComparingLongComparator` with `fields[0] = fn_ref`.
pub(crate) fn native_comparator_comparing_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_ref = extract_ref_arg(args, 0)?;
    let r = heap.allocate("duke/util/ComparingLongComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(Some(fn_ref));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ComparingLongComparator.compare(O,O)I` — calls `fn.applyAsLong(o)` for each element.
pub(crate) fn native_comparing_long_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(Ljava/lang/Object;)J",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Long(0));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(Ljava/lang/Object;)J",
            vec![fn_slot, b],
        )?
        .unwrap_or(Slot::Long(0));
    let result = match (ka, kb) {
        (Slot::Long(la), Slot::Long(lb)) => la.cmp(&lb) as i32,
        (Slot::Int(ia), Slot::Int(ib)) => ia.cmp(&ib) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(result)))
}

// ---------------------------------------------------------------------------
// Phase 58: Comparator.comparingDouble, Map.copyOf/entry/ofEntries,
//           Collections.singletonMap/singletonSet/unmodifiableSet
// ---------------------------------------------------------------------------

/// Creates a `duke/util/ComparingDoubleComparator` with `fields[0] = fn_ref`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_comparator_comparing_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_ref = extract_ref_arg(args, 0)?;
    let r = heap.allocate("duke/util/ComparingDoubleComparator".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Reference(Some(fn_ref));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `ComparingDoubleComparator.compare(O,O)I` — calls `fn.applyAsDouble(o)` for each.
pub(crate) fn native_comparing_double_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(Ljava/lang/Object;)D",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Double(0.0));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(Ljava/lang/Object;)D",
            vec![fn_slot, b],
        )?
        .unwrap_or(Slot::Double(0.0));
    let result = match (ka, kb) {
        (Slot::Double(da), Slot::Double(db)) => da.total_cmp(&db) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(result)))
}

/// Native: `Map.copyOf(Map)Map` — returns an unmodifiable copy backed by `HashMap`.
pub(crate) fn native_map_copy_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let src_ref = extract_ref_arg(args, 0)?;
    let copy_ref = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(copy_ref))], heap, out, control)?;
    // Iterate source map's interleaved key-val pairs: fields[0]=size, fields[1..]=k,v,k,v,...
    let size = match heap.get(src_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let pairs: Vec<Slot> = heap.get(src_ref)?.fields[1..=size * 2].to_vec();
    let mut i = 0;
    while i + 1 < pairs.len() {
        let k = pairs[i];
        let v = pairs[i + 1];
        native_hashmap_put(&[Slot::Reference(Some(copy_ref)), k, v], heap, out, control)?;
        i += 2;
    }
    Ok(Some(Slot::Reference(Some(copy_ref))))
}

/// Native: `Map.entry(K,V)Map.Entry` — creates an immutable Map.Entry.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_map_entry_factory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let key = extract_slot_arg(args, 0);
    let val = extract_slot_arg(args, 1);
    let r = heap.allocate("java/util/Map$Entry".to_string(), 2);
    heap.get_mut(r)?.fields[0] = key;
    heap.get_mut(r)?.fields[1] = val;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Map.ofEntries(Map.Entry[])Map` — builds a `HashMap` from varargs Entry array.
pub(crate) fn native_map_of_entries(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let map_ref = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(map_ref))], heap, out, control)?;
    // args[0] is the Object[] array of Map.Entry objects (anewarray layout: fields = elements)
    if let Some(Slot::Reference(Some(arr_ref))) = args.first().copied() {
        let entries: Vec<Slot> = heap.get(arr_ref)?.fields.clone();
        for entry_slot in entries {
            let Slot::Reference(Some(entry_ref)) = entry_slot else {
                continue;
            };
            let key = extract_first_field_arg(heap, entry_ref)?;
            let val = extract_field_arg(heap, entry_ref, 1)?;
            native_hashmap_put(
                &[Slot::Reference(Some(map_ref)), key, val],
                heap,
                out,
                control,
            )?;
        }
    }
    Ok(Some(Slot::Reference(Some(map_ref))))
}

/// Native: `Collections.singletonMap(K,V)Map` — returns a single-entry unmodifiable map.
pub(crate) fn native_collections_singleton_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_map_of(args, heap, out, control)
}

/// Native: `Collections.singleton(E)Set` — returns a single-element unmodifiable set.
pub(crate) fn native_collections_singleton_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_set_of_factory(args, heap, out, control)
}

/// Native: `Collections.unmodifiableSet(Set)Set` — returns a view of the set (same backing object).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collections_unmodifiable_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // Wrap the source set in a UnmodifiableSet (same field layout as HashSet).
    let src_ref = extract_ref_arg(args, 0)?;
    let src_fields = heap.get(src_ref)?.fields.clone();
    let class_name = heap.get(src_ref)?.class_name.clone();
    let n = src_fields.len();
    let wrapper_ref = heap.allocate("java/util/UnmodifiableSet".to_string(), n);
    // Copy field layout from source set
    let _ = class_name;
    for (i, f) in src_fields.into_iter().enumerate() {
        heap.get_mut(wrapper_ref)?.fields[i] = f;
    }
    Ok(Some(Slot::Reference(Some(wrapper_ref))))
}

// ---------------------------------------------------------------------------
// Phase 59: Stream.flatMapToInt/Long/Double, Collectors.toMap (3-arg),
//           forEach on HashSet/TreeSet/TreeMap/LinkedList/LinkedHashMap/PriorityQueue
// ---------------------------------------------------------------------------

/// Native: `Stream.flatMapToInt(Function<T,IntStream>)IntStream`
pub(crate) fn native_stream_flat_map_to_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result: Vec<i32> = Vec::new();
    for elem in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, elem],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(int_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}

/// Native: `Stream.flatMapToLong(Function<T,LongStream>)LongStream`
pub(crate) fn native_stream_flat_map_to_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let size = match heap.get(stream_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(stream_ref)?.fields[1..=size].to_vec();
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result: Vec<i64> = Vec::new();
    for elem in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, elem],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(long_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}

/// Native: `Stream.flatMapToDouble(Function<T,DoubleStream>)DoubleStream`
pub(crate) fn native_stream_flat_map_to_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = stream_elements(heap, stream_ref)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result: Vec<f64> = Vec::new();
    for elem in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, elem],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(double_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, result,
    )))))
}

/// Native: `Collectors.toMap(keyFn, valFn, mergeFn)Collector` — stores three functions.
pub(crate) fn native_collectors_to_map_merge(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    _ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let key_fn = extract_slot_arg(args, 0);
    let val_fn = extract_slot_arg(args, 1);
    let merge_fn = extract_slot_arg(args, 2);
    let r = heap.allocate("duke/util/ToMapMergeCollector".to_string(), 3);
    heap.get_mut(r)?.fields[0] = key_fn;
    heap.get_mut(r)?.fields[1] = val_fn;
    heap.get_mut(r)?.fields[2] = merge_fn;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `HashSet.forEach(Consumer)V`
pub(crate) fn native_hashset_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(consumer_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(None);
    };
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for elem in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![Slot::Reference(Some(consumer_ref)), elem],
        )?;
    }
    Ok(None)
}

/// Native: `TreeSet.forEach(Consumer)V`
pub(crate) fn native_treeset_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    // TreeSet uses same layout as HashSet: fields[0]=size, fields[1..size]=elements
    native_hashset_for_each(args, heap, out, control, ops)
}

/// Native: `TreeMap.forEach(BiConsumer)V`
pub(crate) fn native_treemap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    // TreeMap uses same layout as HashMap: fields[0]=size, fields[1,2]=k0/v0 ...
    native_hashmap_for_each(args, heap, out, control, ops)
}

/// Native: `LinkedHashMap.forEach(BiConsumer)V`
pub(crate) fn native_linkedhashmap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_hashmap_for_each(args, heap, out, control, ops)
}

/// Native: `LinkedList.forEach(Consumer)V`
pub(crate) fn native_linked_list_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_arraylist_for_each(args, heap, out, control, ops)
}

/// Native: `PriorityQueue.forEach(Consumer)V`
pub(crate) fn native_priorityqueue_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_arraylist_for_each(args, heap, out, control, ops)
}

// ---------------------------------------------------------------------------
// Phase 60: IntStream/LongStream/DoubleStream takeWhile/dropWhile,
//           Integer/Long/Double compare/max/min,
//           TreeMap.keySet/values/getOrDefault, TreeSet.stream
// ---------------------------------------------------------------------------

/// Native: `IntStream.takeWhile(IntPredicate)IntStream` — keeps prefix while predicate holds.
pub(crate) fn native_int_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(I)Z",
            vec![pred_slot, Slot::Int(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        } else {
            break;
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, kept)))))
}

/// Native: `IntStream.dropWhile(IntPredicate)IntStream` — drops prefix while predicate holds.
pub(crate) fn native_int_stream_drop_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut dropping = true;
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        if dropping {
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(I)Z",
                vec![pred_slot, Slot::Int(v)],
            )?;
            if !matches!(result, Some(Slot::Int(n)) if n != 0) {
                dropping = false;
                kept.push(v);
            }
        } else {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, kept)))))
}

/// Native: `LongStream.takeWhile(LongPredicate)LongStream` — keeps prefix while predicate holds.
pub(crate) fn native_long_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(J)Z",
            vec![pred_slot, Slot::Long(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        } else {
            break;
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, kept)))))
}

/// Native: `LongStream.dropWhile(LongPredicate)LongStream` — drops prefix while predicate holds.
pub(crate) fn native_long_stream_drop_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut dropping = true;
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        if dropping {
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(J)Z",
                vec![pred_slot, Slot::Long(v)],
            )?;
            if !matches!(result, Some(Slot::Int(n)) if n != 0) {
                dropping = false;
                kept.push(v);
            }
        } else {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, kept)))))
}

/// Native: `DoubleStream.takeWhile(DoublePredicate)DoubleStream` — keeps prefix while predicate holds.
pub(crate) fn native_double_stream_take_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(D)Z",
            vec![pred_slot, Slot::Double(v)],
        )?;
        if matches!(result, Some(Slot::Int(n)) if n != 0) {
            kept.push(v);
        } else {
            break;
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, kept)))))
}

/// Native: `DoubleStream.dropWhile(DoublePredicate)DoubleStream` — drops prefix while predicate holds.
pub(crate) fn native_double_stream_drop_while(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let Slot::Reference(Some(pred_ref)) = extract_slot_arg(args, 1)
    else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let pred_slot = Slot::Reference(Some(pred_ref));
    let mut dropping = true;
    let mut kept = Vec::with_capacity(elems.len());
    for v in elems {
        if dropping {
            let result = ops.invoke(
                heap,
                out,
                &pred_class,
                "test",
                "(D)Z",
                vec![pred_slot, Slot::Double(v)],
            )?;
            if !matches!(result, Some(Slot::Int(n)) if n != 0) {
                dropping = false;
                kept.push(v);
            }
        } else {
            kept.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, kept)))))
}

/// Native: `Integer.compare(int,int)int` — returns negative/zero/positive.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_integer_compare(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(a.cmp(&b) as i32)))
}

/// Native: `Integer.max(int,int)int`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_integer_max(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(a.max(b))))
}

/// Native: `Integer.min(int,int)int`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_integer_min(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(a.min(b))))
}

/// Native: `Long.compare(long,long)int`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_compare(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(Some(Slot::Int(a.cmp(&b) as i32)))
}

/// Native: `Long.max(long,long)long`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_max(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(Some(Slot::Long(a.max(b))))
}

/// Native: `Long.min(long,long)long`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_min(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(Some(Slot::Long(a.min(b))))
}

/// Native: `Double.compare(double,double)int`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_compare(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    Ok(Some(Slot::Int(a.total_cmp(&b) as i32)))
}

/// Native: `Double.max(double,double)double`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_max(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    Ok(Some(Slot::Double(a.max(b))))
}

/// Native: `Double.min(double,double)double`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_min(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    Ok(Some(Slot::Double(a.min(b))))
}

/// Native: `TreeMap.keySet()Set` — delegates to `HashMap` keySet (same field layout).
pub(crate) fn native_treemap_key_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_hashmap_key_set(args, heap, out, control)
}

/// Native: `TreeMap.values()Collection` — delegates to `HashMap` values (same field layout).
pub(crate) fn native_treemap_values(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_hashmap_values(args, heap, out, control)
}

/// Native: `TreeMap.getOrDefault(Object,Object)Object` — looks up key; returns default if absent.
pub(crate) fn native_treemap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let default_val = extract_slot_arg(args, 2);
    let found_idx = {
        let fields = &heap.get(this_ref)?.fields;
        find_hashmap_entry_index(fields, &key, heap)
    };
    Ok(Some(match found_idx {
        Some(i) => heap.get(this_ref)?.fields[i + 1],
        None => default_val,
    }))
}

/// Native: `TreeSet.stream()Stream` — wraps sorted elements into a `duke/util/Stream`.
pub(crate) fn native_treeset_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // TreeSet layout: fields[0]=size, fields[1..=size]=elements (same as HashSet)
    native_hashset_stream(args, heap, out, control)
}

/// Native: `Optional.or(Supplier<Optional>)Optional` (Java 9) —
/// returns this Optional if present; otherwise invokes supplier and returns its result.
pub(crate) fn native_optional_or(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let supplier_slot = extract_slot_arg(args, 1);
    let value = extract_first_field_arg(heap, opt_ref)?;
    // If present (non-null value stored), return self
    if !matches!(value, Slot::Reference(None)) {
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    }
    let Slot::Reference(Some(supplier_ref)) = supplier_slot else {
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    };
    let supplier_class = heap.get(supplier_ref)?.class_name.clone();
    let result = ops.invoke(
        heap,
        out,
        &supplier_class,
        "get",
        "()Ljava/lang/Object;",
        vec![supplier_slot],
    )?;
    Ok(Some(result.unwrap_or(Slot::Reference(None))))
}

/// Native: `Optional.ifPresentOrElse(Consumer, Runnable)V` (Java 9) —
/// if value present invokes consumer, otherwise invokes runnable.
pub(crate) fn native_optional_if_present_or_else(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let opt_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let runnable_slot = extract_slot_arg(args, 2);
    let value = extract_first_field_arg(heap, opt_ref)?;
    if matches!(value, Slot::Reference(None)) {
        let Slot::Reference(Some(runnable_ref)) = runnable_slot else {
            return Ok(None);
        };
        let runnable_class = heap.get(runnable_ref)?.class_name.clone();
        ops.invoke(
            heap,
            out,
            &runnable_class,
            "run",
            "()V",
            vec![runnable_slot],
        )?;
    } else {
        let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
            return Ok(None);
        };
        let consumer_class = heap.get(consumer_ref)?.class_name.clone();
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, value],
        )?;
    }
    Ok(None)
}

/// Native: `Collectors.toUnmodifiableList()Collector` (Java 10) —
/// returns the same `ToListCollector` sentinel; our interpreter treats all lists as modifiable.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_unmodifiable_list(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/ToUnmodifiableListCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.toUnmodifiableSet()Collector` (Java 10) —
/// returns the same `ToSetCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_unmodifiable_set(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/ToSetCollector".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Phase 54: IntStream/LongStream/DoubleStream limit/skip,
//           IntStream/LongStream flatMap, Collectors.mapping
// ---------------------------------------------------------------------------

/// Native: `IntStream.limit(long)IntStream` — truncate to at most n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<i32> = int_stream_elems(heap, r).into_iter().take(n).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, elems)))))
}

/// Native: `IntStream.skip(long)IntStream` — skip first n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_int_stream_skip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<i32> = int_stream_elems(heap, r).into_iter().skip(n).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, elems)))))
}

/// Native: `IntStream.flatMap(IntFunction<IntStream>)IntStream` — map each int to an `IntStream` and concatenate.
pub(crate) fn native_int_stream_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = int_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(I)Ljava/lang/Object;",
            vec![fn_slot, Slot::Int(v)],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            let sub_elems = int_stream_elems(heap, sub_ref);
            result.extend(sub_elems);
        }
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}

/// Native: `LongStream.limit(long)LongStream` — truncate to at most n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<i64> = long_stream_elems(heap, r).into_iter().take(n).collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, elems)))))
}

/// Native: `LongStream.skip(long)LongStream` — skip first n elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_stream_skip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<i64> = long_stream_elems(heap, r).into_iter().skip(n).collect();
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, elems)))))
}

/// Native: `LongStream.flatMap(LongFunction<LongStream>)LongStream`
pub(crate) fn native_long_stream_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(J)Ljava/lang/Object;",
            vec![fn_slot, Slot::Long(v)],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            let sub_elems = long_stream_elems(heap, sub_ref);
            result.extend(sub_elems);
        }
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}

/// Native: `DoubleStream.limit(long)DoubleStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<f64> = double_stream_elems(heap, r).into_iter().take(n).collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, elems)))))
}

/// Native: `DoubleStream.skip(long)DoubleStream`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_skip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Long(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        Some(Slot::Int(v)) => usize::try_from(v.max(0)).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<f64> = double_stream_elems(heap, r).into_iter().skip(n).collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(heap, elems)))))
}

/// Native: `Collectors.groupingBy(Function, Collector)Collector` — 2-arg version with downstream.
/// Creates a `duke/util/GroupingBy2Collector` with `fields[0]`=keyFn, `fields[1]`=downstream.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_grouping_by_2(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let downstream_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/GroupingBy2Collector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = fn_slot;
    heap.get_mut(r)?.fields[1] = downstream_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.mapping(Function, Collector)Collector` — transforms elements before
/// feeding to a downstream collector.  Creates a `duke/util/MappingCollector` sentinel.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_mapping(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let mapper_slot = extract_slot_arg(args, 0);
    let downstream_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/MappingCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = mapper_slot;
    heap.get_mut(r)?.fields[1] = downstream_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Phase 55: Complete DoubleStream, LongStream gaps, Collectors.summingLong/averagingDouble
// ---------------------------------------------------------------------------

/// Native: `DoubleStream.forEach(DoubleConsumer)V`
pub(crate) fn native_double_stream_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(consumer_ref)) = consumer_slot else {
        return Ok(None);
    };
    let elems = double_stream_elems(heap, r);
    let consumer_class = heap.get(consumer_ref)?.class_name.clone();
    for v in elems {
        ops.invoke(
            heap,
            out,
            &consumer_class,
            "accept",
            "(D)V",
            vec![consumer_slot, Slot::Double(v)],
        )?;
    }
    Ok(None)
}

/// Native: `DoubleStream.anyMatch(DoublePredicate)Z`
pub(crate) fn native_double_stream_any_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(D)Z",
            vec![pred_slot, Slot::Double(v)],
        )?;
        if matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

/// Native: `DoubleStream.allMatch(DoublePredicate)Z`
pub(crate) fn native_double_stream_all_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(D)Z",
            vec![pred_slot, Slot::Double(v)],
        )?;
        if !matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `DoubleStream.noneMatch(DoublePredicate)Z`
pub(crate) fn native_double_stream_none_match(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let pred_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Ok(Some(Slot::Int(1)));
    };
    let elems = double_stream_elems(heap, r);
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    for v in elems {
        let result = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(D)Z",
            vec![pred_slot, Slot::Double(v)],
        )?;
        if matches!(result, Some(Slot::Int(1))) {
            return Ok(Some(Slot::Int(0)));
        }
    }
    Ok(Some(Slot::Int(1)))
}

/// Native: `DoubleStream.findFirst()OptionalDouble`
pub(crate) fn native_double_stream_find_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let opt_ref = make_optional_double_val(heap, elems.into_iter().next());
    Ok(Some(Slot::Reference(Some(opt_ref))))
}

/// Native: `DoubleStream.reduce(double, DoubleBinaryOperator)D`
pub(crate) fn native_double_stream_reduce_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let identity = match args.get(1).copied() {
        Some(Slot::Double(d)) => d,
        _ => 0.0,
    };
    let fn_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Double(identity)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = double_stream_elems(heap, r);
    let mut acc = identity;
    for v in elems {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(DD)D",
                vec![fn_slot, Slot::Double(acc), Slot::Double(v)],
            )?
            .unwrap_or(Slot::Double(0.0));
        acc = match result {
            Slot::Double(d) => d,
            Slot::Float(f) => f64::from(f),
            Slot::Int(n) => f64::from(n),
            _ => acc,
        };
    }
    Ok(Some(Slot::Double(acc)))
}

/// Native: `DoubleStream.reduce(DoubleBinaryOperator)OptionalDouble`
pub(crate) fn native_double_stream_reduce_optional(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        let opt_ref = make_optional_double_val(heap, None);
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = double_stream_elems(heap, r);
    if elems.is_empty() {
        return Ok(Some(Slot::Reference(Some(make_optional_double_val(
            heap, None,
        )))));
    }
    let mut acc = elems[0];
    for &v in &elems[1..] {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsDouble",
                "(DD)D",
                vec![fn_slot, Slot::Double(acc), Slot::Double(v)],
            )?
            .unwrap_or(Slot::Double(0.0));
        acc = match result {
            Slot::Double(d) => d,
            Slot::Float(f) => f64::from(f),
            Slot::Int(n) => f64::from(n),
            _ => acc,
        };
    }
    Ok(Some(Slot::Reference(Some(make_optional_double_val(
        heap,
        Some(acc),
    )))))
}

/// Native: `DoubleStream.flatMap(DoubleFunction<DoubleStream>)DoubleStream`
pub(crate) fn native_double_stream_flat_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let sub = ops.invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(D)Ljava/lang/Object;",
            vec![fn_slot, Slot::Double(v)],
        )?;
        if let Some(Slot::Reference(Some(sub_ref))) = sub {
            result.extend(double_stream_elems(heap, sub_ref));
        }
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, result,
    )))))
}

/// Native: `DoubleStream.mapToInt(DoubleToIntFunction)IntStream`
pub(crate) fn native_double_stream_map_to_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_int_stream(heap, vec![])))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(D)I",
            vec![fn_slot, Slot::Double(v)],
        )?;
        result.push(match r {
            Some(Slot::Int(n)) => n,
            _ => 0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, result)))))
}

/// Native: `DoubleStream.mapToLong(DoubleToLongFunction)LongStream`
pub(crate) fn native_double_stream_map_to_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_long_stream(heap, vec![])))));
    };
    let elems = double_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(D)J",
            vec![fn_slot, Slot::Double(v)],
        )?;
        result.push(match r {
            Some(Slot::Long(n)) => n,
            Some(Slot::Int(n)) => i64::from(n),
            _ => 0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_long_stream(heap, result)))))
}

/// Native: `DoubleStream.distinct()DoubleStream` — removes duplicate values.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_double_stream_distinct(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<f64> = elems
        .into_iter()
        .filter(|&v| seen.insert(v.to_bits()))
        .collect();
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, deduped,
    )))))
}

/// Native: `DoubleStream.boxed()Stream` — boxes each double into `java/lang/Double`.
pub(crate) fn native_double_stream_boxed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let elems = double_stream_elems(heap, r);
    let n = i32::try_from(elems.len()).unwrap_or(0);
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(n);
    for v in elems {
        let boxed_ref = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(boxed_ref)?.fields[0] = Slot::Double(v);
        heap.get_mut(stream_ref)?
            .fields
            .push(Slot::Reference(Some(boxed_ref)));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `OptionalDouble.isPresent()Z`
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_double_is_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let present = matches!(heap.get(r)?.fields.get(1), Some(Slot::Int(1)));
    Ok(Some(Slot::Int(i32::from(present))))
}

/// Native: `LongStream.reduce(LongBinaryOperator)OptionalLong`
pub(crate) fn native_long_stream_reduce_optional(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        let opt_ref = make_optional_long(heap, None);
        return Ok(Some(Slot::Reference(Some(opt_ref))));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let elems = long_stream_elems(heap, r);
    if elems.is_empty() {
        return Ok(Some(Slot::Reference(Some(make_optional_long(heap, None)))));
    }
    let mut acc = elems[0];
    for &v in &elems[1..] {
        let result = ops
            .invoke(
                heap,
                out,
                &fn_class,
                "applyAsLong",
                "(JJ)J",
                vec![fn_slot, Slot::Long(acc), Slot::Long(v)],
            )?
            .unwrap_or(Slot::Long(0));
        acc = match result {
            Slot::Long(n) => n,
            Slot::Int(n) => i64::from(n),
            _ => acc,
        };
    }
    Ok(Some(Slot::Reference(Some(make_optional_long(
        heap,
        Some(acc),
    )))))
}

/// Native: `LongStream.mapToDouble(LongToDoubleFunction)DoubleStream`
pub(crate) fn native_long_stream_map_to_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let r = extract_ref_arg(args, 0)?;
    let fn_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Reference(Some(make_double_stream(
            heap,
            vec![],
        )))));
    };
    let elems = long_stream_elems(heap, r);
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let mut result = Vec::with_capacity(elems.len());
    for v in elems {
        let r = ops.invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(J)D",
            vec![fn_slot, Slot::Long(v)],
        )?;
        result.push(match r {
            Some(Slot::Double(d)) => d,
            Some(Slot::Float(f)) => f64::from(f),
            Some(Slot::Int(n)) => f64::from(n),
            _ => 0.0,
        });
    }
    Ok(Some(Slot::Reference(Some(make_double_stream(
        heap, result,
    )))))
}

/// Native: `Collectors.summingLong(ToLongFunction)Collector` — returns a `SummingLongCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_summing_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/SummingLongCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Phase 56: Collectors.minBy/maxBy, summingDouble, averagingLong,
//           toUnmodifiableMap, collectingAndThen
// ---------------------------------------------------------------------------

/// Native: `Collectors.minBy(Comparator)Collector` — returns a `MinByCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_min_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let cmp_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/MinByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = cmp_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.maxBy(Comparator)Collector` — returns a `MaxByCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_max_by(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let cmp_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/MaxByCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = cmp_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.summingDouble(ToDoubleFunction)Collector` — returns a `SummingDoubleCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_summing_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/SummingDoubleCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.averagingLong(ToLongFunction)Collector` — returns an `AveragingLongCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_averaging_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/AveragingLongCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.toUnmodifiableMap(keyFn, valueFn)Collector` — same sentinel as `ToMapCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_to_unmodifiable_map(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let key_fn_slot = extract_slot_arg(args, 0);
    let val_fn_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ToUnmodifiableMapCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = key_fn_slot;
    heap.get_mut(r)?.fields[1] = val_fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.collectingAndThen(downstream, finisher)Collector` — returns a
/// `CollectingAndThenCollector` with `fields[0]`=downstream, `fields[1]`=finisher.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_collecting_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let downstream_slot = extract_slot_arg(args, 0);
    let finisher_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/CollectingAndThenCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = downstream_slot;
    heap.get_mut(r)?.fields[1] = finisher_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.averagingDouble(ToDoubleFunction)Collector` — returns an `AveragingDoubleCollector`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_averaging_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/AveragingDoubleCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

// ---------------------------------------------------------------------------
// Phase 57: Collectors.reducing, Stream.iterate predicate, Optional.stream,
//           ArrayDeque completion
// ---------------------------------------------------------------------------

/// Native: `Collectors.reducing(BinaryOperator)` — returns a
/// `ReducingNoIdentityCollector` with `fields[0]`=op.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_reducing_no_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let op_slot = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/ReducingNoIdentityCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = op_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.reducing(T, BinaryOperator)` — returns a
/// `ReducingCollector` with `fields[0]`=identity, `fields[1]`=op.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_reducing_with_identity(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let identity_slot = extract_slot_arg(args, 0);
    let op_slot = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ReducingCollector".to_string(), 2);
    heap.get_mut(r)?.fields[0] = identity_slot;
    heap.get_mut(r)?.fields[1] = op_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Collectors.reducing(U, Function, BinaryOperator)` — returns a
/// `ReducingMappingCollector` with `fields[0]`=identity, `fields[1]`=mapper, `fields[2]`=op.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_collectors_reducing_mapping(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let identity_slot = extract_slot_arg(args, 0);
    let mapper_slot = extract_slot_arg(args, 1);
    let op_slot = extract_slot_arg(args, 2);
    let r = heap.allocate("duke/util/ReducingMappingCollector".to_string(), 3);
    heap.get_mut(r)?.fields[0] = identity_slot;
    heap.get_mut(r)?.fields[1] = mapper_slot;
    heap.get_mut(r)?.fields[2] = op_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Stream.iterate(seed, Predicate, UnaryOperator)Stream` — Java 9 3-arg form.
/// Eagerly materialises elements while predicate returns true, capped at 10,000.
pub(crate) fn native_stream_iterate_predicate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let seed = extract_slot_arg(args, 0);
    let pred_slot = extract_slot_arg(args, 1);
    let next_slot = extract_slot_arg(args, 2);
    let Slot::Reference(Some(pred_ref)) = pred_slot else {
        return Err(Error::NullPointerException);
    };
    let Slot::Reference(Some(next_ref)) = next_slot else {
        return Err(Error::NullPointerException);
    };
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let next_class = heap.get(next_ref)?.class_name.clone();

    let mut elems: Vec<Slot> = Vec::new();
    let mut current = seed;
    for _ in 0..10_000usize {
        let test = ops.invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![pred_slot, current],
        )?;
        match test {
            Some(Slot::Int(1)) => {}
            _ => break,
        }
        elems.push(current);
        current = ops
            .invoke(
                heap,
                out,
                &next_class,
                "apply",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
                vec![next_slot, current],
            )?
            .unwrap_or(Slot::Reference(None));
    }
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    let size = i32::try_from(elems.len()).unwrap_or(i32::MAX);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(size);
    for elem in elems {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `Optional.stream()Stream` — returns a stream of 0 or 1 elements.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_optional_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = extract_first_field_arg(heap, this_ref)?;
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    match value {
        Slot::Reference(None) => {
            heap.get_mut(out_ref)?.fields[0] = Slot::Int(0);
        }
        v => {
            heap.get_mut(out_ref)?.fields[0] = Slot::Int(1);
            heap.get_mut(out_ref)?.fields.push(v);
        }
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `ArrayDeque.addFirst(Object)V` — inserts element at front.
pub(crate) fn native_arraydeque_add_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraydeque_push(args, heap, out, control)
}

/// Native: `ArrayDeque.addLast(Object)V` — appends element at back.
pub(crate) fn native_arraydeque_add_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(None)
}

/// Native: `ArrayDeque.offerFirst(Object)Z` — inserts at front, returns true.
pub(crate) fn native_arraydeque_offer_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraydeque_push(args, heap, out, control)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `ArrayDeque.offerLast(Object)Z` — appends at back, returns true.
pub(crate) fn native_arraydeque_offer_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(Slot::Int(1)))
}

/// Native: `ArrayDeque.peekFirst()Object` — same as peek (front element, null if empty).
pub(crate) fn native_arraydeque_peek_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraydeque_peek(args, heap, out, control)
}

/// Native: `ArrayDeque.peekLast()Object` — returns last element without removing; null if empty.
pub(crate) fn native_arraydeque_peek_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    Ok(Some(heap.get(this_ref)?.fields[size]))
}

/// Native: `ArrayDeque.pollFirst()Object` — same as poll (remove front, null if empty).
pub(crate) fn native_arraydeque_poll_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraydeque_poll(args, heap, out, control)
}

/// Native: `ArrayDeque.pollLast()Object` — removes and returns last element; null if empty.
pub(crate) fn native_arraydeque_poll_last(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    if size == 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    let elem = heap.get_mut(this_ref)?.fields.remove(size);
    let new_size = i32::try_from(size - 1).unwrap_or(0);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(new_size);
    Ok(Some(elem))
}

/// Native: `ArrayDeque.contains(Object)Z` — returns true if element is present.
pub(crate) fn native_arraydeque_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let found = elems.iter().any(|e| slots_equal(e, &target, heap));
    Ok(Some(Slot::Int(i32::from(found))))
}

/// Native: `ArrayDeque.stream()Stream` — returns elements as an eager Stream.
pub(crate) fn native_arraydeque_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    let out_ref = heap.allocate("duke/util/Stream".to_string(), 1);
    heap.get_mut(out_ref)?.fields[0] = Slot::Int(i32::try_from(size).unwrap_or(0));
    for elem in elems {
        heap.get_mut(out_ref)?.fields.push(elem);
    }
    Ok(Some(Slot::Reference(Some(out_ref))))
}

/// Native: `ArrayDeque.forEach(Consumer)V` — invokes consumer for each element.
pub(crate) fn native_arraydeque_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let consumer_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(cons_ref)) = consumer_slot else {
        return Err(Error::NullPointerException);
    };
    let cons_class = heap.get(cons_ref)?.class_name.clone();
    let size = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let elems: Vec<Slot> = heap.get(this_ref)?.fields[1..=size].to_vec();
    for elem in elems {
        ops.invoke(
            heap,
            out,
            &cons_class,
            "accept",
            "(Ljava/lang/Object;)V",
            vec![consumer_slot, elem],
        )?;
    }
    Ok(None)
}

/// Native: `ArrayDeque.clear()V` — removes all elements.
pub(crate) fn native_arraydeque_clear(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields.truncate(1);
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

// ---------------------------------------------------------------------------
// Phase 62: java.time (LocalDate, LocalDateTime, Instant, Duration, Period)
// ---------------------------------------------------------------------------

/// Convert (year, month, day) to a proleptic Gregorian epoch day count.
/// Day 0 = 1970-01-01.  Howard Hinnant's branchless algorithm.
#[allow(
    clippy::cast_lossless,         // i32/u32 → i64 widening casts
    clippy::cast_possible_truncation, // result fits i32 for any valid Gregorian date
    clippy::missing_const_for_fn   // i64::from not const-stable yet
)]
fn ymd_to_epoch_days(year: i32, month: u32, day: u32) -> i32 {
    let (y, m, d) = (year as i64, month as i64, day as i64);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400); // year of era [0, 399]
    let day_of_year = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + day_of_year; // day of era [0, 146096]
    (era * 146_097 + doe - 719_468) as i32
}

/// Convert a proleptic Gregorian epoch day to (year, month, day).
#[allow(
    clippy::cast_lossless,            // i32 → i64 widening cast
    clippy::cast_possible_truncation, // y fits i32 for valid dates
    clippy::cast_sign_loss,           // m/d are [1,12]/[1,31], sign-safe u32
    clippy::missing_const_for_fn      // i64::from not const-stable yet
)]
fn epoch_days_to_ymd(epoch_days: i32) -> (i32, u32, u32) {
    let z = epoch_days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097); // [0, 146096]
    let yoe = (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let day_of_year = day_of_era - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * day_of_year + 2) / 153; // [0, 11]
    let d = day_of_year - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

// ---- LocalDate layout: fields[0] = Slot::Int(epoch_days) ----

/// Native: `LocalDate.of(int, int, int) -> LocalDate`
#[allow(clippy::cast_sign_loss)] // month/day from Java int are always positive
pub(crate) fn native_localdate_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let year = extract_int_arg(args, 0)?;
    let month = extract_int_arg(args, 1)? as u32;
    let day = extract_int_arg(args, 2)? as u32;
    let epoch = ymd_to_epoch_days(year, month, day);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, epoch)?))))
}

/// Native: `LocalDate.now() -> LocalDate` — returns 1970-01-01 (epoch 0) in this interpreter.
pub(crate) fn native_localdate_now(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("java/time/LocalDate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(0); // epoch 0 = 1970-01-01
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDate.getYear() -> int`
pub(crate) fn native_localdate_get_year(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (year, _, _) = epoch_days_to_ymd(epoch);
    Ok(Some(Slot::Int(year)))
}

/// Native: `LocalDate.getMonthValue() -> int`
pub(crate) fn native_localdate_get_month_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (_, month, _) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_wrap)] // month is [1,12], fits i32
    Ok(Some(Slot::Int(month as i32)))
}

/// Native: `LocalDate.getDayOfMonth() -> int`
pub(crate) fn native_localdate_get_day_of_month(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (_, _, day) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_wrap)] // day is [1,31], fits i32
    Ok(Some(Slot::Int(day as i32)))
}

/// Native: `LocalDate.plusDays(long) -> LocalDate`
pub(crate) fn native_localdate_plus_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let days = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    #[allow(clippy::cast_possible_truncation)] // saturating_add handles out-of-range
    let new_epoch = epoch.saturating_add(days as i32);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, new_epoch)?))))
}

/// Native: `LocalDate.minusDays(long) -> LocalDate`
pub(crate) fn native_localdate_minus_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let days = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    #[allow(clippy::cast_possible_truncation)] // saturating_sub handles out-of-range
    let new_epoch = epoch.saturating_sub(days as i32);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, new_epoch)?))))
}

/// Native: `LocalDate.plusMonths(long) -> LocalDate`
pub(crate) fn native_localdate_plus_months(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let months = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (y, m, d) = epoch_days_to_ymd(epoch);
    let (ny, nm, nd) = shift_year_month_day(y, m, d, months);
    let new_epoch = ymd_to_epoch_days(ny, nm, nd);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, new_epoch)?))))
}

/// Native: `LocalDate.plusYears(long) -> LocalDate`
pub(crate) fn native_localdate_plus_years(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let years = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (y, m, d) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_truncation)] // year range is reasonable for Java dates
    let ny = y + years as i32;
    let max_day = days_in_month(ny, m);
    let nd = d.min(max_day);
    let new_epoch = ymd_to_epoch_days(ny, m, nd);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, new_epoch)?))))
}

/// Native: `LocalDate.isBefore(LocalDate) -> boolean`
pub(crate) fn native_localdate_is_before(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(a < b))))
}

/// Native: `LocalDate.isAfter(LocalDate) -> boolean`
pub(crate) fn native_localdate_is_after(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(a > b))))
}

/// Native: `LocalDate.isEqual(LocalDate) -> boolean`
pub(crate) fn native_localdate_is_equal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(a == b))))
}

/// Native: `LocalDate.toEpochDay() -> long`
pub(crate) fn native_localdate_to_epoch_day(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(i64::from(epoch))))
}

/// Native: `LocalDate.toString() -> String`
pub(crate) fn native_localdate_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (y, m, d) = epoch_days_to_ymd(epoch);
    let s = format!("{y:04}-{m:02}-{d:02}");
    let sr = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(sr))))
}

/// Native: `LocalDate.minusMonths(long) -> LocalDate`
pub(crate) fn native_localdate_minus_months(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let months = extract_long_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (year, month, day) = epoch_days_to_ymd(epoch);
    let (new_year, new_month, new_day) = shift_year_month_day(year, month, day, -months);
    let new_epoch = ymd_to_epoch_days(new_year, new_month, new_day);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, new_epoch)?))))
}

/// Native: `LocalDate.withYear(int) -> LocalDate`
pub(crate) fn native_localdate_with_year(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let year = extract_int_arg(args, 1)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (_, month, day) = epoch_days_to_ymd(epoch);
    let new_day = day.min(days_in_month(year, month));
    let new_epoch = ymd_to_epoch_days(year, month, new_day);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, new_epoch)?))))
}

/// Native: `LocalDate.parse(CharSequence) -> LocalDate`
pub(crate) fn native_localdate_parse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let text = extract_string_arg_value(args, 0, heap)?;
    let (year, month, day) =
        parse_iso_local_date_components(&text).ok_or_else(|| date_time_parse_error(&text))?;
    let epoch = ymd_to_epoch_days(year, month, day);
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, epoch)?))))
}

/// Native: `LocalDate.parse(CharSequence, DateTimeFormatter) -> LocalDate`
pub(crate) fn native_localdate_parse_with_formatter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 1)?;
    native_localdate_parse(args, heap, out, control)
}

fn localdate_compare_impl(heap: &duke_gc::Heap, this_ref: u64, other_ref: u64) -> Result<i32> {
    let other = heap.get(other_ref)?;
    if other.class_name != "java/time/LocalDate" {
        return Err(class_cast_error());
    }
    let this_epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let other_epoch = match other.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(ordering_to_int(this_epoch.cmp(&other_epoch)))
}

/// Native: `LocalDate.compareTo(ChronoLocalDate) -> int`
pub(crate) fn native_localdate_compare_to(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    Ok(Some(Slot::Int(localdate_compare_impl(heap, this_ref, other_ref)?)))
}

/// Native: bridge `LocalDate.compareTo(Object) -> int`
pub(crate) fn native_localdate_compare_to_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_localdate_compare_to(args, heap, out, control)
}

/// Native: `LocalDate.equals(Object) -> boolean`
pub(crate) fn native_localdate_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    let other = heap.get(other_ref)?;
    let equal = other.class_name == "java/time/LocalDate"
        && other.fields.first() == heap.get(this_ref)?.fields.first();
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `LocalDate.hashCode() -> int`
pub(crate) fn native_localdate_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(epoch)))
}

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

const NANOS_PER_SECOND_I128: i128 = 1_000_000_000;
const NANOS_PER_MILLI_I128: i128 = 1_000_000;
const SECONDS_PER_DAY_I64: i64 = 86_400;

fn clamp_i128_to_i64(value: i128) -> i64 {
    match i64::try_from(value) {
        Ok(value) => value,
        Err(_) if value.is_negative() => i64::MIN,
        Err(_) => i64::MAX,
    }
}

fn clamp_i64_to_i32(value: i64) -> i32 {
    match i32::try_from(value) {
        Ok(value) => value,
        Err(_) if value.is_negative() => i32::MIN,
        Err(_) => i32::MAX,
    }
}

fn normalize_seconds_nanos(total_nanos: i128) -> (i64, i32) {
    let seconds = clamp_i128_to_i64(total_nanos.div_euclid(NANOS_PER_SECOND_I128));
    let nanos = i32::try_from(total_nanos.rem_euclid(NANOS_PER_SECOND_I128)).unwrap_or(0);
    (seconds, nanos)
}

fn shift_year_month_day(year: i32, month: u32, day: u32, delta_months: i64) -> (i32, u32, u32) {
    let total_months = i64::from(year) * 12 + (i64::from(month) - 1) + delta_months;
    let new_year = clamp_i64_to_i32(total_months.div_euclid(12));
    let new_month = u32::try_from(total_months.rem_euclid(12) + 1).unwrap_or(1);
    let new_day = day.min(days_in_month(new_year, new_month));
    (new_year, new_month, new_day)
}

fn parse_fraction_to_nanos(fraction: &str) -> Option<i32> {
    if fraction.is_empty() || fraction.len() > 9 || !fraction.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let digits: i32 = fraction.parse().ok()?;
    let scale = 10_i32.pow(u32::try_from(9 - fraction.len()).ok()?);
    Some(digits.saturating_mul(scale))
}

fn parse_iso_local_date_components(text: &str) -> Option<(i32, u32, u32)> {
    let mut parts = text.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some()
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
    {
        return None;
    }
    Some((year, month, day))
}

fn parse_iso_time_components(text: &str) -> Option<(i32, i32, i32, i32)> {
    let (clock_part, nanos) = match text.split_once('.') {
        Some((clock, fraction)) => (clock, parse_fraction_to_nanos(fraction)?),
        None => (text, 0),
    };
    let mut parts = clock_part.split(':');
    let hour: i32 = parts.next()?.parse().ok()?;
    let minute: i32 = parts.next()?.parse().ok()?;
    let second: i32 = parts.next()?.parse().ok()?;
    if parts.next().is_some()
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }
    Some((hour, minute, second, nanos))
}

fn parse_iso_localdatetime_components(text: &str) -> Option<(i32, u32, u32, i32, i32, i32, i32)> {
    let (date_part, time_part) = text.split_once('T')?;
    let (year, month, day) = parse_iso_local_date_components(date_part)?;
    let (hour, minute, second, nanos) = parse_iso_time_components(time_part)?;
    Some((year, month, day, hour, minute, second, nanos))
}

fn parse_iso_instant_components(text: &str) -> Option<(i64, i32)> {
    let body = text.strip_suffix('Z')?;
    let (year, month, day, hour, minute, second, nanos) = parse_iso_localdatetime_components(body)?;
    let epoch_day = i64::from(ymd_to_epoch_days(year, month, day));
    let epoch_seconds =
        epoch_day * SECONDS_PER_DAY_I64 + i64::from(hour * 3600 + minute * 60 + second);
    Some((epoch_seconds, nanos))
}

fn epoch_seconds_to_datetime_parts(epoch_seconds: i64) -> (i32, u32, u32, i32, i32, i32) {
    let epoch_days = clamp_i64_to_i32(epoch_seconds.div_euclid(SECONDS_PER_DAY_I64));
    let second_of_day = epoch_seconds.rem_euclid(SECONDS_PER_DAY_I64);
    let hour = i32::try_from(second_of_day / 3600).unwrap_or(0);
    let minute = i32::try_from((second_of_day % 3600) / 60).unwrap_or(0);
    let second = i32::try_from(second_of_day % 60).unwrap_or(0);
    let (year, month, day) = epoch_days_to_ymd(epoch_days);
    (year, month, day, hour, minute, second)
}

fn date_time_parse_error(input: &str) -> Error {
    push_pending_java_exception_message(
        "java/time/format/DateTimeParseException",
        format!("Text '{input}' could not be parsed"),
    );
    Error::JavaException {
        class_name: "java/time/format/DateTimeParseException".to_string(),
    }
}

fn class_cast_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/ClassCastException".to_string(),
    }
}

fn duration_parts_from_ref(heap: &duke_gc::Heap, duration_ref: u64) -> Result<(i64, i32)> {
    let obj = heap.get(duration_ref)?;
    let seconds = match obj.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let nanos = match obj.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok((seconds, nanos))
}

fn duration_total_nanos_from_ref(heap: &duke_gc::Heap, duration_ref: u64) -> Result<i128> {
    let (seconds, nanos) = duration_parts_from_ref(heap, duration_ref)?;
    Ok(i128::from(seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos))
}

fn allocate_duration_from_total_nanos(heap: &mut duke_gc::Heap, total_nanos: i128) -> Result<u64> {
    let (seconds, nanos) = normalize_seconds_nanos(total_nanos);
    let duration_ref = heap.allocate("java/time/Duration".to_string(), 2);
    heap.get_mut(duration_ref)?.fields[0] = Slot::Long(seconds);
    heap.get_mut(duration_ref)?.fields[1] = Slot::Int(nanos);
    Ok(duration_ref)
}

fn instant_parts_from_ref(heap: &duke_gc::Heap, instant_ref: u64) -> Result<(i64, i32)> {
    let obj = heap.get(instant_ref)?;
    let seconds = match obj.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let nanos = match obj.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok((seconds, nanos))
}

fn instant_total_nanos_from_ref(heap: &duke_gc::Heap, instant_ref: u64) -> Result<i128> {
    let (seconds, nanos) = instant_parts_from_ref(heap, instant_ref)?;
    Ok(i128::from(seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos))
}

fn allocate_instant_from_total_nanos(heap: &mut duke_gc::Heap, total_nanos: i128) -> Result<u64> {
    let (seconds, nanos) = normalize_seconds_nanos(total_nanos);
    let instant_ref = heap.allocate("java/time/Instant".to_string(), 2);
    heap.get_mut(instant_ref)?.fields[0] = Slot::Long(seconds);
    heap.get_mut(instant_ref)?.fields[1] = Slot::Int(nanos);
    Ok(instant_ref)
}

fn localdatetime_components_from_ref(
    heap: &duke_gc::Heap,
    localdatetime_ref: u64,
) -> Result<(i32, i32, i32, i32, i32)> {
    let obj = heap.get(localdatetime_ref)?;
    let epoch_day = match obj.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let hour = match obj.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let minute = match obj.fields.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let second = match obj.fields.get(3) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let nanos = match obj.fields.get(4) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok((epoch_day, hour, minute, second, nanos))
}

fn localdatetime_total_nanos_from_ref(
    heap: &duke_gc::Heap,
    localdatetime_ref: u64,
) -> Result<i128> {
    let (epoch_day, hour, minute, second, nanos) =
        localdatetime_components_from_ref(heap, localdatetime_ref)?;
    let epoch_seconds =
        i64::from(epoch_day) * SECONDS_PER_DAY_I64 + i64::from(hour * 3600 + minute * 60 + second);
    Ok(i128::from(epoch_seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos))
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

// ---- Duration layout: fields[0]=Slot::Long(seconds), fields[1]=Slot::Int(nanos_adj) ----

/// Native: `Duration.ofSeconds(long) -> Duration`
pub(crate) fn native_duration_of_seconds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let secs = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        i128::from(secs) * NANOS_PER_SECOND_I128,
    )?))))
}

/// Native: `Duration.ofMinutes(long) -> Duration`
pub(crate) fn native_duration_of_minutes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let mins = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        i128::from(mins) * 60 * NANOS_PER_SECOND_I128,
    )?))))
}

/// Native: `Duration.ofHours(long) -> Duration`
pub(crate) fn native_duration_of_hours(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let hrs = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        i128::from(hrs) * 3600 * NANOS_PER_SECOND_I128,
    )?))))
}

/// Native: `Duration.ofDays(long) -> Duration`
pub(crate) fn native_duration_of_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let days = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        i128::from(days) * i128::from(SECONDS_PER_DAY_I64) * NANOS_PER_SECOND_I128,
    )?))))
}

/// Native: `Duration.getSeconds() -> long`
pub(crate) fn native_duration_get_seconds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs)))
}

/// Native: `Duration.toSeconds() -> long`
pub(crate) fn native_duration_to_seconds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_duration_get_seconds(args, heap, out, control)
}

/// Native: `Duration.toMinutes() -> long`
pub(crate) fn native_duration_to_minutes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs / 60)))
}

/// Native: `Duration.toHours() -> long`
pub(crate) fn native_duration_to_hours(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs / 3600)))
}

/// Native: `Duration.toDays() -> long`
pub(crate) fn native_duration_to_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs / 86_400)))
}

/// Native: `Duration.plus(Duration) -> Duration`
pub(crate) fn native_duration_plus(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let total = duration_total_nanos_from_ref(heap, this_ref)?
        + duration_total_nanos_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Duration.minus(Duration) -> Duration`
pub(crate) fn native_duration_minus(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let total = duration_total_nanos_from_ref(heap, this_ref)?
        - duration_total_nanos_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Duration.isNegative() -> boolean`
pub(crate) fn native_duration_is_negative(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(secs < 0))))
}

/// Native: `Duration.isZero() -> boolean`
pub(crate) fn native_duration_is_zero(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let nano = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(secs == 0 && nano == 0))))
}

/// Native: `Duration.ofMillis(long) -> Duration`
pub(crate) fn native_duration_of_millis(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let millis = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        i128::from(millis) * NANOS_PER_MILLI_I128,
    )?))))
}

/// Native: `Duration.ofNanos(long) -> Duration`
pub(crate) fn native_duration_of_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let nanos = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        i128::from(nanos),
    )?))))
}

/// Native: `Duration.between(start, end) -> Duration`
pub(crate) fn native_duration_between(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let start_ref = extract_ref_arg(args, 0)?;
    let end_ref = extract_ref_arg(args, 1)?;
    let start = heap.get(start_ref)?;
    let total_nanos = match start.class_name.as_str() {
        "java/time/Instant" => {
            instant_total_nanos_from_ref(heap, end_ref)? - instant_total_nanos_from_ref(heap, start_ref)?
        }
        "java/time/LocalDateTime" => {
            localdatetime_total_nanos_from_ref(heap, end_ref)?
                - localdatetime_total_nanos_from_ref(heap, start_ref)?
        }
        "java/time/LocalDate" => {
            let start_epoch = match start.fields.first() {
                Some(Slot::Int(v)) => *v,
                _ => 0,
            };
            let end_epoch = match heap.get(end_ref)?.fields.first() {
                Some(Slot::Int(v)) => *v,
                _ => 0,
            };
            i128::from(end_epoch - start_epoch)
                * i128::from(SECONDS_PER_DAY_I64)
                * NANOS_PER_SECOND_I128
        }
        _ => return Err(class_cast_error()),
    };
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap, total_nanos,
    )?))))
}

/// Native: `Duration.toMillis() -> long`
pub(crate) fn native_duration_to_millis(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let millis = clamp_i128_to_i64(duration_total_nanos_from_ref(heap, this_ref)? / NANOS_PER_MILLI_I128);
    Ok(Some(Slot::Long(millis)))
}

/// Native: `Duration.toNanos() -> long`
pub(crate) fn native_duration_to_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Long(clamp_i128_to_i64(duration_total_nanos_from_ref(
        heap, this_ref,
    )?))))
}

/// Native: `Duration.negated() -> Duration`
pub(crate) fn native_duration_negated(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_duration_from_total_nanos(
        heap,
        -duration_total_nanos_from_ref(heap, this_ref)?,
    )?))))
}

fn duration_compare_impl(heap: &duke_gc::Heap, this_ref: u64, other_ref: u64) -> Result<i32> {
    let other = heap.get(other_ref)?;
    if other.class_name != "java/time/Duration" {
        return Err(class_cast_error());
    }
    let this_total = duration_total_nanos_from_ref(heap, this_ref)?;
    let other_total = duration_total_nanos_from_ref(heap, other_ref)?;
    Ok(ordering_to_int(this_total.cmp(&other_total)))
}

/// Native: `Duration.compareTo(Duration) -> int`
pub(crate) fn native_duration_compare_to(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    Ok(Some(Slot::Int(duration_compare_impl(heap, this_ref, other_ref)?)))
}

/// Native: bridge `Duration.compareTo(Object) -> int`
pub(crate) fn native_duration_compare_to_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_duration_compare_to(args, heap, out, control)
}

/// Native: `Duration.equals(Object) -> boolean`
pub(crate) fn native_duration_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    let other = heap.get(other_ref)?;
    let equal = other.class_name == "java/time/Duration"
        && duration_total_nanos_from_ref(heap, this_ref)?
            == duration_total_nanos_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `Duration.hashCode() -> int`
pub(crate) fn native_duration_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (seconds, nanos) = duration_parts_from_ref(heap, this_ref)?;
    let folded = seconds ^ (seconds >> 32);
    #[allow(clippy::cast_possible_truncation)]
    Ok(Some(Slot::Int((folded as i32) ^ nanos)))
}

// ---- Period layout: fields[0]=years(Int), fields[1]=months(Int), fields[2]=days(Int) ----

/// Native: `Period.of(int, int, int) -> Period`
pub(crate) fn native_period_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let years = extract_int_arg(args, 0)?;
    let months = extract_int_arg(args, 1)?;
    let days = extract_int_arg(args, 2)?;
    let r = heap.allocate("java/time/Period".to_string(), 3);
    heap.get_mut(r)?.fields[0] = Slot::Int(years);
    heap.get_mut(r)?.fields[1] = Slot::Int(months);
    heap.get_mut(r)?.fields[2] = Slot::Int(days);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Period.ofDays(int) -> Period`
pub(crate) fn native_period_of_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let days = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/time/Period".to_string(), 3);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    heap.get_mut(r)?.fields[1] = Slot::Int(0);
    heap.get_mut(r)?.fields[2] = Slot::Int(days);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Period.ofMonths(int) -> Period`
pub(crate) fn native_period_of_months(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let months = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/time/Period".to_string(), 3);
    heap.get_mut(r)?.fields[0] = Slot::Int(0);
    heap.get_mut(r)?.fields[1] = Slot::Int(months);
    heap.get_mut(r)?.fields[2] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Period.ofYears(int) -> Period`
pub(crate) fn native_period_of_years(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let years = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/time/Period".to_string(), 3);
    heap.get_mut(r)?.fields[0] = Slot::Int(years);
    heap.get_mut(r)?.fields[1] = Slot::Int(0);
    heap.get_mut(r)?.fields[2] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Period.getYears() -> int`
pub(crate) fn native_period_get_years(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}

/// Native: `Period.getMonths() -> int`
pub(crate) fn native_period_get_months(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}

/// Native: `Period.getDays() -> int`
pub(crate) fn native_period_get_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}

/// Native: `Period.isNegative() -> boolean`
pub(crate) fn native_period_is_negative(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let neg = heap.get(this_ref)?.fields.iter().any(|s| matches!(s, Slot::Int(v) if *v < 0));
    Ok(Some(Slot::Int(i32::from(neg))))
}

/// Native: `Period.isZero() -> boolean`
pub(crate) fn native_period_is_zero(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let zero = heap.get(this_ref)?.fields.iter().all(|s| matches!(s, Slot::Int(0)));
    Ok(Some(Slot::Int(i32::from(zero))))
}

// ---- Instant layout: fields[0]=Slot::Long(epoch_seconds), fields[1]=Slot::Int(nanos_adj) ----

/// Native: `Instant.ofEpochSecond(long) -> Instant`
pub(crate) fn native_instant_of_epoch_second(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let secs = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap,
        i128::from(secs) * NANOS_PER_SECOND_I128,
    )?))))
}

/// Native: `Instant.ofEpochMilli(long) -> Instant`
pub(crate) fn native_instant_of_epoch_milli(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let millis = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap,
        i128::from(millis) * NANOS_PER_MILLI_I128,
    )?))))
}

/// Native: `Instant.getEpochSecond() -> long`
pub(crate) fn native_instant_get_epoch_second(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs)))
}

/// Native: `Instant.toEpochMilli() -> long`
pub(crate) fn native_instant_to_epoch_milli(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let secs = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let nanos = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(secs * 1000 + i64::from(nanos) / 1_000_000)))
}

/// Native: `Instant.isBefore(Instant) -> boolean`
pub(crate) fn native_instant_is_before(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let this_parts = instant_parts_from_ref(heap, this_ref)?;
    let other_parts = instant_parts_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Int(i32::from(this_parts < other_parts))))
}

/// Native: `Instant.isAfter(Instant) -> boolean`
pub(crate) fn native_instant_is_after(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let this_parts = instant_parts_from_ref(heap, this_ref)?;
    let other_parts = instant_parts_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Int(i32::from(this_parts > other_parts))))
}

/// Native: `Instant.ofEpochSecond(long, long) -> Instant`
pub(crate) fn native_instant_of_epoch_second_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let seconds = extract_long_arg(args, 0)?;
    let nanos_adjustment = extract_long_arg(args, 1)?;
    let total = i128::from(seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos_adjustment);
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Instant.getNano() -> int`
pub(crate) fn native_instant_get_nano(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let nanos = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(nanos)))
}

/// Native: `Instant.now() -> Instant`
pub(crate) fn native_instant_now(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let millis = system_time_to_epoch_millis(std::time::SystemTime::now());
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap,
        i128::from(millis) * NANOS_PER_MILLI_I128,
    )?))))
}

/// Native: `Instant.plusSeconds(long) -> Instant`
pub(crate) fn native_instant_plus_seconds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let seconds = extract_long_arg(args, 1)?;
    let total = instant_total_nanos_from_ref(heap, this_ref)?
        + i128::from(seconds) * NANOS_PER_SECOND_I128;
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Instant.plusNanos(long) -> Instant`
pub(crate) fn native_instant_plus_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let nanos = extract_long_arg(args, 1)?;
    let total = instant_total_nanos_from_ref(heap, this_ref)? + i128::from(nanos);
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Instant.minusMillis(long) -> Instant`
pub(crate) fn native_instant_minus_millis(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let millis = extract_long_arg(args, 1)?;
    let total = instant_total_nanos_from_ref(heap, this_ref)?
        - i128::from(millis) * NANOS_PER_MILLI_I128;
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Instant.parse(CharSequence) -> Instant`
pub(crate) fn native_instant_parse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let text = extract_string_arg_value(args, 0, heap)?;
    let (seconds, nanos) =
        parse_iso_instant_components(&text).ok_or_else(|| date_time_parse_error(&text))?;
    let total = i128::from(seconds) * NANOS_PER_SECOND_I128 + i128::from(nanos);
    Ok(Some(Slot::Reference(Some(allocate_instant_from_total_nanos(
        heap, total,
    )?))))
}

/// Native: `Instant.toString() -> String`
pub(crate) fn native_instant_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (seconds, nanos) = instant_parts_from_ref(heap, this_ref)?;
    let (year, month, day, hour, minute, second) = epoch_seconds_to_datetime_parts(seconds);
    let mut text = format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}");
    if nanos != 0 {
        let mut fraction = format!("{nanos:09}");
        while fraction.ends_with('0') {
            fraction.pop();
        }
        text.push('.');
        text.push_str(&fraction);
    }
    text.push('Z');
    let text_ref = heap.allocate_string(text);
    Ok(Some(Slot::Reference(Some(text_ref))))
}

fn instant_compare_impl(heap: &duke_gc::Heap, this_ref: u64, other_ref: u64) -> Result<i32> {
    let other = heap.get(other_ref)?;
    if other.class_name != "java/time/Instant" {
        return Err(class_cast_error());
    }
    let this_parts = instant_parts_from_ref(heap, this_ref)?;
    let other_parts = instant_parts_from_ref(heap, other_ref)?;
    Ok(ordering_to_int(this_parts.cmp(&other_parts)))
}

/// Native: `Instant.compareTo(Instant) -> int`
pub(crate) fn native_instant_compare_to(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    Ok(Some(Slot::Int(instant_compare_impl(heap, this_ref, other_ref)?)))
}

/// Native: bridge `Instant.compareTo(Object) -> int`
pub(crate) fn native_instant_compare_to_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_instant_compare_to(args, heap, out, control)
}

/// Native: `Instant.equals(Object) -> boolean`
pub(crate) fn native_instant_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    let other = heap.get(other_ref)?;
    let equal = other.class_name == "java/time/Instant"
        && instant_parts_from_ref(heap, this_ref)? == instant_parts_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `Instant.hashCode() -> int`
pub(crate) fn native_instant_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (seconds, nanos) = instant_parts_from_ref(heap, this_ref)?;
    let folded = seconds ^ (seconds >> 32);
    #[allow(clippy::cast_possible_truncation)]
    Ok(Some(Slot::Int((folded as i32) ^ nanos)))
}

// ---------------------------------------------------------------------------
// Phase 63: java.time.LocalDateTime
// Layout: fields[0]=epoch_days(Int), fields[1]=hour(Int),
//         fields[2]=minute(Int), fields[3]=second(Int), fields[4]=nano(Int)
// ---------------------------------------------------------------------------

/// Native: `LocalDateTime.of(int,int,int,int,int) -> LocalDateTime`
#[allow(clippy::cast_sign_loss)] // month/day from Java int are always positive
pub(crate) fn native_localdatetime_of_ymd_hm(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let year = extract_int_arg(args, 0)?;
    let month = extract_int_arg(args, 1)? as u32;
    let day = extract_int_arg(args, 2)? as u32;
    let hour = extract_int_arg(args, 3)?;
    let minute = extract_int_arg(args, 4)?;
    let epoch = ymd_to_epoch_days(year, month, day);
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap, epoch, hour, minute, 0, 0,
    )?))))
}

/// Native: `LocalDateTime.of(int,int,int,int,int,int) -> LocalDateTime`
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_localdatetime_of_ymd_hms(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let year = extract_int_arg(args, 0)?;
    let month = extract_int_arg(args, 1)? as u32;
    let day = extract_int_arg(args, 2)? as u32;
    let hour = extract_int_arg(args, 3)?;
    let minute = extract_int_arg(args, 4)?;
    let second = extract_int_arg(args, 5)?;
    let epoch = ymd_to_epoch_days(year, month, day);
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap, epoch, hour, minute, second, 0,
    )?))))
}

/// Native: `LocalDateTime.of(LocalDate, int, int, int) -> LocalDateTime`
/// Synthetic overload: accepts `LocalDate` ref + hour/minute/second as ints.
pub(crate) fn native_localdatetime_of_date_hms(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let date_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(date_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let hour = extract_int_arg(args, 1)?;
    let minute = extract_int_arg(args, 2)?;
    let second = extract_int_arg(args, 3)?;
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap, epoch, hour, minute, second, 0,
    )?))))
}

/// Native: `LocalDateTime.now() -> LocalDateTime` — returns 1970-01-01T00:00:00 in interpreter.
pub(crate) fn native_localdatetime_now(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    for i in 0..5 {
        heap.get_mut(r)?.fields[i] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `LocalDateTime.getYear() -> int`
pub(crate) fn native_localdatetime_get_year(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (year, _, _) = epoch_days_to_ymd(epoch);
    Ok(Some(Slot::Int(year)))
}

/// Native: `LocalDateTime.getMonthValue() -> int`
pub(crate) fn native_localdatetime_get_month_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (_, month, _) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_wrap)] // month is [1,12]
    Ok(Some(Slot::Int(month as i32)))
}

/// Native: `LocalDateTime.getDayOfMonth() -> int`
pub(crate) fn native_localdatetime_get_day_of_month(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (_, _, day) = epoch_days_to_ymd(epoch);
    #[allow(clippy::cast_possible_wrap)] // day is [1,31]
    Ok(Some(Slot::Int(day as i32)))
}

/// Native: `LocalDateTime.getHour() -> int`
pub(crate) fn native_localdatetime_get_hour(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}

/// Native: `LocalDateTime.getMinute() -> int`
pub(crate) fn native_localdatetime_get_minute(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}

/// Native: `LocalDateTime.getSecond() -> int`
pub(crate) fn native_localdatetime_get_second(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let v = match heap.get(this_ref)?.fields.get(3) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(v)))
}

/// Native: `LocalDateTime.toLocalDate() -> LocalDate`
pub(crate) fn native_localdatetime_to_local_date(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Reference(Some(allocate_localdate(heap, epoch)?))))
}

/// Native: `LocalDateTime.isBefore(LocalDateTime) -> boolean`
pub(crate) fn native_localdatetime_is_before(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a_epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b_epoch = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    if a_epoch != b_epoch {
        return Ok(Some(Slot::Int(i32::from(a_epoch < b_epoch))));
    }
    // same day — compare time fields
    for idx in 1..=4 {
        let a = match heap.get(this_ref)?.fields.get(idx) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        let b = match heap.get(other_ref)?.fields.get(idx) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        if a != b {
            return Ok(Some(Slot::Int(i32::from(a < b))));
        }
    }
    Ok(Some(Slot::Int(0))) // equal
}

/// Native: `LocalDateTime.isAfter(LocalDateTime) -> boolean`
pub(crate) fn native_localdatetime_is_after(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let a_epoch = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b_epoch = match heap.get(other_ref)?.fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    if a_epoch != b_epoch {
        return Ok(Some(Slot::Int(i32::from(a_epoch > b_epoch))));
    }
    for idx in 1..=4 {
        let a = match heap.get(this_ref)?.fields.get(idx) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        let b = match heap.get(other_ref)?.fields.get(idx) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        if a != b {
            return Ok(Some(Slot::Int(i32::from(a > b))));
        }
    }
    Ok(Some(Slot::Int(0))) // equal
}

/// Native: `LocalDateTime.toString() -> String` — ISO-8601 format
pub(crate) fn native_localdatetime_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let epoch = match fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let hour = match fields.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let min = match fields.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let sec = match fields.get(3) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let nanos = match fields.get(4) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let (y, m, d) = epoch_days_to_ymd(epoch);
    let mut s = format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}");
    if nanos != 0 {
        let mut fraction = format!("{nanos:09}");
        while fraction.ends_with('0') {
            fraction.pop();
        }
        s.push('.');
        s.push_str(&fraction);
    }
    let sr = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(sr))))
}

/// Native: `LocalDateTime.plusDays(long) -> LocalDateTime`
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_localdatetime_plus_days(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let days = extract_long_arg(args, 1)?;
    let fields = heap.get(this_ref)?.fields.clone();
    let epoch = match fields.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let new_epoch = epoch.saturating_add(days as i32);
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap,
        new_epoch,
        match fields.get(1) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        },
        match fields.get(2) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        },
        match fields.get(3) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        },
        match fields.get(4) {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        },
    )?))))
}

/// Native: `LocalDateTime.withHour(int) -> LocalDateTime`
pub(crate) fn native_localdatetime_with_hour(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let hour = extract_int_arg(args, 1)?;
    let f0 = heap.get(this_ref)?.fields.first().copied().unwrap_or(Slot::Int(0));
    let f1 = heap.get(this_ref)?.fields.get(1).copied().unwrap_or(Slot::Int(0));
    let f2 = heap.get(this_ref)?.fields.get(2).copied().unwrap_or(Slot::Int(0));
    let f3 = heap.get(this_ref)?.fields.get(3).copied().unwrap_or(Slot::Int(0));
    let f4 = heap.get(this_ref)?.fields.get(4).copied().unwrap_or(Slot::Int(0));
    let epoch = match f0 {
        Slot::Int(v) => v,
        _ => 0,
    };
    let minute = match f2 {
        Slot::Int(v) => v,
        _ => 0,
    };
    let second = match f3 {
        Slot::Int(v) => v,
        _ => 0,
    };
    let nanos = match f4 {
        Slot::Int(v) => v,
        _ => 0,
    };
    let _ = f1;
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap, epoch, hour, minute, second, nanos,
    )?))))
}

/// Native: `LocalDateTime.plusHours(long) -> LocalDateTime`
pub(crate) fn native_localdatetime_plus_hours(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let hours = extract_long_arg(args, 1)?;
    let (epoch_day, hour, minute, second, nanos) =
        localdatetime_components_from_ref(heap, this_ref)?;
    let total_seconds = i64::from(hour * 3600 + minute * 60 + second) + hours * 3600;
    let day_delta = total_seconds.div_euclid(SECONDS_PER_DAY_I64);
    let second_of_day = total_seconds.rem_euclid(SECONDS_PER_DAY_I64);
    let new_hour = i32::try_from(second_of_day / 3600).unwrap_or(0);
    let new_minute = i32::try_from((second_of_day % 3600) / 60).unwrap_or(0);
    let new_second = i32::try_from(second_of_day % 60).unwrap_or(0);
    #[allow(clippy::cast_possible_truncation)]
    let new_epoch_day = epoch_day.saturating_add(day_delta as i32);
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap,
        new_epoch_day,
        new_hour,
        new_minute,
        new_second,
        nanos,
    )?))))
}

/// Native: `LocalDateTime.parse(CharSequence) -> LocalDateTime`
pub(crate) fn native_localdatetime_parse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let text = extract_string_arg_value(args, 0, heap)?;
    let (year, month, day, hour, minute, second, nanos) =
        parse_iso_localdatetime_components(&text).ok_or_else(|| date_time_parse_error(&text))?;
    let epoch_day = ymd_to_epoch_days(year, month, day);
    Ok(Some(Slot::Reference(Some(allocate_localdatetime(
        heap,
        epoch_day,
        hour,
        minute,
        second,
        nanos,
    )?))))
}

/// Native: `LocalDateTime.parse(CharSequence, DateTimeFormatter) -> LocalDateTime`
pub(crate) fn native_localdatetime_parse_with_formatter(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 1)?;
    native_localdatetime_parse(args, heap, out, control)
}

fn localdatetime_compare_impl(heap: &duke_gc::Heap, this_ref: u64, other_ref: u64) -> Result<i32> {
    let other = heap.get(other_ref)?;
    if other.class_name != "java/time/LocalDateTime" {
        return Err(class_cast_error());
    }
    let this_parts = localdatetime_components_from_ref(heap, this_ref)?;
    let other_parts = localdatetime_components_from_ref(heap, other_ref)?;
    Ok(ordering_to_int(this_parts.cmp(&other_parts)))
}

/// Native: `LocalDateTime.compareTo(ChronoLocalDateTime) -> int`
pub(crate) fn native_localdatetime_compare_to(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    Ok(Some(Slot::Int(localdatetime_compare_impl(
        heap, this_ref, other_ref,
    )?)))
}

/// Native: bridge `LocalDateTime.compareTo(Object) -> int`
pub(crate) fn native_localdatetime_compare_to_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_localdatetime_compare_to(args, heap, out, control)
}

/// Native: `LocalDateTime.equals(Object) -> boolean`
pub(crate) fn native_localdatetime_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    let other = heap.get(other_ref)?;
    let equal = other.class_name == "java/time/LocalDateTime"
        && localdatetime_components_from_ref(heap, this_ref)?
            == localdatetime_components_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `LocalDateTime.hashCode() -> int`
pub(crate) fn native_localdatetime_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let (epoch_day, hour, minute, second, nanos) =
        localdatetime_components_from_ref(heap, this_ref)?;
    let mut hash = epoch_day;
    hash = hash.wrapping_mul(31).wrapping_add(hour);
    hash = hash.wrapping_mul(31).wrapping_add(minute);
    hash = hash.wrapping_mul(31).wrapping_add(second);
    hash = hash.wrapping_mul(31).wrapping_add(nanos);
    Ok(Some(Slot::Int(hash)))
}

// ---------------------------------------------------------------------------
// Phase 64: String.indent, StringBuilder.setCharAt, Collections.disjoint,
//           HashMap.computeIfPresent
// ---------------------------------------------------------------------------

/// Native: `String.indent(int) -> String` — prepends `n` spaces to each line.
/// Negative `n` removes up to `|n|` leading spaces per line (Java 12+ semantics).
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_string_indent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let n = extract_int_arg(args, 1)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let result: String = if n >= 0 {
        let n_usize = n as usize;
        let max_size = 1024 * 1024 * 128; // 128 MB max string size
        let num_lines = s.lines().count().max(1);
        let extra_len = n_usize.checked_mul(num_lines);

        if extra_len.is_none() || extra_len.unwrap().checked_add(s.len()).is_none_or(|l| l > max_size) {
            return Err(Error::JavaException {
                class_name: "java/lang/OutOfMemoryError".to_string(),
            });
        }

        let prefix = " ".repeat(n_usize);
        s.lines()
            .map(|line| {
                let mut out = String::with_capacity(prefix.len() + line.len() + 1);
                out.push_str(&prefix);
                out.push_str(line);
                out.push('\n');
                out
            })
            .collect()
    } else {
        let remove = n.unsigned_abs() as usize;
        s.lines()
            .map(|line| {
                let stripped = line.trim_start_matches(' ');
                let leading = line.len() - stripped.len();
                let keep = leading.saturating_sub(remove);
                let spaces = " ".repeat(keep);
                let mut out = String::with_capacity(keep + stripped.len() + 1);
                out.push_str(&spaces);
                out.push_str(stripped);
                out.push('\n');
                out
            })
            .collect()
    };
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `StringBuilder.setCharAt(int, char) -> void`
///
/// **Bolt Optimization:**
/// Eliminates a `Vec<char>` intermediate allocation by utilizing `char_indices` to map
/// character indexes to byte offsets, allowing direct, in-place `replace_range` mutations on the `String`.
pub(crate) fn native_stringbuilder_set_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = usize::try_from(extract_int_arg(args, 1)?).unwrap_or(usize::MAX);
    let ch = match args.get(2) {
        Some(Slot::Int(v)) => char::from_u32(u32::from_ne_bytes(v.to_ne_bytes())).unwrap_or('\0'),
        _ => '\0',
    };
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    if let Some((byte_offset, old_ch)) = buf.char_indices().nth(idx) {
        let mut b = [0; 4];
        buf.replace_range(byte_offset..byte_offset + old_ch.len_utf8(), ch.encode_utf8(&mut b));
    }
    Ok(None)
}

/// Native: `Collections.disjoint(Collection, Collection) -> boolean`
/// Returns true if the two collections have no elements in common.
pub(crate) fn native_collections_disjoint(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a_ref = extract_ref_arg(args, 0)?;
    let b_ref = extract_ref_arg(args, 1)?;
    // Both use ArrayList/HashSet layout: fields[0]=size, fields[1..=size]=elements
    let a_size = match heap.get(a_ref)?.fields.first() {
        Some(Slot::Int(v)) => usize::try_from(*v).unwrap_or(0),
        _ => 0,
    };
    let b_size = match heap.get(b_ref)?.fields.first() {
        Some(Slot::Int(v)) => usize::try_from(*v).unwrap_or(0),
        _ => 0,
    };
    let a_elems: Vec<Slot> = heap.get(a_ref)?.fields
        [1..=a_size.min(heap.get(a_ref)?.fields.len().saturating_sub(1))]
        .to_vec();
    let b_elems: Vec<Slot> = heap.get(b_ref)?.fields
        [1..=b_size.min(heap.get(b_ref)?.fields.len().saturating_sub(1))]
        .to_vec();
    let disjoint = a_elems
        .iter()
        .all(|a| !b_elems.iter().any(|b| slots_equal(a, b, heap)));
    Ok(Some(Slot::Int(i32::from(disjoint))))
}

/// Native: `HashMap.computeIfPresent(K, BiFunction<K,V,V>) -> V`
/// If key is present, applies the function to (key, `old_value`); replaces with result.
/// If function returns null, removes the key.
pub(crate) fn native_hashmap_compute_if_present(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    // Look up existing value.
    let existing = native_hashmap_get(&[Slot::Reference(Some(this_ref)), key], heap, out, control)?;
    let old_value = match existing {
        Some(v) if !matches!(v, Slot::Reference(None)) => v,
        _ => return Ok(Some(Slot::Reference(None))),
    };
    // Key present — invoke the remapping function.
    let fn_ref = extract_ref_arg(args, 2)?;
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let new_value = ops.invoke(
        heap,
        out,
        &fn_class,
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        vec![Slot::Reference(Some(fn_ref)), key, old_value],
    )?;
    match new_value {
        Some(v) if !matches!(v, Slot::Reference(None)) => {
            native_hashmap_put(
                &[Slot::Reference(Some(this_ref)), key, v],
                heap,
                out,
                control,
            )?;
            Ok(Some(v))
        }
        _ => {
            // null return → remove key
            native_hashmap_remove(&[Slot::Reference(Some(this_ref)), key], heap, out, control)?;
            Ok(Some(Slot::Reference(None)))
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 117: Map.remove(k,v), Collections.emptyList (immutable), Stream.concat,
//            Map.replace, Integer.sum, IntStream.mapToObj, Optional.map,
//            UnmodifiableSet
// ---------------------------------------------------------------------------

/// Native: `HashMap.remove(Object, Object) -> boolean`
/// Conditional remove: only removes if key is present AND value equals the provided value.
pub(crate) fn native_hashmap_remove_key_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let key = extract_slot_arg(args, 1);
    let expected_val = extract_slot_arg(args, 2);
    let fields = heap.get(this_ref)?.fields.clone();
    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let actual_val = fields[i + 1];
        if slots_equal(&actual_val, &expected_val, heap) {
            native_hashmap_remove(&[Slot::Reference(Some(this_ref)), key], heap, out, control)?;
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}

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
