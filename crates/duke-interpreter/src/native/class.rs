fn class_extends(
    registry: &ClassRegistry,
    class_name: &str,
    expected_super: &str,
) -> bool {
    let mut current = Some(class_name.to_string());
    while let Some(name) = current {
        if name == expected_super {
            return true;
        }
        current = registry.get(&name).ok().and_then(|ctx| ctx.super_class.clone());
    }
    false
}
fn class_path_from_url_spec(spec: &str) -> Option<String> {
    if let Some(jar_spec) = spec.strip_prefix("jar:")
        && let Some(file_url) = jar_spec.split("!/").next()
    {
        return file_url_to_path(file_url)
            .ok()
            .map(|path| path.to_string_lossy().to_string());
    }
    if spec.starts_with("file://") {
        return file_url_to_path(spec)
            .ok()
            .map(|path| path.to_string_lossy().to_string());
    }
    None
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
        Err(Error::ClassNotFound { .. }) => {
            Err(Error::JavaException {
                class_name: "java/lang/ClassNotFoundException".to_string(),
            })
        }
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
        Some(Slot::Reference(Some(loader_ref))) => {
            (
                ops.ensure_loaded_with_runtime_loader(heap, *loader_ref, &internal_name),
                ops.class_key_for_runtime_loader(heap, *loader_ref, &internal_name)?,
            )
        }
        Some(Slot::Reference(None)) | None => {
            (
                ops.ensure_loaded(&internal_name),
                ops.class_key_for_loaded_class(&internal_name)?,
            )
        }
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
        Err(Error::ClassNotFound { .. }) => {
            Err(Error::JavaException {
                class_name: "java/lang/ClassNotFoundException".to_string(),
            })
        }
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
    Ok(Some(Slot::Reference(ops.runtime_loader_for_class(&class_key)?)))
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
fn class_resource_base(class_internal_name: &str) -> String {
    class_internal_name
        .rsplit_once('/')
        .map_or_else(
            || "<default-package>".to_string(),
            |(package, _)| package.to_string(),
        )
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
    let parameter_descriptor = parameter_descriptor_from_class_array(
        heap,
        extract_slot_arg(args, 2),
    )?;
    let Some(method) = reflected
        .methods
        .into_iter()
        .find(|method| {
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
    let parameter_descriptor = parameter_descriptor_from_class_array(
        heap,
        extract_slot_arg(args, 2),
    )?;
    let Some((declaring_class, method)) = lookup_public_reflected_method(
        ops,
        &class_key,
        &method_name,
        &parameter_descriptor,
    )? else {
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
    let Some(field) = reflected.fields.into_iter().find(|field| field.name == field_name)
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
    let parameter_descriptor = parameter_descriptor_from_class_array(
        heap,
        extract_slot_arg(args, 1),
    )?;
    let Some(constructor) = lookup_reflected_constructor(
        reflected,
        &parameter_descriptor,
        false,
    ) else {
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
    let Some((declaring_class, field)) = lookup_public_reflected_field(
        ops,
        &class_key,
        &field_name,
    )? else {
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
    let parameter_descriptor = parameter_descriptor_from_class_array(
        heap,
        extract_slot_arg(args, 1),
    )?;
    let Some(constructor) = lookup_reflected_constructor(
        reflected,
        &parameter_descriptor,
        true,
    ) else {
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
        .cloned() else {
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
    match ops
        .invoke(
            heap,
            output,
            &class_key,
            "<init>",
            &constructor.descriptor,
            vec![Slot::Reference(Some(instance_ref))],
        )
    {
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
    let array_ref = allocate_reference_array(
        heap,
        "[Ljava/lang/reflect/Method;",
        &method_refs,
    )?;
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
    let array_ref = allocate_reference_array(
        heap,
        "[Ljava/lang/reflect/Constructor;",
        &constructor_refs,
    )?;
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
    let array_ref = allocate_reference_array(
        heap,
        "[Ljava/lang/reflect/Method;",
        &method_refs,
    )?;
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
    let array_ref = allocate_reference_array(
        heap,
        "[Ljava/lang/reflect/Constructor;",
        &constructor_refs,
    )?;
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
    let array_ref = allocate_reference_array(
        heap,
        "[Ljava/lang/reflect/Field;",
        &field_refs,
    )?;
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
    let array_ref = allocate_reference_array(
        heap,
        "[Ljava/lang/reflect/Field;",
        &field_refs,
    )?;
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
fn class_literal_descriptor_to_key(descriptor: &str) -> String {
    if descriptor.starts_with('L') && descriptor.ends_with(';') {
        annotation_descriptor_to_internal_name(descriptor)
    } else {
        descriptor.to_string()
    }
}
fn class_internal_name_from_key(class_key: &str) -> &str {
    class_key.split_once('\0').map_or(class_key, |(internal_name, _)| internal_name)
}
fn class_key_from_ref(heap: &duke_gc::Heap, class_ref: u64) -> Result<String> {
    heap.get(class_ref)?
        .string_value
        .clone()
        .ok_or(Error::InvalidRef {
            address: class_ref,
        })
}
fn class_internal_name_from_ref(heap: &duke_gc::Heap, class_ref: u64) -> Result<String> {
    Ok(class_internal_name_from_key(&class_key_from_ref(heap, class_ref)?).to_string())
}
fn class_descriptor_from_class_ref(
    heap: &duke_gc::Heap,
    class_ref: u64,
) -> Result<String> {
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    if internal_name.starts_with('[') || internal_name.len() == 1 {
        Ok(internal_name)
    } else {
        Ok(format!("L{internal_name};"))
    }
}
fn class_cast_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/ClassCastException".to_string(),
    }
}
