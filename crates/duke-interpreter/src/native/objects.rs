/// Native: `Objects.isNull(Object)Z` — returns 1 if argument is null.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_objects_is_null(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let is_null = matches!(args.first(), Some(Slot::Reference(None)) | None);
    Ok(Some(Slot::Int(i32::from(is_null))))
}
/// Native: `Objects.nonNull(Object)Z` — returns 1 if argument is not null.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_objects_non_null(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let is_null = matches!(args.first(), Some(Slot::Reference(None)) | None);
    Ok(Some(Slot::Int(i32::from(!is_null))))
}
/// Native: `Objects.requireNonNull(Object)Object` — throws NPE if null, else returns arg.
pub(crate) fn native_objects_require_non_null(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(None)) | None => Err(Error::NullPointerException),
        Some(s) => Ok(Some(*s)),
    }
}
/// Native: `Objects.requireNonNull(Object, String)Object` — throws NPE with message if null.
pub(crate) fn native_objects_require_non_null_msg(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(None)) | None => Err(Error::NullPointerException),
        Some(s) => Ok(Some(*s)),
    }
}
/// Native: `Objects.equals(Object, Object)Z` — null-safe equality check.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_objects_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_slot_arg(args, 0);
    let b = extract_slot_arg(args, 1);
    let equal = slots_equal(&a, &b, heap);
    Ok(Some(Slot::Int(i32::from(equal))))
}
/// Native: `Objects.toString(Object)` — returns `"null"` if null, else `string_value` or class name.
pub(crate) fn native_objects_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let s = match args.first() {
        Some(Slot::Reference(None)) | None => heap.allocate_string("null".to_string()),
        Some(Slot::Reference(Some(r))) => {
            let obj = heap.get(*r)?;
            let text = obj
                .string_value
                .as_deref()
                .map_or_else(|| format!("{}@{}", obj.class_name, r), str::to_owned);
            let _ = obj;
            heap.allocate_string(text)
        }
        Some(Slot::Int(n)) => heap.allocate_string(n.to_string()),
        Some(Slot::Long(n)) => heap.allocate_string(n.to_string()),
        Some(other) => heap.allocate_string(format!("{other:?}")),
    };
    Ok(Some(Slot::Reference(Some(s))))
}
/// Native: `Objects.toString(Object, String)String` — returns nullDefault if null, else toString.
pub(crate) fn native_objects_tostring_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(None)) | None => {
            let default_ref = match args.get(1) {
                Some(Slot::Reference(Some(r))) => *r,
                _ => heap.allocate_string("null".to_string()),
            };
            Ok(Some(Slot::Reference(Some(default_ref))))
        }
        Some(Slot::Reference(Some(r))) => {
            let obj = heap.get(*r)?;
            let text = obj
                .string_value
                .as_deref()
                .map_or_else(|| format!("{}@{}", obj.class_name, r), str::to_owned);
            let _ = obj;
            let s = heap.allocate_string(text);
            Ok(Some(Slot::Reference(Some(s))))
        }
        Some(Slot::Int(n)) => {
            let s = heap.allocate_string(n.to_string());
            Ok(Some(Slot::Reference(Some(s))))
        }
        Some(other) => {
            let s = heap.allocate_string(format!("{other:?}"));
            Ok(Some(Slot::Reference(Some(s))))
        }
    }
}
/// Native: `Objects.hashCode(Object)I` — returns 0 for null, else object identity hash.
#[allow(clippy::cast_possible_truncation, clippy::unnecessary_wraps)]
pub(crate) fn native_objects_hashcode(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let hash = match args.first() {
        Some(Slot::Reference(Some(r))) => (*r & 0x7FFF_FFFF) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(hash)))
}
