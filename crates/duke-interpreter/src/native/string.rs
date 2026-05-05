fn string_value_from_ref(heap: &duke_gc::Heap, string_ref: u64) -> Result<String> {
    heap.get(string_ref)?.string_value.clone().ok_or(Error::NullPointerException)
}
fn string_bytes_for_arg(
    args: &[Slot],
    heap: &duke_gc::Heap,
    charset: StandardCharset,
) -> Result<Vec<u8>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let value = heap.get(this_ref)?.string_value.as_deref().unwrap_or_default();
    Ok(encode_string_with_charset(value, charset))
}
pub(crate) fn native_string_get_bytes_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes = string_bytes_for_arg(args, heap, StandardCharset::Utf8)?;
    Ok(Some(Slot::Reference(Some(allocate_byte_array(heap, &bytes)?))))
}
pub(crate) fn native_string_get_bytes_named(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 1)?;
    let charset = charset_from_name_ref(heap, name_ref, unsupported_encoding_error)?;
    let bytes = string_bytes_for_arg(args, heap, charset)?;
    Ok(Some(Slot::Reference(Some(allocate_byte_array(heap, &bytes)?))))
}
pub(crate) fn native_string_get_bytes_charset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let charset = charset_from_arg(args, 1, heap)?;
    let bytes = string_bytes_for_arg(args, heap, charset)?;
    Ok(Some(Slot::Reference(Some(allocate_byte_array(heap, &bytes)?))))
}
pub(crate) fn native_string_init_bytes_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes_ref = extract_ref_arg(args, 1)?;
    let bytes = full_byte_array(heap, bytes_ref)?;
    init_string_from_bytes(args, heap, &bytes, StandardCharset::Utf8)
}
pub(crate) fn native_string_init_bytes_default_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes_ref = extract_ref_arg(args, 1)?;
    let offset = extract_int_arg(args, 2)?;
    let length = extract_int_arg(args, 3)?;
    let bytes = byte_array_window(heap, bytes_ref, offset, length)?;
    init_string_from_bytes(args, heap, &bytes, StandardCharset::Utf8)
}
pub(crate) fn native_string_init_bytes_named(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes_ref = extract_ref_arg(args, 1)?;
    let name_ref = extract_ref_arg(args, 2)?;
    let charset = charset_from_name_ref(heap, name_ref, unsupported_encoding_error)?;
    let bytes = full_byte_array(heap, bytes_ref)?;
    init_string_from_bytes(args, heap, &bytes, charset)
}
pub(crate) fn native_string_init_bytes_charset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes_ref = extract_ref_arg(args, 1)?;
    let charset = charset_from_arg(args, 2, heap)?;
    let bytes = full_byte_array(heap, bytes_ref)?;
    init_string_from_bytes(args, heap, &bytes, charset)
}
pub(crate) fn native_string_init_bytes_range_charset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes_ref = extract_ref_arg(args, 1)?;
    let offset = extract_int_arg(args, 2)?;
    let length = extract_int_arg(args, 3)?;
    let charset = charset_from_arg(args, 4, heap)?;
    let bytes = byte_array_window(heap, bytes_ref, offset, length)?;
    init_string_from_bytes(args, heap, &bytes, charset)
}
pub(crate) fn native_string_init_bytes_range_named(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes_ref = extract_ref_arg(args, 1)?;
    let offset = extract_int_arg(args, 2)?;
    let length = extract_int_arg(args, 3)?;
    let name_ref = extract_ref_arg(args, 4)?;
    let charset = charset_from_name_ref(heap, name_ref, unsupported_encoding_error)?;
    let bytes = byte_array_window(heap, bytes_ref, offset, length)?;
    init_string_from_bytes(args, heap, &bytes, charset)
}
/// Native: `String.length()` — returns string length as int.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_string_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let len = obj.string_value.as_ref().map_or(0, String::len);
    Ok(Some(Slot::Int(len as i32)))
}
/// Native: `String.equals(Object)` — compares string content.
pub(crate) fn native_string_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    let this_obj = heap.get(this_ref)?;
    let other_obj = heap.get(other_ref)?;
    let this_str = this_obj.string_value.as_deref().unwrap_or_default();
    let other_str = other_obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(this_str == other_str))))
}
/// Native: `String.charAt(int)` — returns char at index as int.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_string_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let index = extract_int_arg(args, 1)?;
    let obj = heap.get(this_ref)?;
    let s = obj.string_value.as_deref().unwrap_or("");
    let ch = s
        .chars()
        .nth(index as usize)
        .ok_or(Error::ArrayIndexOutOfBounds {
            index,
            length: s.len(),
        })?;
    Ok(Some(Slot::Int(ch as i32)))
}
fn string_slot(heap: &mut duke_gc::Heap, value: &str) -> Slot {
    Slot::Reference(Some(heap.allocate_string(value.to_string())))
}
/// Native: `String.<init>(String)V` — copy constructor: copies `string_value` from source.
pub(crate) fn native_string_init_copy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_val = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = src_val;
    Ok(None)
}
fn string_backed_object_value(heap: &duke_gc::Heap, obj_ref: u64) -> Result<String> {
    let Some(value_ref) = first_reference_field(heap, obj_ref)? else {
        return Err(Error::NullPointerException);
    };
    string_value_from_ref(heap, value_ref)
}
fn string_arg(args: &[Slot], idx: usize, heap: &duke_gc::Heap) -> Result<String> {
    let string_ref = extract_ref_arg(args, idx)?;
    string_value_from_ref(heap, string_ref)
}
/// Native: `String.valueOf(int)` — static method, returns string of int.
pub(crate) fn native_string_value_of_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let s = val.to_string();
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.substring(int)` - substring from begin to end.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn native_string_substring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let begin = extract_int_arg(args, 1)? as usize;
    let sub = {
        let obj = heap.get(this_ref)?;
        let s = obj.string_value.as_deref().unwrap_or_default();
        let char_count = s.chars().count();
        if begin > char_count {
            return Err(Error::ArrayIndexOutOfBounds {
                index: i32::try_from(begin).unwrap_or(i32::MAX),
                length: char_count,
            });
        }
        let byte_begin = s.char_indices().nth(begin).map_or(s.len(), |(i, _)| i);
        s[byte_begin..].to_string()
    };
    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.substring(int, int)` — substring from begin to end (exclusive).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn native_string_substring_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let begin = extract_int_arg(args, 1)? as usize;
    let end = extract_int_arg(args, 2)? as usize;
    let sub = {
        let obj = heap.get(this_ref)?;
        let s = obj.string_value.as_deref().unwrap_or_default();
        let char_count = s.chars().count();
        if begin > end || end > char_count {
            return Err(Error::ArrayIndexOutOfBounds {
                index: i32::try_from(end).unwrap_or(i32::MAX),
                length: char_count,
            });
        }
        let byte_begin = s.char_indices().nth(begin).map_or(s.len(), |(i, _)| i);
        let byte_end = s.char_indices().nth(end).map_or(s.len(), |(i, _)| i);
        s[byte_begin..byte_end].to_string()
    };
    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.indexOf(String)` — find first occurrence of target.
pub(crate) fn native_string_indexof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target_ref = extract_ref_arg(args, 1)?;
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let result = s.find(target).map_or(-1, |i| i as i32);
    Ok(Some(Slot::Int(result)))
}
/// Native: `String.indexOf(String, int)I` — first occurrence at or after fromIndex.
pub(crate) fn native_string_indexof_from(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target_ref = extract_ref_arg(args, 1)?;
    #[allow(clippy::cast_sign_loss)]
    let from = extract_int_arg(args, 2).unwrap_or(0).max(0) as usize;
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();
    let search_in = if from < s.len() { &s[from..] } else { "" };
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let result = search_in.find(target).map_or(-1, |i| (from + i) as i32);
    Ok(Some(Slot::Int(result)))
}
/// Native: `String.lastIndexOf(String, int)I` — last occurrence at or before fromIndex.
pub(crate) fn native_string_last_indexof_from(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let sub_ref = extract_ref_arg(args, 1)?;
    #[allow(clippy::cast_sign_loss)]
    let from = extract_int_arg(args, 2).unwrap_or(0).max(0) as usize;
    let this_str = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let sub_str = heap.get(sub_ref)?.string_value.clone().unwrap_or_default();
    let search_in = if from + sub_str.len() < this_str.len() {
        &this_str[..from + sub_str.len()]
    } else {
        &this_str
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let result = search_in.rfind(sub_str.as_str()).map_or(-1, |i| i as i32);
    Ok(Some(Slot::Int(result)))
}
/// Native: `String.contains(CharSequence)` — check if string contains target.
pub(crate) fn native_string_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target_ref = extract_ref_arg(args, 1)?;
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.contains(target)))))
}
/// Native: `String.isEmpty()` — check if string is empty.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_string_isempty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let s = obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.is_empty()))))
}
/// Native: `String.compareTo(String)` — delegates to the Object overload.
pub(crate) fn native_string_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_string_compareto_object(args, heap, out, control)
}
/// Native: `String.compareTo(Object)` — lexicographic comparison via Object descriptor.
pub(crate) fn native_string_compareto_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let str_val = |s: &Slot| -> Result<String> {
        match s {
            Slot::Reference(Some(r)) => {
                Ok(heap.get(*r)?.string_value.clone().unwrap_or_default())
            }
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => str_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = str_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.as_str().cmp(b.as_str())))))
}
/// Native: `String.startsWith(String)` — check if string starts with prefix.
pub(crate) fn native_string_startswith(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let prefix_ref = extract_ref_arg(args, 1)?;
    let prefix = heap.get(prefix_ref)?.string_value.clone().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.starts_with(&prefix)))))
}
/// Native: `String.endsWith(String)` — check if string ends with suffix.
pub(crate) fn native_string_endswith(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let suffix_ref = extract_ref_arg(args, 1)?;
    let suffix = heap.get(suffix_ref)?.string_value.clone().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.ends_with(&suffix)))))
}
/// Native: `String.trim()` — remove leading and trailing whitespace.
pub(crate) fn native_string_trim(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let trimmed = s.trim().to_string();
    let r = heap.allocate_string(trimmed);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.toCharArray()` — convert string to char array.
///
/// **Bolt Optimization:**
/// Eliminates an intermediate `.collect::<Vec<char>>()` allocation by pre-computing
/// the character length via `.count()` and iterating characters directly into the heap array.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_string_tochararray(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let char_count = s.chars().count();
    let arr_ref = heap.allocate("[C".to_string(), char_count);
    for (i, c) in s.chars().enumerate() {
        heap.get_mut(arr_ref)?.fields[i] = Slot::Int(c as i32);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}
/// Native: `String.<init>(char[])V` — constructs a String from a char array.
pub(crate) fn native_string_init_from_chars(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Reference(Some(arr_ref))) = args.get(1).copied() else {
        heap.get_mut(this_ref)?.string_value = Some(String::new());
        return Ok(None);
    };
    let chars: String = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| match s {
            Slot::Int(n) => char::from_u32(u32::try_from(*n).unwrap_or(0)),
            _ => None,
        })
        .collect();
    heap.get_mut(this_ref)?.string_value = Some(chars);
    Ok(None)
}
/// Native: `String.valueOf(char[])String` — creates String from char array.
pub(crate) fn native_string_value_of_char_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let arr_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => {
            return Ok(Some(Slot::Reference(Some(heap.allocate_string(String::new())))));
        }
    };
    let chars: String = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| match s {
            Slot::Int(n) => char::from_u32(u32::try_from(*n).unwrap_or(0)),
            _ => None,
        })
        .collect();
    Ok(Some(Slot::Reference(Some(heap.allocate_string(chars)))))
}
/// Native: `String.valueOf(long)` — converts long to String.
pub(crate) fn native_string_value_of_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.valueOf(double)` — converts double to String.
pub(crate) fn native_string_value_of_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_double_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.valueOf(float)` — converts float to String.
pub(crate) fn native_string_value_of_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_float_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.valueOf(boolean)` — converts boolean to String.
pub(crate) fn native_string_value_of_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v != 0,
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Int(boolean)",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(if val { "true" } else { "false" }.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.valueOf(char)` — converts char to String.
pub(crate) fn native_string_value_of_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('?'),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Int(char)",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.valueOf(Object)` — converts Object to String.
pub(crate) fn native_string_value_of_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let s = heap_object_to_string(heap.get(*r)?, *r);
            let r = heap.allocate_string(s);
            Ok(Some(Slot::Reference(Some(r))))
        }
        Some(Slot::Reference(None)) => {
            let r = heap.allocate_string("null".to_string());
            Ok(Some(Slot::Reference(Some(r))))
        }
        _ => {
            Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            })
        }
    }
}
/// Native: `String.concat(String)` — concatenates two strings.
pub(crate) fn native_string_concat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s1 = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let other_ref = extract_ref_arg(args, 1)?;
    let s2 = heap.get(other_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(format!("{s1}{s2}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.format(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;`
pub(crate) fn native_string_format(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fmt_ref = extract_ref_arg(args, 0)?;
    let fmt = heap.get(fmt_ref)?.string_value.clone().unwrap_or_default();
    let arr_len = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.fields.len(),
        _ => 0,
    };
    let mut result = String::new();
    let mut arg_idx = 0usize;
    let mut chars = fmt.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            result.push(ch);
            continue;
        }
        let mut flags = String::new();
        while let Some(&f) = chars.peek() {
            if matches!(f, '-' | '+' | '0' | ' ' | '#') {
                flags.push(f);
                chars.next();
            } else {
                break;
            }
        }
        let mut width_str = String::new();
        while let Some(&d) = chars.peek() {
            if d.is_ascii_digit() {
                width_str.push(d);
                chars.next();
            } else {
                break;
            }
        }
        let width: Option<usize> = if width_str.is_empty() {
            None
        } else {
            width_str.parse().ok()
        };
        let precision: Option<usize> = if chars.peek() == Some(&'.') {
            chars.next();
            let mut prec_str = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    prec_str.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            prec_str.parse().ok()
        } else {
            None
        };
        let Some(spec) = chars.next() else {
            break;
        };
        match spec {
            '%' => result.push('%'),
            'n' => result.push('\n'),
            's' | 'd' | 'f' | 'x' | 'X' | 'b' | 'c' | 'o' | 'e' | 'E' => {
                let slot = if arg_idx < arr_len {
                    match args.get(1) {
                        Some(Slot::Reference(Some(r))) => {
                            extract_field_arg(heap, *r, arg_idx)?
                        }
                        _ => Slot::Reference(None),
                    }
                } else {
                    Slot::Reference(None)
                };
                arg_idx += 1;
                let formatted = format_arg(spec, &flags, width, precision, &slot, heap)?;
                result.push_str(&formatted);
            }
            _ => {
                result.push('%');
                result.push(spec);
            }
        }
    }
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.toUpperCase()` — returns a new uppercase String.
pub(crate) fn native_string_touppercase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.to_uppercase());
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.toLowerCase()` — returns a new lowercase String.
pub(crate) fn native_string_tolowercase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.to_lowercase());
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.replace(char, char)` — replaces all occurrences of old char with new char.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_string_replace_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let old_char = char::from_u32(extract_int_arg(args, 1)?.cast_unsigned())
        .unwrap_or('?');
    let new_char = char::from_u32(extract_int_arg(args, 2)?.cast_unsigned())
        .unwrap_or('?');
    let result = s.replace(old_char, &new_char.to_string());
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.replace(CharSequence, CharSequence)` — replaces all occurrences of target with replacement.
pub(crate) fn native_string_replace_charsequence(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let target_ref = extract_ref_arg(args, 1)?;
    let target = heap.get(target_ref)?.string_value.clone().unwrap_or_default();
    let replacement_ref = extract_ref_arg(args, 2)?;
    let replacement = heap
        .get(replacement_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let result = s.replace(&*target, &replacement);
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `String.split(String)` — splits string by delimiter, returns String array.
pub(crate) fn native_string_split(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let delim_ref = extract_ref_arg(args, 1)?;
    let delim = heap.get(delim_ref)?.string_value.clone().unwrap_or_default();
    let parts: Vec<String> = if delim.is_empty() {
        s.chars().map(|c| c.to_string()).collect()
    } else {
        regex::Regex::new(&delim)
            .map_or_else(
                |_| s.split(delim.as_str()).map(str::to_string).collect(),
                |re| {
                    let mut v: Vec<String> = re.split(&s).map(str::to_string).collect();
                    while v.last().is_some_and(String::is_empty) {
                        v.pop();
                    }
                    v
                },
            )
    };
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), parts.len());
    for (i, part) in parts.iter().enumerate() {
        let str_ref = heap.allocate_string(part.clone());
        heap.get_mut(arr_ref)?.fields[i] = Slot::Reference(Some(str_ref));
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}
/// Native: `String.split(String, int)` — split with a limit parameter.
pub(crate) fn native_string_split_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let delim_ref = extract_ref_arg(args, 1)?;
    let delim = heap.get(delim_ref)?.string_value.clone().unwrap_or_default();
    let limit = match args.get(2) {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    #[allow(clippy::cast_sign_loss)]
    let parts: Vec<String> = if delim.is_empty() {
        let chars: Vec<String> = s.chars().map(|c| c.to_string()).collect();
        if limit > 0 && (limit as usize) < chars.len() {
            let mut v = chars[..limit as usize - 1].to_vec();
            v.push(chars[limit as usize - 1..].join(""));
            v
        } else {
            chars
        }
    } else {
        let re = regex::Regex::new(&delim)
            .unwrap_or_else(|_| regex::Regex::new(&regex::escape(&delim)).unwrap());
        if limit > 0 {
            re.splitn(&s, limit as usize).map(str::to_string).collect()
        } else {
            let mut v: Vec<String> = re.split(&s).map(str::to_string).collect();
            if limit == 0 {
                while v.last().is_some_and(String::is_empty) {
                    v.pop();
                }
            }
            v
        }
    };
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), parts.len());
    for (i, part) in parts.iter().enumerate() {
        let str_ref = heap.allocate_string(part.clone());
        heap.get_mut(arr_ref)?.fields[i] = Slot::Reference(Some(str_ref));
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}
/// Native: `String.hashCode()` — Java's hash algorithm: `s[0]*31^(n-1) + s[1]*31^(n-2) + ... + s[n-1]`.
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn native_string_hashcode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let mut h: i32 = 0;
    for ch in s.chars() {
        h = h.wrapping_mul(31).wrapping_add(ch as i32);
    }
    Ok(Some(Slot::Int(h)))
}
/// Native: `String.toString()` — identity, returns `this`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_string_tostring(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(extract_slot_arg(args, 0)))
}
/// Native: `String.intern()String` — returns canonical string (identity for our heap strings).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_string_intern(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(extract_slot_arg(args, 0)))
}
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
    let blank = heap
        .get(this_ref)
        .map_or(
            true,
            |o| {
                o.string_value
                    .as_deref()
                    .is_none_or(|s| s.chars().all(char::is_whitespace))
            },
        );
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
    let max_size = 1024 * 1024 * 128;
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
    let fmt_ref = extract_ref_arg(args, 0)?;
    let arr_slot = extract_slot_arg(args, 1);
    native_string_format(&[Slot::Reference(Some(fmt_ref)), arr_slot], heap, out, control)
}
/// Native: `String.join(CharSequence, CharSequence[])String` — joins array elements with delimiter.
pub(crate) fn native_string_join(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let delim_ref = extract_ref_arg(args, 0)?;
    let delim = heap.get(delim_ref)?.string_value.clone().unwrap_or_default();
    let parts: Vec<String> = match args.get(1) {
        Some(Slot::Reference(Some(arr_ref))) => {
            let obj = heap.get(*arr_ref)?;
            if obj.class_name.starts_with('[') {
                let len = obj.fields.len();
                let mut result = Vec::with_capacity(len);
                let slots: Vec<Slot> = obj.fields.clone();
                let _ = obj;
                for slot in slots {
                    let s = match slot {
                        Slot::Reference(Some(r)) => {
                            heap.get(r)?
                                .string_value
                                .clone()
                                .unwrap_or_else(|| "null".to_string())
                        }
                        Slot::Reference(None) => "null".to_string(),
                        Slot::Int(n) => n.to_string(),
                        Slot::Long(n) => n.to_string(),
                        other => format!("{other:?}"),
                    };
                    result.push(s);
                }
                result
            } else {
                let size_val = match obj.fields.first() {
                    Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                    _ => 0,
                };
                let elems: Vec<Slot> = obj
                    .fields[1..=size_val.min(obj.fields.len().saturating_sub(1))]
                    .to_vec();
                let _ = obj;
                let mut result = Vec::with_capacity(size_val);
                for slot in elems {
                    let s = match slot {
                        Slot::Reference(Some(r)) => {
                            heap.get(r)?
                                .string_value
                                .clone()
                                .unwrap_or_else(|| "null".to_string())
                        }
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
    #[allow(clippy::manual_saturating_arithmetic)]
    let mut total_len = parts
        .len()
        .saturating_sub(1)
        .checked_mul(delim.len())
        .unwrap_or(usize::MAX);
    for part in &parts {
        #[allow(clippy::manual_saturating_arithmetic)]
        {
            total_len = total_len.checked_add(part.len()).unwrap_or(usize::MAX);
        }
    }
    let max_size = 1024 * 1024 * 128;
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
            heap.get(this_ref)
                .ok()
                .and_then(|o| {
                    o.string_value.as_deref().map(|s| s[..byte_pos].chars().count())
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
fn string_from_slot(heap: &duke_gc::Heap, slot: Slot) -> Option<String> {
    let Slot::Reference(Some(string_ref)) = slot else {
        return None;
    };
    heap.get(string_ref).ok().and_then(|obj| obj.string_value.clone())
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
        let max_size = 1024 * 1024 * 128;
        let num_lines = s.lines().count().max(1);
        let extra_len = n_usize.checked_mul(num_lines);
        if extra_len.is_none()
            || extra_len.unwrap().checked_add(s.len()).is_none_or(|l| l > max_size)
        {
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
