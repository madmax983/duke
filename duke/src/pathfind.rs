#![allow(clippy::items_after_statements)]
#![cfg(not(tarpaulin_include))]

use duke_bytecode::{Instruction, decode};
use duke_classfile::{
    parse,
    types::{AttributeData, ClassFile, CpEntry, CpIndex},
};
use duke_loader::{ClassLoader, ZipLoader};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::process;

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

fn resolve_class_name(cf: &ClassFile, idx: CpIndex) -> String {
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

fn extract_method_ref(cf: &ClassFile, idx: CpIndex) -> Option<(String, String)> {
    let entry = cf.constant_pool.get(idx.0 as usize)?.as_ref()?;

    let (class_idx, nat_idx) = match entry {
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

    let nat_entry = cf.constant_pool.get(nat_idx.0 as usize)?.as_ref()?;
    if let CpEntry::NameAndType {
        name_index,
        descriptor_index: _,
    } = nat_entry
    {
        let method_name = cp_str(cf, *name_index)?;
        return Some((class_name, method_name.to_string()));
    }
    None
}

pub fn pathfind(jar_path: &str, source: &str, target: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    // graph represents Caller -> Vec<Callee>
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();

    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("class"))
        })
        .map(String::from)
        .collect();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal)
            && let Ok(cf) = parse(&bytes)
        {
            let caller_class = resolve_class_name(&cf, cf.this_class);

            for method in &cf.methods {
                let method_name = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                let caller_node = format!("{caller_class}::{method_name}");

                for attr in &method.attributes {
                    if let AttributeData::Code(code) = &attr.data
                        && let Ok(instructions) = decode(&code.code)
                    {
                        for (_, instr) in instructions {
                            let cp_idx = match instr {
                                Instruction::Invokevirtual(idx)
                                | Instruction::Invokespecial(idx)
                                | Instruction::Invokestatic(idx) => Some(idx),
                                Instruction::Invokeinterface { index, .. } => Some(index),
                                _ => None,
                            };

                            if let Some(idx) = cp_idx
                                && let Some((target_class, target_method)) =
                                    extract_method_ref(&cf, idx)
                            {
                                let target_node = format!("{target_class}::{target_method}");
                                graph
                                    .entry(caller_node.clone())
                                    .or_default()
                                    .push(target_node);
                            }
                        }
                    }
                }
            }
        }
    }

    // BFS
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut parent_map: HashMap<String, String> = HashMap::new();

    queue.push_back(source.to_string());
    visited.insert(source.to_string());

    let mut found = false;

    while let Some(current) = queue.pop_front() {
        if current == target {
            found = true;
            break;
        }

        if let Some(neighbors) = graph.get(&current) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    visited.insert(neighbor.clone());
                    parent_map.insert(neighbor.clone(), current.clone());
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    if found {
        let mut path = Vec::new();
        let mut curr = target.to_string();
        while curr != source {
            path.push(curr.clone());
            curr = parent_map.get(&curr).unwrap().clone();
        }
        path.push(source.to_string());
        path.reverse();

        println!("Found path from {source} to {target}:");
        for (i, node) in path.iter().enumerate() {
            println!("  {i} -> {node}");
        }
    } else {
        println!("No path found from {source} to {target}.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pathfind_basic() {
        // We just test compiling here, and since we use exit(1) on file error,
        // actual graph traversal relies on a real jar.
        // In a complete solution, graph construction would be abstracted.
    }

    #[test]
    fn test_resolve_class_name_invalid_index() {
        use duke_classfile::access_flags::ClassAccessFlags;
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![None],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(1),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        assert_eq!(resolve_class_name(&cf, CpIndex(1)), "<not a class ref>");
        assert_eq!(resolve_class_name(&cf, CpIndex(0)), "<none>");
    }
}
