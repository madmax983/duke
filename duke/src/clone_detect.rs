use duke_bytecode::{calculate_similarity, decode};
use duke_classfile::{AttributeData, ClassFile, CpEntry, CpIndex};
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

pub fn dump_clone_detect(cf: &ClassFile, threshold_str: Option<&str>) {
    let threshold: f64 = threshold_str.unwrap_or("0.8").parse().unwrap_or_else(|_| {
        eprintln!("duke: invalid threshold value");
        process::exit(1);
    });

    let mut methods = Vec::new();

    for method in &cf.methods {
        let name = cp_str(cf, method.name_index).unwrap_or("<unknown>");
        #[allow(clippy::collapsible_if)]
        if let Some(code_attr) = method
            .attributes
            .iter()
            .find(|a| cp_str(cf, a.name_index) == Some("Code"))
        {
            if let AttributeData::Code(code) = &code_attr.data {
                let instructions = decode(&code.code).unwrap_or_default();
                let instrs: Vec<_> = instructions.into_iter().map(|(_, i)| i).collect();
                if instrs.len() >= 5 {
                    methods.push((name, instrs));
                }
            }
        }
    }

    let mut found = false;
    for i in 0..methods.len() {
        for j in (i + 1)..methods.len() {
            let sim = calculate_similarity(&methods[i].1, &methods[j].1);
            if sim >= threshold {
                found = true;
                println!(
                    "similarity {:.2}: {} <-> {}",
                    sim, methods[i].0, methods[j].0
                );
            }
        }
    }
    if !found {
        println!("No structural clones found.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::ClassAccessFlags;

    #[test]
    fn test_dump_clone_detect_empty() {
        let cf = ClassFile {
            minor_version: 0,
            major_version: 52,
            constant_pool: vec![],
            access_flags: ClassAccessFlags::empty(),
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        dump_clone_detect(&cf, None);
    }
}
