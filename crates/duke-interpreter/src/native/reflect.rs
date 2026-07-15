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

    // NOTE (reflection access control): the JVM's `Method.invoke` access check is
    // *caller-sensitive* — a class may always reflectively access its OWN
    // (private/nestmate) members without `setAccessible(true)`; only CROSS-class
    // access to a non-accessible member raises IllegalAccessException. Duke cannot
    // yet distinguish the two here because the invoking frame's class is not
    // available to this native (the stack snapshot in `native_control_for_call` /
    // `native_needs_stack_snapshot` is only captured for Throwable-init and
    // `Reflection.getCallerClass`, not `Method.invoke`). We therefore keep the
    // coarse `!is_public && !is_accessible → throw`, which is CORRECT for the
    // cross-class case that real code (and the gson canary / ReflectionTest) rely
    // on, but WRONG for the legitimate same-class case (e.g. the commons-logging
    // ladder's `LadderApplication.main` reflectively invoking its own private
    // static `summarize`). Making that case pass requires threading the caller
    // class into this native (common.rs + interpreter lane) — see wave-9 pin in
    // docs/findings/2026-07-10-spring-boot-real-app.md.
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

// ─── java/lang/reflect/Array natives (commons-lang3 canary) ──────────────────
// Appended (append-only, shared file). Support commons-lang3 ArrayUtils growth
// helpers, which call Array.newInstance(Integer.TYPE, n)/getLength/set on a
// primitive int[].

/// Given a component type's internal name (a primitive descriptor letter such
/// as `I`, an array descriptor such as `[I`, or a reference internal name such
/// as `java/lang/String`), return the array class descriptor plus the default
/// element `Slot` that a freshly allocated array of that type uses.
fn array_descriptor_and_default(component_internal: &str) -> (String, Slot) {
    match component_internal {
        "J" => ("[J".to_string(), Slot::Long(0)),
        "F" => ("[F".to_string(), Slot::Float(0.0)),
        "D" => ("[D".to_string(), Slot::Double(0.0)),
        // Boolean/byte/char/short/int arrays are all backed by Slot::Int.
        "Z" | "B" | "C" | "S" | "I" => (format!("[{component_internal}"), Slot::Int(0)),
        // Component already an array type (multi-dimensional) → prefix another '['.
        other if other.starts_with('[') => (format!("[{other}"), Slot::Reference(None)),
        // Reference component type.
        other => (format!("[L{other};"), Slot::Reference(None)),
    }
}

/// The element-type descriptor char for an array whose class descriptor is
/// `array_descriptor` (e.g. `[I` → `I`, `[Ljava/lang/String;` → `L`).
fn array_element_type_char(array_descriptor: &str) -> char {
    array_descriptor
        .strip_prefix('[')
        .and_then(|rest| rest.chars().next())
        .unwrap_or('L')
}

/// Native: `java/lang/reflect/Array.newInstance(Ljava/lang/Class;I)Ljava/lang/Object;`
pub(crate) fn native_reflect_array_new_instance(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let component_ref = extract_ref_arg(args, 0)?;
    let length = extract_int_arg(args, 1)?;
    if length < 0 {
        return Err(Error::NegativeArraySize { size: length });
    }
    let component_key = class_key_from_ref(heap, component_ref)?;
    let component_internal = class_internal_name_from_key(&component_key);
    if component_internal == "V" {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalArgumentException".to_string(),
        });
    }
    let (descriptor, default) = array_descriptor_and_default(component_internal);
    let count = usize::try_from(length).map_err(|_| index_out_of_bounds_error())?;
    let array_ref = heap.allocate(descriptor, count);
    let obj = heap.get_mut(array_ref)?;
    for slot in &mut obj.fields {
        *slot = default;
    }
    Ok(Some(Slot::Reference(Some(array_ref))))
}

/// Native: `java/lang/reflect/Array.getLength(Ljava/lang/Object;)I`
pub(crate) fn native_reflect_array_get_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let array_ref = extract_ref_arg(args, 0)?;
    let length = i32::try_from(heap.get(array_ref)?.fields.len())
        .map_err(|_| index_out_of_bounds_error())?;
    Ok(Some(Slot::Int(length)))
}

/// Native: `java/lang/reflect/Array.set(Ljava/lang/Object;ILjava/lang/Object;)V`
///
/// Stores `value` at `index`. For primitive-typed arrays the boxed `value`
/// (e.g. `Integer`) is unboxed to the matching primitive slot.
pub(crate) fn native_reflect_array_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let array_ref = extract_ref_arg(args, 0)?;
    let index = extract_int_arg(args, 1)?;
    let value_slot = extract_slot_arg(args, 2);
    let element_type = array_element_type_char(&heap.get(array_ref)?.class_name);
    let stored = unbox_reflection_argument(heap, element_type, value_slot)?;
    let idx = usize::try_from(index).map_err(|_| index_out_of_bounds_error())?;
    if idx >= heap.get(array_ref)?.fields.len() {
        return Err(index_out_of_bounds_error());
    }
    heap.write_field(array_ref, idx, stored)?;
    Ok(None)
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

/// Native: `Class.getModifiers()I` — the real access modifiers of the class.
///
/// Reads the raw `ClassFile.access_flags` carried on [`ReflectedClassInfo`] and
/// masks them to the set `HotSpot`'s `JVM_GetClassModifiers` reports (JLS recognized
/// class modifiers, with `ACC_SUPER`/`ACC_MODULE` stripped) via
/// [`class_modifiers_from_access_flags`]. A synthetic stub with no classfile flags
/// falls back to `ACC_PUBLIC`, matching the legacy default.
pub(crate) fn native_class_get_modifiers(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let access_flags = ops
        .inspect_class(&internal_name)
        .map_or(SYNTHETIC_CLASS_ACCESS_FLAGS, |info| info.access_flags);
    Ok(Some(Slot::Int(class_modifiers_from_access_flags(
        access_flags,
    ))))
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
/// Reads the real `ACC_INTERFACE` bit from the class's `ClassFile.access_flags`
/// carried on [`ReflectedClassInfo`]. A synthetic stub with no classfile flags (or
/// a primitive/array mirror inspect cannot resolve) reports false.
pub(crate) fn native_class_is_interface(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let is_interface = ops
        .inspect_class(&internal_name)
        .is_ok_and(|info| is_interface_from_access_flags(info.access_flags));
    Ok(Some(Slot::Int(i32::from(is_interface))))
}

/// Native: `Field.getModifiers()I` — the real modifier bits of a field.
///
/// Resolves the declaring class via [`CallbackOps::inspect_class`] and reads the
/// matching field's raw `field_info.access_flags` (carried on
/// [`ReflectedFieldInfo`]), masked to `JVM_RECOGNIZED_FIELD_MODIFIERS` via
/// [`field_modifiers_from_access_flags`] — so `final` (0x10), `transient` (0x80)
/// and `volatile` (0x40) are now reported, not just public/static. When the
/// declaring class is a synthetic stub (or the field cannot be resolved) it falls
/// back to the public/static bits recorded on the `Field` mirror.
pub(crate) fn native_reflect_field_get_modifiers(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let field = reflected_field_handle(heap, field_ref)?;
    let declaring = class_internal_name_from_key(&field.declaring_class_key).to_string();
    let resolved = ops.inspect_class(&declaring).ok().and_then(|info| {
        info.fields
            .into_iter()
            .find(|candidate| {
                let name_matches = candidate.name == field.field_name;
                let descriptor_matches = candidate.descriptor == field.descriptor;
                name_matches && descriptor_matches
            })
            .map(|candidate| candidate.access_flags)
    });
    let modifiers = resolved.map_or_else(
        || {
            let mut fallback = 0;
            if field.is_public {
                fallback |= 0x0001; // ACC_PUBLIC
            }
            if field.is_static {
                fallback |= 0x0008; // ACC_STATIC
            }
            fallback
        },
        field_modifiers_from_access_flags,
    );
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

/// True when the class is an enum: `ACC_ENUM` set or its direct superclass is
/// `java/lang/Enum`.
fn class_is_enum(ops: &mut dyn CallbackOps, internal_name: &str) -> bool {
    ops.inspect_class(internal_name).is_ok_and(|info| {
        info.access_flags & 0x4000 != 0 || info.super_class.as_deref() == Some("java/lang/Enum")
    })
}

/// Native: `Class.isEnum()Z`.
pub(crate) fn native_class_is_enum(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    Ok(Some(Slot::Int(i32::from(class_is_enum(ops, &internal_name)))))
}

/// Native: `Class.getEnumConstants()[Ljava/lang/Object;` — fresh array of this
/// enum's constants in ordinal order, or `null` when the class is not an enum.
pub(crate) fn native_class_get_enum_constants(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    if !class_is_enum(ops, &internal_name) {
        return Ok(Some(Slot::Reference(None)));
    }
    ops.ensure_class_initialized(heap, out, &internal_name)?;
    let universe = enum_constants_in_order(heap, &internal_name)?;
    // Real getEnumConstants returns a T[] of the enum's own type; javac inserts a
    // checkcast to `[L<enum>;` at the call site, so the array must carry that type.
    let array_class_name = format!("[L{internal_name};");
    let array_ref = allocate_reference_array(heap, &array_class_name, &universe)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}
