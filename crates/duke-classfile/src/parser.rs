//! `duke-classfile::parser` — JVM `.class` file parser implementation

use crate::{
    access_flags::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags},
    attributes::{
        Annotation, AttributeData, AttributeInfo, BootstrapMethodEntry, CodeAttribute,
        ElementValue, ElementValuePair, ExceptionTableEntry, LineNumberEntry, LocalVariableEntry,
    },
    class::{ClassFile, FieldInfo, MethodInfo},
    constant_pool::{CpEntry, CpIndex},
    error::{Error, Result},
};

const MAGIC: u32 = 0xCAFE_BABE;
/// Java SE 21 = class file version 65.
const MAX_MAJOR_VERSION: u16 = 65;

// ---------------------------------------------------------------------------
// Cursor
// ---------------------------------------------------------------------------

/// Byte-level cursor over an immutable slice.
///
/// Every read operation checks bounds and returns `Error::UnexpectedEof`
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

    const fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Calculates a safe pre-allocation capacity that will not exceed the remaining
    /// available bytes in the buffer, preventing OOM attacks from maliciously large counts.
    const fn safe_capacity(&self, requested: usize, bytes_per_item: usize) -> usize {
        let max_possible = self.remaining() / bytes_per_item;
        if requested < max_possible {
            requested
        } else {
            max_possible
        }
    }

    fn read_u8(&mut self) -> Result<u8> {
        if self.pos >= self.data.len() {
            return Err(Error::UnexpectedEof { offset: self.pos });
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_u16(&mut self) -> Result<u16> {
        let hi = u16::from(self.read_u8()?);
        let lo = u16::from(self.read_u8()?);
        Ok((hi << 8) | lo)
    }

    #[allow(dead_code)]
    fn read_i16(&mut self) -> Result<i16> {
        Ok(self.read_u16()?.cast_signed())
    }

    fn read_u32(&mut self) -> Result<u32> {
        let hi = u32::from(self.read_u16()?);
        let lo = u32::from(self.read_u16()?);
        Ok((hi << 16) | lo)
    }

    fn read_i32(&mut self) -> Result<i32> {
        Ok(self.read_u32()?.cast_signed())
    }

    fn read_u64(&mut self) -> Result<u64> {
        let hi = u64::from(self.read_u32()?);
        let lo = u64::from(self.read_u32()?);
        Ok((hi << 32) | lo)
    }

    fn read_i64(&mut self) -> Result<i64> {
        Ok(self.read_u64()?.cast_signed())
    }

    fn read_f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.read_u32()?))
    }

    fn read_f64(&mut self) -> Result<f64> {
        Ok(f64::from_bits(self.read_u64()?))
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.pos + len > self.data.len() {
            return Err(Error::UnexpectedEof { offset: self.pos });
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    fn read_cp_index(&mut self) -> Result<CpIndex> {
        Ok(CpIndex(self.read_u16()?))
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse a JVM `.class` file from raw bytes.
///
/// Returns a fully-parsed [`ClassFile`] or a [`Error`] describing
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
pub fn parse(bytes: &[u8]) -> Result<ClassFile> {
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

fn parse_class_members(
    c: &mut Cursor<'_>,
    minor_version: u16,
    major_version: u16,
    constant_pool: Vec<Option<CpEntry>>,
) -> Result<ClassFile> {
    let cp_len = constant_pool.len(); // for bounds validation helpers

    let access_flags = ClassAccessFlags::from_bits_truncate(c.read_u16()?);
    let this_class = c.read_cp_index()?;
    let super_class = c.read_cp_index()?;

    // interfaces
    let interfaces_count = c.read_u16()?;
    let mut interfaces = Vec::with_capacity(c.safe_capacity(interfaces_count as usize, 2));
    for _ in 0..interfaces_count {
        interfaces.push(c.read_cp_index()?);
    }

    // fields
    let fields_count = c.read_u16()?;
    let mut fields = Vec::with_capacity(c.safe_capacity(fields_count as usize, 8));
    for _ in 0..fields_count {
        fields.push(parse_field(c, cp_len)?);
    }

    // methods
    let methods_count = c.read_u16()?;
    let mut methods = Vec::with_capacity(c.safe_capacity(methods_count as usize, 8));
    for _ in 0..methods_count {
        methods.push(parse_method(c, cp_len)?);
    }

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

fn parse_class_file(c: &mut Cursor<'_>) -> Result<ClassFile> {
    // §4.1 — magic
    let magic = c.read_u32()?;
    if magic != MAGIC {
        return Err(Error::BadMagic { got: magic });
    }

    let minor_version = c.read_u16()?;
    let major_version = c.read_u16()?;
    if major_version > MAX_MAJOR_VERSION {
        return Err(Error::UnsupportedVersion {
            major: major_version,
            minor: minor_version,
        });
    }

    let constant_pool = parse_constant_pool(c)?;
    parse_class_members(c, minor_version, major_version, constant_pool)
}

// ---------------------------------------------------------------------------
// Constant pool (§4.4)
// ---------------------------------------------------------------------------

fn parse_cp_entry(c: &mut Cursor<'_>, tag: u8, i: usize) -> Result<CpEntry> {
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
}

fn parse_constant_pool(c: &mut Cursor<'_>) -> Result<Vec<Option<CpEntry>>> {
    let count = c.read_u16()? as usize;
    // Index 0 is unused; spec uses 1-based indexing.
    // `count` is one more than the actual number of entries.
    let mut pool: Vec<Option<CpEntry>> = Vec::with_capacity(count.min(1 + c.remaining()));
    pool.push(None); // slot 0 — reserved

    let mut i = 1usize;
    while i < count {
        let tag = c.read_u8()?;
        let entry = parse_cp_entry(c, tag, i)?;
        let is_double_slot = matches!(entry, CpEntry::Long(_) | CpEntry::Double(_));
        pool.push(Some(entry));
        if is_double_slot {
            pool.push(None); // phantom slot
            i += 2;
        } else {
            i += 1;
        }
    }

    Ok(pool)
}

// ---------------------------------------------------------------------------
// Fields and methods
// ---------------------------------------------------------------------------

fn parse_field(c: &mut Cursor<'_>, cp_len: usize) -> Result<FieldInfo> {
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

fn parse_method(c: &mut Cursor<'_>, cp_len: usize) -> Result<MethodInfo> {
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

fn parse_attributes(c: &mut Cursor<'_>, cp_len: usize) -> Result<Vec<AttributeInfo>> {
    let count = c.read_u16()?;
    let mut attributes = Vec::with_capacity(c.safe_capacity(count as usize, 6));
    for _ in 0..count {
        attributes.push(parse_attribute(c, cp_len)?);
    }
    Ok(attributes)
}

fn parse_attribute(c: &mut Cursor<'_>, _cp_len: usize) -> Result<AttributeInfo> {
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
/// ⚡ Bolt: Pre-allocates vectors for known attribute table sizes to eliminate intermediate heap allocations.
pub fn resolve_attributes(attrs: &mut [AttributeInfo], pool: &[Option<CpEntry>]) -> Result<()> {
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

fn decode_known_attribute(name: &str, raw: &[u8]) -> Result<AttributeData> {
    let mut c = Cursor::new(raw);
    let data = match name {
        "ConstantValue" => AttributeData::ConstantValue {
            constant_value_index: decode_constant_value(&mut c)?,
        },
        "SourceFile" => AttributeData::SourceFile {
            sourcefile_index: decode_source_file(&mut c)?,
        },
        "Code" => AttributeData::Code(parse_code_attribute(&mut c)?),
        "LineNumberTable" => AttributeData::LineNumberTable(decode_line_number_table(&mut c)?),
        "LocalVariableTable" => {
            AttributeData::LocalVariableTable(decode_local_variable_table(&mut c)?)
        }
        "Exceptions" => AttributeData::Exceptions {
            exception_index_table: decode_exceptions(&mut c)?,
        },
        "BootstrapMethods" => AttributeData::BootstrapMethods(decode_bootstrap_methods(&mut c)?),
        "RuntimeVisibleAnnotations" => {
            AttributeData::RuntimeVisibleAnnotations(decode_runtime_visible_annotations(&mut c)?)
        }
        "AnnotationDefault" => AttributeData::AnnotationDefault(decode_element_value(&mut c, 0)?),
        _ => AttributeData::Raw(raw.to_vec()),
    };
    Ok(data)
}

fn decode_constant_value(c: &mut Cursor<'_>) -> Result<CpIndex> {
    c.read_cp_index()
}

fn decode_source_file(c: &mut Cursor<'_>) -> Result<CpIndex> {
    c.read_cp_index()
}

fn decode_line_number_table(c: &mut Cursor<'_>) -> Result<Vec<LineNumberEntry>> {
    let len = c.read_u16()? as usize;
    let mut entries = Vec::with_capacity(c.safe_capacity(len, 4));
    for _ in 0..len {
        entries.push(LineNumberEntry {
            start_pc: c.read_u16()?,
            line_number: c.read_u16()?,
        });
    }
    Ok(entries)
}

fn decode_local_variable_table(c: &mut Cursor<'_>) -> Result<Vec<LocalVariableEntry>> {
    let len = c.read_u16()? as usize;
    let mut entries = Vec::with_capacity(c.safe_capacity(len, 10));
    for _ in 0..len {
        entries.push(LocalVariableEntry {
            start_pc: c.read_u16()?,
            length: c.read_u16()?,
            name_index: c.read_cp_index()?,
            descriptor_index: c.read_cp_index()?,
            index: c.read_u16()?,
        });
    }
    Ok(entries)
}

fn decode_exceptions(c: &mut Cursor<'_>) -> Result<Vec<CpIndex>> {
    let num = c.read_u16()? as usize;
    let mut table = Vec::with_capacity(c.safe_capacity(num, 2));
    for _ in 0..num {
        table.push(c.read_cp_index()?);
    }
    Ok(table)
}

fn decode_bootstrap_methods(c: &mut Cursor<'_>) -> Result<Vec<BootstrapMethodEntry>> {
    let num = c.read_u16()? as usize;
    let mut entries = Vec::with_capacity(c.safe_capacity(num, 4));
    for _ in 0..num {
        let method_ref = c.read_cp_index()?;
        let num_args = c.read_u16()? as usize;
        let mut arguments = Vec::with_capacity(c.safe_capacity(num_args, 2));
        for _ in 0..num_args {
            arguments.push(c.read_cp_index()?);
        }
        entries.push(BootstrapMethodEntry {
            method_ref,
            arguments,
        });
    }
    Ok(entries)
}

fn decode_runtime_visible_annotations(c: &mut Cursor<'_>) -> Result<Vec<Annotation>> {
    let num_annotations = c.read_u16()? as usize;
    let mut annotations = Vec::with_capacity(c.safe_capacity(num_annotations, 4));
    for _ in 0..num_annotations {
        annotations.push(decode_annotation(c, 0)?);
    }
    Ok(annotations)
}

fn decode_annotation(c: &mut Cursor<'_>, depth: usize) -> Result<Annotation> {
    if depth > 16 {
        return Err(Error::RecursionLimitExceeded { offset: c.pos });
    }
    let type_index = c.read_cp_index()?;
    let num_pairs = c.read_u16()? as usize;
    let mut element_value_pairs = Vec::with_capacity(c.safe_capacity(num_pairs, 3));
    for _ in 0..num_pairs {
        element_value_pairs.push(ElementValuePair {
            element_name_index: c.read_cp_index()?,
            value: decode_element_value(c, depth + 1)?,
        });
    }
    Ok(Annotation {
        type_index,
        element_value_pairs,
    })
}

fn decode_element_value(c: &mut Cursor<'_>, depth: usize) -> Result<ElementValue> {
    if depth > 16 {
        return Err(Error::RecursionLimitExceeded { offset: c.pos });
    }
    let tag = c.read_u8()?;
    match tag {
        b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's' => {
            Ok(ElementValue::ConstValueIndex(c.read_cp_index()?))
        }
        b'e' => Ok(ElementValue::EnumConstValue {
            type_name_index: c.read_cp_index()?,
            const_name_index: c.read_cp_index()?,
        }),
        b'c' => Ok(ElementValue::ClassInfoIndex(c.read_cp_index()?)),
        b'@' => Ok(ElementValue::AnnotationValue(decode_annotation(
            c,
            depth + 1,
        )?)),
        b'[' => {
            let num_values = c.read_u16()? as usize;
            let mut values = Vec::with_capacity(c.safe_capacity(num_values, 3));
            for _ in 0..num_values {
                values.push(decode_element_value(c, depth + 1)?);
            }
            Ok(ElementValue::ArrayValue(values))
        }
        _ => Err(Error::InvalidAnnotationElementValueTag { tag }),
    }
}

/// ⚡ Bolt: Pre-allocates vectors for known attribute table sizes to eliminate intermediate heap allocations.
fn parse_code_attribute(c: &mut Cursor<'_>) -> Result<CodeAttribute> {
    let max_stack = c.read_u16()?;
    let max_locals = c.read_u16()?;
    let code_len = c.read_u32()? as usize;
    let code = c.read_bytes(code_len)?.to_vec();

    let ex_count = c.read_u16()? as usize;
    let mut exception_table = Vec::with_capacity(c.safe_capacity(ex_count, 8));
    for _ in 0..ex_count {
        exception_table.push(ExceptionTableEntry {
            start_pc: c.read_u16()?,
            end_pc: c.read_u16()?,
            handler_pc: c.read_u16()?,
            catch_type: c.read_cp_index()?,
        });
    }

    // Code sub-attributes (LineNumberTable etc.) — stored as Raw for now;
    // resolve_attributes will decode them.
    let attr_count = c.read_u16()? as usize;
    let mut attributes = Vec::with_capacity(c.safe_capacity(attr_count, 6));
    for _ in 0..attr_count {
        let name_index = c.read_cp_index()?;
        let attr_len = c.read_u32()? as usize;
        let raw = c.read_bytes(attr_len)?.to_vec();
        attributes.push(AttributeInfo {
            name_index,
            data: AttributeData::Raw(raw),
        });
    }

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
pub fn cp_utf8(pool: &[Option<CpEntry>], idx: CpIndex) -> Result<&str> {
    let i = idx.0 as usize;
    if i == 0 {
        return Err(Error::CpIndexZero);
    }
    match pool.get(i) {
        Some(None) => Err(Error::CpPhantomSlot { index: idx.0 }),
        Some(Some(CpEntry::Utf8(s))) => Ok(s.as_str()),
        None | Some(Some(_)) => Err(Error::CpIndexOutOfBounds {
            index: idx.0,
            pool_size: pool.len(),
        }),
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_cursor_operations() {
        let data = [
            0x01, // u8
            0x02, 0x03, // u16
            0x04, 0x05, 0x06, 0x07, // u32
            0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, // u64
            0x3F, 0x80, 0x00, 0x00, // f32 (1.0)
            0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // f64 (1.0)
            0xFF, // i8/u8 for cast check
            0xFF, 0xFF, // i16
            0xFF, 0xFF, 0xFF, 0xFF, // i32
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // i64
            0x00, 0x2A, // cp_index
        ];
        let mut cursor = Cursor::new(&data);
        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.remaining(), data.len());

        assert_eq!(cursor.read_u8().unwrap(), 0x01);
        assert_eq!(cursor.read_u16().unwrap(), 0x0203);
        assert_eq!(cursor.read_u32().unwrap(), 0x0405_0607);
        assert_eq!(cursor.read_u64().unwrap(), 0x0809_0A0B_0C0D_0E0F);
        assert!((cursor.read_f32().unwrap() - 1.0).abs() < f32::EPSILON);
        assert!((cursor.read_f64().unwrap() - 1.0).abs() < f64::EPSILON);

        let _ = cursor.read_u8().unwrap(); // skip FF
        assert_eq!(cursor.read_i16().unwrap(), -1);
        assert_eq!(cursor.read_i32().unwrap(), -1);
        assert_eq!(cursor.read_i64().unwrap(), -1);
        assert_eq!(cursor.read_cp_index().unwrap().0, 42);

        // Out of bounds
        assert!(matches!(cursor.read_u8(), Err(Error::UnexpectedEof { .. })));
        assert!(matches!(
            cursor.read_bytes(1),
            Err(Error::UnexpectedEof { .. })
        ));
    }

    #[test]
    fn test_cursor_read_bytes() {
        let data = [0xAA, 0xBB, 0xCC];
        let mut cursor = Cursor::new(&data);
        assert_eq!(cursor.read_bytes(2).unwrap(), &[0xAA, 0xBB]);
        assert_eq!(cursor.position(), 2);
    }

    use super::*;

    #[test]
    fn should_return_error_when_cp_index_is_zero() {
        let pool = vec![];
        let result = cp_utf8(&pool, CpIndex(0));
        assert!(
            matches!(result, Err(Error::CpIndexZero)),
            "Expected Error::CpIndexZero, got {result:?}"
        );
    }

    #[test]
    fn should_return_error_when_cp_index_is_phantom_slot() {
        let pool = vec![None, None]; // phantom slot
        let result = cp_utf8(&pool, CpIndex(1));
        assert!(
            matches!(result, Err(Error::CpPhantomSlot { index: 1 })),
            "Expected Error::CpPhantomSlot, got {result:?}"
        );
    }

    #[test]
    fn should_return_error_when_cp_index_is_out_of_bounds_or_wrong_type() {
        let pool = vec![None, Some(CpEntry::Integer(42))];
        let result = cp_utf8(&pool, CpIndex(1));
        assert!(
            matches!(result, Err(Error::CpIndexOutOfBounds { index: 1, .. })),
            "Expected Error::CpIndexOutOfBounds for wrong type, got {result:?}"
        );

        let result = cp_utf8(&pool, CpIndex(99));
        assert!(
            matches!(result, Err(Error::CpIndexOutOfBounds { index: 99, .. })),
            "Expected Error::CpIndexOutOfBounds for missing index, got {result:?}"
        );
    }

    #[test]
    fn should_return_i16_for_cursor() {
        let mut cursor = Cursor::new(&[0xFF, 0xFE]);
        let val = cursor.read_i16().unwrap();
        assert_eq!(val, -2);
    }

    #[test]
    fn should_return_remaining_bytes_for_cursor() {
        let mut cursor = Cursor::new(&[0x01, 0x02, 0x03]);
        assert_eq!(cursor.remaining(), 3);
        cursor.read_u8().unwrap();
        assert_eq!(cursor.remaining(), 2);
    }

    #[test]
    fn should_decode_runtime_visible_annotations_with_nested_values() {
        let raw = [
            0x00, 0x01, // num_annotations
            0x00, 0x02, // annotation.type_index
            0x00, 0x03, // num_element_value_pairs
            0x00, 0x04, // pair #1 name index
            b's', 0x00, 0x05, // const string value index
            0x00, 0x06, // pair #2 name index
            b'@', 0x00, 0x07, 0x00, 0x00, // nested annotation with no pairs
            0x00, 0x08, // pair #3 name index
            b'[', 0x00, 0x02, // array[2]
            b'c', 0x00, 0x09, // class literal
            b'e', 0x00, 0x0A, 0x00, 0x0B, // enum const
        ];

        let decoded = decode_known_attribute("RuntimeVisibleAnnotations", &raw).unwrap();
        let AttributeData::RuntimeVisibleAnnotations(annotations) = decoded else {
            panic!("expected RuntimeVisibleAnnotations");
        };
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].type_index, CpIndex(2));
        assert_eq!(annotations[0].element_value_pairs.len(), 3);
    }

    #[test]
    fn should_decode_annotation_default_value() {
        let raw = [
            b'[', 0x00, 0x02, // array[2]
            b'I', 0x00, 0x01, // int const
            b's', 0x00, 0x02, // string const
        ];

        let decoded = decode_known_attribute("AnnotationDefault", &raw).unwrap();
        let AttributeData::AnnotationDefault(ElementValue::ArrayValue(values)) = decoded else {
            panic!("expected AnnotationDefault array value");
        };
        assert_eq!(values.len(), 2);
    }
}
