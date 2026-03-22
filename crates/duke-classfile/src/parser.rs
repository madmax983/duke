//! `duke-classfile::parser` — JVM `.class` file parser implementation

use crate::{
    access_flags::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags},
    error::{ParseError, ParseResult},
    types::{
        AttributeData, AttributeInfo, BootstrapMethodEntry, ClassFile, CodeAttribute, CpEntry,
        CpIndex, ExceptionTableEntry, FieldInfo, LineNumberEntry, LocalVariableEntry, MethodInfo,
    },
};

const MAGIC: u32 = 0xCAFE_BABE;
/// Java SE 21 = class file version 65.
const MAX_MAJOR_VERSION: u16 = 65;

// ---------------------------------------------------------------------------
// Cursor
// ---------------------------------------------------------------------------

/// Byte-level cursor over an immutable slice.
///
/// Every read operation checks bounds and returns `ParseError::UnexpectedEof`
/// on failure — no panics on malformed input.
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    // Current read position.
    const fn position(&self) -> usize {
        self.pos
    }

    #[allow(dead_code)]
    const fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn read_u8(&mut self) -> ParseResult<u8> {
        if self.pos >= self.data.len() {
            return Err(ParseError::UnexpectedEof { offset: self.pos });
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_u16(&mut self) -> ParseResult<u16> {
        let hi = u16::from(self.read_u8()?);
        let lo = u16::from(self.read_u8()?);
        Ok((hi << 8) | lo)
    }

    #[allow(dead_code)]
    fn read_i16(&mut self) -> ParseResult<i16> {
        Ok(self.read_u16()?.cast_signed())
    }

    fn read_u32(&mut self) -> ParseResult<u32> {
        let hi = u32::from(self.read_u16()?);
        let lo = u32::from(self.read_u16()?);
        Ok((hi << 16) | lo)
    }

    fn read_i32(&mut self) -> ParseResult<i32> {
        Ok(self.read_u32()?.cast_signed())
    }

    fn read_u64(&mut self) -> ParseResult<u64> {
        let hi = u64::from(self.read_u32()?);
        let lo = u64::from(self.read_u32()?);
        Ok((hi << 32) | lo)
    }

    fn read_i64(&mut self) -> ParseResult<i64> {
        Ok(self.read_u64()?.cast_signed())
    }

    fn read_f32(&mut self) -> ParseResult<f32> {
        Ok(f32::from_bits(self.read_u32()?))
    }

    fn read_f64(&mut self) -> ParseResult<f64> {
        Ok(f64::from_bits(self.read_u64()?))
    }

    fn read_bytes(&mut self, len: usize) -> ParseResult<&'a [u8]> {
        if self.pos + len > self.data.len() {
            return Err(ParseError::UnexpectedEof { offset: self.pos });
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    fn read_cp_index(&mut self) -> ParseResult<CpIndex> {
        Ok(CpIndex(self.read_u16()?))
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse a JVM `.class` file from raw bytes.
///
/// Returns a fully-parsed [`ClassFile`] or a [`ParseError`] describing
/// exactly why the input is invalid.
///
/// # Errors
///
/// Returns an error if the input is truncated, has an invalid magic number,
/// an unsupported version, or any structural inconsistency.
///
/// # Examples
///
/// ```
/// use duke_classfile::parse;
///
/// // Create a minimal but invalid class file (wrong magic)
/// let bytes = [0x00, 0x00, 0x00, 0x00];
/// let result = parse(&bytes);
/// assert!(result.is_err());
/// ```
pub fn parse(bytes: &[u8]) -> ParseResult<ClassFile> {
    let mut cursor = Cursor::new(bytes);
    let mut class_file = parse_class_file(&mut cursor)?;

    // ⚡ Bolt: Removed expensive `class_file.constant_pool.clone()` allocation.
    // Rust allows disjoint borrowing, so we can borrow `constant_pool` immutably
    // while mutably borrowing `attributes`, `fields`, and `methods`.

    // Resolve raw attribute bytes into typed variants now that we have the full CP.
    let pool = &class_file.constant_pool;
    resolve_attributes(&mut class_file.attributes, pool)?;
    for field in &mut class_file.fields {
        resolve_attributes(&mut field.attributes, pool)?;
    }
    for method in &mut class_file.methods {
        resolve_attributes(&mut method.attributes, pool)?;
        // Also resolve Code sub-attributes.
        for attr in &mut method.attributes {
            if let AttributeData::Code(code) = &mut attr.data {
                resolve_attributes(&mut code.attributes, pool)?;
            }
        }
    }

    Ok(class_file)
}

// ---------------------------------------------------------------------------
// Class file structure
// ---------------------------------------------------------------------------

fn parse_class_file(c: &mut Cursor<'_>) -> ParseResult<ClassFile> {
    // §4.1 — magic
    let magic = c.read_u32()?;
    if magic != MAGIC {
        return Err(ParseError::BadMagic { got: magic });
    }

    let minor_version = c.read_u16()?;
    let major_version = c.read_u16()?;
    if major_version > MAX_MAJOR_VERSION {
        return Err(ParseError::UnsupportedVersion {
            major: major_version,
            minor: minor_version,
        });
    }

    let constant_pool = parse_constant_pool(c)?;
    let cp_len = constant_pool.len(); // for bounds validation helpers

    let access_flags = ClassAccessFlags::from_bits_truncate(c.read_u16()?);
    let this_class = c.read_cp_index()?;
    let super_class = c.read_cp_index()?;

    // interfaces
    let interfaces_count = c.read_u16()?;
    let interfaces: Vec<_> = (0..interfaces_count)
        .map(|_| c.read_cp_index())
        .collect::<ParseResult<_>>()?;

    // fields
    let fields_count = c.read_u16()?;
    let fields: Vec<_> = (0..fields_count)
        .map(|_| parse_field(c, cp_len))
        .collect::<ParseResult<_>>()?;

    // methods
    let methods_count = c.read_u16()?;
    let methods: Vec<_> = (0..methods_count)
        .map(|_| parse_method(c, cp_len))
        .collect::<ParseResult<_>>()?;

    // class-level attributes
    let attributes = parse_attributes(c, cp_len)?;

    Ok(ClassFile {
        minor_version,
        major_version,
        constant_pool,
        access_flags,
        this_class,
        super_class,
        interfaces,
        fields,
        methods,
        attributes,
    })
}

// ---------------------------------------------------------------------------
// Constant pool (§4.4)
// ---------------------------------------------------------------------------

fn parse_constant_pool(c: &mut Cursor<'_>) -> ParseResult<Vec<Option<CpEntry>>> {
    let count = c.read_u16()? as usize;
    // Index 0 is unused; spec uses 1-based indexing.
    // `count` is one more than the actual number of entries.
    let mut pool: Vec<Option<CpEntry>> = Vec::with_capacity(count);
    pool.push(None); // slot 0 — reserved

    let mut i = 1usize;
    while i < count {
        let tag = c.read_u8()?;
        #[allow(clippy::match_same_arms)]
        let entry = match tag {
            1 => {
                let len = c.read_u16()? as usize;
                let bytes = c.read_bytes(len)?;
                let s = String::from_utf8(bytes.to_vec())?;
                CpEntry::Utf8(s)
            }
            3 => CpEntry::Integer(c.read_i32()?),
            4 => CpEntry::Float(c.read_f32()?),
            5 => {
                let v = CpEntry::Long(c.read_i64()?);
                pool.push(Some(v));
                pool.push(None); // phantom slot
                i += 2;
                continue;
            }
            6 => {
                let v = CpEntry::Double(c.read_f64()?);
                pool.push(Some(v));
                pool.push(None); // phantom slot
                i += 2;
                continue;
            }
            7 => CpEntry::Class {
                name_index: c.read_cp_index()?,
            },
            8 => CpEntry::String {
                string_index: c.read_cp_index()?,
            },
            9 => CpEntry::Fieldref {
                class_index: c.read_cp_index()?,
                name_and_type_index: c.read_cp_index()?,
            },
            10 => CpEntry::Methodref {
                class_index: c.read_cp_index()?,
                name_and_type_index: c.read_cp_index()?,
            },
            11 => CpEntry::InterfaceMethodref {
                class_index: c.read_cp_index()?,
                name_and_type_index: c.read_cp_index()?,
            },
            12 => CpEntry::NameAndType {
                name_index: c.read_cp_index()?,
                descriptor_index: c.read_cp_index()?,
            },
            15 => {
                let reference_kind = c.read_u8()?;
                if !(1..=9).contains(&reference_kind) {
                    return Err(ParseError::InvalidMethodHandleKind {
                        kind: reference_kind,
                    });
                }
                CpEntry::MethodHandle {
                    reference_kind,
                    reference_index: c.read_cp_index()?,
                }
            }
            16 => CpEntry::MethodType {
                descriptor_index: c.read_cp_index()?,
            },
            17 => CpEntry::Dynamic {
                bootstrap_method_attr_index: c.read_u16()?,
                name_and_type_index: c.read_cp_index()?,
            },
            18 => CpEntry::InvokeDynamic {
                bootstrap_method_attr_index: c.read_u16()?,
                name_and_type_index: c.read_cp_index()?,
            },
            19 => CpEntry::Module {
                name_index: c.read_cp_index()?,
            },
            20 => CpEntry::Package {
                name_index: c.read_cp_index()?,
            },
            other => {
                return Err(ParseError::UnknownCpTag {
                    tag: other,
                    index: u16::try_from(i).unwrap_or(u16::MAX),
                });
            }
        };
        pool.push(Some(entry));
        i += 1;
    }

    Ok(pool)
}

// ---------------------------------------------------------------------------
// Fields and methods
// ---------------------------------------------------------------------------

fn parse_field(c: &mut Cursor<'_>, cp_len: usize) -> ParseResult<FieldInfo> {
    let access_flags = FieldAccessFlags::from_bits_truncate(c.read_u16()?);
    let name_index = c.read_cp_index()?;
    let descriptor_index = c.read_cp_index()?;
    let attributes = parse_attributes(c, cp_len)?;
    Ok(FieldInfo {
        access_flags,
        name_index,
        descriptor_index,
        attributes,
    })
}

fn parse_method(c: &mut Cursor<'_>, cp_len: usize) -> ParseResult<MethodInfo> {
    let access_flags = MethodAccessFlags::from_bits_truncate(c.read_u16()?);
    let name_index = c.read_cp_index()?;
    let descriptor_index = c.read_cp_index()?;
    let attributes = parse_attributes(c, cp_len)?;
    Ok(MethodInfo {
        access_flags,
        name_index,
        descriptor_index,
        attributes,
    })
}

// ---------------------------------------------------------------------------
// Attributes (§4.7)
// ---------------------------------------------------------------------------

fn parse_attributes(c: &mut Cursor<'_>, cp_len: usize) -> ParseResult<Vec<AttributeInfo>> {
    let count = c.read_u16()?;
    let attrs: Vec<_> = (0..count)
        .map(|_| parse_attribute(c, cp_len))
        .collect::<ParseResult<_>>()?;
    Ok(attrs)
}

fn parse_attribute(c: &mut Cursor<'_>, _cp_len: usize) -> ParseResult<AttributeInfo> {
    let name_index = c.read_cp_index()?;
    let attr_len = c.read_u32()? as usize;

    // Record position so we can verify the attribute consumed exactly attr_len bytes.
    let start = c.position();

    // We can't resolve the name string here (we don't have the constant pool string).
    // So we read the raw bytes and let the caller (who has the pool) re-parse if needed.
    // For now, we embed attribute-type detection via a separate helper after full parse.
    // Raw strategy: grab all bytes, then we'll decode known attributes by name post-parse.
    let raw = c.read_bytes(attr_len)?.to_vec();
    let consumed = c.position() - start;
    debug_assert_eq!(consumed, attr_len, "attribute byte mismatch");

    Ok(AttributeInfo {
        name_index,
        data: AttributeData::Raw(raw),
    })
}

// ---------------------------------------------------------------------------
// Post-parse attribute resolution (requires constant pool string lookup)
// ---------------------------------------------------------------------------

/// Resolve raw attributes into typed forms using the constant pool.
///
/// Called after the whole class file is parsed, when we have the full CP.
pub(crate) fn resolve_attributes(
    attrs: &mut [AttributeInfo],
    pool: &[Option<CpEntry>],
) -> ParseResult<()> {
    for attr in attrs.iter_mut() {
        let name = cp_utf8(pool, attr.name_index)?;
        let raw = match &mut attr.data {
            AttributeData::Raw(b) => std::mem::take(b),
            _ => continue, // already resolved
        };
        attr.data = decode_known_attribute(name, &raw)?;
    }
    Ok(())
}

fn decode_known_attribute(name: &str, raw: &[u8]) -> ParseResult<AttributeData> {
    let mut c = Cursor::new(raw);
    let data = match name {
        "ConstantValue" => {
            let idx = c.read_cp_index()?;
            AttributeData::ConstantValue {
                constant_value_index: idx,
            }
        }
        "SourceFile" => {
            let idx = c.read_cp_index()?;
            AttributeData::SourceFile {
                sourcefile_index: idx,
            }
        }
        "Code" => AttributeData::Code(parse_code_attribute(&mut c)?),
        "LineNumberTable" => {
            let len = c.read_u16()?;
            let entries: Vec<_> = (0..len)
                .map(|_| {
                    Ok(LineNumberEntry {
                        start_pc: c.read_u16()?,
                        line_number: c.read_u16()?,
                    })
                })
                .collect::<ParseResult<_>>()?;
            AttributeData::LineNumberTable(entries)
        }
        "LocalVariableTable" => {
            let len = c.read_u16()?;
            let entries: Vec<_> = (0..len)
                .map(|_| {
                    Ok(LocalVariableEntry {
                        start_pc: c.read_u16()?,
                        length: c.read_u16()?,
                        name_index: c.read_cp_index()?,
                        descriptor_index: c.read_cp_index()?,
                        index: c.read_u16()?,
                    })
                })
                .collect::<ParseResult<_>>()?;
            AttributeData::LocalVariableTable(entries)
        }
        "Exceptions" => {
            let num = c.read_u16()?;
            let table: Vec<_> = (0..num)
                .map(|_| c.read_cp_index())
                .collect::<ParseResult<_>>()?;
            AttributeData::Exceptions {
                exception_index_table: table,
            }
        }
        "BootstrapMethods" => {
            let num = c.read_u16()?;
            let entries: Vec<_> = (0..num)
                .map(|_| {
                    let method_ref = c.read_cp_index()?;
                    let num_args = c.read_u16()?;
                    let arguments: Vec<_> = (0..num_args)
                        .map(|_| c.read_cp_index())
                        .collect::<ParseResult<_>>()?;
                    Ok(BootstrapMethodEntry {
                        method_ref,
                        arguments,
                    })
                })
                .collect::<ParseResult<_>>()?;
            AttributeData::BootstrapMethods(entries)
        }
        _ => AttributeData::Raw(raw.to_vec()),
    };
    Ok(data)
}

fn parse_code_attribute(c: &mut Cursor<'_>) -> ParseResult<CodeAttribute> {
    let max_stack = c.read_u16()?;
    let max_locals = c.read_u16()?;
    let code_len = c.read_u32()? as usize;
    let code = c.read_bytes(code_len)?.to_vec();

    let ex_count = c.read_u16()?;
    let exception_table: Vec<_> = (0..ex_count)
        .map(|_| {
            Ok(ExceptionTableEntry {
                start_pc: c.read_u16()?,
                end_pc: c.read_u16()?,
                handler_pc: c.read_u16()?,
                catch_type: c.read_cp_index()?,
            })
        })
        .collect::<ParseResult<_>>()?;

    // Code sub-attributes (LineNumberTable etc.) — stored as Raw for now;
    // resolve_attributes will decode them.
    let attr_count = c.read_u16()?;
    let attributes: Vec<_> = (0..attr_count)
        .map(|_| {
            let name_index = c.read_cp_index()?;
            let attr_len = c.read_u32()? as usize;
            let raw = c.read_bytes(attr_len)?.to_vec();
            Ok(AttributeInfo {
                name_index,
                data: AttributeData::Raw(raw),
            })
        })
        .collect::<ParseResult<_>>()?;

    Ok(CodeAttribute {
        max_stack,
        max_locals,
        code,
        exception_table,
        attributes,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Look up a UTF-8 string in the constant pool.
pub(crate) fn cp_utf8(pool: &[Option<CpEntry>], idx: CpIndex) -> ParseResult<&str> {
    let i = idx.0 as usize;
    if i == 0 {
        return Err(ParseError::CpIndexZero);
    }
    match pool.get(i) {
        Some(None) => Err(ParseError::CpPhantomSlot { index: idx.0 }),
        Some(Some(CpEntry::Utf8(s))) => Ok(s.as_str()),
        None | Some(Some(_)) => Err(ParseError::CpIndexOutOfBounds {
            index: idx.0,
            pool_size: pool.len(),
        }),
    }
}
