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
/// Native: `String.equalsIgnoreCase(String)` — locale-independent
/// case-insensitive comparison using Java SE 21's two-step fold.
pub(crate) fn native_string_equalsignorecase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    // Null argument -> false: "Returns true if and only if the argument is not
    // null and ..." (Java SE 21 contract).
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    let this_obj = heap.get(this_ref)?;
    let other_obj = heap.get(other_ref)?;
    let this_str = this_obj.string_value.as_deref().unwrap_or_default();
    let other_str = other_obj.string_value.as_deref().unwrap_or_default();
    // Fast path: identical content (also covers receiver-equals-self).
    if this_str == other_str {
        return Ok(Some(Slot::Int(1)));
    }
    // Length mismatch -> false without folding characters.
    if this_str.chars().count() != other_str.chars().count() {
        return Ok(Some(Slot::Int(0)));
    }
    let equal = this_str
        .chars()
        .zip(other_str.chars())
        .all(|(a, b)| chars_equal_ignore_case(a, b));
    Ok(Some(Slot::Int(i32::from(equal))))
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
/// Native: `Object.hashCode()` — returns the object's stable identity hash.
///
/// The hash is assigned lazily by the heap and carried across relocation, so it
/// stays constant even when a minor GC copies or promotes the object (unlike the
/// old index-derived hash).
pub(crate) fn native_object_hashcode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => Ok(Some(Slot::Int(heap.identity_hash(*r)?))),
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
/// Native: `Enum.<init>(Ljava/lang/String;I)V` — stores name + ordinal.
/// args: `[this_ref, name_ref, ordinal_int]`
pub(crate) fn native_enum_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let name_slot = extract_slot_arg(args, 1);
    let ordinal = match args.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() >= 2 {
        obj.fields[0] = name_slot;
        obj.fields[1] = Slot::Int(ordinal);
    }
    Ok(None)
}
/// Native: `Enum.ordinal()I`
pub(crate) fn native_enum_ordinal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.get(1) {
        Some(Slot::Int(v)) => Ok(Some(Slot::Int(*v))),
        _ => Ok(Some(Slot::Int(0))),
    }
}
/// Native: `Enum.name()Ljava/lang/String;`
pub(crate) fn native_enum_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Ok(Some(Slot::Reference(None))),
    }
}
/// Native: `Enum.valueOf(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;`
/// Searches heap for enum constants of the given class matching the name.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_enum_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let target_name = heap.get(name_ref)?.string_value.clone().unwrap_or_default();
    let enum_class_name = heap
        .get(class_ref)?
        .string_value
        .clone()
        .unwrap_or_default();

    let obj_count = heap.len();
    for i in 0..obj_count {
        let obj = heap.get(i as u64)?;
        if obj.class_name == enum_class_name
            && obj.fields.len() >= 2
            && let Some(Slot::Reference(Some(name_r))) = obj.fields.first()
            && let Ok(name_obj) = heap.get(*name_r)
            && name_obj.string_value.as_deref() == Some(target_name.as_str())
        {
            return Ok(Some(Slot::Reference(Some(i as u64))));
        }
    }

    Err(Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    })
}
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
/// Native: `Class.isArray()Z` — true when the class represents an array type.
pub(crate) fn native_class_is_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    Ok(Some(Slot::Int(i32::from(internal_name.starts_with('[')))))
}
/// Native: `Class.getPrimitiveClass(String)Class` — static factory returning the
/// `Class` mirror for a primitive type name ("int", "long", ...).
///
/// Returns the same string-keyed mirror that
/// [`initialize_primitive_wrapper_type_field`] uses for `Integer.TYPE` etc.,
/// so `int.class` and `Integer.TYPE` share object identity across the runtime.
pub(crate) fn native_class_get_primitive_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .ok_or(Error::NullPointerException)?;
    let descriptor = match name.as_str() {
        "int" => "I",
        "long" => "J",
        "float" => "F",
        "double" => "D",
        "boolean" => "Z",
        "byte" => "B",
        "char" => "C",
        "short" => "S",
        "void" => "V",
        _ => {
            return Err(Error::JavaException {
                class_name: "java/lang/ClassNotFoundException".to_string(),
            });
        }
    };
    let class_ref = allocate_class_object(heap, descriptor)?;
    Ok(Some(Slot::Reference(Some(class_ref))))
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
pub(crate) fn native_class_loader_get_system_resource_as_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let requested_name = string_arg(args, 0, heap)?;
    let classpath = classpath_debug_label(None);
    let Some(resolved_name) = normalize_resource_name(&requested_name) else {
        log_resource_lookup_miss(&requested_name, "<system>", &classpath);
        return Ok(Some(Slot::Reference(None)));
    };
    let resource = ops.find_resource_entry(heap, None, &resolved_name)?;
    if resource.is_none() {
        log_resource_lookup_miss(&resolved_name, "<system>", &classpath);
    }
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
pub(crate) fn native_system_exit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let code = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 1,
    };
    Err(Error::SystemExit { code })
}
pub(crate) fn native_system_get_property(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let key_ref = extract_ref_arg(args, 0)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let result = system_property_value(&key).map_or(Slot::Reference(None), |value| {
        Slot::Reference(Some(heap.allocate_string(value)))
    });
    Ok(Some(result))
}
pub(crate) fn native_system_set_property(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let key_ref = extract_ref_arg(args, 0)?;
    let value_ref = extract_ref_arg(args, 1)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let value = string_value_from_ref(heap, value_ref)?;
    let previous = {
        let mut overrides = system_property_overrides()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let prev = overrides.get(&key).cloned().or_else(|| system_property_value_fallback(&key));
        overrides.insert(key, value);
        prev
    };
    let result = previous.map_or(Slot::Reference(None), |previous| {
        Slot::Reference(Some(heap.allocate_string(previous)))
    });
    Ok(Some(result))
}
pub(crate) fn native_system_get_property_with_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let key_ref = extract_ref_arg(args, 0)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let result = system_property_value(&key).map_or_else(
        || extract_slot_arg(args, 1),
        |value| Slot::Reference(Some(heap.allocate_string(value))),
    );
    Ok(Some(result))
}
/// Native: `System.getSecurityManager()SecurityManager` - Duke runs without a security manager.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_system_get_security_manager(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Reference(None)))
}
/// Native: `System.lineSeparator()String` — returns the platform line separator.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_system_line_separator(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate_string("\n".to_string());
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `System.identityHashCode(Object)I` — returns a stable, non-negative
/// identity hash. Backed by the heap's relocation-stable identity hash so the
/// value does not change when a minor GC moves the object; `null` hashes to 0.
pub(crate) fn native_system_identity_hash_code(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let hash = match args.first() {
        Some(Slot::Reference(Some(r))) => heap.identity_hash(*r)? & 0x7FFF_FFFF,
        _ => 0,
    };
    Ok(Some(Slot::Int(hash)))
}
#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_system_current_time_millis(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Long(system_time_to_epoch_millis(
        std::time::SystemTime::now(),
    ))))
}
#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_system_nano_time(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Long(monotonic_nano_time_now())))
}
pub(crate) fn native_thread_current_thread(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = heap.allocate("java/lang/Thread".to_string(), 5);
    let thread = heap.get_mut(thread_ref)?;
    thread.fields[THREAD_TARGET_SLOT] = Slot::Reference(None);
    thread.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    thread.fields[THREAD_INTERRUPTED_SLOT] = Slot::Int(i32::from(current_host_thread_is_interrupted()));
    thread.fields[THREAD_HOST_KEY_SLOT] = Slot::Int(java_host_key_for_current_host().unwrap_or(-1));
    thread.fields[THREAD_CONTEXT_CLASS_LOADER_SLOT] = Slot::Reference(None);
    Ok(Some(Slot::Reference(Some(thread_ref))))
}
#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_thread_get_name(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let name_ref = heap.allocate_string("main".to_string());
    Ok(Some(Slot::Reference(Some(name_ref))))
}
pub(crate) fn native_thread_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let this = heap.get_mut(this_ref)?;
    this.fields[THREAD_TARGET_SLOT] = Slot::Reference(None);
    this.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    this.fields[THREAD_INTERRUPTED_SLOT] = Slot::Int(0);
    this.fields[THREAD_HOST_KEY_SLOT] = Slot::Int(-1);
    this.fields[THREAD_CONTEXT_CLASS_LOADER_SLOT] = Slot::Reference(None);
    Ok(None)
}
pub(crate) fn native_thread_init_runnable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let this = heap.get_mut(this_ref)?;
    this.fields[THREAD_TARGET_SLOT] = target;
    this.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    this.fields[THREAD_INTERRUPTED_SLOT] = Slot::Int(0);
    this.fields[THREAD_HOST_KEY_SLOT] = Slot::Int(-1);
    this.fields[THREAD_CONTEXT_CLASS_LOADER_SLOT] = Slot::Reference(None);
    Ok(None)
}
pub(crate) fn native_thread_start(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    control.request(NativeThreadAction::Start { thread_ref });
    Ok(None)
}
pub(crate) fn native_thread_join(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let thread = heap.get(thread_ref)?;
    let Some(Slot::Int(thread_id)) = thread.fields.get(THREAD_ID_SLOT) else {
        return Ok(None);
    };
    if *thread_id >= 0 {
        control.request(NativeThreadAction::Join {
            thread_id: *thread_id,
        });
    }
    Ok(None)
}
pub(crate) fn native_thread_sleep(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let millis = match args.first() {
        Some(Slot::Long(value)) => *value,
        _ => {
            return Err(Error::TypeMismatch {
                expected: "long",
                got: "other",
            });
        }
    };
    let millis = u64::try_from(millis.max(0)).unwrap_or(0);
    control.request(NativeThreadAction::Sleep(std::time::Duration::from_millis(
        millis,
    )));
    Ok(None)
}
pub(crate) fn native_thread_interrupt(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let host_key = match heap.get(thread_ref)?.fields.get(THREAD_HOST_KEY_SLOT) {
        Some(Slot::Int(host_key)) => *host_key,
        _ => -1,
    };
    heap.write_field(thread_ref, THREAD_INTERRUPTED_SLOT, Slot::Int(1))?;
    if let Some(host_thread_id) = host_thread_for_java_thread(host_key) {
        interrupt_host_thread(host_thread_id);
    }
    Ok(None)
}
pub(crate) fn native_thread_is_interrupted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let field_interrupted = matches!(
        heap.get(thread_ref)?.fields.get(THREAD_INTERRUPTED_SLOT),
        Some(Slot::Int(value)) if *value != 0
    );
    let host_key = match heap.get(thread_ref)?.fields.get(THREAD_HOST_KEY_SLOT) {
        Some(Slot::Int(host_key)) => *host_key,
        _ => -1,
    };
    let host_interrupted = host_thread_for_java_thread(host_key)
        .is_some_and(|host_thread_id| {
            interrupted_host_threads()
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .contains(&host_thread_id)
        });
    Ok(Some(Slot::Int(i32::from(
        field_interrupted || host_interrupted,
    ))))
}
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_thread_interrupted(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Int(i32::from(
        take_current_host_thread_interrupted(),
    ))))
}
pub(crate) fn native_thread_get_context_class_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let loader = heap
        .get(thread_ref)?
        .fields
        .get(THREAD_CONTEXT_CLASS_LOADER_SLOT)
        .copied()
        .unwrap_or(Slot::Reference(None));
    Ok(Some(loader))
}
pub(crate) fn native_thread_set_context_class_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let loader = extract_slot_arg(args, 1);
    heap.write_field(thread_ref, THREAD_CONTEXT_CLASS_LOADER_SLOT, loader)?;
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
    let safe_from = if s.is_char_boundary(from) {
        from
    } else {
        let mut idx = from;
        while idx < s.len() && !s.is_char_boundary(idx) {
            idx += 1;
        }
        idx
    };
    let search_in = if safe_from < s.len() { &s[safe_from..] } else { "" };
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let result = search_in.find(target).map_or(-1, |i| (safe_from + i) as i32);
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
        let max_byte = from + sub_str.len();
        if this_str.is_char_boundary(max_byte) {
            &this_str[..max_byte]
        } else {
            // Find the nearest char boundary safely
            let mut safe_idx = max_byte;
            while safe_idx > 0 && !this_str.is_char_boundary(safe_idx) {
                safe_idx -= 1;
            }
            &this_str[..safe_idx]
        }
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
/// Native: `Integer.parseInt(String)` — parses string to int.
pub(crate) fn native_integer_parseint(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(args, heap, i32::MIN, i32::MAX)?;
    Ok(Some(Slot::Int(val)))
}
/// Native: `Integer.parseInt(String,int)` — parses string to int with radix.
pub(crate) fn native_integer_parseint_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(args, heap, i32::MIN, i32::MAX)?;
    Ok(Some(Slot::Int(val)))
}
/// Native: `Integer.valueOf(int)` — boxes int into Integer object.
pub(crate) fn native_integer_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.valueOf(String)` — parses and boxes int.
pub(crate) fn native_integer_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(args, heap, i32::MIN, i32::MAX)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.valueOf(String,int)` — parses and boxes int with radix.
pub(crate) fn native_integer_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(args, heap, i32::MIN, i32::MAX)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.decode(String)` — parses prefixed string and boxes int.
pub(crate) fn native_integer_decode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i32_decode_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.intValue()` — unboxes Integer to int.
pub(crate) fn native_integer_intvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
/// Native: `Integer.toString(int)` — static, converts int to String.
pub(crate) fn native_integer_tostring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.toHexString(int)` — unsigned lowercase hex string.
pub(crate) fn native_integer_tohexstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let r = heap.allocate_string(format!("{val:x}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.toOctalString(int)` — unsigned octal string.
pub(crate) fn native_integer_tooctalstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let r = heap.allocate_string(format!("{val:o}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.toBinaryString(int)` — unsigned binary string.
pub(crate) fn native_integer_tobinarystring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let r = heap.allocate_string(format!("{val:b}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Integer.toUnsignedLong(int)` — widen via unsigned 32-bit interpretation.
pub(crate) fn native_integer_tounsignedlong_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    Ok(Some(Slot::Long(i64::from(val))))
}
/// Native: `Integer.compareUnsigned(int,int)` — compares ints as unsigned 32-bit values.
pub(crate) fn native_integer_compareunsigned_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let b = u32::from_ne_bytes(extract_int_arg(args, 1)?.to_ne_bytes());
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Integer.compareTo(Object)` — compares two boxed Integers.
pub(crate) fn native_integer_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let int_val = |s: &Slot| -> Result<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => int_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = int_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Integer.bitCount(int)` — count number of set bits (popcount).
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
pub(crate) fn native_integer_bitcount(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.count_ones() as i32)))
}
/// Native: `Integer.numberOfLeadingZeros(int)` — count leading zero bits.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
pub(crate) fn native_integer_leading_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.leading_zeros() as i32)))
}
/// Native: `Integer.numberOfTrailingZeros(int)` — count trailing zero bits.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
pub(crate) fn native_integer_trailing_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.trailing_zeros() as i32)))
}
/// Native: `Integer.highestOneBit(int)` — return value with only the highest set bit.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_integer_highest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    let result = if v == 0 { 0u32 } else { 1u32 << v.ilog2() };
    Ok(Some(Slot::Int(result as i32)))
}
/// Native: `Integer.lowestOneBit(int)` — return value with only the lowest set bit.
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn native_integer_lowest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(v & v.wrapping_neg())))
}
/// Native: `Integer.reverse(int)` — reverse bit order.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_integer_reverse(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.reverse_bits() as i32)))
}
/// Native: `Integer.reverseBytes(int)` — reverse byte order (swap endianness).
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_integer_reverse_bytes(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.swap_bytes() as i32)))
}
/// Native: `Integer.signum(int)` — returns -1, 0, or 1.
pub(crate) fn native_integer_signum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(v.signum())))
}
/// Native: `Integer.compare(int, int)` — static two-value comparison.
pub(crate) fn native_integer_compare_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Integer.sum(int, int)` — static addition (functional interface target).
pub(crate) fn native_integer_sum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.wrapping_add(b))))
}
/// Native: `Integer.max(int, int)` — static max (functional interface target).
pub(crate) fn native_integer_max_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.max(b))))
}
/// Native: `Integer.min(int, int)` — static min (functional interface target).
pub(crate) fn native_integer_min_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.min(b))))
}
/// Native: `Long.bitCount(long)` — count number of set bits.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
pub(crate) fn native_long_bitcount(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Int(v.count_ones() as i32)))
}
/// Native: `Long.numberOfLeadingZeros(long)` — count leading zero bits.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
pub(crate) fn native_long_leading_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Int(v.leading_zeros() as i32)))
}
/// Native: `Long.numberOfTrailingZeros(long)` — count trailing zero bits.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
pub(crate) fn native_long_trailing_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Int(v.trailing_zeros() as i32)))
}
/// Native: `Long.highestOneBit(long)` — return value with only the highest set bit.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_long_highest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    let result = if v == 0 { 0u64 } else { 1u64 << v.ilog2() };
    Ok(Some(Slot::Long(result as i64)))
}
/// Native: `Long.lowestOneBit(long)` — return value with only the lowest set bit.
pub(crate) fn native_long_lowest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Long(v & v.wrapping_neg())))
}
/// Native: `Long.reverse(long)` — reverse bit order.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_long_reverse(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Long(v.reverse_bits() as i64)))
}
/// Native: `Long.reverseBytes(long)` — reverse byte order (swap endianness).
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) fn native_long_reverse_bytes(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Long(v.swap_bytes() as i64)))
}
/// Native: `Long.signum(long)` — returns -1, 0, or 1.
pub(crate) fn native_long_signum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let v = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Int(v.signum() as i32)))
}
/// Native: `Long.compare(long, long)` — static two-value comparison.
pub(crate) fn native_long_compare_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Long.sum(long, long)` — static addition (functional interface target).
pub(crate) fn native_long_sum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(a.wrapping_add(b))))
}
/// Native: `Math.random()D` — returns a pseudo-random double in [0.0, 1.0).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_math_random(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // Use a simple deterministic seed based on stack pointer heuristic
    // For a JVM interpreter we just use a fixed-seed LCG for reproducibility
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(12345);
    let old = SEED.load(Ordering::Relaxed);
    let new = old.wrapping_mul(25_214_903_917).wrapping_add(11) & 0x0000_FFFF_FFFF_FFFF;
    SEED.store(new, Ordering::Relaxed);
    #[allow(clippy::cast_precision_loss)]
    let v = (new as f64) / (1_u64 << 48) as f64;
    Ok(Some(Slot::Double(v)))
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
    // ⚡ Bolt: Eliminate intermediate format! allocation
    let mut combined = String::with_capacity(s1.len() + s2.len());
    combined.push_str(&s1);
    combined.push_str(&s2);
    let r = heap.allocate_string(combined);
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
/// Native: `Math.max(int, int)` — returns the larger value.
pub(crate) fn native_math_max_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.max(b))))
}
/// Native: `Math.min(int, int)` — returns the smaller value.
pub(crate) fn native_math_min_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.min(b))))
}
/// Native: `Math.abs(int)` — returns absolute value.
pub(crate) fn native_math_abs_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(a.wrapping_abs())))
}
/// Native: `Math.floorMod(int, int)` — remainder with the divisor's sign.
pub(crate) fn native_math_floor_mod_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    if b == 0 {
        return Err(Error::DivisionByZero);
    }
    let remainder = a.wrapping_rem(b);
    let floor_mod = if remainder != 0 && (remainder < 0) != (b < 0) {
        remainder.wrapping_add(b)
    } else {
        remainder
    };
    Ok(Some(Slot::Int(floor_mod)))
}
/// Native: `Math.sqrt(double)` — returns square root.
pub(crate) fn native_math_sqrt(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.sqrt())))
}
/// Native: `Math.pow(double, double)` — returns a raised to the power b.
pub(crate) fn native_math_pow(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    let b = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(a.powf(b))))
}
/// Native: `Math.floor(double)` — returns floor value.
pub(crate) fn native_math_floor(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.floor())))
}
/// Native: `Math.ceil(double)` — returns ceiling value.
pub(crate) fn native_math_ceil(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.ceil())))
}
/// Native: `Math.round(double)` — returns closest long.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_math_round_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Long(a.round() as i64)))
}
/// Native: `Math.abs(long)` — returns absolute value.
pub(crate) fn native_math_abs_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Long(a.wrapping_abs())))
}
/// Native: `Math.abs(double)` — returns absolute value.
pub(crate) fn native_math_abs_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.abs())))
}
/// Native: `Math.max(long, long)` — returns the larger value.
pub(crate) fn native_math_max_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(a.max(b))))
}
/// Native: `Math.min(long, long)` — returns the smaller value.
pub(crate) fn native_math_min_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(a.min(b))))
}
/// Native: `Math.max(double, double)` — returns the larger value.
pub(crate) fn native_math_max_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    let b = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(a.max(b))))
}
/// Native: `Math.min(double, double)` — returns the smaller value.
pub(crate) fn native_math_min_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    let b = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(a.min(b))))
}
/// Native: `Math.sin(double)` — sine (argument in radians).
pub(crate) fn native_math_sin(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.sin())))
}
/// Native: `Math.cos(double)` — cosine (argument in radians).
pub(crate) fn native_math_cos(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.cos())))
}
/// Native: `Math.tan(double)` — tangent (argument in radians).
pub(crate) fn native_math_tan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.tan())))
}
/// Native: `Math.asin(double)` — arc sine, result in [-π/2, π/2].
pub(crate) fn native_math_asin(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.asin())))
}
/// Native: `Math.acos(double)` — arc cosine, result in [0, π].
pub(crate) fn native_math_acos(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.acos())))
}
/// Native: `Math.atan(double)` — arc tangent, result in [-π/2, π/2].
pub(crate) fn native_math_atan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.atan())))
}
/// Native: `Math.atan2(double, double)` — angle of vector (y, x) in [-π, π].
pub(crate) fn native_math_atan2(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let y = extract_double_arg(args, 0)?;
    let x = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(y.atan2(x))))
}
/// Native: `Math.log(double)` — natural logarithm.
pub(crate) fn native_math_log(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.ln())))
}
/// Native: `Math.log10(double)` — base-10 logarithm.
pub(crate) fn native_math_log10(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.log10())))
}
/// Native: `Math.exp(double)` — Euler's number raised to the given power.
pub(crate) fn native_math_exp(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.exp())))
}
/// Native: `Math.signum(double)` — sign of a: -1.0, 0.0, or 1.0.
pub(crate) fn native_math_signum_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.signum())))
}
/// Native: `Math.signum(float)` — sign of a as float: -1.0, 0.0, or 1.0.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_math_signum_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_float_arg(args, 0)?;
    Ok(Some(Slot::Float(a.signum())))
}
/// Native: `Math.toRadians(double)` — converts degrees to radians.
pub(crate) fn native_math_to_radians(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.to_radians())))
}
/// Native: `Math.toDegrees(double)` — converts radians to degrees.
pub(crate) fn native_math_to_degrees(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.to_degrees())))
}
/// Native: `Math.cbrt(double)` — cube root.
pub(crate) fn native_math_cbrt(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.cbrt())))
}
/// Native: `Math.hypot(double, double)` — sqrt(x²+y²) without overflow.
pub(crate) fn native_math_hypot(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let x = extract_double_arg(args, 0)?;
    let y = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(x.hypot(y))))
}
/// Native: `Math.floorDiv(int, int)` — largest int ≤ quotient.
pub(crate) fn native_math_floor_div_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    if b == 0 {
        return Err(Error::DivisionByZero);
    }
    Ok(Some(Slot::Int(
        a.wrapping_div_euclid(b) - i32::from(a.wrapping_rem(b) != 0 && (a < 0) != (b < 0)),
    )))
}
/// Native: `Math.round(float)` — rounds float to nearest int.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_math_round_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = extract_float_arg(args, 0)?;
    Ok(Some(Slot::Int(a.round() as i32)))
}
/// Native: `System.arraycopy(Object src, int srcPos, Object dst, int dstPos, int length)`.
/// Copies `length` elements from `src` starting at `srcPos` into `dst` starting at `dstPos`.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_system_arraycopy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let src_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let src_pos = extract_int_arg(args, 1)?;
    let dst_ref = match args.get(2) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let dst_pos = extract_int_arg(args, 3)?;
    let length = extract_int_arg(args, 4)?;
    if length < 0 || src_pos < 0 || dst_pos < 0 {
        return Err(Error::NegativeArraySize {
            size: length.min(src_pos).min(dst_pos),
        });
    }
    let src_pos = src_pos as usize;
    let dst_pos = dst_pos as usize;
    let length = length as usize;
    // Copy elements one by one to support src == dst (overlapping ranges handled via clone).
    let src_len = heap.get(src_ref)?.fields.len();
    if src_pos + length > src_len {
        return Err(Error::ArrayIndexOutOfBounds {
            index: i32::try_from(src_pos + length - 1).unwrap_or(i32::MAX),
            length: src_len,
        });
    }
    let src_elems: Vec<Slot> = heap.get(src_ref)?.fields[src_pos..src_pos + length].to_vec();
    let dst_len = heap.get(dst_ref)?.fields.len();
    if dst_pos + length > dst_len {
        return Err(Error::ArrayIndexOutOfBounds {
            index: i32::try_from(dst_pos + length - 1).unwrap_or(i32::MAX),
            length: dst_len,
        });
    }
    let dst_fields = &mut heap.get_mut(dst_ref)?.fields;
    for (i, slot) in src_elems.into_iter().enumerate() {
        dst_fields[dst_pos + i] = slot;
    }
    Ok(None)
}
/// Native: `Long.parseLong(String)` — parses string to long.
pub(crate) fn native_long_parselong(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_from_string_arg(args, heap)?;
    Ok(Some(Slot::Long(val)))
}
/// Native: `Long.parseLong(String,int)` — parses string to long with radix.
pub(crate) fn native_long_parselong_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_from_string_and_radix_args(args, heap)?;
    Ok(Some(Slot::Long(val)))
}
/// Native: `Long.valueOf(long)` — boxes long into Long object.
pub(crate) fn native_long_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.valueOf(String)` — parses and boxes long.
pub(crate) fn native_long_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.valueOf(String,int)` — parses and boxes long with radix.
pub(crate) fn native_long_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_from_string_and_radix_args(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.decode(String)` — parses prefixed string and boxes long.
pub(crate) fn native_long_decode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_i64_decode_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.longValue()` — unboxes Long to long.
pub(crate) fn native_long_longvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
/// Native: `Long.intValue()I` — returns the long value narrowed to int.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_long_intvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = match heap.get(this_ref)?.fields.first() {
        #[allow(clippy::cast_possible_truncation)] // enum ordinals fit i32
        Some(Slot::Long(v)) => *v as i32,
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(val)))
}
/// Native: `Long.toString(long)` — static, converts long to String.
pub(crate) fn native_long_tostring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.toHexString(long)` — unsigned lowercase hex string.
pub(crate) fn native_long_tohexstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(format!("{val:x}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.toOctalString(long)` — unsigned octal string.
pub(crate) fn native_long_tooctalstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(format!("{val:o}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.toBinaryString(long)` — unsigned binary string.
pub(crate) fn native_long_tobinarystring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(format!("{val:b}"));
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Long.compareUnsigned(long,long)` — compares longs as unsigned 64-bit values.
pub(crate) fn native_long_compareunsigned_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let a = u64::from_ne_bytes(extract_long_arg(args, 0)?.to_ne_bytes());
    let b = u64::from_ne_bytes(extract_long_arg(args, 1)?.to_ne_bytes());
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Long.compareTo(Object)` — compares two boxed Longs.
pub(crate) fn native_long_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let long_val = |s: &Slot| -> Result<i64> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Long(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => long_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = long_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Double.parseDouble(String)` — parses string to double.
pub(crate) fn native_double_parsedouble(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f64 = s.trim().parse().map_err(|_| Error::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Double(val)))
}
/// Native: `Double.valueOf(double)` — boxes double into Double object.
pub(crate) fn native_double_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_double_arg(args, 0)?;
    let r = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Double(val);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Double.doubleValue()` — unboxes Double to double.
pub(crate) fn native_double_doublevalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
pub(crate) fn native_float_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_float_arg(args, 0)?;
    let r = heap.allocate("java/lang/Float".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Float(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_float_floatvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
/// Native: `Float.floatToRawIntBits(float)` — reinterprets the float's bits as
/// an int without NaN canonicalization.
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn native_float_float_to_raw_int_bits(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let f = extract_float_arg(args, 0)?;
    Ok(Some(Slot::Int(f.to_bits() as i32)))
}
/// Native: `Float.floatToIntBits(float)` — like `floatToRawIntBits` but
/// collapses all NaN encodings to the canonical `0x7fc00000`.
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn native_float_float_to_int_bits(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let f = extract_float_arg(args, 0)?;
    let bits = if f.is_nan() { 0x7fc0_0000 } else { f.to_bits() };
    Ok(Some(Slot::Int(bits as i32)))
}
/// Native: `Float.intBitsToFloat(int)` — reinterprets the int's bits as a float.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_float_int_bits_to_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let i = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Float(f32::from_bits(i as u32))))
}
/// Native: `Double.doubleToRawLongBits(double)` — reinterprets the double's
/// bits as a long without NaN canonicalization.
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn native_double_double_to_raw_long_bits(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let d = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Long(d.to_bits() as i64)))
}
/// Native: `Double.doubleToLongBits(double)` — like `doubleToRawLongBits` but
/// collapses all NaN encodings to the canonical `0x7ff8000000000000`.
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn native_double_double_to_long_bits(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let d = extract_double_arg(args, 0)?;
    let bits = if d.is_nan() {
        0x7ff8_0000_0000_0000
    } else {
        d.to_bits()
    };
    Ok(Some(Slot::Long(bits as i64)))
}
/// Native: `Double.longBitsToDouble(long)` — reinterprets the long's bits as a
/// double.
#[allow(clippy::cast_sign_loss)]
pub(crate) fn native_double_long_bits_to_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let l = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Double(f64::from_bits(l as u64))))
}
pub(crate) fn native_float_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let float_val = |s: &Slot| -> Result<f32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Float(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => float_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = float_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.total_cmp(&b)))))
}
/// Native: `Float.parseFloat(String)` — parses string to float.
pub(crate) fn native_float_parsefloat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f32 = s.trim().parse().map_err(|_| Error::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Float(val)))
}
pub(crate) fn native_boolean_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Boolean".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(i32::from(val != 0));
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_boolean_booleanvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
pub(crate) fn native_boolean_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bool_val = |s: &Slot| -> Result<bool> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n != 0),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => bool_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = bool_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Boolean.parseBoolean(String)` — case-insensitive "true" → 1, else 0.
pub(crate) fn native_boolean_parseboolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let s = heap.get(*r)?.string_value.clone().unwrap_or_default();
            let val = s.eq_ignore_ascii_case("true");
            Ok(Some(Slot::Int(i32::from(val))))
        }
        Some(Slot::Reference(None)) => Ok(Some(Slot::Int(0))),
        _ => Err(Error::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}
pub(crate) fn native_byte_parsebyte(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i8::MIN), i32::from(i8::MAX))?;
    Ok(Some(Slot::Int(val)))
}
pub(crate) fn native_byte_parsebyte_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i8::MIN),
        i32::from(i8::MAX),
    )?;
    Ok(Some(Slot::Int(val)))
}
pub(crate) fn native_byte_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_byte_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i8::MIN), i32::from(i8::MAX))?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_byte_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i8::MIN),
        i32::from(i8::MAX),
    )?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_byte_bytevalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
pub(crate) fn native_byte_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let byte_val = |s: &Slot| -> Result<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => byte_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = byte_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
pub(crate) fn native_short_parseshort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i16::MIN), i32::from(i16::MAX))?;
    Ok(Some(Slot::Int(val)))
}
pub(crate) fn native_short_parseshort_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i16::MIN),
        i32::from(i16::MAX),
    )?;
    Ok(Some(Slot::Int(val)))
}
pub(crate) fn native_short_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_short_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i16::MIN), i32::from(i16::MAX))?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_short_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i16::MIN),
        i32::from(i16::MAX),
    )?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_short_shortvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
pub(crate) fn native_short_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let short_val = |s: &Slot| -> Result<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(Error::InvalidRef { address: *r }),
            },
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => short_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = short_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
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
/// Native: `StringBuilder.<init>(I)V` — initialise empty buffer, ignoring the initial capacity.
pub(crate) fn native_sb_init_with_capacity(
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
/// Native: `StringBuilder.append(Ljava/lang/CharSequence;II)Ljava/lang/StringBuilder;`
/// Appends the subsequence `[start, end)` of the given `CharSequence`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn native_sb_append_charsequence_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let begin = extract_int_arg(args, 2)? as usize;
    let end = extract_int_arg(args, 3)? as usize;
    let sub = match args.get(1) {
        Some(Slot::Reference(Some(r))) => {
            let s = heap.get(*r)?.string_value.clone().unwrap_or_default();
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
        }
        _ => "null".to_string(),
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&sub);
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
                let max_idx = size_val.min(obj.fields.len().saturating_sub(1));
                let elems: Vec<Slot> = if max_idx >= 1 {
                    obj.fields[1..=max_idx].to_vec()
                } else {
                    Vec::new()
                };
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
/// Native: `Runtime.availableProcessors()` — number of processors available to
/// the JVM. The `this` receiver is ignored.
#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_runtime_available_processors(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let n = std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get) as i32;
    Ok(Some(Slot::Int(n)))
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