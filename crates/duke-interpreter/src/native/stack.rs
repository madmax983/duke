fn stack_trace_element_text(heap: &duke_gc::Heap, element_ref: u64) -> Result<String> {
    let fields = heap.get(element_ref)?.fields.clone();
    let class_name = slot_string(
            heap,
            fields.first().copied().unwrap_or(Slot::Reference(None)),
        )?
        .unwrap_or_default();
    let method_name = slot_string(
            heap,
            fields.get(1).copied().unwrap_or(Slot::Reference(None)),
        )?
        .unwrap_or_default();
    let file_name = slot_string(
        heap,
        fields.get(2).copied().unwrap_or(Slot::Reference(None)),
    )?;
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
fn stack_trace_element_field(
    args: &[Slot],
    heap: &duke_gc::Heap,
    index: usize,
) -> Result<Slot> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(heap.get(this_ref)?.fields.get(index).copied().unwrap_or(Slot::Reference(None)))
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
    let is_native = matches!(stack_trace_element_field(args, heap, 3) ?, Slot::Int(- 2));
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
/// Native: `Stack.<init>()V`
pub(crate) fn native_stack_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}
/// Native: `Stack.push(E)E` — appends to tail, returns the element.
pub(crate) fn native_stack_push(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let elem = extract_slot_arg(args, 1);
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(elem))
}
/// Native: `Stack.pop()E` — removes and returns the top element.
pub(crate) fn native_stack_pop(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_linked_list_remove_last(args, heap, out, control)
}
/// Native: `Stack.peek()E` — returns the top element without removal.
pub(crate) fn native_stack_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_linked_list_peek_last(args, heap, out, control)
}
/// Native: `Stack.empty()Z` — returns true if the stack is empty.
pub(crate) fn native_stack_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}
/// Native: `Stack.size()I`
pub(crate) fn native_stack_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}
fn stack_frame_for_method(
    registry: &ClassRegistry,
    class_name: &str,
    method_idx: usize,
    bci: usize,
) -> Option<NativeStackFrame> {
    let method = registry.get(class_name).ok()?.methods.get(method_idx)?;
    let line_number = if method.is_native {
        -2
    } else {
        line_number_for_bci(&method.line_number_table, bci)
    };
    Some(NativeStackFrame {
        class_name: class_name.to_string(),
        method_name: method.name.clone(),
        file_name: method.source_file.clone(),
        line_number,
    })
}
