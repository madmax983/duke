fn is_throwable_class(registry: &ClassRegistry, class_name: &str) -> bool {
    let mut current = Some(class_name.to_string());
    while let Some(name) = current {
        if name == "java/lang/Throwable" {
            return true;
        }
        current = registry.get(&name).ok().and_then(|ctx| ctx.super_class.clone());
    }
    false
}
/// Check if `from` is a subtype of `to` (i.e., `from` can be assigned where `to` is expected).
///
/// Walks the class hierarchy from `from` upward through superclasses.
/// Returns `true` if `to` is found in the chain, or if `to` is `"java/lang/Object"`.
/// Return `true` if a reference of type `from` can be used where `to` is expected.
///
/// BFS over the full type graph (superclass + all implemented interfaces at each
/// level), so `String instanceof Comparable` resolves correctly.
fn is_assignable_from(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    from: &str,
    to: &str,
    target_source_class: Option<&str>,
) -> bool {
    if let Some(annotation_type) = annotation_proxy_type(from) {
        let target = class_internal_name_from_key(to);
        return matches!(target, "java/lang/Object" | "java/lang/annotation/Annotation")
            || target == annotation_type;
    }
    let from_key = if registry.contains(from) {
        from.to_string()
    } else {
        registry.class_key_from_source(from, Some(from))
    };
    let to_key = if registry.contains(to) {
        to.to_string()
    } else {
        registry.class_key_from_source(to, target_source_class)
    };
    let from_internal = registry.internal_name_for_class(&from_key);
    let to_internal = registry.internal_name_for_class(&to_key);
    if from_key == to_key || to_internal == "java/lang/Object" {
        return true;
    }
    if from_key.starts_with("$$Lambda$")
        && let Some(lambda_info) = registry.get_lambda(&from_key)
        && (lambda_info.sam_interface == to_key
            || lambda_info.sam_interface == to_internal)
    {
        return true;
    }
    if from_internal.starts_with('[') {
        return matches!(to_internal, "java/lang/Cloneable" | "java/io/Serializable");
    }
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    queue.push_back(from_key);
    while let Some(current) = queue.pop_front() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if !registry.contains(&current) {
            let _ = registry.ensure_loaded_from(&current, Some(from), loader);
        }
        let (super_class, interfaces) = match registry.get(&current) {
            Ok(ctx) => (ctx.super_class.clone(), ctx.interfaces.clone()),
            Err(_) => continue,
        };
        if let Some(sc) = super_class {
            if sc == to_key {
                return true;
            }
            queue.push_back(sc);
        }
        for iface in interfaces {
            if iface == to_key {
                return true;
            }
            queue.push_back(iface);
        }
    }
    false
}
const fn is_duke_provider_name(name: &str) -> bool {
    name.eq_ignore_ascii_case(DUKE_SECURITY_PROVIDER)
}
const fn is_message_digest_service_type(service_type: &str) -> bool {
    service_type.eq_ignore_ascii_case(MESSAGE_DIGEST_SERVICE_TYPE)
}
const fn is_properties_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\u{000c}')
}
