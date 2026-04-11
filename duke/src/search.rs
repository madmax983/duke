use duke_bytecode::decode;
use duke_classfile::{
    parse,
    types::{AttributeData, CpEntry, CpIndex},
};
use std::process;

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

/// Searches for a specific bytecode mnemonic string in all methods of a class file.
///
/// **Why it exists:** When debugging compiled Java applications, it is often necessary
/// to find exactly where a specific instruction (like `invokedynamic` or `getfield`)
/// is being used without decompiling the entire JAR. This function provides a quick
/// `grep`-like experience specifically tailored for JVM bytecode.
///
/// This command-line utility prints out the method names and instructions that
/// match the given `query` (case-insensitive).
///
/// # Examples
///
/// ```ignore
/// // Example assumes a valid .class file path exists
/// use duke::search::dump_search;
///
/// // Find all usages of 'invokevirtual' in MyClass.class
/// dump_search("MyClass.class", "invokevirtual");
/// ```
pub fn dump_search(path: &str, query: &str) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    let query_lower = query.to_lowercase();
    let mut found_any = false;

    for method in &cf.methods {
        let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
        let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
        let full_name = format!("{name_str}{desc_str}");

        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                #[allow(clippy::collapsible_if)]
                if let Ok(instructions) = decode(&code.code) {
                    let mut found_in_method = false;
                    for (pc, instr) in instructions {
                        let mnemonic = instr.mnemonic().to_lowercase();
                        if mnemonic.contains(&query_lower) {
                            if !found_in_method {
                                println!("Method: {full_name}");
                                found_in_method = true;
                                found_any = true;
                            }
                            println!("  {pc:>4}: {mnemonic}");
                        }
                    }
                    if found_in_method {
                        println!();
                    }
                }
            }
        }
    }

    if !found_any {
        println!("No instructions matching '{query}' found.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::access_flags::ClassAccessFlags;
    use duke_classfile::types::ClassFile;

    #[test]
    fn test_dump_search_no_match() {
        // Just testing cp_str helper mostly, dump_search is tested by its effects.
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![None, Some(CpEntry::Utf8("testMethod".to_string()))],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        assert_eq!(cp_str(&cf, CpIndex(1)), Some("testMethod"));
        assert_eq!(cp_str(&cf, CpIndex(2)), None);
    }
}
