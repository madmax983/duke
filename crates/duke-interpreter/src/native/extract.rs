fn extract_slot_arg(args: &[Slot], idx: usize) -> Slot {
    args.get(idx).copied().unwrap_or(Slot::Reference(None))
}
#[inline]
fn extract_ref_arg(args: &[Slot], idx: usize) -> Result<u64> {
    match args.get(idx) {
        Some(Slot::Reference(Some(r))) => Ok(*r),
        _ => Err(Error::NullPointerException),
    }
}
#[inline]
fn extract_field_arg(heap: &duke_gc::Heap, obj_ref: u64, idx: usize) -> Result<Slot> {
    Ok(heap.get(obj_ref)?.fields.get(idx).copied().unwrap_or(Slot::Reference(None)))
}
#[inline]
fn extract_first_field_arg(heap: &duke_gc::Heap, obj_ref: u64) -> Result<Slot> {
    extract_field_arg(heap, obj_ref, 0)
}
#[inline]
fn extract_io_fd(heap: &duke_gc::Heap, obj_ref: u64) -> Result<i32> {
    match heap.get(obj_ref)?.fields.first() {
        Some(Slot::Int(id)) => Ok(*id),
        _ => {
            Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            })
        }
    }
}
#[inline]
fn extract_io_fd_at(heap: &duke_gc::Heap, obj_ref: u64, idx: usize) -> Result<i32> {
    match heap.get(obj_ref)?.fields.get(idx) {
        Some(Slot::Int(id)) => Ok(*id),
        _ => {
            Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            })
        }
    }
}
#[inline]
fn extract_int_arg(args: &[Slot], idx: usize) -> Result<i32> {
    match args.get(idx) {
        Some(Slot::Int(v)) => Ok(*v),
        _ => {
            Err(Error::TypeMismatch {
                expected: "Int",
                got: "other",
            })
        }
    }
}
#[inline]
fn extract_long_arg(args: &[Slot], idx: usize) -> Result<i64> {
    match args.get(idx) {
        Some(Slot::Long(v)) => Ok(*v),
        _ => {
            Err(Error::TypeMismatch {
                expected: "Long",
                got: "other",
            })
        }
    }
}
#[inline]
fn extract_float_arg(args: &[Slot], idx: usize) -> Result<f32> {
    match args.get(idx) {
        Some(Slot::Float(v)) => Ok(*v),
        _ => {
            Err(Error::TypeMismatch {
                expected: "Float",
                got: "other",
            })
        }
    }
}
#[inline]
fn extract_double_arg(args: &[Slot], idx: usize) -> Result<f64> {
    match args.get(idx) {
        Some(Slot::Double(v)) => Ok(*v),
        _ => {
            Err(Error::TypeMismatch {
                expected: "Double",
                got: "other",
            })
        }
    }
}
fn extract_string_arg_value(
    args: &[Slot],
    index: usize,
    heap: &duke_gc::Heap,
) -> Result<String> {
    let str_ref = extract_ref_arg(args, index)?;
    Ok(heap.get(str_ref)?.string_value.clone().unwrap_or_default())
}
fn extract_parse_radix_arg(args: &[Slot], index: usize) -> Result<u32> {
    let radix = extract_int_arg(args, index)?;
    if !(2..=36).contains(&radix) {
        return Err(Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        });
    }
    Ok(u32::try_from(radix).unwrap_or(0))
}
