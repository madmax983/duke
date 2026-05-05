const fn charset_for_name(name: &str) -> Option<StandardCharset> {
    if name.eq_ignore_ascii_case("UTF-8") || name.eq_ignore_ascii_case("UTF8")
        || name.eq_ignore_ascii_case("utf8")
    {
        return Some(StandardCharset::Utf8);
    }
    if name.eq_ignore_ascii_case("UTF-16") || name.eq_ignore_ascii_case("UTF16") {
        return Some(StandardCharset::Utf16);
    }
    if name.eq_ignore_ascii_case("UTF-16BE") || name.eq_ignore_ascii_case("UTF16BE") {
        return Some(StandardCharset::Utf16Be);
    }
    if name.eq_ignore_ascii_case("UTF-16LE") || name.eq_ignore_ascii_case("UTF16LE") {
        return Some(StandardCharset::Utf16Le);
    }
    if name.eq_ignore_ascii_case("US-ASCII") || name.eq_ignore_ascii_case("ASCII")
        || name.eq_ignore_ascii_case("US_ASCII")
    {
        return Some(StandardCharset::UsAscii);
    }
    if name.eq_ignore_ascii_case("ISO-8859-1") || name.eq_ignore_ascii_case("ISO8859-1")
        || name.eq_ignore_ascii_case("ISO8859_1") || name.eq_ignore_ascii_case("latin1")
    {
        return Some(StandardCharset::Iso88591);
    }
    None
}
fn charset_ref(heap: &mut duke_gc::Heap, charset: StandardCharset) -> u64 {
    allocate_standard_charset(heap, charset.canonical_name())
}
fn charset_from_name_ref(
    heap: &duke_gc::Heap,
    string_ref: u64,
    error_for_unknown: fn(&str) -> Error,
) -> Result<StandardCharset> {
    let name = string_value_from_ref(heap, string_ref)?;
    charset_for_name(&name).ok_or_else(|| error_for_unknown(&name))
}
fn charset_from_arg(
    args: &[Slot],
    idx: usize,
    heap: &duke_gc::Heap,
) -> Result<StandardCharset> {
    let charset_ref = extract_ref_arg(args, idx)?;
    let obj = heap.get(charset_ref)?;
    if obj.class_name != "java/nio/charset/Charset" {
        return Err(Error::ClassCastException {
            from: obj.class_name.clone(),
            to: "java/nio/charset/Charset".to_string(),
        });
    }
    let name = obj
        .string_value
        .as_deref()
        .ok_or(Error::TypeMismatch {
            expected: "Charset.string_value",
            got: "None",
        })?;
    StandardCharset::from_canonical(name).ok_or_else(|| unsupported_charset_error(name))
}
pub(crate) fn native_charset_for_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let charset = charset_from_name_ref(heap, name_ref, unsupported_charset_error)?;
    Ok(Some(Slot::Reference(Some(charset_ref(heap, charset)))))
}
/// `Charset.defaultCharset()` returns UTF-8.
///
/// Duke intentionally mirrors JDK 18+ / JEP 400's deterministic UTF-8 default
/// instead of inheriting a host-process locale.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_charset_default_charset(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Reference(Some(charset_ref(heap, StandardCharset::Utf8)))))
}
pub(crate) fn native_charset_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let charset = charset_from_arg(args, 0, heap)?;
    let name_ref = heap.allocate_string(charset.canonical_name().to_string());
    Ok(Some(Slot::Reference(Some(name_ref))))
}
pub(crate) fn native_charset_is_registered(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _charset = charset_from_arg(args, 0, heap)?;
    Ok(Some(Slot::Int(1)))
}
pub(crate) fn native_charset_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_charset = charset_from_arg(args, 0, heap)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    let other_obj = heap.get(other_ref)?;
    if other_obj.class_name != "java/nio/charset/Charset" {
        return Ok(Some(Slot::Int(0)));
    }
    let equal = other_obj
        .string_value
        .as_deref()
        .is_some_and(|name| name == this_charset.canonical_name());
    Ok(Some(Slot::Int(i32::from(equal))))
}
pub(crate) fn native_charset_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let charset = charset_from_arg(args, 0, heap)?;
    Ok(Some(Slot::Int(java_string_hash(charset.canonical_name()))))
}
