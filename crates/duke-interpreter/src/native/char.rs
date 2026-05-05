pub(crate) fn native_char_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let char_val = |s: &Slot| -> Result<i32> {
        match s {
            Slot::Reference(Some(r)) => {
                match heap.get(*r)?.fields.first() {
                    Some(Slot::Int(n)) => Ok(*n),
                    _ => Err(Error::InvalidRef { address: *r }),
                }
            }
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => char_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = char_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// `Character.digit(char, int)int` — numeric value of char in given radix, or -1.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_char_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let ch = match args.first() {
        Some(Slot::Int(n)) => (*n).cast_unsigned(),
        _ => return Ok(Some(Slot::Int(-1))),
    };
    let radix = match args.get(1) {
        Some(Slot::Int(n)) => (*n).cast_unsigned(),
        _ => 10,
    };
    let result = char::from_u32(ch)
        .and_then(|c| c.to_digit(radix))
        .map_or(-1, u32::cast_signed);
    Ok(Some(Slot::Int(result)))
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
