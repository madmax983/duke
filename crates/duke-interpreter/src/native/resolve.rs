fn resolve_class_resource_name(class_internal_name: &str, name: &str) -> Option<String> {
    if let Some(absolute) = name.strip_prefix('/') {
        return normalize_resource_name(absolute);
    }
    let mut resolved = String::new();
    if let Some((package, _)) = class_internal_name.rsplit_once('/') {
        resolved.push_str(package);
        resolved.push('/');
    }
    resolved.push_str(name);
    normalize_resource_name(&resolved)
}
fn resolve_thread_entry(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &duke_gc::Heap,
    thread_ref: u64,
) -> Result<Option<(String, usize, Vec<Slot>)>> {
    let actual_class = heap.get(thread_ref)?.class_name.clone();
    if actual_class != "java/lang/Thread"
        && let Some((dispatch_class, method_idx)) = resolve_method_in_hierarchy(
            registry,
            loader,
            &actual_class,
            "run",
            "()V",
        )
    {
        return Ok(
            Some((dispatch_class, method_idx, vec![Slot::Reference(Some(thread_ref))])),
        );
    }
    let target_ref = match heap.get(thread_ref)?.fields.get(THREAD_TARGET_SLOT) {
        Some(Slot::Reference(Some(target_ref))) => *target_ref,
        _ => return Ok(None),
    };
    let target_class = heap.get(target_ref)?.class_name.clone();
    let (dispatch_class, method_idx) = resolve_method_in_hierarchy(
            registry,
            loader,
            &target_class,
            "run",
            "()V",
        )
        .ok_or_else(|| Error::AbstractMethodError {
            class_name: target_class.clone(),
            method_name: "run".to_string(),
        })?;
    Ok(Some((dispatch_class, method_idx, vec![Slot::Reference(Some(target_ref))])))
}
/// Resolve a CP Class entry to its name string.
fn resolve_class_name(cp: &[Option<CpEntry>], cp_idx: usize) -> Result<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Class { name_index }) => {
            match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => {
                    Err(Error::InvalidCpIndex {
                        index: name_index.0 as usize,
                    })
                }
            }
        }
        _ => {
            Err(Error::InvalidCpIndex {
                index: cp_idx,
            })
        }
    }
}
fn resolve_annotation_value(
    cp: &[Option<CpEntry>],
    value: &duke_classfile::types::ElementValue,
) -> Option<ReflectedAnnotationValue> {
    use duke_classfile::types::ElementValue;
    match value {
        ElementValue::ConstValueIndex(index) => {
            cp_annotation_const(cp, index.0 as usize)
                .map(ReflectedAnnotationValue::Const)
        }
        ElementValue::EnumConstValue { type_name_index, const_name_index } => {
            let type_descriptor = cp_utf8_string(cp, type_name_index.0 as usize).ok()?;
            let const_name = cp_utf8_string(cp, const_name_index.0 as usize).ok()?;
            Some(ReflectedAnnotationValue::Enum {
                type_name: annotation_descriptor_to_internal_name(&type_descriptor),
                const_name,
            })
        }
        ElementValue::ClassInfoIndex(index) => {
            let descriptor = cp_utf8_string(cp, index.0 as usize).ok()?;
            Some(
                ReflectedAnnotationValue::Class(
                    class_literal_descriptor_to_key(&descriptor),
                ),
            )
        }
        ElementValue::AnnotationValue(annotation) => {
            resolve_annotation(cp, annotation)
                .map(Box::new)
                .map(ReflectedAnnotationValue::Annotation)
        }
        ElementValue::ArrayValue(values) => {
            values
                .iter()
                .map(|value| resolve_annotation_value(cp, value))
                .collect::<Option<Vec<_>>>()
                .map(ReflectedAnnotationValue::Array)
        }
    }
}
fn resolve_annotation(
    cp: &[Option<CpEntry>],
    annotation: &duke_classfile::types::Annotation,
) -> Option<ReflectedAnnotation> {
    let descriptor = cp_utf8_string(cp, annotation.type_index.0 as usize).ok()?;
    let elements = annotation
        .element_value_pairs
        .iter()
        .map(|pair| {
            Some(ReflectedAnnotationElement {
                name: cp_utf8_string(cp, pair.element_name_index.0 as usize).ok()?,
                value: resolve_annotation_value(cp, &pair.value)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ReflectedAnnotation {
        type_name: annotation_descriptor_to_internal_name(&descriptor),
        elements,
    })
}
/// Walk the class hierarchy to find a bytecode method by name and descriptor,
/// stopping early if an exact native override owns that slot.
fn resolve_method_in_hierarchy_lookup(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    start_class: &str,
    method_name: &str,
    method_desc: &str,
) -> MethodHierarchyLookup {
    let mut current = if registry.contains(start_class) {
        start_class.to_string()
    } else {
        registry.class_key_from_source(start_class, Some(start_class))
    };
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return MethodHierarchyLookup::Missing;
        }
        if has_registered_native_override(registry, &current, method_name, method_desc) {
            return MethodHierarchyLookup::NativeOverride;
        }
        if !registry.contains(&current) {
            let _ = registry.ensure_loaded_from(&current, Some(start_class), loader);
            current = registry.class_key_from_source(&current, Some(start_class));
        }
        match registry.get(&current) {
            Ok(ctx) => {
                if let Some(idx) = ctx
                    .methods
                    .iter()
                    .position(|m| m.name == method_name && m.descriptor == method_desc)
                {
                    let method = &ctx.methods[idx];
                    if method.is_native {
                        return MethodHierarchyLookup::NativeOverride;
                    }
                    if !method.is_abstract {
                        return MethodHierarchyLookup::Bytecode(current, idx);
                    }
                }
                match &ctx.super_class {
                    Some(s) => current.clone_from(s),
                    None => return MethodHierarchyLookup::Missing,
                }
            }
            Err(_) => return MethodHierarchyLookup::Missing,
        }
    }
}
/// Walk the class hierarchy to find a method by name and descriptor.
/// Returns `(class_name_where_found, method_index)` or `None`.
fn resolve_method_in_hierarchy(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    start_class: &str,
    method_name: &str,
    method_desc: &str,
) -> Option<(String, usize)> {
    match resolve_method_in_hierarchy_lookup(
        registry,
        loader,
        start_class,
        method_name,
        method_desc,
    ) {
        MethodHierarchyLookup::Bytecode(class_name, method_idx) => {
            Some((class_name, method_idx))
        }
        MethodHierarchyLookup::NativeOverride | MethodHierarchyLookup::Missing => None,
    }
}
/// Resolve a constant pool Methodref to (`class_name`, `method_name`, `descriptor`).
fn resolve_methodref(
    cp: &[Option<CpEntry>],
    idx: usize,
) -> Result<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(
            CpEntry::Methodref { class_index, name_and_type_index }
            | CpEntry::InterfaceMethodref { class_index, name_and_type_index },
        ) => {
            let class_name = match cp
                .get(class_index.0 as usize)
                .and_then(|e| e.as_ref())
            {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => {
                            return Err(Error::InvalidMethodref {
                                index: idx,
                            });
                        }
                    }
                }
                _ => {
                    return Err(Error::InvalidMethodref {
                        index: idx,
                    });
                }
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType { name_index, descriptor_index }) => {
                    let name = match cp
                        .get(name_index.0 as usize)
                        .and_then(|e| e.as_ref())
                    {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => {
                            return Err(Error::InvalidMethodref {
                                index: idx,
                            });
                        }
                    };
                    let desc = match cp
                        .get(descriptor_index.0 as usize)
                        .and_then(|e| e.as_ref())
                    {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => {
                            return Err(Error::InvalidMethodref {
                                index: idx,
                            });
                        }
                    };
                    Ok((class_name, name, desc))
                }
                _ => {
                    Err(Error::InvalidMethodref {
                        index: nat_idx,
                    })
                }
            }
        }
        _ => {
            Err(Error::InvalidMethodref {
                index: idx,
            })
        }
    }
}
/// Resolve a constant pool Fieldref to (`class_name`, `field_name`, descriptor).
fn resolve_fieldref(
    cp: &[Option<CpEntry>],
    idx: usize,
) -> Result<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Fieldref { class_index, name_and_type_index }) => {
            let class_name = match cp
                .get(class_index.0 as usize)
                .and_then(|e| e.as_ref())
            {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => {
                            return Err(Error::InvalidFieldref {
                                index: idx,
                            });
                        }
                    }
                }
                _ => {
                    return Err(Error::InvalidFieldref {
                        index: idx,
                    });
                }
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType { name_index, descriptor_index }) => {
                    let name = match cp
                        .get(name_index.0 as usize)
                        .and_then(|e| e.as_ref())
                    {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => {
                            return Err(Error::InvalidFieldref {
                                index: idx,
                            });
                        }
                    };
                    let desc = match cp
                        .get(descriptor_index.0 as usize)
                        .and_then(|e| e.as_ref())
                    {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => {
                            return Err(Error::InvalidFieldref {
                                index: idx,
                            });
                        }
                    };
                    Ok((class_name, name, desc))
                }
                _ => {
                    Err(Error::InvalidFieldref {
                        index: nat_idx,
                    })
                }
            }
        }
        _ => {
            Err(Error::InvalidFieldref {
                index: idx,
            })
        }
    }
}
/// Resolve a `MethodHandle` CP entry to (`reference_kind`, `class_name`, `method_name`, descriptor).
fn resolve_method_handle(
    cp: &[Option<CpEntry>],
    cp_idx: usize,
) -> Result<(u8, String, String, String)> {
    let (kind, ref_idx) = match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::MethodHandle { reference_kind, reference_index }) => {
            (*reference_kind, reference_index.0 as usize)
        }
        _ => {
            return Err(Error::InvalidCpIndex {
                index: cp_idx,
            });
        }
    };
    let (class_name, method_name, descriptor) = resolve_methodref(cp, ref_idx)?;
    Ok((kind, class_name, method_name, descriptor))
}
/// Resolve a `NameAndType` CP entry to (name, descriptor).
fn resolve_name_and_type(
    cp: &[Option<CpEntry>],
    cp_idx: usize,
) -> Result<(String, String)> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::NameAndType { name_index, descriptor_index }) => {
            let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(Error::InvalidCpIndex {
                        index: name_index.0 as usize,
                    });
                }
            };
            let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref())
            {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(Error::InvalidCpIndex {
                        index: descriptor_index.0 as usize,
                    });
                }
            };
            Ok((name, desc))
        }
        _ => {
            Err(Error::InvalidCpIndex {
                index: cp_idx,
            })
        }
    }
}
/// Resolve a CP String entry to its UTF-8 content. Also handles bare Utf8 entries.
fn resolve_cp_string(cp: &[Option<CpEntry>], cp_idx: usize) -> Result<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::String { string_index }) => {
            match cp.get(string_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => {
                    Err(Error::InvalidCpIndex {
                        index: string_index.0 as usize,
                    })
                }
            }
        }
        Some(CpEntry::Utf8(s)) => Ok(s.clone()),
        _ => {
            Err(Error::InvalidCpIndex {
                index: cp_idx,
            })
        }
    }
}
