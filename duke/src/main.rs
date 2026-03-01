use std::process;

use duke_bytecode::decode;
use duke_classfile::{
    ClassFile, parse,
    types::{AttributeData, CpEntry, CpIndex},
};
use duke_interpreter::{ClassContext, MethodEntry, execute_class};
use duke_loader::{ClassLoader, DirectoryLoader};
use duke_runtime::Slot;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: duke <classfile.class>");
        eprintln!("       duke dump <classfile.class>");
        eprintln!("       duke load <ClassName>");
        eprintln!("       duke exec <classfile.class> <method> [int-arg...]");
        process::exit(1);
    }

    // Dispatch `load` before trying to read a file.
    if args.len() >= 3 && args[1] == "load" {
        load_and_dump(&args[2]);
        return;
    }

    // Dispatch `exec`: run a static method and print the result.
    if args.len() >= 4 && args[1] == "exec" {
        exec_method(&args[2..]);
        return;
    }

    // Default: dump — reads the file at the given path.
    let (subcommand, path) = if args.len() >= 3 {
        (args[1].as_str(), args[2].as_str())
    } else {
        ("dump", args[1].as_str())
    };

    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{}': {e}", path);
        process::exit(1);
    });

    let class_file = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error in '{}': {e}", path);
        process::exit(1);
    });

    match subcommand {
        "dump" => dump_class_file(&class_file),
        other => {
            eprintln!("duke: unknown subcommand '{other}'");
            process::exit(1);
        }
    }
}

/// `duke load <ClassName>` — load a class by internal name from the current
/// directory and dump its structure.
///
/// Example: `duke load HelloWorld` loads `./HelloWorld.class`.
fn load_and_dump(class_name: &str) {
    let loader = DirectoryLoader::new(".");
    let bytes = loader.find_class(class_name).unwrap_or_else(|e| {
        eprintln!("duke: {e}");
        process::exit(1);
    });
    let class_file = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error for '{class_name}': {e}");
        process::exit(1);
    });
    dump_class_file(&class_file);
}

// ---------------------------------------------------------------------------
// Exec
// ---------------------------------------------------------------------------

/// `duke exec <classfile.class> <method> [int-arg...]`
///
/// Parses and executes a static method, printing the return value.
fn exec_method(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: duke exec <classfile.class> <method> [int-arg...]");
        process::exit(1);
    }
    let path = &args[0];
    let method_name = &args[1];
    let int_args: Vec<Slot> = args[2..]
        .iter()
        .map(|s| {
            Slot::Int(s.parse::<i32>().unwrap_or_else(|_| {
                eprintln!("duke: argument '{s}' is not an integer");
                process::exit(1);
            }))
        })
        .collect();

    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    // Find the target method's descriptor.
    let target = cf
        .methods
        .iter()
        .find(|m| {
            let Some(Some(CpEntry::Utf8(s))) = cf.constant_pool.get(m.name_index.0 as usize) else {
                return false;
            };
            s.as_str() == method_name.as_str()
        })
        .unwrap_or_else(|| {
            eprintln!("duke: method '{method_name}' not found");
            process::exit(1);
        });
    let descriptor = cp_str(&cf, target.descriptor_index)
        .unwrap_or("")
        .to_string();

    // Build ClassContext -- decode all methods.
    let methods: Vec<MethodEntry> = cf
        .methods
        .iter()
        .filter_map(|m| {
            let name = match cf.constant_pool.get(m.name_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let desc = match cf.constant_pool.get(m.descriptor_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let code = m.attributes.iter().find_map(|a| {
                if let AttributeData::Code(c) = &a.data {
                    Some(c)
                } else {
                    None
                }
            })?;
            let instructions = decode(&code.code).ok()?;
            Some(MethodEntry {
                name,
                descriptor: desc,
                instructions,
                max_stack: code.max_stack,
                max_locals: code.max_locals,
            })
        })
        .collect();

    let ctx = ClassContext {
        constant_pool: cf.constant_pool,
        methods,
    };

    match execute_class(&ctx, method_name, &descriptor, &int_args) {
        Ok(Some(result)) => println!("{result:?}"),
        Ok(None) => println!("(void)"),
        Err(e) => {
            eprintln!("duke: runtime error: {e}");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Dump
// ---------------------------------------------------------------------------

fn dump_class_file(cf: &ClassFile) {
    let this_name = resolve_class_name(cf, cf.this_class);
    let super_name = resolve_class_name(cf, cf.super_class);

    println!("Class:        {this_name}");
    println!("Super:        {super_name}");
    println!("Version:      {}.{}", cf.major_version, cf.minor_version);
    println!("Access flags: {:?}", cf.access_flags);
    println!();

    // Constant pool
    println!(
        "=== Constant Pool ({} slots) ===",
        cf.constant_pool.len() - 1
    );
    for (i, entry) in cf.constant_pool.iter().enumerate() {
        if i == 0 {
            continue; // slot 0 is reserved
        }
        match entry {
            None => println!("  #{i:3}  (phantom — occupied by preceding Long/Double)"),
            Some(e) => println!("  #{i:3}  {}", format_cp_entry(cf, e)),
        }
    }
    println!();

    // Interfaces
    if !cf.interfaces.is_empty() {
        println!("=== Interfaces ===");
        for idx in &cf.interfaces {
            println!("  {}", resolve_class_name(cf, *idx));
        }
        println!();
    }

    // Fields
    if !cf.fields.is_empty() {
        println!("=== Fields ({}) ===", cf.fields.len());
        for field in &cf.fields {
            let name = cp_str(cf, field.name_index).unwrap_or("<invalid>");
            let desc = cp_str(cf, field.descriptor_index).unwrap_or("<invalid>");
            println!("  [{:?}] {name}: {desc}", field.access_flags);
        }
        println!();
    }

    // Methods
    println!("=== Methods ({}) ===", cf.methods.len());
    for method in &cf.methods {
        let name = cp_str(cf, method.name_index).unwrap_or("<invalid>");
        let desc = cp_str(cf, method.descriptor_index).unwrap_or("<invalid>");
        println!("  [{:?}] {name}{desc}", method.access_flags);

        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                println!(
                    "    code: {} bytes, max_stack={}, max_locals={}",
                    code.code.len(),
                    code.max_stack,
                    code.max_locals,
                );
                // Decode and print instructions
                match decode(&code.code) {
                    Ok(instructions) => {
                        for (pc, instr) in &instructions {
                            println!("      {:4}: {}", pc, instr.mnemonic());
                        }
                    }
                    Err(e) => println!("      [decode error: {e}]"),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

fn format_cp_entry(cf: &ClassFile, entry: &CpEntry) -> String {
    match entry {
        CpEntry::Utf8(s) => format!("Utf8           \"{s}\""),
        CpEntry::Integer(v) => format!("Integer         {v}"),
        CpEntry::Float(v) => format!("Float           {v}"),
        CpEntry::Long(v) => format!("Long            {v}L"),
        CpEntry::Double(v) => format!("Double          {v}"),
        CpEntry::Class { name_index } => {
            let name = cp_str(cf, *name_index).unwrap_or("?");
            format!("Class           #{} // {name}", name_index.0)
        }
        CpEntry::String { string_index } => {
            let s = cp_str(cf, *string_index).unwrap_or("?");
            format!("String          #{} // \"{s}\"", string_index.0)
        }
        CpEntry::Fieldref {
            class_index,
            name_and_type_index,
        } => {
            format!(
                "Fieldref        #{}.#{}",
                class_index.0, name_and_type_index.0
            )
        }
        CpEntry::Methodref {
            class_index,
            name_and_type_index,
        } => {
            format!(
                "Methodref       #{}.#{}",
                class_index.0, name_and_type_index.0
            )
        }
        CpEntry::InterfaceMethodref {
            class_index,
            name_and_type_index,
        } => {
            format!(
                "IfaceMethodref  #{}.#{}",
                class_index.0, name_and_type_index.0
            )
        }
        CpEntry::NameAndType {
            name_index,
            descriptor_index,
        } => {
            let name = cp_str(cf, *name_index).unwrap_or("?");
            let desc = cp_str(cf, *descriptor_index).unwrap_or("?");
            format!(
                "NameAndType     #{}.#{} // {name}:{desc}",
                name_index.0, descriptor_index.0
            )
        }
        CpEntry::MethodHandle {
            reference_kind,
            reference_index,
        } => {
            format!(
                "MethodHandle    kind={reference_kind} ref=#{}",
                reference_index.0
            )
        }
        CpEntry::MethodType { descriptor_index } => {
            let desc = cp_str(cf, *descriptor_index).unwrap_or("?");
            format!("MethodType      #{} // {desc}", descriptor_index.0)
        }
        CpEntry::Dynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        } => {
            format!(
                "Dynamic         bsm={bootstrap_method_attr_index} nat=#{}",
                name_and_type_index.0
            )
        }
        CpEntry::InvokeDynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        } => {
            format!(
                "InvokeDynamic   bsm={bootstrap_method_attr_index} nat=#{}",
                name_and_type_index.0
            )
        }
        CpEntry::Module { name_index } => {
            let name = cp_str(cf, *name_index).unwrap_or("?");
            format!("Module          #{} // {name}", name_index.0)
        }
        CpEntry::Package { name_index } => {
            let name = cp_str(cf, *name_index).unwrap_or("?");
            format!("Package         #{} // {name}", name_index.0)
        }
    }
}
