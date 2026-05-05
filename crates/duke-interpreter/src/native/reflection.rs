pub(crate) fn native_reflection_member_get_declaring_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_declaring_class_slot(heap, member_ref)?))
}
pub(crate) fn native_reflection_member_set_accessible(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    let accessible = extract_int_arg(args, 1)? != 0;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_ACCESSIBLE_FIELD,
        Slot::Int(i32::from(accessible)),
    )?;
    Ok(None)
}
fn reflection_member_name_slot(heap: &duke_gc::Heap, member_ref: u64) -> Result<Slot> {
    heap.get(member_ref)?
        .fields
        .get(REFLECTION_MEMBER_NAME_FIELD)
        .copied()
        .ok_or(Error::InvalidRef {
            address: member_ref,
        })
}
fn reflection_member_declaring_class_slot(
    heap: &duke_gc::Heap,
    member_ref: u64,
) -> Result<Slot> {
    heap.get(member_ref)?
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied()
        .ok_or(Error::InvalidRef {
            address: member_ref,
        })
}
fn reflection_member_declaring_class_name_slot(
    heap: &mut duke_gc::Heap,
    member_ref: u64,
) -> Result<Slot> {
    let Slot::Reference(Some(class_ref)) = reflection_member_declaring_class_slot(
        heap,
        member_ref,
    )? else {
        return Err(Error::InvalidRef {
            address: member_ref,
        });
    };
    let class_key = class_key_from_ref(heap, class_ref)?;
    let binary_name = internal_name_to_binary_name(
        class_internal_name_from_key(&class_key),
    );
    let name_ref = heap.allocate_string(binary_name);
    Ok(Slot::Reference(Some(name_ref)))
}
fn reflection_array_elements(
    heap: &duke_gc::Heap,
    args_slot: Slot,
) -> Result<Vec<Slot>> {
    match args_slot {
        Slot::Reference(None) => Ok(Vec::new()),
        Slot::Reference(Some(array_ref)) => Ok(heap.get(array_ref)?.fields.clone()),
        _ => {
            Err(Error::TypeMismatch {
                expected: "reference array",
                got: "other",
            })
        }
    }
}
