use duke_bytecode::{Instruction, decode};
use duke_classfile::{
    ClassFile,
    types::{AttributeData, CpEntry, CpIndex},
};
use duke_loader::{ClassLoader, ZipLoader};
use std::collections::HashSet;
use std::path::Path;
use std::process;

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn cp_str(cf: &ClassFile, idx: CpIndex) -> Option<&str> {
    cf.constant_pool
        .get(idx.0 as usize)
        .and_then(|slot| slot.as_ref())
        .and_then(|entry| {
            if let CpEntry::Utf8(s) = entry {
                Some(s.as_str())
            } else {
                None
            }
        })
}

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn resolve_class_name(cf: &ClassFile, idx: CpIndex) -> &str {
    if idx.0 == 0 {
        return "<none>";
    }
    let class_entry = cf
        .constant_pool
        .get(idx.0 as usize)
        .and_then(|s| s.as_ref());
    if let Some(CpEntry::Class { name_index }) = class_entry {
        cp_str(cf, *name_index).unwrap_or("<invalid utf8>")
    } else {
        "<not a class ref>"
    }
}

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn extract_method_ref(cf: &ClassFile, idx: CpIndex) -> Option<(String, String, String)> {
    let entry = cf.constant_pool.get(idx.0 as usize)?.as_ref()?;

    let (CpEntry::Methodref {
        class_index: class_idx,
        name_and_type_index: nat_idx,
    }
    | CpEntry::InterfaceMethodref {
        class_index: class_idx,
        name_and_type_index: nat_idx,
    }) = entry
    else {
        return None;
    };

    let class_name = resolve_class_name(cf, *class_idx).to_string();

    let nat_entry = cf.constant_pool.get(nat_idx.0 as usize)?.as_ref()?;
    if let CpEntry::NameAndType {
        name_index,
        descriptor_index,
    } = nat_entry
    {
        let method_name = cp_str(cf, *name_index)?.to_string();
        let method_desc = cp_str(cf, *descriptor_index)?.to_string();
        Some((class_name, method_name, method_desc))
    } else {
        None
    }
}

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::collapsible_if,
    clippy::items_after_statements
)]
pub fn dump_dead_code(cf: &ClassFile) {
    let mut defined_methods = HashSet::new();
    let mut called_methods = HashSet::new();

    let class_name = resolve_class_name(cf, cf.this_class);

    for method in &cf.methods {
        let name_str = cp_str(cf, method.name_index).unwrap_or("<invalid>");
        let desc_str = cp_str(cf, method.descriptor_index).unwrap_or("<invalid>");
        let full_name = format!("{class_name}::{name_str}{desc_str}");

        defined_methods.insert(full_name.clone());

        if name_str == "main" || name_str == "<clinit>" || name_str == "<init>" {
            called_methods.insert(full_name);
        }

        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                if let Ok(instructions) = decode(&code.code) {
                    for (_, instr) in instructions {
                        let target_idx = match instr {
                            Instruction::Invokevirtual(idx)
                            | Instruction::Invokespecial(idx)
                            | Instruction::Invokestatic(idx)
                            | Instruction::Invokeinterface { index: idx, .. } => Some(idx),
                            _ => None,
                        };

                        if let Some(idx) = target_idx {
                            if let Some((target_class, target_method, target_descriptor)) =
                                extract_method_ref(cf, idx)
                            {
                                if target_class == class_name {
                                    called_methods.insert(format!(
                                        "{target_class}::{target_method}{target_descriptor}"
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("======================================");
    println!(" Class Dead Code Analysis");
    println!("======================================");
    println!("Class:                {class_name}");

    let mut dead_methods: Vec<_> = defined_methods.difference(&called_methods).collect();
    dead_methods.sort();

    if dead_methods.is_empty() {
        println!("✅ No dead internal methods detected.");
    } else {
        println!("🚨 Potentially Unreachable Methods (Dead Code):");
        for method in dead_methods {
            println!("  - {method}");
        }
    }
}

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::cast_precision_loss,
    clippy::collapsible_if,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::items_after_statements
)]
pub fn dump_jar_dead_code(jar_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let mut defined_methods = HashSet::new();
    let mut called_methods = HashSet::new();
    let mut total_classes = 0;

    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(std::string::ToString::to_string)
        .collect();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = duke_classfile::parse(&bytes) {
                total_classes += 1;
                let class_name = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{class_name}::{name_str}{desc_str}");

                    defined_methods.insert(full_name.clone());

                    if name_str == "main" || name_str == "<clinit>" || name_str == "<init>" {
                        called_methods.insert(full_name);
                    }

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                for (_, instr) in instructions {
                                    let target_idx = match instr {
                                        Instruction::Invokevirtual(idx)
                                        | Instruction::Invokespecial(idx)
                                        | Instruction::Invokestatic(idx)
                                        | Instruction::Invokeinterface { index: idx, .. } => {
                                            Some(idx)
                                        }
                                        _ => None,
                                    };

                                    if let Some(idx) = target_idx {
                                        if let Some((
                                            target_class,
                                            target_method,
                                            target_descriptor,
                                        )) = extract_method_ref(&cf, idx)
                                        {
                                            called_methods.insert(format!(
                                                "{target_class}::{target_method}{target_descriptor}"
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("======================================");
    println!(" JAR Dead Code Analysis");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Total Classes:        {total_classes}");

    let mut dead_methods: Vec<_> = defined_methods.difference(&called_methods).collect();
    dead_methods.sort();

    if dead_methods.is_empty() {
        println!("✅ No dead methods detected. Great architecture!");
    } else {
        println!("🚨 Potentially Unreachable Methods (Dead Code):");
        for method in dead_methods {
            println!("  - {method}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "nova")]
    fn test_dump_dead_code() {
        use duke_classfile::access_flags::{ClassAccessFlags, MethodAccessFlags};
        use duke_classfile::types::{AttributeInfo, CodeAttribute, MethodInfo};

        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None,
                Some(CpEntry::Class {
                    name_index: CpIndex(2),
                }),
                Some(CpEntry::Utf8("MyClass".to_string())),
                Some(CpEntry::Utf8("myMethod".to_string())),
                Some(CpEntry::Utf8("()V".to_string())),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(1),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![MethodInfo {
                access_flags: MethodAccessFlags::PUBLIC,
                name_index: CpIndex(3),
                descriptor_index: CpIndex(4),
                attributes: vec![AttributeInfo {
                    name_index: CpIndex(0),
                    data: AttributeData::Code(CodeAttribute {
                        max_stack: 1,
                        max_locals: 1,
                        code: vec![0xb1],
                        exception_table: vec![],
                        attributes: vec![],
                    }),
                }],
            }],
            attributes: vec![],
        };
        dump_dead_code(&cf);
    }
}
