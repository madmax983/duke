#![allow(clippy::items_after_statements)]
#[cfg(feature = "nova")]
use duke_bytecode::decode;
#[cfg(feature = "nova")]
use duke_classfile::{
    parse, {AttributeData, CpEntry, CpIndex},
};
use std::process;

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn cp_str(cf: &duke_classfile::ClassFile, idx: CpIndex) -> Option<&str> {
    cf.constant_pool
        .get(idx.0 as usize)
        .and_then(|slot: &Option<CpEntry>| slot.as_ref())
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
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if
)]
pub fn dump_purity_analysis(path: &str) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    println!("=== Purity Analysis Report ===");
    println!("Class File: {path}");
    println!();

    let mut pure_methods = Vec::new();
    let mut impure_methods = Vec::new();

    for method in &cf.methods {
        let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
        let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
        let full_name = format!("{name_str}{desc_str}");

        let mut is_pure = true;

        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                if let Ok(instructions) = decode(&code.code) {
                    for (_, instr) in instructions {
                        let mnemonic = instr.mnemonic();
                        if mnemonic == "putstatic"
                            || mnemonic == "putfield"
                            || mnemonic == "invokevirtual"
                            || mnemonic == "invokespecial"
                            || mnemonic == "invokestatic"
                            || mnemonic == "invokeinterface"
                            || mnemonic == "invokedynamic"
                        {
                            is_pure = false;
                            break;
                        }
                    }
                }
            }
        }
        if is_pure {
            pure_methods.push(full_name);
        } else {
            impure_methods.push(full_name);
        }
    }

    println!("Pure Methods:");
    for m in pure_methods {
        println!("  - {m}");
    }
    println!();
    println!("Impure Methods:");
    for m in impure_methods {
        println!("  - {m}");
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_purity_analysis_valid() {
        let valid_bytes = vec![
            0xca, 0xfe, 0xba, 0xbe, // magic
            0x00, 0x00, // minor
            0x00, 0x3d, // major (61)
            0x00, 0x04, // constant_pool_count (4)
            // #1: Utf8 "oldMethod"
            0x01, 0x00, 0x09, 0x6f, 0x6c, 0x64, 0x4d, 0x65, 0x74, 0x68, 0x6f, 0x64,
            // #2: Utf8 "()V"
            0x01, 0x00, 0x03, 0x28, 0x29, 0x56, // #3: Utf8 "Code"
            0x01, 0x00, 0x04, 0x43, 0x6f, 0x64, 0x65, 0x00, 0x01, // access_flags (public)
            0x00, 0x00, // this_class (0)
            0x00, 0x00, // super_class (0)
            0x00, 0x00, // interfaces_count (0)
            0x00, 0x00, // fields_count (0)
            0x00, 0x01, // methods_count (1)
            0x00, 0x01, // method[0].access_flags
            0x00, 0x01, // method[0].name_index
            0x00, 0x02, // method[0].descriptor_index
            0x00, 0x01, // method[0].attributes_count
            0x00, 0x03, // attr name index
            0x00, 0x00, 0x00, 0x0e, // attr len
            0x00, 0x01, // max stack
            0x00, 0x01, // max locals
            0x00, 0x00, 0x00, 0x02, // code len
            0x03, 0xac, // iconst_0, ireturn
            0x00, 0x00, // exception table len
            0x00, 0x00, // attributes count
            0x00, 0x00, // class attributes_count (0)
        ];
        let dir = std::env::temp_dir();
        let path = dir.join("purity_test_file.class");
        std::fs::write(&path, &valid_bytes).unwrap();

        dump_purity_analysis(path.to_str().unwrap());

        std::fs::remove_file(&path).unwrap();
    }
}
