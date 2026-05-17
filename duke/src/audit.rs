#![allow(clippy::items_after_statements)]
#[cfg(feature = "nova")]
use duke_bytecode::Instruction;
#[cfg(feature = "nova")]
use duke_bytecode::decode;
#[cfg(feature = "nova")]
use duke_classfile::{
    parse, {AttributeData, CpEntry, CpIndex},
};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::path::Path;
use std::process;

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs, dead_code)]
fn cp_str(cf: &duke_classfile::ClassFile, idx: CpIndex) -> Option<&str> {
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

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs, dead_code)]
fn resolve_class_name(cf: &duke_classfile::ClassFile, idx: CpIndex) -> String {
    if idx.0 == 0 {
        return "<none>".to_string();
    }
    let class_entry = cf
        .constant_pool
        .get(idx.0 as usize)
        .and_then(|s| s.as_ref());
    if let Some(CpEntry::Class { name_index }) = class_entry {
        cp_str(cf, *name_index)
            .unwrap_or("<invalid utf8>")
            .to_string()
    } else {
        "<not a class ref>".to_string()
    }
}

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs, dead_code)]
fn resolve_method_ref(cf: &duke_classfile::ClassFile, idx: CpIndex) -> Option<(String, String)> {
    let entry = cf.constant_pool.get(idx.0 as usize)?.as_ref()?;

    let (class_idx, nt_idx) = match entry {
        CpEntry::Methodref {
            class_index,
            name_and_type_index,
        }
        | CpEntry::InterfaceMethodref {
            class_index,
            name_and_type_index,
        } => (*class_index, *name_and_type_index),
        _ => return None,
    };

    let class_name = resolve_class_name(cf, class_idx);

    let nt_entry = cf.constant_pool.get(nt_idx.0 as usize)?.as_ref()?;
    if let CpEntry::NameAndType { name_index, .. } = nt_entry {
        let method_name = cp_str(cf, *name_index).unwrap_or("<invalid>").to_string();
        Some((class_name, method_name))
    } else {
        None
    }
}

#[cfg(feature = "nova")]
#[allow(clippy::print_stdout, clippy::collapsible_if, clippy::use_debug)]
pub fn dump_jar_audit(jar_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let reader = loader.reader();
    let mut findings = Vec::new();

    for entry_name in reader.entry_names() {
        if !std::path::Path::new(&entry_name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("class"))
        {
            continue;
        }
        let class_name_internal = &entry_name[..entry_name.len() - 6];
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let this_class = resolve_class_name(&cf, cf.this_class);
                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{this_class}::{name_str}{desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                for (pc, instr) in instructions {
                                    let method_ref_idx = match instr {
                                        Instruction::Invokevirtual(idx)
                                        | Instruction::Invokespecial(idx)
                                        | Instruction::Invokestatic(idx) => Some(idx),
                                        Instruction::Invokeinterface { index, .. } => Some(index),
                                        _ => None,
                                    };

                                    if let Some(idx) = method_ref_idx {
                                        if let Some((target_class, target_method)) =
                                            resolve_method_ref(&cf, idx)
                                        {
                                            if target_class == "java/lang/System"
                                                && target_method == "exit"
                                            {
                                                findings.push(format!("  [!] System.exit() called in {full_name} @ {pc}"));
                                            } else if target_class == "java/lang/Runtime"
                                                && target_method == "exec"
                                            {
                                                findings.push(format!("  [!] Runtime.exec() called in {full_name} @ {pc}"));
                                            } else if target_class == "java/lang/reflect/Method"
                                                && target_method == "invoke"
                                            {
                                                findings.push(format!("  [!] Method.invoke() (Reflection) used in {full_name} @ {pc}"));
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
    }

    println!("======================================");
    println!(" JAR Security Audit");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!();

    if findings.is_empty() {
        println!("✅ No dangerous API calls detected.");
    } else {
        println!("🚨 Dangerous API usages found:");
        for finding in findings {
            println!("{finding}");
        }
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_jar_audit_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        dump_jar_audit(path_str);
    }
}
