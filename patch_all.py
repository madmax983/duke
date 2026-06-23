with open('crates/duke-classfile/src/parser.rs', 'r') as f:
    text = f.read()

new_code = """fn parse_cp_entry(c: &mut Cursor<'_>, tag: u8, i: usize) -> Result<CpEntry> {
    match tag {
        1 => parse_cp_utf8(c),
        3 => Ok(CpEntry::Integer(c.read_i32()?)),
        4 => Ok(CpEntry::Float(c.read_f32()?)),
        5 => Ok(CpEntry::Long(c.read_i64()?)),
        6 => Ok(CpEntry::Double(c.read_f64()?)),
        7 => parse_cp_class(c),
        8 => parse_cp_string(c),
        9 => parse_cp_fieldref(c),
        10 => parse_cp_methodref(c),
        11 => parse_cp_interface_methodref(c),
        12 => parse_cp_name_and_type(c),
        15 => parse_cp_method_handle(c),
        16 => parse_cp_method_type(c),
        17 => parse_cp_dynamic(c),
        18 => parse_cp_invoke_dynamic(c),
        19 => parse_cp_module(c),
        20 => parse_cp_package(c),
        other => Err(Error::UnknownCpTag {
            tag: other,
            index: u16::try_from(i).unwrap_or(u16::MAX),
        }),
    }
}

fn parse_cp_utf8(c: &mut Cursor<'_>) -> Result<CpEntry> {
    let len = c.read_u16()? as usize;
    let bytes = c.read_bytes(len)?;
    let s = String::from_utf8(bytes.to_vec())?;
    Ok(CpEntry::Utf8(s))
}

fn parse_cp_class(c: &mut Cursor<'_>) -> Result<CpEntry> {
    Ok(CpEntry::Class {
        name_index: c.read_cp_index()?,
    })
}

fn parse_cp_string(c: &mut Cursor<'_>) -> Result<CpEntry> {
    Ok(CpEntry::String {
        string_index: c.read_cp_index()?,
    })
}

fn parse_cp_fieldref(c: &mut Cursor<'_>) -> Result<CpEntry> {
    Ok(CpEntry::Fieldref {
        class_index: c.read_cp_index()?,
        name_and_type_index: c.read_cp_index()?,
    })
}

fn parse_cp_methodref(c: &mut Cursor<'_>) -> Result<CpEntry> {
    Ok(CpEntry::Methodref {
        class_index: c.read_cp_index()?,
        name_and_type_index: c.read_cp_index()?,
    })
}

fn parse_cp_interface_methodref(c: &mut Cursor<'_>) -> Result<CpEntry> {
    Ok(CpEntry::InterfaceMethodref {
        class_index: c.read_cp_index()?,
        name_and_type_index: c.read_cp_index()?,
    })
}

fn parse_cp_name_and_type(c: &mut Cursor<'_>) -> Result<CpEntry> {
    Ok(CpEntry::NameAndType {
        name_index: c.read_cp_index()?,
        descriptor_index: c.read_cp_index()?,
    })
}

fn parse_cp_method_handle(c: &mut Cursor<'_>) -> Result<CpEntry> {
    let reference_kind = c.read_u8()?;
    if !(1..=9).contains(&reference_kind) {
        return Err(Error::InvalidMethodHandleKind {
            kind: reference_kind,
        });
    }
    Ok(CpEntry::MethodHandle {
        reference_kind,
        reference_index: c.read_cp_index()?,
    })
}

fn parse_cp_method_type(c: &mut Cursor<'_>) -> Result<CpEntry> {
    Ok(CpEntry::MethodType {
        descriptor_index: c.read_cp_index()?,
    })
}

fn parse_cp_dynamic(c: &mut Cursor<'_>) -> Result<CpEntry> {
    Ok(CpEntry::Dynamic {
        bootstrap_method_attr_index: c.read_u16()?,
        name_and_type_index: c.read_cp_index()?,
    })
}

fn parse_cp_invoke_dynamic(c: &mut Cursor<'_>) -> Result<CpEntry> {
    Ok(CpEntry::InvokeDynamic {
        bootstrap_method_attr_index: c.read_u16()?,
        name_and_type_index: c.read_cp_index()?,
    })
}

fn parse_cp_module(c: &mut Cursor<'_>) -> Result<CpEntry> {
    Ok(CpEntry::Module {
        name_index: c.read_cp_index()?,
    })
}

fn parse_cp_package(c: &mut Cursor<'_>) -> Result<CpEntry> {
    Ok(CpEntry::Package {
        name_index: c.read_cp_index()?,
    })
}"""

old_code = """fn parse_cp_entry(c: &mut Cursor<'_>, tag: u8, i: usize) -> Result<CpEntry> {
    match tag {
        1 => {
            let len = c.read_u16()? as usize;
            let bytes = c.read_bytes(len)?;
            let s = String::from_utf8(bytes.to_vec())?;
            Ok(CpEntry::Utf8(s))
        }
        3 => Ok(CpEntry::Integer(c.read_i32()?)),
        4 => Ok(CpEntry::Float(c.read_f32()?)),
        5 => Ok(CpEntry::Long(c.read_i64()?)),
        6 => Ok(CpEntry::Double(c.read_f64()?)),
        7 => Ok(CpEntry::Class {
            name_index: c.read_cp_index()?,
        }),
        8 => Ok(CpEntry::String {
            string_index: c.read_cp_index()?,
        }),
        9 => Ok(CpEntry::Fieldref {
            class_index: c.read_cp_index()?,
            name_and_type_index: c.read_cp_index()?,
        }),
        10 => Ok(CpEntry::Methodref {
            class_index: c.read_cp_index()?,
            name_and_type_index: c.read_cp_index()?,
        }),
        11 => Ok(CpEntry::InterfaceMethodref {
            class_index: c.read_cp_index()?,
            name_and_type_index: c.read_cp_index()?,
        }),
        12 => Ok(CpEntry::NameAndType {
            name_index: c.read_cp_index()?,
            descriptor_index: c.read_cp_index()?,
        }),
        15 => {
            let reference_kind = c.read_u8()?;
            if !(1..=9).contains(&reference_kind) {
                return Err(Error::InvalidMethodHandleKind {
                    kind: reference_kind,
                });
            }
            Ok(CpEntry::MethodHandle {
                reference_kind,
                reference_index: c.read_cp_index()?,
            })
        }
        16 => Ok(CpEntry::MethodType {
            descriptor_index: c.read_cp_index()?,
        }),
        17 => Ok(CpEntry::Dynamic {
            bootstrap_method_attr_index: c.read_u16()?,
            name_and_type_index: c.read_cp_index()?,
        }),
        18 => Ok(CpEntry::InvokeDynamic {
            bootstrap_method_attr_index: c.read_u16()?,
            name_and_type_index: c.read_cp_index()?,
        }),
        19 => Ok(CpEntry::Module {
            name_index: c.read_cp_index()?,
        }),
        20 => Ok(CpEntry::Package {
            name_index: c.read_cp_index()?,
        }),
        other => Err(Error::UnknownCpTag {
            tag: other,
            index: u16::try_from(i).unwrap_or(u16::MAX),
        }),
    }
}"""

if old_code in text:
    print("Found parser! Replacing...")
    text = text.replace(old_code, new_code)
    with open('crates/duke-classfile/src/parser.rs', 'w') as f:
        f.write(text)
else:
    print("parser Not found :(")
