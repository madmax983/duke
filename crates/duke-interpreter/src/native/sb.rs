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
        Some(Slot::Reference(Some(r))) => {
            heap.get(*r)?.string_value.clone().unwrap_or_default()
        }
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
        Some(Slot::Reference(Some(r))) => {
            heap.get(*r)?.string_value.clone().unwrap_or_default()
        }
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
        Some(Slot::Reference(Some(r))) => {
            heap.get(*r)?.string_value.clone().unwrap_or_default()
        }
        Some(Slot::Reference(None)) | None => "null".to_string(),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "String",
                got: "other",
            });
        }
    };
    let buf = heap.get_mut(this_ref)?.string_value.get_or_insert_with(String::new);
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
    let buf = heap.get_mut(this_ref)?.string_value.get_or_insert_with(String::new);
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
    let buf = heap.get_mut(this_ref)?.string_value.get_or_insert_with(String::new);
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
    let buf = heap.get_mut(this_ref)?.string_value.get_or_insert_with(String::new);
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
    let buf = heap.get_mut(this_ref)?.string_value.get_or_insert_with(String::new);
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
    let buf = heap.get_mut(this_ref)?.string_value.get_or_insert_with(String::new);
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
    let len = heap.get(this_ref)?.string_value.as_ref().map_or(0, String::len);
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}
