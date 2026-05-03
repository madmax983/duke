with open("crates/duke-classfile/src/parser.rs", "r") as f:
    code = f.read()

# 1. Update `decode_runtime_visible_annotations`
old1 = """fn decode_runtime_visible_annotations(c: &mut Cursor<'_>) -> Result<Vec<Annotation>> {
    let num_annotations = c.read_u16()? as usize;
    let mut annotations = Vec::with_capacity(num_annotations.min(c.remaining() / 4));
    for _ in 0..num_annotations {
        annotations.push(decode_annotation(c)?);
    }
    Ok(annotations)
}"""

new1 = """fn decode_runtime_visible_annotations(c: &mut Cursor<'_>) -> Result<Vec<Annotation>> {
    let num_annotations = c.read_u16()? as usize;
    let mut annotations = Vec::with_capacity(num_annotations.min(c.remaining() / 4));
    for _ in 0..num_annotations {
        annotations.push(decode_annotation(c, &mut 0)?);
    }
    Ok(annotations)
}"""
code = code.replace(old1, new1)

# 2. Update `decode_annotation`
old2 = """fn decode_annotation(c: &mut Cursor<'_>) -> Result<Annotation> {
    let type_index = c.read_cp_index()?;
    let num_pairs = c.read_u16()? as usize;
    let mut element_value_pairs = Vec::with_capacity(num_pairs.min(c.remaining() / 3));
    for _ in 0..num_pairs {
        element_value_pairs.push(ElementValuePair {
            element_name_index: c.read_cp_index()?,
            value: decode_element_value(c)?,
        });
    }
    Ok(Annotation {
        type_index,
        element_value_pairs,
    })
}"""

new2 = """fn decode_annotation(c: &mut Cursor<'_>, depth: &mut usize) -> Result<Annotation> {
    *depth += 1;
    if *depth > 16 {
        return Err(Error::RecursionLimitExceeded { offset: c.position() });
    }
    let type_index = c.read_cp_index()?;
    let num_pairs = c.read_u16()? as usize;
    let mut element_value_pairs = Vec::with_capacity(num_pairs.min(c.remaining() / 3));
    for _ in 0..num_pairs {
        element_value_pairs.push(ElementValuePair {
            element_name_index: c.read_cp_index()?,
            value: decode_element_value(c, depth)?,
        });
    }
    *depth -= 1;
    Ok(Annotation {
        type_index,
        element_value_pairs,
    })
}"""
code = code.replace(old2, new2)

# 3. Update `decode_element_value`
old3 = """fn decode_element_value(c: &mut Cursor<'_>) -> Result<ElementValue> {
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
        b'@' => Ok(ElementValue::AnnotationValue(decode_annotation(c)?)),
        b'[' => {
            let num_values = c.read_u16()? as usize;
            let mut values = Vec::with_capacity(num_values.min(c.remaining() / 3));
            for _ in 0..num_values {
                values.push(decode_element_value(c)?);
            }
            Ok(ElementValue::ArrayValue(values))
        }
        _ => Err(Error::InvalidAnnotationElementValueTag { tag }),
    }
}"""

new3 = """fn decode_element_value(c: &mut Cursor<'_>, depth: &mut usize) -> Result<ElementValue> {
    *depth += 1;
    if *depth > 16 {
        return Err(Error::RecursionLimitExceeded { offset: c.position() });
    }
    let tag = c.read_u8()?;
    let res = match tag {
        b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's' => {
            Ok(ElementValue::ConstValueIndex(c.read_cp_index()?))
        }
        b'e' => Ok(ElementValue::EnumConstValue {
            type_name_index: c.read_cp_index()?,
            const_name_index: c.read_cp_index()?,
        }),
        b'c' => Ok(ElementValue::ClassInfoIndex(c.read_cp_index()?)),
        b'@' => Ok(ElementValue::AnnotationValue(decode_annotation(c, depth)?)),
        b'[' => {
            let num_values = c.read_u16()? as usize;
            let mut values = Vec::with_capacity(num_values.min(c.remaining() / 3));
            for _ in 0..num_values {
                values.push(decode_element_value(c, depth)?);
            }
            Ok(ElementValue::ArrayValue(values))
        }
        _ => Err(Error::InvalidAnnotationElementValueTag { tag }),
    };
    *depth -= 1;
    res
}"""
code = code.replace(old3, new3)


# Fix the one caller of `decode_element_value` in `AnnotationDefault` parsing:
code = code.replace("AttributeData::AnnotationDefault(decode_element_value(&mut c)?)", "AttributeData::AnnotationDefault(decode_element_value(&mut c, &mut 0)?)")


with open("crates/duke-classfile/src/parser.rs", "w") as f:
    f.write(code)
