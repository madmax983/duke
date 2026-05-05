pub(crate) fn native_reflect_method_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_name_slot(heap, method_ref)?))
}
pub(crate) fn native_reflect_method_get_annotations(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let annotations = annotations_for_reflected_method(heap, method_ref, ops)?;
    allocate_annotation_array(heap, out, ops, &annotations)
}
pub(crate) fn native_reflect_method_get_declared_annotations(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_reflect_method_get_annotations(args, heap, out, control, ops)
}
pub(crate) fn native_reflect_method_get_annotation(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let annotation_type_ref = extract_ref_arg(args, 1)?;
    let requested_type = class_key_from_ref(heap, annotation_type_ref)?;
    let annotations = annotations_for_reflected_method(heap, method_ref, ops)?;
    match find_annotation(&annotations, &requested_type) {
        Some(annotation) => {
            let annotation_ref = allocate_annotation_proxy(heap, out, ops, annotation)?;
            Ok(Some(Slot::Reference(Some(annotation_ref))))
        }
        None => Ok(Some(Slot::Reference(None))),
    }
}
pub(crate) fn native_reflect_constructor_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let constructor_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_declaring_class_name_slot(heap, constructor_ref)?))
}
pub(crate) fn native_reflect_method_get_return_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let method = reflected_method_handle(heap, method_ref)?;
    Ok(
        Some(
            descriptor_class_slot_from_source(
                heap,
                ops,
                method_return_descriptor(&method.descriptor),
                Some(method.declaring_class_key.as_str()),
            )?,
        ),
    )
}
pub(crate) fn native_reflect_field_get_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let field = reflected_field_handle(heap, field_ref)?;
    Ok(
        Some(
            descriptor_class_slot_from_source(
                heap,
                ops,
                &field.descriptor,
                Some(field.declaring_class_key.as_str()),
            )?,
        ),
    )
}
pub(crate) fn native_reflect_method_get_parameter_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let method = reflected_method_handle(heap, method_ref)?;
    let count = i32::try_from(parse_arg_count(&method.descriptor)).unwrap_or(i32::MAX);
    Ok(Some(Slot::Int(count)))
}
pub(crate) fn native_reflect_executable_get_parameter_types(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    let member = reflected_method_handle(heap, member_ref)?;
    let parameter_descriptors = parse_arg_descriptors(&member.descriptor);
    let mut class_refs = Vec::with_capacity(parameter_descriptors.len());
    for descriptor in parameter_descriptors {
        let Slot::Reference(Some(class_ref)) = descriptor_class_slot_from_source(
            heap,
            ops,
            &descriptor,
            Some(member.declaring_class_key.as_str()),
        )? else {
            return Err(Error::TypeMismatch {
                expected: "class reference",
                got: "other",
            });
        };
        class_refs.push(class_ref);
    }
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/Class;", &class_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}
pub(crate) fn native_reflect_field_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_name_slot(heap, field_ref)?))
}
pub(crate) fn native_reflect_field_get_annotations(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let annotations = annotations_for_reflected_field(heap, field_ref, ops)?;
    allocate_annotation_array(heap, out, ops, &annotations)
}
pub(crate) fn native_reflect_field_get_declared_annotations(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_reflect_field_get_annotations(args, heap, out, control, ops)
}
pub(crate) fn native_reflect_field_get_annotation(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let annotation_type_ref = extract_ref_arg(args, 1)?;
    let requested_type = class_key_from_ref(heap, annotation_type_ref)?;
    let annotations = annotations_for_reflected_field(heap, field_ref, ops)?;
    match find_annotation(&annotations, &requested_type) {
        Some(annotation) => {
            let annotation_ref = allocate_annotation_proxy(heap, out, ops, annotation)?;
            Ok(Some(Slot::Reference(Some(annotation_ref))))
        }
        None => Ok(Some(Slot::Reference(None))),
    }
}
pub(crate) fn native_reflect_field_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let field = reflected_field_handle(heap, field_ref)?;
    if !field.is_public && !field.is_accessible {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }
    let field_type = field.descriptor.chars().next().unwrap_or('L');
    let raw_value = if field.is_static {
        ops.ensure_class_initialized(heap, output, &field.declaring_class_key)?;
        ops.read_static_field(&field.declaring_class_key, &field.field_name)?
    } else {
        let Slot::Reference(Some(target_ref)) = target_slot else {
            return Err(Error::NullPointerException);
        };
        ops.read_instance_field(
            heap,
            target_ref,
            &field.declaring_class_key,
            &field.field_name,
        )?
    };
    Ok(Some(box_reflection_return_value(heap, field_type, Some(raw_value))?))
}
pub(crate) fn native_reflect_field_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let value_slot = extract_slot_arg(args, 2);
    let field = reflected_field_handle(heap, field_ref)?;
    if !field.is_public && !field.is_accessible {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }
    let field_type = field.descriptor.chars().next().unwrap_or('L');
    let value = unbox_reflection_argument(heap, field_type, value_slot)?;
    if field.is_static {
        ops.ensure_class_initialized(heap, output, &field.declaring_class_key)?;
        ops.write_static_field(&field.declaring_class_key, &field.field_name, value)?;
        return Ok(None);
    }
    let Slot::Reference(Some(target_ref)) = target_slot else {
        return Err(Error::NullPointerException);
    };
    ops.write_instance_field(
        heap,
        target_ref,
        &field.declaring_class_key,
        &field.field_name,
        value,
    )?;
    Ok(None)
}
pub(crate) fn native_reflect_method_invoke(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let invoke_arg_slots = reflection_array_elements(heap, extract_slot_arg(args, 2))?;
    let method = reflected_method_handle(heap, method_ref)?;
    if !method.is_public && !method.is_accessible {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }
    let invoke_args = build_reflection_invoke_args(
        heap,
        target_slot,
        &method.descriptor,
        invoke_arg_slots,
        method.is_static,
    )?;
    ops.ensure_loaded(&method.declaring_class_key)?;
    match ops
        .invoke(
            heap,
            output,
            &method.declaring_class_key,
            &method.method_name,
            &method.descriptor,
            invoke_args,
        )
    {
        Ok(result) => {
            Ok(
                Some(
                    box_reflection_return_value(
                        heap,
                        descriptor_return_type(&method.descriptor),
                        result,
                    )?,
                ),
            )
        }
        Err(Error::JavaException { .. }) => {
            Err(Error::JavaException {
                class_name: "java/lang/reflect/InvocationTargetException".to_string(),
            })
        }
        Err(err) => Err(err),
    }
}
pub(crate) fn native_reflect_constructor_new_instance(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let constructor_ref = extract_ref_arg(args, 0)?;
    let invoke_arg_slots = reflection_array_elements(heap, extract_slot_arg(args, 1))?;
    let constructor = reflected_method_handle(heap, constructor_ref)?;
    if !constructor.is_public && !constructor.is_accessible {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }
    let instance_ref = ops
        .allocate_instance(heap, output, &constructor.declaring_class_key)?;
    let invoke_args = build_reflection_invoke_args(
        heap,
        Slot::Reference(Some(instance_ref)),
        &constructor.descriptor,
        invoke_arg_slots,
        false,
    )?;
    match ops
        .invoke(
            heap,
            output,
            &constructor.declaring_class_key,
            &constructor.method_name,
            &constructor.descriptor,
            invoke_args,
        )
    {
        Ok(_) => Ok(Some(Slot::Reference(Some(instance_ref)))),
        Err(Error::JavaException { .. }) => {
            Err(Error::JavaException {
                class_name: "java/lang/reflect/InvocationTargetException".to_string(),
            })
        }
        Err(err) => Err(err),
    }
}
