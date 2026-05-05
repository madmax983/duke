pub(crate) fn annotation_proxy_type(class_name: &str) -> Option<&str> {
    class_name.strip_prefix(ANNOTATION_PROXY_PREFIX)
}
fn annotation_element_values(
    annotation: &ReflectedAnnotation,
    ops: &mut dyn CallbackOps,
) -> Result<Vec<(String, String, ReflectedAnnotationValue)>> {
    let annotation_info = ops.inspect_class(&annotation.type_name)?;
    let mut values = Vec::new();
    for method in annotation_info
        .methods
        .into_iter()
        .filter(|method| method.descriptor.starts_with("()"))
    {
        let explicit = annotation
            .elements
            .iter()
            .find(|element| element.name == method.name)
            .map(|element| element.value.clone());
        let Some(value) = explicit.or_else(|| method.annotation_default.clone()) else {
            continue;
        };
        values
            .push((
                method.name,
                method_return_descriptor(&method.descriptor).to_string(),
                value,
            ));
    }
    Ok(values)
}
pub(crate) fn annotation_proxy_element_slot(
    heap: &duke_gc::Heap,
    proxy_ref: u64,
    method_name: &str,
    descriptor: &str,
) -> Result<Option<Slot>> {
    let proxy = heap.get(proxy_ref)?;
    if annotation_proxy_type(&proxy.class_name).is_none() {
        return Ok(None);
    }
    if method_name == "annotationType" && descriptor == "()Ljava/lang/Class;" {
        return Ok(proxy.fields.first().copied());
    }
    for chunk in proxy.fields[1..].chunks(3) {
        let [Slot::Reference(Some(name_ref)), Slot::Reference(Some(desc_ref)), value] = chunk
        else {
            continue;
        };
        if string_value_from_ref(heap, *name_ref)? == method_name
            && string_value_from_ref(heap, *desc_ref)?
                == method_return_descriptor(descriptor)
        {
            return Ok(Some(*value));
        }
    }
    Ok(None)
}
fn annotation_descriptor_to_internal_name(descriptor: &str) -> String {
    descriptor
        .strip_prefix('L')
        .and_then(|s| s.strip_suffix(';'))
        .unwrap_or(descriptor)
        .to_string()
}
fn annotation_default_from_attrs(
    cp: &[Option<CpEntry>],
    attrs: &[duke_classfile::types::AttributeInfo],
) -> Option<ReflectedAnnotationValue> {
    attrs
        .iter()
        .find_map(|attr| {
            if let duke_classfile::types::AttributeData::AnnotationDefault(value) = &attr
                .data
            {
                resolve_annotation_value(cp, value)
            } else {
                None
            }
        })
}
