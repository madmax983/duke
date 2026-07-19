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
/// Native: `AccessibleObject.isAccessible()Z` (Field / Method / Constructor) —
/// reads back the `accessibleFlag` last written by `setAccessible`. Spring's
/// `ReflectionUtils.makeAccessible` calls it to avoid a redundant
/// `setAccessible(true)` on an already-accessible member.
pub(crate) fn native_reflection_member_is_accessible(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    let accessible = matches!(
        heap.get(member_ref)?.fields.get(REFLECTION_MEMBER_ACCESSIBLE_FIELD),
        Some(Slot::Int(flag)) if *flag != 0
    );
    Ok(Some(Slot::Int(i32::from(accessible))))
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
/// Return `true` when the frame that invoked this caller-sensitive reflect native
/// (`control.stack_trace()[0]`) belongs to the same class as the member's
/// declaring class. `native_needs_stack_snapshot` captures the invoking Java frame
/// for `Method.invoke`/`Constructor.newInstance`, so `frames[0]` is the caller.
/// Both sides are normalized to their plain internal name (stripping any
/// `\0loader:N` qualifier) before comparison, matching Duke's by-name class
/// identity model.
fn caller_is_same_class(control: &NativeControl, declaring_class_key: &str) -> bool {
    let declaring_internal = class_internal_name_from_key(declaring_class_key);
    control
        .stack_trace()
        .first()
        .is_some_and(|frame| class_internal_name_from_key(&frame.class_name) == declaring_internal)
}

pub(crate) fn native_reflect_method_invoke(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let invoke_arg_slots = reflection_array_elements(heap, extract_slot_arg(args, 2))?;
    let method = reflected_method_handle(heap, method_ref)?;

    // Reflection access control (JLS 6.6 / JVMS 5.4.4): `Method.invoke` is
    // *caller-sensitive*. A class may always reflectively access its OWN
    // (private/protected/package) members without `setAccessible(true)`; only
    // CROSS-class access to a non-accessible member raises IllegalAccessException.
    // `setAccessible(true)` (recorded as `is_accessible`) bypasses the check
    // entirely, exactly as before. `native_needs_stack_snapshot` now captures the
    // invoking Java frame for `Method.invoke`, so `control.stack_trace()[0]` names
    // the caller class; we allow same-class access and otherwise keep the coarse
    // `!is_public → throw` that the cross-class gson canary and
    // `ReflectionTest.privateMethodRaisesIllegalAccess` depend on.
    //
    // NOTE: nest-mate access (JEP 181, NestHost/NestMembers) is not yet modelled —
    // `ReflectedClassInfo` does not carry nest membership — so only strict
    // same-class access is permitted here. Cross-nest private invoke still throws.
    if !method.is_public
        && !method.is_accessible
        && !caller_is_same_class(control, &method.declaring_class_key)
    {
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
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let constructor_ref = extract_ref_arg(args, 0)?;
    let invoke_arg_slots = reflection_array_elements(heap, extract_slot_arg(args, 1))?;
    let constructor = reflected_method_handle(heap, constructor_ref)?;

    // Same caller-sensitive rule as `Method.invoke` above: a class may always
    // reflectively construct via its OWN non-public constructor; `setAccessible`
    // bypasses; cross-class private construction still throws. Nest-mates are not
    // yet modelled.
    if !constructor.is_public
        && !constructor.is_accessible
        && !caller_is_same_class(control, &constructor.declaring_class_key)
    {
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

/// Native: `Class.getSuperclass()Ljava/lang/Class;`.
///
/// Returns the `Class` mirror of the direct superclass, or `null` for
/// `java.lang.Object`, interfaces, and primitive types (per the JLS). Array
/// classes report `Object`. The superclass name is normalized to its bare
/// internal form so the mirror interns consistently with every other `Class`
/// mirror Duke mints. Reached from Spring's annotation-hierarchy scanning.
pub(crate) fn native_class_get_superclass(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    // An array class's superclass is Object.
    if internal_name.starts_with('[') {
        let object_ref = allocate_class_object(heap, "java/lang/Object")?;
        return Ok(Some(Slot::Reference(Some(object_ref))));
    }
    // Primitive mirrors (single-character descriptor keys) have no superclass.
    if internal_name.len() == 1 {
        return Ok(Some(Slot::Reference(None)));
    }
    let Ok(info) = ops.inspect_class(&internal_name) else {
        return Ok(Some(Slot::Reference(None)));
    };
    // Interfaces report null even though their classfile super is Object.
    if info.access_flags & 0x0200 != 0 {
        return Ok(Some(Slot::Reference(None)));
    }
    match info.super_class {
        Some(super_name) => {
            let bare = internal_name_fragment(&super_name).to_string();
            let super_ref = allocate_class_object(heap, &bare)?;
            Ok(Some(Slot::Reference(Some(super_ref))))
        }
        None => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Class.getInterfaces()[Ljava/lang/Class;`.
///
/// Returns the `Class` mirrors of the interfaces this class/interface directly
/// declares, in declaration order (an empty array when there are none). Each
/// interface name is normalized to its bare internal form so the mirrors intern
/// consistently. Reached from Spring's annotation-hierarchy scanning.
pub(crate) fn native_class_get_interfaces(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let interfaces = ops
        .inspect_class(&internal_name)
        .map(|info| info.interfaces)
        .unwrap_or_default();
    let mut class_refs = Vec::with_capacity(interfaces.len());
    for iface in interfaces {
        let bare = internal_name_fragment(&iface).to_string();
        class_refs.push(allocate_class_object(heap, &bare)?);
    }
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/Class;", &class_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
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

// ---------------------------------------------------------------------------
// Generics: java.lang.reflect.Type materialization from parsed signatures.
//
// The duke/internal/reflect/*Impl backing classes registered in stdlib.rs have
// fixed field layouts, mirrored by these indices. Objects are allocated directly
// (heap.allocate + write_field, no <init> bytecode); heap.allocate never
// relocates existing objects, so refs held across successive allocations stay
// valid without pinning.
// ---------------------------------------------------------------------------

/// `duke/internal/reflect/TypeVariableImpl` field indices.
const TYPEVAR_NAME_FIELD: usize = 0;
const TYPEVAR_BOUNDS_FIELD: usize = 1;
const TYPEVAR_GENERIC_DECL_FIELD: usize = 2;
/// `duke/internal/reflect/ParameterizedTypeImpl` field indices.
const PARAMTYPE_RAW_FIELD: usize = 0;
const PARAMTYPE_ARGS_FIELD: usize = 1;
const PARAMTYPE_OWNER_FIELD: usize = 2;
/// `duke/internal/reflect/GenericArrayTypeImpl` field indices.
const GENARRAY_COMPONENT_FIELD: usize = 0;
/// `duke/internal/reflect/WildcardTypeImpl` field indices.
const WILDCARD_UPPER_FIELD: usize = 0;
const WILDCARD_LOWER_FIELD: usize = 1;

/// Read the reference/int [`Slot`] at field `idx` of a backing-type object.
fn type_impl_field(heap: &duke_gc::Heap, obj_ref: u64, idx: usize) -> Result<Slot> {
    heap.get(obj_ref)?
        .fields
        .get(idx)
        .copied()
        .ok_or(Error::InvalidRef { address: obj_ref })
}

/// Allocate a `TypeVariableImpl` mirror carrying its name, erased bounds array,
/// and generic-declaration reference.
fn allocate_type_variable_mirror(
    heap: &mut duke_gc::Heap,
    name: &str,
    bound_refs: &[u64],
    generic_declaration: Option<u64>,
) -> Result<u64> {
    let tv_ref = heap.allocate("duke/internal/reflect/TypeVariableImpl".to_string(), 3);
    let name_ref = heap.allocate_string(name.to_string());
    let bounds_array = allocate_reference_array(heap, "[Ljava/lang/reflect/Type;", bound_refs)?;
    heap.write_field(tv_ref, TYPEVAR_NAME_FIELD, Slot::Reference(Some(name_ref)))?;
    heap.write_field(
        tv_ref,
        TYPEVAR_BOUNDS_FIELD,
        Slot::Reference(Some(bounds_array)),
    )?;
    heap.write_field(
        tv_ref,
        TYPEVAR_GENERIC_DECL_FIELD,
        Slot::Reference(generic_declaration),
    )?;
    Ok(tv_ref)
}

/// Allocate a `ParameterizedTypeImpl` mirror (raw type + actual type arguments).
fn allocate_parameterized_type_mirror(
    heap: &mut duke_gc::Heap,
    raw_ref: u64,
    arg_refs: &[u64],
) -> Result<u64> {
    let pt_ref = heap.allocate("duke/internal/reflect/ParameterizedTypeImpl".to_string(), 3);
    let args_array = allocate_reference_array(heap, "[Ljava/lang/reflect/Type;", arg_refs)?;
    heap.write_field(pt_ref, PARAMTYPE_RAW_FIELD, Slot::Reference(Some(raw_ref)))?;
    heap.write_field(
        pt_ref,
        PARAMTYPE_ARGS_FIELD,
        Slot::Reference(Some(args_array)),
    )?;
    heap.write_field(pt_ref, PARAMTYPE_OWNER_FIELD, Slot::Reference(None))?;
    Ok(pt_ref)
}

/// Allocate a `GenericArrayTypeImpl` mirror (generic component type).
fn allocate_generic_array_type_mirror(
    heap: &mut duke_gc::Heap,
    component_ref: u64,
) -> Result<u64> {
    let ga_ref = heap.allocate("duke/internal/reflect/GenericArrayTypeImpl".to_string(), 1);
    heap.write_field(
        ga_ref,
        GENARRAY_COMPONENT_FIELD,
        Slot::Reference(Some(component_ref)),
    )?;
    Ok(ga_ref)
}

/// Allocate a `WildcardTypeImpl` mirror (upper + lower bound arrays).
fn allocate_wildcard_type_mirror(
    heap: &mut duke_gc::Heap,
    upper_refs: &[u64],
    lower_refs: &[u64],
) -> Result<u64> {
    let wc_ref = heap.allocate("duke/internal/reflect/WildcardTypeImpl".to_string(), 2);
    let upper_array = allocate_reference_array(heap, "[Ljava/lang/reflect/Type;", upper_refs)?;
    let lower_array = allocate_reference_array(heap, "[Ljava/lang/reflect/Type;", lower_refs)?;
    heap.write_field(
        wc_ref,
        WILDCARD_UPPER_FIELD,
        Slot::Reference(Some(upper_array)),
    )?;
    heap.write_field(
        wc_ref,
        WILDCARD_LOWER_FIELD,
        Slot::Reference(Some(lower_array)),
    )?;
    Ok(wc_ref)
}

/// Materialize a parsed [`duke_classfile::TypeSignature`] into a live
/// `java.lang.reflect.Type` heap object (erased `Class`, `ParameterizedType`,
/// `GenericArrayType`, or `TypeVariable`).
fn materialize_type_signature(
    heap: &mut duke_gc::Heap,
    sig: &duke_classfile::TypeSignature,
) -> Result<u64> {
    use duke_classfile::TypeSignature;
    match sig {
        TypeSignature::Primitive(c) => allocate_class_object(heap, &c.to_string()),
        TypeSignature::Void => allocate_class_object(heap, "V"),
        TypeSignature::TypeVariable(name) => {
            // A bare type-variable reference, represented by its name with the
            // default {Object} bound and no declaration link.
            let object_ref = allocate_class_object(heap, "java/lang/Object")?;
            allocate_type_variable_mirror(heap, name, &[object_ref], None)
        }
        TypeSignature::Array(inner) => {
            let component_ref = materialize_type_signature(heap, inner)?;
            allocate_generic_array_type_mirror(heap, component_ref)
        }
        TypeSignature::Class(cts) => materialize_class_type_signature(heap, cts),
    }
}

/// Materialize a parsed [`duke_classfile::ClassTypeSignature`]: an erased `Class`
/// when it carries no type arguments, else a `ParameterizedType`.
fn materialize_class_type_signature(
    heap: &mut duke_gc::Heap,
    cts: &duke_classfile::ClassTypeSignature,
) -> Result<u64> {
    if cts.type_arguments.is_empty() {
        return allocate_class_object(heap, &cts.name);
    }
    let raw_ref = allocate_class_object(heap, &cts.name)?;
    let mut arg_refs = Vec::with_capacity(cts.type_arguments.len());
    for arg in &cts.type_arguments {
        arg_refs.push(materialize_type_argument(heap, arg)?);
    }
    allocate_parameterized_type_mirror(heap, raw_ref, &arg_refs)
}

/// Materialize a parsed [`duke_classfile::TypeArgument`]: an exact type, or a
/// `WildcardType` for `*` / `+Bound` / `-Bound`.
fn materialize_type_argument(
    heap: &mut duke_gc::Heap,
    arg: &duke_classfile::TypeArgument,
) -> Result<u64> {
    use duke_classfile::TypeArgument;
    match arg {
        TypeArgument::Exact(t) => materialize_type_signature(heap, t),
        TypeArgument::Wildcard => {
            let object_ref = allocate_class_object(heap, "java/lang/Object")?;
            allocate_wildcard_type_mirror(heap, &[object_ref], &[])
        }
        TypeArgument::Extends(t) => {
            let bound = materialize_type_signature(heap, t)?;
            allocate_wildcard_type_mirror(heap, &[bound], &[])
        }
        TypeArgument::Super(t) => {
            let object_ref = allocate_class_object(heap, "java/lang/Object")?;
            let bound = materialize_type_signature(heap, t)?;
            allocate_wildcard_type_mirror(heap, &[object_ref], &[bound])
        }
    }
}

/// Native: `Class.getTypeParameters()[Ljava/lang/reflect/TypeVariable;`.
///
/// Resolves the class's parsed `Signature` (JVMS §4.7.9.1) via
/// [`CallbackOps::inspect_class`] and builds a `TypeVariable[]` whose length is
/// exactly the number of declared formal type parameters — the count Spring's
/// `ResolvableType.forClassWithGenerics` asserts against. Each element carries
/// its name, its erased bounds (defaulting to `{Object}`), and this `Class` as
/// the generic declaration. Returns a zero-length array when the class is not
/// generic (no signature).
pub(crate) fn native_class_get_type_parameters(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let type_params = ops
        .inspect_class(&internal_name)
        .ok()
        .and_then(|info| info.signature)
        .and_then(|sig| duke_classfile::parse_class_signature(&sig).ok())
        .map(|class_sig| class_sig.type_params)
        .unwrap_or_default();

    let mut element_refs = Vec::with_capacity(type_params.len());
    for tp in &type_params {
        let mut bound_refs = Vec::with_capacity(tp.bounds.len());
        for bound in &tp.bounds {
            bound_refs.push(materialize_type_signature(heap, bound)?);
        }
        if bound_refs.is_empty() {
            // JLS: a type variable with no explicit bound has bound {Object}.
            bound_refs.push(allocate_class_object(heap, "java/lang/Object")?);
        }
        let tv_ref =
            allocate_type_variable_mirror(heap, &tp.name, &bound_refs, Some(class_ref))?;
        element_refs.push(tv_ref);
    }
    let array_ref =
        allocate_reference_array(heap, "[Ljava/lang/reflect/TypeVariable;", &element_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

/// Native: `Class.getGenericSuperclass()Ljava/lang/reflect/Type;`.
///
/// When the class's `Signature` records a *parameterized* superclass (e.g.
/// `extends AbstractList<String>`), returns a real `ParameterizedType`. Otherwise
/// returns the erased superclass `Class` (which implements
/// `java/lang/reflect/Type`), or `null` when the class has no superclass.
pub(crate) fn native_class_get_generic_superclass(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let info = ops.inspect_class(&internal_name).ok();
    if let Some(class_sig) = info
        .as_ref()
        .and_then(|i| i.signature.as_deref())
        .and_then(|sig| duke_classfile::parse_class_signature(sig).ok())
        && !class_sig.super_class.type_arguments.is_empty()
    {
        let pt_ref = materialize_class_type_signature(heap, &class_sig.super_class)?;
        return Ok(Some(Slot::Reference(Some(pt_ref))));
    }
    match info.and_then(|i| i.super_class) {
        Some(super_name) => {
            let super_ref = allocate_class_object(heap, &super_name)?;
            Ok(Some(Slot::Reference(Some(super_ref))))
        }
        None => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Class.getGenericInterfaces()[Ljava/lang/reflect/Type;`.
///
/// Materializes each directly-implemented interface from the class's `Signature`
/// (a `ParameterizedType` when it carries type arguments, else the erased
/// `Class`). Falls back to erased `Class` objects for every interface when the
/// class has no generic signature.
pub(crate) fn native_class_get_generic_interfaces(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let info = ops.inspect_class(&internal_name).ok();
    if let Some(class_sig) = info
        .as_ref()
        .and_then(|i| i.signature.as_deref())
        .and_then(|sig| duke_classfile::parse_class_signature(sig).ok())
    {
        let mut refs = Vec::with_capacity(class_sig.interfaces.len());
        for iface in &class_sig.interfaces {
            refs.push(materialize_class_type_signature(heap, iface)?);
        }
        let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Type;", &refs)?;
        return Ok(Some(Slot::Reference(Some(array_ref))));
    }
    let interfaces = info.map(|i| i.interfaces).unwrap_or_default();
    let mut refs = Vec::with_capacity(interfaces.len());
    for iface in &interfaces {
        refs.push(allocate_class_object(heap, iface)?);
    }
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Type;", &refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

// ---- Type accessor natives (registered on the interface names) --------------

/// Native: `TypeVariable.getName()Ljava/lang/String;`.
pub(crate) fn native_type_variable_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this = extract_ref_arg(args, 0)?;
    Ok(Some(type_impl_field(heap, this, TYPEVAR_NAME_FIELD)?))
}

/// Native: `TypeVariable.getBounds()[Ljava/lang/reflect/Type;`.
pub(crate) fn native_type_variable_get_bounds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this = extract_ref_arg(args, 0)?;
    Ok(Some(type_impl_field(heap, this, TYPEVAR_BOUNDS_FIELD)?))
}

/// Native: `TypeVariable.getGenericDeclaration()Ljava/lang/reflect/GenericDeclaration;`.
pub(crate) fn native_type_variable_get_generic_declaration(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this = extract_ref_arg(args, 0)?;
    Ok(Some(type_impl_field(heap, this, TYPEVAR_GENERIC_DECL_FIELD)?))
}

/// Native: `ParameterizedType.getRawType()Ljava/lang/reflect/Type;`.
pub(crate) fn native_parameterized_type_get_raw_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this = extract_ref_arg(args, 0)?;
    Ok(Some(type_impl_field(heap, this, PARAMTYPE_RAW_FIELD)?))
}

/// Native: `ParameterizedType.getActualTypeArguments()[Ljava/lang/reflect/Type;`.
pub(crate) fn native_parameterized_type_get_actual_type_arguments(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this = extract_ref_arg(args, 0)?;
    Ok(Some(type_impl_field(heap, this, PARAMTYPE_ARGS_FIELD)?))
}

/// Native: `ParameterizedType.getOwnerType()Ljava/lang/reflect/Type;`.
pub(crate) fn native_parameterized_type_get_owner_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this = extract_ref_arg(args, 0)?;
    Ok(Some(type_impl_field(heap, this, PARAMTYPE_OWNER_FIELD)?))
}

/// Native: `GenericArrayType.getGenericComponentType()Ljava/lang/reflect/Type;`.
pub(crate) fn native_generic_array_type_get_component_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this = extract_ref_arg(args, 0)?;
    Ok(Some(type_impl_field(heap, this, GENARRAY_COMPONENT_FIELD)?))
}

/// Native: `WildcardType.getUpperBounds()[Ljava/lang/reflect/Type;`.
pub(crate) fn native_wildcard_type_get_upper_bounds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this = extract_ref_arg(args, 0)?;
    Ok(Some(type_impl_field(heap, this, WILDCARD_UPPER_FIELD)?))
}

/// Native: `WildcardType.getLowerBounds()[Ljava/lang/reflect/Type;`.
pub(crate) fn native_wildcard_type_get_lower_bounds(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this = extract_ref_arg(args, 0)?;
    Ok(Some(type_impl_field(heap, this, WILDCARD_LOWER_FIELD)?))
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

/// Native: `Method.getModifiers()I` / `Constructor.getModifiers()I` — the
/// public/static modifier bits recorded on the reflected-member mirror.
///
/// `ReflectedMethodInfo` (unlike `ReflectedFieldInfo`) does not carry the raw
/// classfile `access_flags`, so the honest, available signal is the
/// `publicFlag`/`staticFlag` the `Method`/`Constructor` mirror was minted with.
/// That is exactly the set callers like Spring's
/// `SpringFactoriesLoader.instantiateFactory` test
/// (`Modifier.isPublic(constructor.getModifiers())`); other bits (`final`,
/// `abstract`, `private` vs package-private) are not modelled and read as 0.
pub(crate) fn native_reflect_method_get_modifiers(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let method = reflected_method_handle(heap, method_ref)?;
    let mut modifiers = 0;
    if method.is_public {
        modifiers |= 0x0001; // ACC_PUBLIC
    }
    if method.is_static {
        modifiers |= 0x0008; // ACC_STATIC
    }
    Ok(Some(Slot::Int(modifiers)))
}

// ─── java/lang/reflect/Modifier predicates ───────────────────────────────────
// Pure access-flag bit tests over a modifier `int`, exactly as
// `java.lang.reflect.Modifier` defines them (JVMS §4 access_flags values). First
// reached from Spring's `ReflectionUtils.makeAccessible`, which gates
// `setAccessible(true)` on `Modifier.isPublic(ctor.getModifiers())`. Append-only,
// no heap/callback interaction.

/// Return `Slot::Int(1)` when `(modifier & bit) != 0`, else `Slot::Int(0)`.
fn modifier_bit_test(args: &[Slot], bit: i32) -> Result<Option<Slot>> {
    let modifiers = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(modifiers & bit != 0))))
}

/// Native: `Modifier.isPublic(I)Z`.
pub(crate) fn native_modifier_is_public(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    modifier_bit_test(args, 0x0001)
}

/// Native: `Modifier.isPrivate(I)Z`.
pub(crate) fn native_modifier_is_private(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    modifier_bit_test(args, 0x0002)
}

/// Native: `Modifier.isProtected(I)Z`.
pub(crate) fn native_modifier_is_protected(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    modifier_bit_test(args, 0x0004)
}

/// Native: `Modifier.isStatic(I)Z`.
pub(crate) fn native_modifier_is_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    modifier_bit_test(args, 0x0008)
}

/// Native: `Modifier.isFinal(I)Z`.
pub(crate) fn native_modifier_is_final(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    modifier_bit_test(args, 0x0010)
}

/// Native: `Modifier.isSynchronized(I)Z`.
pub(crate) fn native_modifier_is_synchronized(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    modifier_bit_test(args, 0x0020)
}

/// Native: `Modifier.isVolatile(I)Z`.
pub(crate) fn native_modifier_is_volatile(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    modifier_bit_test(args, 0x0040)
}

/// Native: `Modifier.isTransient(I)Z`.
pub(crate) fn native_modifier_is_transient(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    modifier_bit_test(args, 0x0080)
}

/// Native: `Modifier.isNative(I)Z`.
pub(crate) fn native_modifier_is_native(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    modifier_bit_test(args, 0x0100)
}

/// Native: `Modifier.isInterface(I)Z`.
pub(crate) fn native_modifier_is_interface(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    modifier_bit_test(args, 0x0200)
}

/// Native: `Modifier.isAbstract(I)Z`.
pub(crate) fn native_modifier_is_abstract(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    modifier_bit_test(args, 0x0400)
}

/// Native: `Modifier.isStrict(I)Z`.
pub(crate) fn native_modifier_is_strict(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    modifier_bit_test(args, 0x0800)
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
/// When the field carries a generic `Signature` (e.g.
/// `Ljava/util/List<Ljava/lang/String;>;`) this materializes the real generic
/// type (a `ParameterizedType`/`GenericArrayType`/`TypeVariable`). Otherwise it
/// returns the erased declared type (the field's `Class`, which implements
/// `java/lang/reflect/Type`) — exactly correct for non-parameterized fields such
/// as Pojo's `int`/`String`.
pub(crate) fn native_reflect_field_get_generic_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let field = reflected_field_handle(heap, field_ref)?;
    let declaring = class_internal_name_from_key(&field.declaring_class_key).to_string();
    // Prefer the field's own generic Signature attribute, if present.
    if let Some(sig) = ops.inspect_class(&declaring).ok().and_then(|info| {
        info.fields.into_iter().find_map(|candidate| {
            let matches = candidate.name == field.field_name
                && candidate.descriptor == field.descriptor;
            if matches { candidate.signature } else { None }
        })
    }) && let Ok(type_sig) = duke_classfile::parse_field_signature(&sig)
    {
        let type_ref = materialize_type_signature(heap, &type_sig)?;
        return Ok(Some(Slot::Reference(Some(type_ref))));
    }
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

/// Strip a class key's `\0`-delimited provenance suffix (e.g. `\0loader:<id>` or
/// `\0code:<hash>`) down to the bare internal name.
///
/// Classes loaded through a runtime `ClassLoader` (the Spring Boot fat-jar
/// `LaunchedClassLoader`, for instance) are keyed as `internal/Name\0loader:<id>`,
/// and `CallbackOps::inspect_class` reports their superclass/interface names in
/// that same loader-qualified form. The `Class` mirrors that reflection compares
/// against (`from`/`to` here) carry the bare internal name, so an un-normalized
/// walk never matches a loader-qualified supertype. Duke models one logical type
/// per internal name, so comparing on the bare fragment is the faithful behaviour
/// (it mirrors the registry's own `class_internal_name_fragment`).
fn internal_name_fragment(name: &str) -> &str {
    name.split_once('\0').map_or(name, |(bare, _)| bare)
}

/// Walk `from`'s superclass/interface closure looking for `to`, resolving each level
/// through `CallbackOps::inspect_class`. Inspection failures are treated as "no such
/// supertype" rather than propagated, so this cannot itself error.
fn class_is_assignable_via_callback(from: &str, to: &str, ops: &mut dyn CallbackOps) -> bool {
    let from = internal_name_fragment(from);
    let to = internal_name_fragment(to);
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
            queue.push_back(internal_name_fragment(&super_class).to_string());
        }
        for iface in info.interfaces {
            queue.push_back(internal_name_fragment(&iface).to_string());
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
