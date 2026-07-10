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
    Ok(Some(reflection_member_declaring_class_name_slot(
        heap,
        constructor_ref,
    )?))
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
    Ok(Some(descriptor_class_slot_from_source(
        heap,
        ops,
        method_return_descriptor(&method.descriptor),
        Some(method.declaring_class_key.as_str()),
    )?))
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
    Ok(Some(descriptor_class_slot_from_source(
        heap,
        ops,
        &field.descriptor,
        Some(field.declaring_class_key.as_str()),
    )?))
}
pub(crate) fn native_reflection_member_get_declaring_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_declaring_class_slot(
        heap, member_ref,
    )?))
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
        )?
        else {
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

    Ok(Some(box_reflection_return_value(
        heap,
        field_type,
        Some(raw_value),
    )?))
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
    match ops.invoke(
        heap,
        output,
        &method.declaring_class_key,
        &method.method_name,
        &method.descriptor,
        invoke_args,
    ) {
        Ok(result) => Ok(Some(box_reflection_return_value(
            heap,
            descriptor_return_type(&method.descriptor),
            result,
        )?)),
        Err(Error::JavaException { .. }) => Err(Error::JavaException {
            class_name: "java/lang/reflect/InvocationTargetException".to_string(),
        }),
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

    let instance_ref = ops.allocate_instance(heap, output, &constructor.declaring_class_key)?;
    let invoke_args = build_reflection_invoke_args(
        heap,
        Slot::Reference(Some(instance_ref)),
        &constructor.descriptor,
        invoke_arg_slots,
        false,
    )?;

    match ops.invoke(
        heap,
        output,
        &constructor.declaring_class_key,
        &constructor.method_name,
        &constructor.descriptor,
        invoke_args,
    ) {
        Ok(_) => Ok(Some(Slot::Reference(Some(instance_ref)))),
        Err(Error::JavaException { .. }) => Err(Error::JavaException {
            class_name: "java/lang/reflect/InvocationTargetException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

/// Native: `Class.isAssignableFrom(Class)Z` — true when the receiver class/interface
/// is the same as, a superclass of, or a superinterface of the argument class.
///
/// Mirrors the interpreter's `is_assignable_from` (checkcast/instanceof) but walks the
/// hierarchy through `CallbackOps::inspect_class`, since natives cannot reach the
/// `ClassRegistry` directly. `to` is the receiver (the supertype under test); `from` is
/// the argument (the candidate subtype).
pub(crate) fn native_class_is_assignable_from(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let other_ref = extract_ref_arg(args, 1)?;
    let to = class_internal_name_from_ref(heap, this_ref)?;
    let from = class_internal_name_from_ref(heap, other_ref)?;
    Ok(Some(Slot::Int(i32::from(class_is_assignable_via_callback(
        &from, &to, ops,
    )))))
}

/// Native: `Class.getModifiers()I` — access modifiers of the class.
///
/// Duke does not retain raw `ClassAccessFlags` on `ClassContext`, so this reports a
/// concrete, non-interface, non-abstract class (`ACC_PUBLIC`). gson only reads these
/// modifiers to test `Modifier.isInterface`/`isAbstract` (both correctly false here)
/// and `Modifier.isStatic` (handled explicitly by `isAnonymousClass`/`isLocalClass`,
/// which return false for every named class in this path).
#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_class_get_modifiers(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    // TODO(known-limitation): getModifiers/isInterface report ACC_PUBLIC-only; needs real access flags
    // (ReflectedClassInfo does not carry ClassAccessFlags; plumbing them requires
    // touching registry.rs, which is out of scope here).
    // ACC_PUBLIC
    Ok(Some(Slot::Int(0x0001)))
}

/// Native: `Class.isAnonymousClass()Z` — false for every named class Duke loads.
#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_class_is_anonymous_class(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(0)))
}

/// Native: `Class.isLocalClass()Z` — false for every named class Duke loads.
#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_class_is_local_class(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(0)))
}

/// Native: `Class.isRecord()Z` — Duke does not model record classes, so this is
/// always false (gson uses it to decide between record and reflective adapters).
#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_class_is_record(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(0)))
}

/// Native: `Class.isPrimitive()Z` — true when the mirror represents a primitive.
///
/// Primitive `Class` mirrors are keyed by their single-character field descriptor
/// (`I`, `J`, ..., `V`), so a length-1 primitive-descriptor internal name is the
/// reliable signal. gson relies on this to treat Pojo's `int` field correctly.
pub(crate) fn native_class_is_primitive(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let is_primitive = matches!(
        internal_name.as_str(),
        "I" | "J" | "F" | "D" | "Z" | "B" | "C" | "S" | "V"
    );
    Ok(Some(Slot::Int(i32::from(is_primitive))))
}

/// Native: `Class.getGenericSuperclass()Ljava/lang/reflect/Type;`.
///
/// Duke does not parse the `Signature` attribute, so this returns the erased
/// superclass `Class` (which implements `java/lang/reflect/Type`), or `null` when the
/// class has no superclass. gson walks this while resolving type variables; for
/// Pojo (extends `Object`) it yields `Object.class` and the walk terminates.
pub(crate) fn native_class_get_generic_superclass(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let super_class = ops
        .inspect_class(&internal_name)
        .ok()
        .and_then(|info| info.super_class);
    match super_class {
        Some(super_name) => {
            let super_ref = allocate_class_object(heap, &super_name)?;
            Ok(Some(Slot::Reference(Some(super_ref))))
        }
        None => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Class.isInterface()Z`.
///
/// Duke does not retain the `ACC_INTERFACE` flag, and no reliable signal for it
/// exists in the reflection metadata; gson calls this on the concrete raw type it
/// is (de)serializing, which is never an interface, so this reports false.
#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_class_is_interface(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    // TODO(known-limitation): getModifiers/isInterface report ACC_PUBLIC-only; needs real access flags
    // (ReflectedClassInfo does not carry ClassAccessFlags; plumbing them requires
    // touching registry.rs, which is out of scope here).
    Ok(Some(Slot::Int(0)))
}

/// Native: `Field.getModifiers()I` — the modifier bits Duke tracks for a field.
///
/// `ReflectedFieldHandle` records `ACC_PUBLIC`/`ACC_STATIC`; `final`/`transient` are
/// not retained. gson reads these to skip static/transient fields, so reporting the
/// tracked bits is sufficient (Pojo's package-private instance fields yield 0 and are
/// correctly serialized).
pub(crate) fn native_reflect_field_get_modifiers(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let field = reflected_field_handle(heap, field_ref)?;
    let mut modifiers = 0;
    if field.is_public {
        modifiers |= 0x0001; // ACC_PUBLIC
    }
    if field.is_static {
        modifiers |= 0x0008; // ACC_STATIC
    }
    // TODO(known-limitation): Field.getModifiers omits transient/final
    // (ReflectedFieldHandle/ReflectedFieldInfo only carry public/static; the
    // transient/final bits are dropped when the Field mirror is built, and
    // retaining them requires touching registry.rs, which is out of scope here).
    Ok(Some(Slot::Int(modifiers)))
}

/// Native: `Field.isSynthetic()Z` — Duke does not track the `ACC_SYNTHETIC` flag;
/// user-declared fields (which gson filters on) are never synthetic, so false.
pub(crate) fn native_reflect_field_is_synthetic(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let _ = reflected_field_handle(heap, field_ref)?;
    Ok(Some(Slot::Int(0)))
}

/// Native: `Field.getGenericType()Ljava/lang/reflect/Type;`.
///
/// Duke does not parse the `Signature` attribute, so generic type arguments are not
/// modelled; this returns the erased declared type (the field's `Class`, which
/// implements `java/lang/reflect/Type`). gson then wraps it in a `TypeToken`, which is
/// exactly correct for non-parameterized fields such as Pojo's `int`/`String`.
pub(crate) fn native_reflect_field_get_generic_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let field = reflected_field_handle(heap, field_ref)?;
    Ok(Some(descriptor_class_slot_from_source(
        heap,
        ops,
        &field.descriptor,
        Some(field.declaring_class_key.as_str()),
    )?))
}

/// Native: `Class.cast(Object)Object` — narrows `obj` to this class.
///
/// Returns `null` for a null argument, the object itself when its runtime type is
/// assignable to this class, and throws `ClassCastException` otherwise. gson calls
/// this to finalise `fromJson` results.
pub(crate) fn native_class_cast(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let obj_slot = extract_slot_arg(args, 1);
    let Slot::Reference(Some(obj_ref)) = obj_slot else {
        // Class.cast(null) == null.
        return Ok(Some(Slot::Reference(None)));
    };
    let to = class_internal_name_from_ref(heap, class_ref)?;
    let from = heap.get(obj_ref)?.class_name.clone();
    if class_is_assignable_via_callback(&from, &to, ops) {
        Ok(Some(obj_slot))
    } else {
        Err(Error::JavaException {
            class_name: "java/lang/ClassCastException".to_string(),
        })
    }
}

/// Walk `from`'s superclass/interface closure looking for `to`, resolving each level
/// through `CallbackOps::inspect_class`. Inspection failures are treated as "no such
/// supertype" rather than propagated, so this cannot itself error.
fn class_is_assignable_via_callback(from: &str, to: &str, ops: &mut dyn CallbackOps) -> bool {
    if from == to || to == "java/lang/Object" {
        return true;
    }
    // Arrays implement only Cloneable and Serializable beyond Object.
    if from.starts_with('[') {
        return matches!(to, "java/lang/Cloneable" | "java/io/Serializable");
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    queue.push_back(from.to_string());
    while let Some(current) = queue.pop_front() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if current == to {
            return true;
        }
        let Ok(info) = ops.inspect_class(&current) else {
            continue;
        };
        if let Some(super_class) = info.super_class {
            queue.push_back(super_class);
        }
        for iface in info.interfaces {
            queue.push_back(iface);
        }
    }
    false
}
