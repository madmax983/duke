#[cfg(feature = "nova")]
use duke_bytecode::decode;
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, ClassFile, CpEntry, CpIndex};

#[cfg(feature = "nova")]
#[allow(unexpected_cfgs)]
fn cp_str(cf: &ClassFile, idx: CpIndex) -> Option<&str> {
    cf.constant_pool
        .get(idx.0 as usize)
        .and_then(|slot: &Option<duke_classfile::CpEntry>| slot.as_ref())
        .and_then(|entry| {
            if let CpEntry::Utf8(s) = entry {
                Some(s.as_str())
            } else {
                None
            }
        })
}

#[cfg(feature = "nova")]
#[allow(unexpected_cfgs, clippy::print_stdout)]
pub fn dump_simulate(path: &str, method_name: &str) {
    let bytes = std::fs::read(path).expect("Failed to read class file");
    let cf = duke_classfile::parse(&bytes).expect("Failed to parse class file");

    let target = cf
        .methods
        .iter()
        .find(|m| cp_str(&cf, m.name_index) == Some(method_name));

    if let Some(target) = target {
        println!("Simulating execution of method '{method_name}'...");
        for attr in &target.attributes {
            #[allow(clippy::collapsible_if)]
            if let AttributeData::Code(code) = &attr.data {
                if let Ok(decoded) = decode(&code.code) {
                    println!("--- Instruction Trace ---");
                    for (pc, instr) in decoded {
                        println!("PC {pc:>3}: Executing {}", instr.mnemonic());
                    }
                    println!("--- End Trace ---");
                }
            }
        }
    } else {
        println!("duke: method '{method_name}' not found");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::{
        ClassAccessFlags, {ClassFile, CpEntry, CpIndex},
    };

    #[test]
    fn test_cp_str() {
        let cf1 = ClassFile {
            major_version: 52,
            minor_version: 0,
            constant_pool: vec![None, Some(CpEntry::Utf8("test".to_string()))],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        assert_eq!(cp_str(&cf1, CpIndex(1)), Some("test"));
    }
}
