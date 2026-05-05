pub(crate) fn native_println_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let string_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => {
            writeln!(out, "null").ok();
            return Ok(None);
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let obj = heap.get(string_ref)?;
    let text = obj.string_value.as_deref().unwrap_or("null");
    writeln!(out, "{text}").ok();
    Ok(None)
}

pub(crate) fn native_string_get_bytes_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bytes = string_bytes_for_arg(args, heap, StandardCharset::Utf8)?;
    Ok(Some(Slot::Reference(Some(allocate_byte_array(
        heap, &bytes,
    )?))))
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
    Ok(Some(Slot::Reference(Some(allocate_byte_array(
        heap, &bytes,
    )?))))
}

pub(crate) fn native_string_get_bytes_charset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let charset = charset_from_arg(args, 1, heap)?;
    let bytes = string_bytes_for_arg(args, heap, charset)?;
    Ok(Some(Slot::Reference(Some(allocate_byte_array(
        heap, &bytes,
    )?))))
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
    // Fetch objects from heap in one go to keep borrows short
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

/// Native: `Object.<init>()V` - root constructor is a no-op after null-checking `this`.
pub(crate) fn native_object_init(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(None)
}

/// Native: `Object.getClass()` — returns a lightweight `Class` object for the runtime type.
pub(crate) fn native_object_get_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let class_name = heap.get(this_ref)?.class_name.clone();
    let class_ref = allocate_class_object(heap, &class_name)?;
    Ok(Some(Slot::Reference(Some(class_ref))))
}

/// Native: `Object.equals(Object)` — default Java object identity comparison.
pub(crate) fn native_object_equals(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let equal = extract_ref_arg(args, 1) == Ok(this_ref);
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `Object.hashCode()` — returns heap address as hash.
pub(crate) fn native_object_hashcode(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(Slot::Reference(Some(r))) => Ok(Some(Slot::Int(*r as i32))),
        _ => Err(Error::NullPointerException),
    }
}

/// Native: `Object.toString()` — delegates to `heap_object_to_string` so String,
/// boxed primitives, and opaque objects all produce the correct Java representation.
pub(crate) fn native_object_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap_object_to_string(heap.get(this_ref)?, this_ref);
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Object.clone()` — shallow-copies a heap object.
pub(crate) fn native_object_clone(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let new_ref = heap.clone_object(this_ref)?;
    Ok(Some(Slot::Reference(Some(new_ref))))
}

const THROWABLE_CAUSE_FIELD: usize = 0;
const THROWABLE_STACK_TRACE_FIELD: usize = 1;
const THROWABLE_SUPPRESSED_FIELD: usize = 2;
const STACK_TRACE_ELEMENT_CLASS: &str = "java/lang/StackTraceElement";
const STACK_TRACE_ARRAY_CLASS: &str = "[Ljava/lang/StackTraceElement;";
const THROWABLE_ARRAY_CLASS: &str = "[Ljava/lang/Throwable;";

fn set_object_field(heap: &mut duke_gc::Heap, obj_ref: u64, index: usize, value: Slot) -> Result<()> {
    let obj = heap.get_mut(obj_ref)?;
    if obj.fields.len() <= index {
        obj.fields.resize(index + 1, Slot::Reference(None));
    }
    obj.fields[index] = value;
    Ok(())
}

fn allocate_slot_array(heap: &mut duke_gc::Heap, class_name: &str, elements: &[Slot]) -> Result<u64> {
    let array_ref = heap.allocate(class_name.to_string(), elements.len());
    heap.get_mut(array_ref)?.fields.clone_from_slice(elements);
    Ok(array_ref)
}

fn allocate_empty_reference_array(heap: &mut duke_gc::Heap, class_name: &str) -> Result<u64> {
    allocate_slot_array(heap, class_name, &[])
}

fn string_slot(heap: &mut duke_gc::Heap, value: &str) -> Slot {
    Slot::Reference(Some(heap.allocate_string(value.to_string())))
}

fn optional_string_slot(heap: &mut duke_gc::Heap, value: Option<&str>) -> Slot {
    value.map_or(Slot::Reference(None), |s| string_slot(heap, s))
}

fn slot_string(heap: &duke_gc::Heap, slot: Slot) -> Result<Option<String>> {
    match slot {
        Slot::Reference(Some(r)) => Ok(heap.get(r)?.string_value.clone()),
        _ => Ok(None),
    }
}

fn allocate_stack_trace_element(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    method_name: &str,
    file_name: Option<&str>,
    line_number: i32,
) -> Result<u64> {
    let element_ref = heap.allocate(STACK_TRACE_ELEMENT_CLASS.to_string(), 4);
    let declaring_class = class_name.replace('/', ".");
    let class_slot = string_slot(heap, &declaring_class);
    let method_slot = string_slot(heap, method_name);
    let file_slot = optional_string_slot(heap, file_name);
    let obj = heap.get_mut(element_ref)?;
    obj.fields[0] = class_slot;
    obj.fields[1] = method_slot;
    obj.fields[2] = file_slot;
    obj.fields[3] = Slot::Int(line_number);
    Ok(element_ref)
}

fn store_throwable_stack_trace_from_frames(
    heap: &mut duke_gc::Heap,
    throwable_ref: u64,
    frames: &[NativeStackFrame],
) -> Result<()> {
    let mut elements = Vec::with_capacity(frames.len());
    for frame in frames {
        let element_ref = allocate_stack_trace_element(
            heap,
            &frame.class_name,
            &frame.method_name,
            frame.file_name.as_deref(),
            frame.line_number,
        )?;
        elements.push(Slot::Reference(Some(element_ref)));
    }
    let array_ref = allocate_slot_array(heap, STACK_TRACE_ARRAY_CLASS, &elements)?;
    set_object_field(
        heap,
        throwable_ref,
        THROWABLE_STACK_TRACE_FIELD,
        Slot::Reference(Some(array_ref)),
    )
}

fn clone_reference_array(
    heap: &mut duke_gc::Heap,
    slot: Slot,
    default_class_name: &str,
) -> Result<Slot> {
    let Some(array_ref) = slot.as_reference() else {
        let empty_ref = allocate_empty_reference_array(heap, default_class_name)?;
        return Ok(Slot::Reference(Some(empty_ref)));
    };
    let (class_name, elements) = {
        let obj = heap.get(array_ref)?;
        (obj.class_name.clone(), obj.fields.clone())
    };
    let cloned_ref = allocate_slot_array(heap, &class_name, &elements)?;
    Ok(Slot::Reference(Some(cloned_ref)))
}

fn throwable_field_slot(heap: &duke_gc::Heap, throwable_ref: u64, index: usize) -> Result<Slot> {
    Ok(heap
        .get(throwable_ref)?
        .fields
        .get(index)
        .copied()
        .unwrap_or(Slot::Reference(None)))
}

fn throwable_header(heap: &duke_gc::Heap, throwable_ref: u64) -> Result<String> {
    let obj = heap.get(throwable_ref)?;
    let class_name = obj.class_name.replace('/', ".");
    Ok(match &obj.string_value {
        Some(msg) => format!("{class_name}: {msg}"),
        None => class_name,
    })
}

fn stack_trace_element_text(heap: &duke_gc::Heap, element_ref: u64) -> Result<String> {
    let fields = heap.get(element_ref)?.fields.clone();
    let class_name = slot_string(heap, fields.first().copied().unwrap_or(Slot::Reference(None)))?
        .unwrap_or_default();
    let method_name = slot_string(heap, fields.get(1).copied().unwrap_or(Slot::Reference(None)))?
        .unwrap_or_default();
    let file_name = slot_string(heap, fields.get(2).copied().unwrap_or(Slot::Reference(None)))?;
    let line_number = match fields.get(3) {
        Some(Slot::Int(line)) => *line,
        _ => -1,
    };
    let location = match (file_name, line_number) {
        (_, -2) => "Native Method".to_string(),
        (Some(file), line) if line >= 0 => format!("{file}:{line}"),
        (Some(file), _) => file,
        (None, _) => "Unknown Source".to_string(),
    };
    Ok(format!("{class_name}.{method_name}({location})"))
}

fn append_throwable_trace(
    heap: &duke_gc::Heap,
    throwable_ref: u64,
    caption: &str,
    frame_indent: &str,
    out: &mut String,
    visited: &mut std::collections::HashSet<u64>,
) -> Result<()> {
    if !visited.insert(throwable_ref) {
        out.push_str(caption);
        out.push_str("[CIRCULAR REFERENCE: ");
        out.push_str(&throwable_header(heap, throwable_ref)?);
        out.push_str("]\n");
        return Ok(());
    }

    out.push_str(caption);
    out.push_str(&throwable_header(heap, throwable_ref)?);
    out.push('\n');

    let stack_slot = throwable_field_slot(heap, throwable_ref, THROWABLE_STACK_TRACE_FIELD)?;
    if let Some(stack_ref) = stack_slot.as_reference() {
        for frame_slot in &heap.get(stack_ref)?.fields {
            if let Slot::Reference(Some(element_ref)) = frame_slot {
                out.push_str(frame_indent);
                out.push_str("\tat ");
                out.push_str(&stack_trace_element_text(heap, *element_ref)?);
                out.push('\n');
            }
        }
    }

    let suppressed_slot = throwable_field_slot(heap, throwable_ref, THROWABLE_SUPPRESSED_FIELD)?;
    if let Some(suppressed_ref) = suppressed_slot.as_reference() {
        for suppressed in &heap.get(suppressed_ref)?.fields {
            if let Slot::Reference(Some(suppressed_ref)) = suppressed {
                let mut suppressed_caption = String::from(frame_indent);
                suppressed_caption.push_str("\tSuppressed: ");
                let mut suppressed_indent = String::from(frame_indent);
                suppressed_indent.push('\t');
                append_throwable_trace(
                    heap,
                    *suppressed_ref,
                    &suppressed_caption,
                    &suppressed_indent,
                    out,
                    visited,
                )?;
            }
        }
    }

    let cause_slot = throwable_field_slot(heap, throwable_ref, THROWABLE_CAUSE_FIELD)?;
    if let Some(cause_ref) = cause_slot.as_reference()
        && cause_ref != throwable_ref
    {
        append_throwable_trace(heap, cause_ref, "Caused by: ", frame_indent, out, visited)?;
    }
    Ok(())
}

fn throwable_trace_string(heap: &duke_gc::Heap, throwable_ref: u64) -> Result<String> {
    let mut out = String::new();
    let mut visited = std::collections::HashSet::new();
    append_throwable_trace(heap, throwable_ref, "", "", &mut out, &mut visited)?;
    Ok(out)
}

fn write_to_print_stream_or_output(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    text: &str,
) -> Result<()> {
    let print_stream_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    if let Some(print_stream_ref) = print_stream_slot.as_reference() {
        let target_slot = heap
            .get(print_stream_ref)?
            .fields
            .first()
            .copied()
            .unwrap_or(Slot::Reference(None));
        if let Some(target_ref) = target_slot.as_reference()
            && heap.get(target_ref)?.class_name == "java/io/ByteArrayOutputStream"
        {
            let target = heap.get_mut(target_ref)?;
            target.string_value.get_or_insert_with(String::new).push_str(text);
            return Ok(());
        }
    }
    write!(out, "{text}").ok();
    Ok(())
}

fn fill_throwable_stack_trace_from_control(
    heap: &mut duke_gc::Heap,
    throwable_ref: u64,
    control: &NativeControl,
) -> Result<()> {
    store_throwable_stack_trace_from_frames(heap, throwable_ref, control.stack_trace())
}

/// `Throwable.addSuppressed(Throwable suppressed)V`
///
/// Signature: `args[0]` = this (Throwable), `args[1]` = suppressed (Throwable)
pub(crate) fn native_throwable_add_suppressed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _stdout: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let suppressed = args.get(1).copied().unwrap_or(Slot::Reference(None));
    if suppressed.as_reference().is_none() {
        return Ok(None);
    }
    let existing = throwable_field_slot(heap, this_ref, THROWABLE_SUPPRESSED_FIELD)?;
    let mut elements = existing.as_reference().map_or_else(Vec::new, |array_ref| {
        heap.get(array_ref)
            .map(|obj| obj.fields.clone())
            .unwrap_or_default()
    });
    elements.push(suppressed);
    let array_ref = allocate_slot_array(heap, THROWABLE_ARRAY_CLASS, &elements)?;
    set_object_field(
        heap,
        this_ref,
        THROWABLE_SUPPRESSED_FIELD,
        Slot::Reference(Some(array_ref)),
    )?;
    Ok(None)
}

/// Native: `Throwable.<init>(String)V` — stores detail message in `string_value`.
/// Native: `Throwable.<init>()V` - captures the construction stack trace.
pub(crate) fn native_throwable_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}

pub(crate) fn native_throwable_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let msg = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = msg;
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}

/// Native: `Throwable.<init>(String, Throwable)V` — stores message + cause.
pub(crate) fn native_throwable_init_string_cause(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let msg = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = msg;
    // Store cause in fields[0] (Throwable.cause field)
    if let Some(&cause_slot) = args.get(2)
        && let Ok(obj) = heap.get_mut(this_ref)
        && !obj.fields.is_empty()
    {
        obj.fields[THROWABLE_CAUSE_FIELD] = cause_slot;
    }
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}

/// Native: `Throwable.<init>(Throwable)V` - stores only the cause.
pub(crate) fn native_throwable_init_cause(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    if let Some(&cause_slot) = args.get(1)
        && let Ok(obj) = heap.get_mut(this_ref)
        && !obj.fields.is_empty()
    {
        obj.fields[THROWABLE_CAUSE_FIELD] = cause_slot;
    }
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(None)
}

/// Native: `Throwable.getCause()Throwable` — returns the stored cause.
#[allow(clippy::unnecessary_wraps)]
/// Native: `Throwable.fillInStackTrace()Throwable`.
pub(crate) fn native_throwable_fill_in_stack_trace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    fill_throwable_stack_trace_from_control(heap, this_ref, control)?;
    Ok(Some(Slot::Reference(Some(this_ref))))
}

pub(crate) fn native_throwable_get_cause(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let cause = throwable_field_slot(heap, *r, THROWABLE_CAUSE_FIELD)?;
            Ok(Some(cause))
        }
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Throwable.getMessage()String` — returns the stored detail message.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_throwable_get_message(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let msg = heap.get(*r)?.string_value.clone();
            let slot = msg.map_or(Slot::Reference(None), |s| {
                Slot::Reference(Some(heap.allocate_string(s)))
            });
            Ok(Some(slot))
        }
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Throwable.toString()String` — returns `"ClassName: message"` or just class name.
pub(crate) fn native_throwable_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = throwable_header(heap, this_ref)?;
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Throwable.getStackTrace()StackTraceElement[]`.
pub(crate) fn native_throwable_get_stack_trace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let slot = throwable_field_slot(heap, this_ref, THROWABLE_STACK_TRACE_FIELD)?;
    Ok(Some(clone_reference_array(
        heap,
        slot,
        STACK_TRACE_ARRAY_CLASS,
    )?))
}

/// Native: `Throwable.setStackTrace(StackTraceElement[])V`.
pub(crate) fn native_throwable_set_stack_trace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let new_trace = clone_reference_array(
        heap,
        args.get(1).copied().unwrap_or(Slot::Reference(None)),
        STACK_TRACE_ARRAY_CLASS,
    )?;
    set_object_field(heap, this_ref, THROWABLE_STACK_TRACE_FIELD, new_trace)?;
    Ok(None)
}

/// Native: `Throwable.getSuppressed()Throwable[]`.
pub(crate) fn native_throwable_get_suppressed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let slot = throwable_field_slot(heap, this_ref, THROWABLE_SUPPRESSED_FIELD)?;
    Ok(Some(clone_reference_array(heap, slot, THROWABLE_ARRAY_CLASS)?))
}

/// Native: `Throwable.printStackTrace()V`.
pub(crate) fn native_throwable_print_stack_trace(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = throwable_trace_string(heap, this_ref)?;
    write!(out, "{text}").ok();
    Ok(None)
}

/// Native: `Throwable.printStackTrace(PrintStream)V`.
pub(crate) fn native_throwable_print_stack_trace_print_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = throwable_trace_string(heap, this_ref)?;
    write_to_print_stream_or_output(args, heap, out, &text)?;
    Ok(None)
}

/// Native: `StackTraceElement.<init>(String,String,String,int)V`.
pub(crate) fn native_stack_trace_element_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let class_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let method_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let file_slot = args.get(3).copied().unwrap_or(Slot::Reference(None));
    let line_slot = args.get(4).copied().unwrap_or(Slot::Int(-1));
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 4 {
        obj.fields.resize(4, Slot::Reference(None));
    }
    obj.fields[0] = class_slot;
    obj.fields[1] = method_slot;
    obj.fields[2] = file_slot;
    obj.fields[3] = line_slot;
    Ok(None)
}

fn stack_trace_element_field(args: &[Slot], heap: &duke_gc::Heap, index: usize) -> Result<Slot> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(heap
        .get(this_ref)?
        .fields
        .get(index)
        .copied()
        .unwrap_or(Slot::Reference(None)))
}

pub(crate) fn native_stack_trace_element_get_class_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(stack_trace_element_field(args, heap, 0)?))
}

pub(crate) fn native_stack_trace_element_get_method_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(stack_trace_element_field(args, heap, 1)?))
}

pub(crate) fn native_stack_trace_element_get_file_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(stack_trace_element_field(args, heap, 2)?))
}

pub(crate) fn native_stack_trace_element_get_line_number(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match stack_trace_element_field(args, heap, 3)? {
        Slot::Int(line) => Ok(Some(Slot::Int(line))),
        _ => Ok(Some(Slot::Int(-1))),
    }
}

pub(crate) fn native_stack_trace_element_is_native_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let is_native = matches!(stack_trace_element_field(args, heap, 3)?, Slot::Int(-2));
    Ok(Some(Slot::Int(i32::from(is_native))))
}

pub(crate) fn native_stack_trace_element_to_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = stack_trace_element_text(heap, this_ref)?;
    let string_ref = heap.allocate_string(text);
    Ok(Some(Slot::Reference(Some(string_ref))))
}

pub(crate) fn native_printstream_init_output_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = args.get(1).copied().unwrap_or(Slot::Reference(None));
    set_object_field(heap, this_ref, 0, target)?;
    Ok(None)
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

/// Native: `PrintStream.print(String)` — no newline.
pub(crate) fn native_print_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let string_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => {
            write!(out, "null").ok();
            return Ok(None);
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let obj = heap.get(string_ref)?;
    let text = obj.string_value.as_deref().unwrap_or("null");
    write!(out, "{text}").ok();
    Ok(None)
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
    // Fetch objects from heap in one go to keep borrows short
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
    #[allow(clippy::cast_sign_loss)] // .max(0) guarantees non-negative
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
    #[allow(clippy::cast_sign_loss)] // .max(0) guarantees non-negative
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
    // Fetch objects from heap in one go to keep borrows short
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
            Slot::Reference(Some(r)) => Ok(heap.get(*r)?.string_value.clone().unwrap_or_default()),
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
    let prefix = heap
        .get(prefix_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
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
    let suffix = heap
        .get(suffix_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
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

// ---- Integer natives ----

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
            return Ok(Some(Slot::Reference(Some(
                heap.allocate_string(String::new()),
            ))));
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
        _ => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

// ---- String.concat ----

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
    let s2 = heap
        .get(other_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let r = heap.allocate_string(format!("{s1}{s2}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Formats a single boxed slot value using the given format specifier.
/// Apply width/alignment/flags to an already-formatted value string.
fn apply_format_width(s: String, width: usize, left_align: bool, zero_pad: bool) -> String {
    // Havoc: bounds check width to prevent OOM
    let max_width = 1024 * 1024 * 128; // 128 MB
    let width = width.min(max_width);

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

#[allow(clippy::too_many_lines)]
fn format_arg(
    spec: char,
    flags: &str,
    width: Option<usize>,
    precision: Option<usize>,
    slot: &Slot,
    heap: &duke_gc::Heap,
) -> Result<String> {
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
                    #[allow(clippy::cast_sign_loss)]
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
                #[allow(clippy::cast_sign_loss)]
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
    // Havoc: bounds check prec to prevent OOM
    let max_prec = 1024 * 1024 * 128; // 128 MB
    let prec = prec.min(max_prec);

    if v == 0.0 {
        let zeros = "0".repeat(prec);
        let e = if upper { 'E' } else { 'e' };
        return format!("0.{zeros}{e}+00");
    }
    #[allow(clippy::cast_possible_truncation)]
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

        // Parse flags: -, +, 0
        let mut flags = String::new();
        while let Some(&f) = chars.peek() {
            if matches!(f, '-' | '+' | '0' | ' ' | '#') {
                flags.push(f);
                chars.next();
            } else {
                break;
            }
        }

        // Parse optional width
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

        // Parse optional precision: .N
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
                        Some(Slot::Reference(Some(r))) => extract_field_arg(heap, *r, arg_idx)?,
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

// ---- Extended String natives ----

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
    let old_char = char::from_u32(extract_int_arg(args, 1)?.cast_unsigned()).unwrap_or('?');
    let new_char = char::from_u32(extract_int_arg(args, 2)?.cast_unsigned()).unwrap_or('?');
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
    let target = heap
        .get(target_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
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
    let delim = heap
        .get(delim_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    // Use regex split (Java's String.split uses regex); remove trailing empty strings
    // to match Java's default split behaviour.
    // Special case: split("") splits into individual chars (Java 21 semantics — no leading "").
    let parts: Vec<String> = if delim.is_empty() {
        s.chars().map(|c| c.to_string()).collect()
    } else {
        regex::Regex::new(&delim).map_or_else(
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
    let delim = heap
        .get(delim_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let limit = match args.get(2) {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
    #[allow(clippy::cast_sign_loss)] // limit is validated > 0 before the cast
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

// ---- Math natives ----

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
