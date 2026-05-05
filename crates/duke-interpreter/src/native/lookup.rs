fn lookup_class_resource(
    args: &[Slot],
    heap: &duke_gc::Heap,
    ops: &mut dyn CallbackOps,
) -> Result<(String, String, String, Option<duke_loader::LocatedResource>)> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let class_internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let loader_ref = ops.runtime_loader_for_class(&class_key)?;
    let requested_name = string_arg(args, 1, heap)?;
    let base = class_resource_base(&class_internal_name);
    let classpath = classpath_debug_label(loader_ref);
    let Some(resolved_name) = resolve_class_resource_name(
        &class_internal_name,
        &requested_name,
    ) else {
        log_resource_lookup_miss(&requested_name, &base, &classpath);
        return Ok((requested_name, base, classpath, None));
    };
    let resource = ops.find_resource_entry(heap, loader_ref, &resolved_name)?;
    if resource.is_none() {
        log_resource_lookup_miss(&resolved_name, &base, &classpath);
    }
    Ok((resolved_name, base, classpath, resource))
}
fn lookup_class_loader_resource(
    args: &[Slot],
    heap: &duke_gc::Heap,
    ops: &mut dyn CallbackOps,
) -> Result<(String, String, String, Option<duke_loader::LocatedResource>)> {
    let loader_ref = extract_ref_arg(args, 0)?;
    let requested_name = string_arg(args, 1, heap)?;
    let base = "<class-loader>".to_string();
    let classpath = classpath_debug_label(Some(loader_ref));
    let Some(resolved_name) = normalize_resource_name(&requested_name) else {
        log_resource_lookup_miss(&requested_name, &base, &classpath);
        return Ok((requested_name, base, classpath, None));
    };
    let resource = ops.find_resource_entry(heap, Some(loader_ref), &resolved_name)?;
    if resource.is_none() {
        log_resource_lookup_miss(&resolved_name, &base, &classpath);
    }
    Ok((resolved_name, base, classpath, resource))
}
fn lookup_reflected_constructor(
    reflected: ReflectedClassInfo,
    parameter_descriptor: &str,
    public_only: bool,
) -> Option<ReflectedMethodInfo> {
    reflected
        .methods
        .into_iter()
        .find(|method| {
            method.name == "<init>" && (!public_only || method.is_public)
                && descriptor_parameter_part(&method.descriptor) == parameter_descriptor
        })
}
fn lookup_public_reflected_field(
    ops: &mut dyn CallbackOps,
    class: &str,
    field_name: &str,
) -> Result<Option<(String, ReflectedFieldInfo)>> {
    lookup_public_reflected_field_inner(ops, class, field_name, &mut HashSet::new())
}
fn lookup_public_reflected_field_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    field_name: &str,
    visited: &mut HashSet<String>,
) -> Result<Option<(String, ReflectedFieldInfo)>> {
    if !visited.insert(class.to_string()) {
        return Ok(None);
    }
    let reflected = ops.inspect_class(class)?;
    if let Some(field) = reflected
        .fields
        .iter()
        .find(|field| field.is_public && field.name == field_name)
        .cloned()
    {
        return Ok(Some((class.to_string(), field)));
    }
    if let Some(super_class) = reflected.super_class.clone()
        && let Some(found) = lookup_public_reflected_field_inner(
            ops,
            &super_class,
            field_name,
            visited,
        )?
    {
        return Ok(Some(found));
    }
    for interface in reflected.interfaces {
        if let Some(found) = lookup_public_reflected_field_inner(
            ops,
            &interface,
            field_name,
            visited,
        )? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}
fn lookup_public_reflected_method(
    ops: &mut dyn CallbackOps,
    class: &str,
    method_name: &str,
    parameter_descriptor: &str,
) -> Result<Option<(String, ReflectedMethodInfo)>> {
    lookup_public_reflected_method_inner(
        ops,
        class,
        method_name,
        parameter_descriptor,
        &mut HashSet::new(),
    )
}
fn lookup_public_reflected_method_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    method_name: &str,
    parameter_descriptor: &str,
    visited: &mut HashSet<String>,
) -> Result<Option<(String, ReflectedMethodInfo)>> {
    if !visited.insert(class.to_string()) {
        return Ok(None);
    }
    let reflected = ops.inspect_class(class)?;
    if let Some(method) = reflected
        .methods
        .iter()
        .find(|method| {
            method.is_public && method.name != "<init>" && method.name != "<clinit>"
                && method.name == method_name
                && descriptor_parameter_part(&method.descriptor) == parameter_descriptor
        })
        .cloned()
    {
        return Ok(Some((class.to_string(), method)));
    }
    if let Some(super_class) = reflected.super_class.clone()
        && let Some(found) = lookup_public_reflected_method_inner(
            ops,
            &super_class,
            method_name,
            parameter_descriptor,
            visited,
        )?
    {
        return Ok(Some(found));
    }
    for interface in reflected.interfaces {
        if let Some(found) = lookup_public_reflected_method_inner(
            ops,
            &interface,
            method_name,
            parameter_descriptor,
            visited,
        )? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}
fn lookup_registered_native_kind(
    registry: &ClassRegistry,
    class_name: &str,
    method_name: &str,
    method_desc: &str,
) -> Option<HandlerKind> {
    registry
        .natives()
        .get_kind(class_name, method_name, method_desc)
        .or_else(|| {
            let internal_name = registry.internal_name_for_class(class_name);
            (internal_name != class_name)
                .then(|| {
                    registry.natives().get_kind(internal_name, method_name, method_desc)
                })
                .flatten()
        })
}
